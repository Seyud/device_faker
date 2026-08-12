#[cfg(target_os = "android")]
mod atexit;
mod companion;
mod config;
mod cow_props;
mod cpu_spoof;
#[cfg(target_os = "android")]
mod file_logger;
mod hooks;

use companion::{handle_companion_request, spoof_system_props_via_companion};
use config::Config;
use cpu_spoof::{apply_cpu_spoof, apply_cpu_spoof_unmount};
use hooks::hook_build_fields;
use jni::{EnvUnowned, errors::ThrowRuntimeExAndDefault};
use log::{LevelFilter, error, info};
use std::collections::HashMap;
use zygisk_api::{
    ZygiskModule,
    api::{V4, ZygiskApi, v4::ZygiskOption},
    raw::ZygiskRaw,
};

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

        let config = match load_config(api) {
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
            // 即使应用未在 config 中，仍可能因其他 spoofed app 的 bind mount 泄漏而受影响。
            // 主动卸载可能存在的 /proc/cpuinfo bind mount，确保本应用 namespace 干净。
            // 内部 spoof_active_flag_exists() 检查决定是否走完整路径，
            // 无 spoofed app 活跃时仅 1 次 access syscall (~13μs)。
            // 不使用 config.has_any_cpu_spoof() 短路：config 热加载删除 cpu_spoof
            // 配置后，仍活跃的 spoofed app 会因配置态判据脱节导致泄漏未清理。
            if let Err(e) = apply_cpu_spoof_unmount(api, config.debug) {
                error!("CPU spoof unmount failed for unconfigured {package_name}: {e:?}");
            } else if config.debug {
                info!("CPU spoof unmount applied for unconfigured {package_name}");
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
            if !prop_map.is_empty() || !delete_props.is_empty() {
                if let Err(e) =
                    spoof_system_props_via_companion(api, &prop_map, &delete_props, &package_name)
                {
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
            // 默认路径：COW 处理，companion 只处理未找到属性和 __DELETE__
            let unfound_props = match cow_props::apply_cow_spoof(&prop_map) {
                Ok(unfound) => unfound,
                Err(e) => {
                    error!("COW spoof failed: {e:?}");
                    Vec::new()
                }
            };

            let delete_props = Config::build_delete_props_list(&merged);
            if !unfound_props.is_empty() || !delete_props.is_empty() {
                let unfound_map: HashMap<String, String> = unfound_props.into_iter().collect();
                if let Err(e) = spoof_system_props_via_companion(
                    api,
                    &unfound_map,
                    &delete_props,
                    &package_name,
                ) {
                    error!("Companion resetprop failed: {e:?}");
                } else if config.debug {
                    info!(
                        "Companion resetprop: {} new + {} delete for {package_name}",
                        unfound_map.len(),
                        delete_props.len()
                    );
                }
            }
        }

        // ④ Companion 按需：CPU spoof
        //    - 配置了 cpu_spoof: 走原有 apply_cpu_spoof 流程
        //    - 未配置 cpu_spoof: 主动卸载可能泄漏到本应用 namespace 的 /proc/cpuinfo
        //      bind mount（内部 spoof_active_flag_exists() 决定是否走完整路径）
        if merged.cpuinfo_content.is_some() {
            if let Err(e) = apply_cpu_spoof(api, &merged, &package_name, config.debug) {
                error!("CPU spoof failed: {e:?}");
            } else if config.debug {
                info!("CPU spoof applied for {package_name}");
            }
        } else {
            if let Err(e) = apply_cpu_spoof_unmount(api, config.debug) {
                error!("CPU spoof unmount failed: {e:?}");
            } else if config.debug {
                info!("CPU spoof unmount applied for {package_name}");
            }
        }

        // ⑤ DlClose（始终执行）
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

fn load_config(api: &mut ZygiskApi<V4>) -> anyhow::Result<Option<Config>> {
    // specialize 阶段运行在 zygote domain，部分 SELinux 策略不允许其遍历 /data/adb。
    // 文件由 root companion 读取，解析与应用匹配仍保留在当前进程。
    let Some(config_content) = companion::load_config_via_companion(api)? else {
        return Ok(None);
    };

    let config = Config::from_toml(&config_content)?;
    Ok(Some(config))
}

fn configure_log_level(debug_enabled: bool) {
    let level = if debug_enabled {
        LevelFilter::Info
    } else {
        LevelFilter::Error
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
