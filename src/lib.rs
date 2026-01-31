#[cfg(target_os = "android")]
mod atexit;
mod companion;
mod config;
mod hooks;
mod state;

use std::{fs, path::Path};

use anyhow::Context;
use companion::{
    handle_companion_request, restore_previous_resetprop_if_needed,
    spoof_system_props_via_companion,
};
use config::{Config, MergedAppConfig};
use hooks::{hook_build_fields, hook_native_property_get, hook_system_properties};
use jni::JNIEnv;
use log::{LevelFilter, error, info};
use state::{FAKE_PROPS, IS_FULL_MODE};
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

    fn on_load(&self, _api: ZygiskApi<V4>, _env: JNIEnv) {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(LevelFilter::Error)
                .with_tag("DeviceFaker"),
        );
    }

    fn pre_app_specialize(
        &self,
        mut api: ZygiskApi<V4>,
        mut env: JNIEnv,
        args: &mut <V4 as ZygiskRaw>::AppSpecializeArgs,
    ) {
        if let Err(err) = self.handle_app_specialize(&mut api, &mut env, args) {
            error!("pre_app_specialize failed: {err:?}");
        }
    }

    fn post_app_specialize(
        &self,
        mut api: ZygiskApi<V4>,
        _env: JNIEnv,
        _args: &<V4 as ZygiskRaw>::AppSpecializeArgs,
    ) {
        let is_full_mode = *IS_FULL_MODE.lock().unwrap();
        if !is_full_mode {
            api.set_option(ZygiskOption::DlCloseModuleLibrary);
        }
    }

    fn pre_server_specialize(
        &self,
        mut api: ZygiskApi<V4>,
        _env: JNIEnv,
        _args: &mut <V4 as ZygiskRaw>::ServerSpecializeArgs,
    ) {
        api.set_option(ZygiskOption::DlCloseModuleLibrary);
    }
}

impl MyModule {
    fn handle_app_specialize(
        &self,
        api: &mut ZygiskApi<V4>,
        env: &mut JNIEnv,
        args: &mut <V4 as ZygiskRaw>::AppSpecializeArgs,
    ) -> anyhow::Result<()> {
        let package_name = Self::extract_package_name(env, args)?;
        let user_id = Self::extract_android_user_id(args);
        let package_with_user = format!("{package_name}@{user_id}");
        restore_previous_resetprop_if_needed(api, &package_with_user)?;

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

        if config.debug {
            info!(
                "Using mode: {} for app: {package_name} (user {user_id})",
                merged.mode
            );
        }

        hook_build_fields(env, &merged)?;
        if config.debug {
            info!("Build fields hooked successfully");
        }

        match SpoofMode::from_mode_str(&merged.mode) {
            SpoofMode::Lite => Self::apply_lite_mode(api, config.debug),
            SpoofMode::Full => Self::apply_full_mode(api, env, &merged, config.debug),
            SpoofMode::Resetprop => {
                Self::apply_resetprop_mode(api, &package_with_user, &merged, config.debug)
            }
        }
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
        env: &mut JNIEnv,
        args: &mut <V4 as ZygiskRaw>::AppSpecializeArgs,
    ) -> anyhow::Result<String> {
        if let Ok(app_data_dir) = env.get_string(args.app_data_dir) {
            let app_data: String = app_data_dir.into();
            if let Some(package) = app_data.rsplit('/').next()
                && !package.is_empty()
            {
                return Ok(package.to_string());
            }
        }

        let mut nice_name: String = env
            .get_string(args.nice_name)
            .context("Failed to get package name")?
            .into();

        if let Some(idx) = nice_name.find(':') {
            nice_name.truncate(idx);
        }

        Ok(nice_name)
    }

    fn apply_lite_mode(api: &mut ZygiskApi<V4>, debug: bool) -> anyhow::Result<()> {
        *FAKE_PROPS.lock().unwrap() = None;
        *IS_FULL_MODE.lock().unwrap() = false;
        if debug {
            info!("Lite mode: only Build fields hooked, unloading module");
        }
        api.set_option(ZygiskOption::DlCloseModuleLibrary);
        Ok(())
    }

    fn apply_full_mode(
        api: &mut ZygiskApi<V4>,
        env: &JNIEnv,
        merged: &MergedAppConfig,
        debug: bool,
    ) -> anyhow::Result<()> {
        if debug {
            info!("Full mode: hooking SystemProperties");
        }

        let prop_map = Config::build_merged_property_map(merged);
        if debug {
            info!("Property map created with {} entries", prop_map.len());
        }

        *FAKE_PROPS.lock().unwrap() = Some(prop_map);
        *IS_FULL_MODE.lock().unwrap() = true;
        hook_system_properties(api, env)?;
        hook_native_property_get(api)?;

        if debug {
            info!("SystemProperties hooked successfully, module will stay loaded");
        }

        Ok(())
    }

    fn apply_resetprop_mode(
        api: &mut ZygiskApi<V4>,
        package_name: &str,
        merged: &MergedAppConfig,
        debug: bool,
    ) -> anyhow::Result<()> {
        if debug {
            info!("Resetprop mode: using companion process");
        }

        let prop_map = Config::build_merged_property_map_for_resetprop(merged);
        let delete_props = Config::build_delete_props_list(merged);
        spoof_system_props_via_companion(api, &prop_map, &delete_props, package_name)?;

        if debug {
            info!("Resetprop spoofing completed");
        }

        *FAKE_PROPS.lock().unwrap() = None;
        *IS_FULL_MODE.lock().unwrap() = false;
        api.set_option(ZygiskOption::DlCloseModuleLibrary);
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum SpoofMode {
    Lite,
    Full,
    Resetprop,
}

impl SpoofMode {
    fn from_mode_str(value: &str) -> Self {
        match value {
            "lite" => Self::Lite,
            "full" => Self::Full,
            "resetprop" => Self::Resetprop,
            other => {
                error!("Mode '{other}' not fully supported, falling back to 'lite' mode");
                Self::Lite
            }
        }
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
        LevelFilter::Error
    };
    log::set_max_level(level);
}

zygisk_api::register_module!(MyModule);
zygisk_api::register_companion!(handle_companion_request);
