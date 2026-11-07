//! 预渲染的 per-package TOML 配置（全内存，不落盘）。
//!
//! Zygisk Next 1.5.3 起，模块的 `pre_app_specialize` 运行在 zygote 域，读 `/data/adb`
//! （`adb_data_file`）会被 SELinux 拒绝。为不改动系统策略，改由 companion（root 域）
//! 把用户配置**按包预合并**成 TOML 文本放在内存里，模块每次启动通过 companion IPC
//! 取自己那份；不产生任何文件、不需要任何 sepolicy 规则。
//!
//! 设计要点：
//! - companion 持有 `HashMap<pkg, TOML>`，`config.toml` 的 mtime 变化时重渲染；
//! - 重渲染由 inotify 事件驱动（不轮询）；
//! - 模块侧只做一次 `GetSpec{keys}` 往返 + 一次 TOML 解析，无磁盘 IO。

use crate::config::{Config, MergedAppConfig, NumberSetting};
use log::{info, warn};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

/// 已渲染的 spec 表：包名 → TOML 文本。
static SPECS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
/// 最近一次渲染依据的 config.toml mtime。
static LAST_RENDER: Mutex<Option<SystemTime>> = Mutex::new(None);
static WATCHER: OnceLock<()> = OnceLock::new();

/// 按顺序返回第一个命中的 spec 文本（`pkg@uid` → 裸包名）。
pub fn lookup_first(keys: &[String]) -> Option<String> {
    let guard = SPECS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let table = guard.as_ref()?;
    keys.iter().find_map(|key| table.get(key).cloned())
}

fn push_opt(table: &mut toml::map::Map<String, toml::Value>, key: &str, value: &Option<String>) {
    if let Some(value) = value {
        table.insert(key.to_string(), toml::Value::String(value.clone()));
    }
}

fn app_table(merged: &MergedAppConfig, package: &str) -> toml::Value {
    let mut app = toml::map::Map::new();
    app.insert(
        "package".to_string(),
        toml::Value::String(package.to_string()),
    );

    push_opt(&mut app, "manufacturer", &merged.manufacturer);
    push_opt(&mut app, "brand", &merged.brand);
    push_opt(&mut app, "marketname", &merged.marketname);
    push_opt(&mut app, "model", &merged.model);
    push_opt(&mut app, "name", &merged.name);
    push_opt(&mut app, "device", &merged.device);
    push_opt(&mut app, "product", &merged.product);
    push_opt(&mut app, "hardware", &merged.hardware);
    push_opt(&mut app, "board", &merged.board);
    push_opt(&mut app, "soc_model", &merged.soc_model);
    push_opt(&mut app, "fingerprint", &merged.fingerprint);
    push_opt(&mut app, "build_id", &merged.build_id);
    push_opt(&mut app, "display_id", &merged.display_id);
    push_opt(&mut app, "incremental", &merged.incremental);
    push_opt(&mut app, "security_patch", &merged.security_patch);
    push_opt(&mut app, "characteristics", &merged.characteristics);
    push_opt(&mut app, "android_version", &merged.android_version);

    match &merged.sdk_int {
        Some(NumberSetting::Value(value)) => {
            app.insert(
                "sdk_int".to_string(),
                toml::Value::Integer(i64::from(*value)),
            );
        }
        Some(NumberSetting::Delete) => {
            app.insert(
                "sdk_int".to_string(),
                toml::Value::String("__DELETE__".to_string()),
            );
        }
        None => {}
    }

    if let Some(dpi) = merged.dpi {
        app.insert("dpi".to_string(), toml::Value::Integer(i64::from(dpi)));
    }

    if let Some(props) = &merged.custom_props {
        let mut table = toml::map::Map::new();
        for (key, value) in props {
            // 派生键（指纹解析出的 ro.build.type/tags）不写进 spec：模块解析 spec 后
            // 会再用 fingerprint 重新派生一次，写成显式条目会改变 patch-only 门控语义。
            if merged.derived_props.contains(key) {
                continue;
            }
            table.insert(key.clone(), toml::Value::String(value.clone()));
        }
        if !table.is_empty() {
            app.insert("custom_props".to_string(), toml::Value::Table(table));
        }
    }

    app.insert(
        "force_denylist_unmount".to_string(),
        toml::Value::Boolean(merged.force_denylist_unmount),
    );
    app.insert(
        "hide_maps".to_string(),
        toml::Value::Array(
            merged
                .hide_maps
                .iter()
                .map(|value| toml::Value::String(value.clone()))
                .collect(),
        ),
    );
    app.insert(
        "companion_resetprop".to_string(),
        toml::Value::Boolean(merged.companion_resetprop),
    );

    toml::Value::Table(app)
}

fn render_doc(config: &Config, merged: &MergedAppConfig, package: &str) -> String {
    let mut root = toml::map::Map::new();
    root.insert("debug".to_string(), toml::Value::Boolean(config.debug));
    root.insert(
        "default_force_denylist_unmount".to_string(),
        toml::Value::Boolean(config.default_force_denylist_unmount),
    );
    root.insert(
        "default_hide_maps".to_string(),
        toml::Value::Array(
            config
                .default_hide_maps
                .iter()
                .map(|value| toml::Value::String(value.clone()))
                .collect(),
        ),
    );
    root.insert(
        "apps".to_string(),
        toml::Value::Array(vec![app_table(merged, package)]),
    );
    toml::to_string(&toml::Value::Table(root)).unwrap_or_default()
}

/// 把所有配置包渲染成 `pkg → TOML` 的内存表。
fn render_map(config: &Config) -> HashMap<String, String> {
    let mut packages: Vec<String> = config.apps.iter().map(|app| app.package.clone()).collect();
    for template in config.templates.values() {
        packages.extend(template.packages.iter().cloned());
    }
    packages.sort();
    packages.dedup();

    let mut table = HashMap::with_capacity(packages.len());
    for package in packages {
        if let Some(merged) = config.get_merged_config(&package) {
            table.insert(package.clone(), render_doc(config, &merged, &package));
        }
    }
    table
}

/// companion 侧调用：config.toml 的 mtime 变了才重新渲染内存表。
pub fn maybe_render() {
    let Ok(metadata) = std::fs::metadata(crate::CONFIG_PATH) else {
        return;
    };
    let Ok(mtime) = metadata.modified() else {
        return;
    };
    {
        let mut last = LAST_RENDER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if last.as_ref() == Some(&mtime) {
            return;
        }
        *last = Some(mtime);
    }

    match crate::load_config() {
        Ok(Some(config)) => {
            let table = render_map(&config);
            let count = table.len();
            let mut guard = SPECS
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = Some(table);
            info!("Rendered {count} per-package spec(s) in memory");
        }
        Ok(None) => {}
        Err(err) => {
            warn!("Spec render skipped, config load failed: {err:#}");
        }
    }
}

/// companion 侧调用一次：起一个 inotify 监听线程，配置文件被写入/替换时立刻重渲染。
/// 纯事件驱动，不做周期轮询。
pub fn ensure_watcher() {
    WATCHER.get_or_init(|| {
        std::thread::spawn(|| {
            #[cfg(target_os = "android")]
            watch_config_events();
        });
    });
}

#[cfg(target_os = "android")]
fn watch_config_events() {
    use std::os::fd::FromRawFd;

    let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
    if fd < 0 {
        warn!("inotify_init1 failed: {}", std::io::Error::last_os_error());
        return;
    }
    let _file = unsafe { std::fs::File::from_raw_fd(fd) };

    let mask = libc::IN_CLOSE_WRITE
        | libc::IN_MOVED_TO
        | libc::IN_MODIFY
        | libc::IN_ATTRIB
        | libc::IN_CREATE
        | libc::IN_DELETE;
    let mut watched = false;
    for dir in [
        "/data/adb/device_faker/config",
        "/data/adb/device_faker",
        "/data/adb",
    ] {
        let Ok(path) = std::ffi::CString::new(dir) else {
            continue;
        };
        if unsafe { libc::inotify_add_watch(fd, path.as_ptr(), mask) } >= 0 {
            watched = true;
            break;
        }
    }
    if !watched {
        warn!("inotify_add_watch failed, config hot reload disabled");
        return;
    }

    let mut buf = [0u8; 4096];
    loop {
        let read = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if read <= 0 {
            break;
        }
        maybe_render();
    }
}
