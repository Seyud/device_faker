#[cfg(target_os = "android")]
mod atexit;
mod companion;
mod config;
mod cow_props;
#[cfg(target_os = "android")]
mod fg_observer;
#[cfg(target_os = "android")]
mod file_logger;
mod hooks;
mod prop_backend;

use std::{collections::HashMap, fs, path::Path};

use anyhow::Context;
use companion::{handle_companion_request, spoof_system_props_via_companion};
use config::Config;
use hooks::hook_build_fields;
use jni::{EnvUnowned, errors::ThrowRuntimeExAndDefault};
use log::{LevelFilter, error, info, warn};
use zygisk_api::{
    ZygiskModule,
    api::{V4, ZygiskApi, v4::ZygiskOption},
    raw::ZygiskRaw,
};

const CONFIG_PATH: &str = "/data/adb/device_faker/config/config.toml";

#[derive(Default)]
struct MyModule;

impl ZygiskModule for MyModule {
    type Api = V4;

    fn on_load(&self, _api: ZygiskApi<V4>, _env: EnvUnowned) {
        #[cfg(target_os = "android")]
        file_logger::init_buffer_only();
    }

    fn pre_app_specialize(
        &self,
        mut api: ZygiskApi<V4>,
        mut env: EnvUnowned,
        args: &mut <V4 as ZygiskRaw>::AppSpecializeArgs,
    ) {
        if let Err(err) = self.handle_app_specialize(&mut api, &mut env, args) {
            error!("pre_app_specialize failed: {err:?}");
        }
    }

    fn post_app_specialize(
        &self,
        mut api: ZygiskApi<V4>,
        _env: EnvUnowned,
        _args: &<V4 as ZygiskRaw>::AppSpecializeArgs,
    ) {
        api.set_option(ZygiskOption::DlCloseModuleLibrary);
    }

    fn pre_server_specialize(
        &self,
        mut api: ZygiskApi<V4>,
        _env: EnvUnowned,
        _args: &mut <V4 as ZygiskRaw>::ServerSpecializeArgs,
    ) {
        api.set_option(ZygiskOption::DlCloseModuleLibrary);
    }
}

impl MyModule {
    fn handle_app_specialize(
        &self,
        api: &mut ZygiskApi<V4>,
        env: &mut EnvUnowned,
        args: &mut <V4 as ZygiskRaw>::AppSpecializeArgs,
    ) -> anyhow::Result<()> {
        let result = self.do_handle_app_specialize(api, env, args);

        // 在 pre_app_specialize 退出前统一 flush，确保 on_load + specialize 期间
        // 产生的所有日志都能发给 companion 落盘。
        if let Err(e) = flush_log_buffer_to_companion(api) {
            // 这里不能用 error!，否则会产生新的日志又无法 flush。
            // 静默失败，日志将丢失。
            let _ = e;
        }

        result
    }

    fn do_handle_app_specialize(
        &self,
        api: &mut ZygiskApi<V4>,
        env: &mut EnvUnowned,
        args: &mut <V4 as ZygiskRaw>::AppSpecializeArgs,
    ) -> anyhow::Result<()> {
        let package_name = Self::extract_package_name(env, args)?;
        let user_id = Self::extract_android_user_id(args);
        let package_with_user = format!("{package_name}@{user_id}");

        // companion 侧现在自己管理会话状态和恢复逻辑；
        // Zygisk 模块侧不再需要跨应用恢复（ACTIVE_RESET_SESSION 已移除）。

        let config = match load_config() {
            Ok(Some(cfg)) => cfg,
            Ok(None) => {
                api.set_option(ZygiskOption::DlCloseModuleLibrary);
                return Ok(());
            }
            Err(err) => {
                error!("Failed to load config: {err:#}");
                api.set_option(ZygiskOption::DlCloseModuleLibrary);
                return Ok(());
            }
        };

        configure_log_level(config.debug);

        if config.debug {
            info!(
                "Config loaded with {} apps and {} templates",
                config.apps.len(),
                config.templates.len()
            );
        }

        let merged = config
            .get_merged_config(&package_with_user)
            .or_else(|| config.get_merged_config(&package_name));

        let Some(merged) = merged else {
            if config.debug {
                info!("App {package_name} (user {user_id}) not in config, unloading module");
            }
            api.set_option(ZygiskOption::DlCloseModuleLibrary);
            return Ok(());
        };

        if merged.force_denylist_unmount {
            api.set_option(ZygiskOption::ForceDenylistUnmount);
            if config.debug {
                info!("Force denylist unmount enabled for {package_name}");
            }
        }

        // ── 统一执行流（按需调度）──────────────────────────────────────────
        // ① JNI 字段覆写（始终执行）
        hook_build_fields(env, &merged)?;
        if config.debug {
            info!("Build fields faked successfully");
        }

        // ①-bis 清除 /proc/self/maps 中匹配模式的属性映射（anti-detection）
        if !merged.hide_maps.is_empty() {
            cow_props::unmap_prop_areas(&merged.hide_maps);
        }

        // ② COW 属性伪造（per-process，覆盖 native 读取，零模块驻留）
        //    companion_resetprop = true 时跳过 COW，全部交给 companion resetprop（全局生效）
        let prop_map = Config::build_merged_property_map(&merged);
        if config.debug {
            info!("Property map: {} entries", prop_map.len());
        }

        if merged.companion_resetprop {
            // 全属性走 companion resetprop（getprop 和进程内读取一致）
            let delete_props = Config::build_delete_props_list(&merged);
            if !prop_map.is_empty() || !delete_props.is_empty() || merged.dpi.is_some() {
                if let Err(e) = spoof_system_props_via_companion(
                    api,
                    &prop_map,
                    &delete_props,
                    &package_name,
                    merged.dpi,
                    config.debug,
                ) {
                    error!("Companion resetprop (full) failed: {e:?}");
                } else if config.debug {
                    info!(
                        "Companion resetprop (full): {} set + {} delete for {package_name}",
                        prop_map.len(),
                        delete_props.len()
                    );
                }
            }
        } else {
            // 严格 COW：COW 无法处理的属性不再降级为全局 resetprop——
            // 自动 fallback 会修改全局属性（getprop/其他 app 可见），破坏
            // COW 模式的 per-process 隔离承诺。需要全局生效（getprop 子进程
            // 可见、__DELETE__ 删除属性）时显式开启 companion_resetprop。
            let unfound_props = match cow_props::apply_cow_spoof(&prop_map) {
                Ok(unfound) => unfound,
                Err(e) => {
                    error!("COW spoof failed: {e:?}");
                    Vec::new()
                }
            };

            for (key, value) in &unfound_props {
                warn!(
                    "Strict COW cannot apply '{key}' ({} bytes) for {package_name}; \
                     enable companion_resetprop for global fallback",
                    value.len()
                );
            }

            let delete_props = Config::build_delete_props_list(&merged);
            for key in &delete_props {
                warn!(
                    "Strict COW cannot delete '{key}' for {package_name}; \
                     enable companion_resetprop to use __DELETE__"
                );
            }

            // dpi 与属性伪装无关（companion 经 wm density 修改全局 settings），
            // 配置了仍走 companion。
            if merged.dpi.is_some() {
                let empty: HashMap<String, String> = HashMap::new();
                if let Err(e) = spoof_system_props_via_companion(
                    api,
                    &empty,
                    &[],
                    &package_name,
                    merged.dpi,
                    config.debug,
                ) {
                    error!("Companion dpi failed: {e:?}");
                } else if config.debug {
                    info!("Companion dpi applied for {package_name}");
                }
            }
        }

        // ③ DlClose（始终执行）
        api.set_option(ZygiskOption::DlCloseModuleLibrary);
        Ok(())
    }

    fn extract_android_user_id(args: &<V4 as ZygiskRaw>::AppSpecializeArgs) -> u32 {
        // Android 的 app UID = userId * 100000 + appId
        // 这里的 userId 对应 /data/user/<userId>/... 里的数字
        const AID_USER_OFFSET: u32 = 100_000;
        let uid = *args.uid;
        if uid <= 0 {
            return 0;
        }
        (uid as u32) / AID_USER_OFFSET
    }

    fn extract_package_name(
        env: &mut EnvUnowned,
        args: &<V4 as ZygiskRaw>::AppSpecializeArgs,
    ) -> anyhow::Result<String> {
        let result: String = env
            .with_env(|_jenv| -> Result<String, jni::errors::Error> {
                let app_data_dir = args.app_data_dir.to_string();

                if let Some(package) = app_data_dir.rsplit('/').next()
                    && !package.is_empty()
                {
                    return Ok(package.to_string());
                }

                let nice_name = args.nice_name.to_string();

                let mut nice_name: String = nice_name;
                if let Some(idx) = nice_name.find(':') {
                    nice_name.truncate(idx);
                }

                Ok(nice_name)
            })
            .resolve::<ThrowRuntimeExAndDefault>();
        Ok(result)
    }
}

fn load_config() -> anyhow::Result<Option<Config>> {
    if !Path::new(CONFIG_PATH).exists() {
        return Ok(None);
    }

    let config_content = fs::read_to_string(CONFIG_PATH)
        .with_context(|| format!("Failed to read config at {CONFIG_PATH}"))?;
    let config = Config::from_toml(&config_content)?;
    Ok(Some(config))
}

fn configure_log_level(debug_enabled: bool) {
    let level = if debug_enabled {
        LevelFilter::Info
    } else {
        // 非 debug 不打印任何日志（Error 也不落盘）。
        LevelFilter::Off
    };
    log::set_max_level(level);
}

fn flush_log_buffer_to_companion(api: &mut ZygiskApi<V4>) -> anyhow::Result<()> {
    let lines = file_logger::drain_lines();
    if lines.is_empty() {
        return Ok(());
    }

    let request = companion::CompanionRequest::WriteLog(companion::WriteLogRequest { lines });
    let response = companion::send_companion_command(api, &request)?;
    if response.status != 0 {
        anyhow::bail!(
            response
                .message
                .unwrap_or_else(|| "companion write log failed".to_string())
        );
    }
    Ok(())
}

// Note: The register_module macro should handle the EnvUnowned properly
// The unwrap_unchecked issue is a macro expansion problem in jni 0.22
// We'll let the macro handle this internally
zygisk_api::register_module!(MyModule);
zygisk_api::register_companion!(handle_companion_request);
