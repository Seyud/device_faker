//! 已有属性的 COW patch（Phase 1 + cross-area）。

use log::{info, warn};
use prop_rs_android::mmap_prop_area::MmapPropArea;

use crate::prop_backend::PropBackend;

use super::area::{
    open_area, original_area_start, readonly_area, writable_area_for, writable_area_for_mapping,
};
use super::mappings::{PropAreaMapping, is_build_area};
use super::symbols::FnSystemPropertyFind;

pub(super) fn cow_patch_existing(
    find_fn: FnSystemPropertyFind,
    key: &str,
    value: &str,
    mappings: &[PropAreaMapping],
    mut serial_pa: Option<&mut MmapPropArea>,
    backend: Option<&PropBackend>,
) -> anyhow::Result<bool> {
    let ckey =
        std::ffi::CString::new(key).map_err(|_| anyhow::anyhow!("invalid property name: {key}"))?;
    let prop_ptr = unsafe { find_fn(ckey.as_ptr()) };
    if prop_ptr.is_null() {
        return Ok(false);
    }

    // 该属性所属的**原始**属性区（prop_ptr 可能已落在私有副本里）。
    let primary_orig = original_area_start(prop_ptr as usize, mappings);

    // ── Phase 1: patch __system_property_find 返回的区（即其私有副本）──
    let Ok((area_addr, area_size)) = writable_area_for(prop_ptr as usize, mappings, backend) else {
        return Ok(false);
    };
    let mut area = open_area(area_addr, area_size)?;

    let data_off = match area.find(key)? {
        Some(off) => off,
        None => {
            info!(
                "COW Phase1: '{key}' MmapPropArea::find returned None in area @{area_addr:#x} \
                 (prop_ptr@{pp:#x})",
                pp = prop_ptr as usize
            );
            return Ok(false);
        }
    };

    info!(
        "COW Phase1: '{key}' found at offset={data_off:#x} in area @{area_addr:#x}, prop_ptr@{pp:#x}",
        pp = prop_ptr as usize
    );

    let pa = serial_pa
        .as_deref_mut()
        .ok_or_else(|| anyhow::anyhow!("serial area not available"))?;
    // ksu_props ≥ ddb6ee7：`update()` 原地支持 ≥ PROP_VALUE_MAX 的长值——内部重新
    // 分配 long buffer 并改写 prop_info 的 long offset，prop_info 地址与 trie 布局
    // 不变（旧 buffer 内容保留，并发读者不会读到撕裂值）。
    // 不再需要 remove+emplace：那样会丢弃原 prop_info 节点、改变 trie 布局，
    // 且缓存了 prop_info* 的读法会读到墓碑。
    // 返回的 need_rebuild 表示旧 long buffer 成为孤儿；副本随进程消亡，无需 rebuild。
    let mut need_rebuild = false;
    area.update(data_off, value, pa, &mut need_rebuild)
        .map_err(|e| anyhow::anyhow!("COW Phase1: update '{key}' failed: {e}"))?;

    // ── Phase 2: 扫描其他 build area，patch bionic prefix routing 可能命中的区域 ──
    // OnePlus/OPPO 设备上 __system_property_find 返回 build_prop 指针，但 bionic 的
    // __system_property_get 按 prefix routing 读 build_odm_prop。需要 patch 所有包含
    // 该属性的 build area。
    let mut cross_patched = 0usize;

    for mapping in mappings {
        if !is_build_area(&mapping.path) {
            continue;
        }
        // 跳过 Phase 1 已 patch 的区
        if primary_orig == Some(mapping.start) {
            continue;
        }
        if mapping.end - mapping.start < 128 {
            continue;
        }
        // 先在**真实映射**上只读确认这个区确实含该 key —— 只有需要 patch 才值得
        // 建私有副本（副本会让该区在本进程内冻结成快照）。
        let Some(mut probe) = readonly_area(mapping) else {
            continue;
        };
        match probe.find(key) {
            Ok(Some(_)) => {}
            Ok(None) => {
                log::debug!("COW cross-area: '{key}' not found in {p}", p = mapping.path);
                continue;
            }
            Err(e) => {
                info!(
                    "COW cross-area: '{key}' find error in {p}: {e}",
                    p = mapping.path
                );
                continue;
            }
        }

        let (maddr, msize) = match writable_area_for_mapping(mapping, mappings, backend) {
            Ok(v) => v,
            Err(e) => {
                log::debug!(
                    "COW cross-area: skip {p} (no writable area: {e})",
                    p = mapping.path
                );
                continue;
            }
        };
        let mut cross_area = match open_area(maddr, msize) {
            Ok(a) => a,
            Err(e) => {
                info!(
                    "COW cross-area: skip {p} (MmapPropArea::new failed: {e})",
                    p = mapping.path
                );
                continue;
            }
        };
        match cross_area.find(key) {
            Ok(Some(off)) => {
                if let Some(pa) = serial_pa.as_deref_mut() {
                    // 同 Phase 1：长值由 update() 原地处理，无需 remove+emplace
                    let mut need_rebuild = false;
                    match cross_area.update(off, value, pa, &mut need_rebuild) {
                        Ok(()) => {
                            cross_patched += 1;
                            info!("COW cross-area: '{key}' patched in {p}", p = mapping.path);
                        }
                        Err(e) => {
                            warn!(
                                "COW cross-area: update '{key}' failed in {p}: {e}",
                                p = mapping.path
                            );
                        }
                    }
                }
            }
            Ok(None) => {
                info!(
                    "COW cross-area: '{key}' lost in {p} clone",
                    p = mapping.path
                );
            }
            Err(e) => {
                info!(
                    "COW cross-area: '{key}' find error in {p}: {e}",
                    p = mapping.path
                );
            }
        }
    }

    if cross_patched > 0 {
        info!(
            "COW cross-area: '{key}' patched in {n} additional area(s)",
            n = cross_patched
        );
    }

    Ok(true)
}
