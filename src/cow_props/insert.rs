//! 新属性插入：按 property context 路由定位目标 area，再在私有副本里做 trie 插入。

use std::cell::RefCell;
use std::collections::HashMap;

use log::{info, warn};
use prop_rs_android::mmap_prop_area::PROP_INFO_LONG_FLAG;

use crate::prop_backend::PropBackend;

use super::area::{open_area, readonly_area, writable_area_for_mapping};
use super::mappings::PropAreaMapping;
use super::symbols::PROP_VALUE_MAX;

// ── 前缀 → area 路径缓存（per-thread，首次遍历后记住正确的 area）──────────

thread_local! {
    // HashMap::new 在此 toolchain 上非 const fn，无法按 clippy 建议包成 const。
    #[allow(clippy::missing_const_for_thread_local)]
    static PREFIX_AREA_CACHE: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

// ── 新增属性：COW trie 插入 ───────────────────────────────────────────────

/// SIBLING_PROBES：sys_prop 不可用时的降级定位手段，用已有属性按前缀猜测 area。
/// 不能作为主路径：猜测结果与 property_contexts 的真实路由经常不一致。
const SIBLING_PROBES: &[(&str, &[&str])] = &[
    (
        "ro.product",
        &["ro.product.model", "ro.product.device", "ro.product.brand"],
    ),
    ("ro.build", &["ro.build.display.id", "ro.build.fingerprint"]),
    ("ro.vendor", &["ro.vendor.build.fingerprint"]),
    ("ro.hardware", &["ro.hardware"]),
    ("persist", &["persist.sys.timezone"]),
    ("ro", &["ro.build.id", "ro.product.model"]),
];

/// sys_prop 一次性初始化（幂等，返回是否可用）。
pub(super) fn sys_prop_available() -> bool {
    static OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OK.get_or_init(|| {
        if let Err(e) = prop_rs_android::sys_prop::init() {
            warn!("sys_prop::init failed: {e}, context-routed insert disabled");
            false
        } else {
            true
        }
    })
}

/// 按 property context 路由解析新属性的目标 prop_area 路径。
///
/// bionic 的属性读取（`SystemProperties.get` / `__system_property_find`）按
/// property_contexts 规则将 key 路由到特定 context 的 area，新属性必须插入
/// 路由目标 area 才能被读到。SIBLING_PROBES 按前缀猜测的 area 与真实路由
/// 经常不一致（如 `ro.product.odm.*` exact 规则路由到 build_odm_prop 而
/// sibling 探测返回 build_prop；无匹配规则的 key 落到 default_prop）。
pub(super) fn context_area_path(key: &str) -> Option<String> {
    if !sys_prop_available() {
        return None;
    }
    match prop_rs_android::sys_prop::area_path(key) {
        Ok(path) => {
            let path = path.to_string_lossy().into_owned();
            if path.starts_with("/dev/__properties__/") {
                Some(path)
            } else {
                None
            }
        }
        Err(e) => {
            info!("context area lookup failed for '{key}': {e}");
            None
        }
    }
}

/// 尝试在可写属性区（私有副本 / 降级原地 COW）中为不存在的属性插入新 trie 节点。
///
/// 目标 area 优先按 property context 路由解析（与 bionic 读取路径一致），
/// 仅插入路由目标 area；sys_prop 不可用时降级为 SIBLING_PROBES 前缀探测。
pub(super) fn cow_patch_new(
    key: &str,
    value: &str,
    mappings: &[PropAreaMapping],
    backend: Option<&PropBackend>,
) -> anyhow::Result<bool> {
    let key_prefix = match key.rfind('.') {
        Some(end) => &key[..end],
        None => key,
    };

    // 1. context 路由优先：只插入 bionic 读取时实际查询的 area。
    //    路由目标 area 未映射在本进程时放弃插入（交给 companion 兜底），
    //    避免插错 area 造成“插入成功但读取不可见”。
    let target_paths: Vec<String> = if let Some(path) = context_area_path(key) {
        if mappings.iter().any(|m| m.path == path) {
            vec![path]
        } else {
            info!("COW trie: context area {path} for '{key}' not mapped, leaving to companion");
            return Ok(false);
        }
    } else {
        // 2. sys_prop 不可用的降级路径：检查 prefix → area 路径缓存
        let cached_paths = PREFIX_AREA_CACHE.with(|c| c.borrow().get(key_prefix).cloned());

        if let Some(paths) = cached_paths {
            // 缓存命中
            paths
        } else {
            // 3. 缓存未命中，遍历 build 相关 area 用 MmapPropArea::find 找包含 sibling 的 area
            let probes: &[&str] = SIBLING_PROBES
                .iter()
                .find(|(pfx, _)| key_prefix == *pfx || key_prefix.starts_with(&format!("{pfx}.")))
                .map(|(_, p)| *p)
                .unwrap_or(&["ro.product.model", "ro.build.id"]);

            let mut found_paths = Vec::new();
            for mapping in mappings {
                if !mapping.path.starts_with("/dev/__properties__/") {
                    continue;
                }
                if !mapping.path.contains("build_prop")
                    && !mapping.path.contains("build_odm_prop")
                    && !mapping.path.contains("build_vendor_prop")
                    && !mapping.path.contains("default_prop")
                {
                    continue;
                }
                if mapping.end - mapping.start < 128 {
                    continue;
                }
                // 只读探测即可（真正需要插入时才建副本）
                let Some(mut area) = readonly_area(mapping) else {
                    continue;
                };
                let has_sibling = probes.iter().any(|p| matches!(area.find(p), Ok(Some(_))));
                if has_sibling {
                    found_paths.push(mapping.path.clone());
                }
            }
            PREFIX_AREA_CACHE.with(|c| {
                c.borrow_mut()
                    .insert(key_prefix.to_string(), found_paths.clone());
            });
            found_paths
        }
    };

    if target_paths.is_empty() {
        return Ok(false);
    }

    // 在所有匹配的 area 里 emplace（确保 bionic 无论读哪个 area 都能拿到）
    let mut any_inserted = false;
    for path in &target_paths {
        let mapping = match mappings.iter().find(|m| &m.path == path) {
            Some(m) => m,
            None => continue,
        };

        let Ok((addr, size)) = writable_area_for_mapping(mapping, mappings, backend) else {
            continue;
        };
        let Ok(mut area) = open_area(addr, size) else {
            continue;
        };

        if let Ok(Some(_)) = area.find(key) {
            continue;
        }

        match area.emplace(key, value.as_bytes(), 0) {
            Ok(()) => {
                if let Ok(Some(data_off)) = area.find(key) {
                    let serial = area.read_serial(data_off);
                    // long prop 的 serial 长度字段是固定 legacy 值
                    // （LONG_LEGACY_ERROR），只验证 long 标志；inline prop
                    // 验证长度字段与值一致。
                    let verified = if serial & PROP_INFO_LONG_FLAG != 0 {
                        value.len() >= PROP_VALUE_MAX
                    } else {
                        (serial >> 24) as usize == value.len()
                    };
                    if verified {
                        info!(
                            "COW trie: inserted '{key}' (serial_ok, len={}) into {}",
                            value.len(),
                            mapping.path
                        );
                        any_inserted = true;
                    }
                }
            }
            Err(e) => {
                warn!(
                    "COW trie: emplace failed for '{key}' in {}: {e}",
                    mapping.path
                );
            }
        }
    }

    Ok(any_inserted)
}
