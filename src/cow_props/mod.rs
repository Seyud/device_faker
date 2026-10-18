//! COW 属性伪造引擎（按职责拆分的模块化实现）。
//!
//! 原单文件 `cow_props.rs`（1400+ 行）拆成子模块：
//!
//! | 模块 | 职责 |
//! | --- | --- |
//! | [`symbols`] | bionic 符号解析（`__system_property_find`） |
//! | [`mappings`] | `/proc/self/maps` 属性区映射收集与路径判定 |
//! | [`area`] | 可写属性区：私有副本建立、迁移 context 节点、seal |
//! | [`anon_name`] | 匿名 VMA 命名（`linker_alloc`，含内核能力探测） |
//! | [`serial`] | `properties_serial` 私有载体 |
//! | [`inplace`] | 降级路径（原地 COW remap）与 `hide_maps` 卸载 |
//! | [`patch`] | 已有属性 patch（Phase 1 + cross-area） |
//! | [`insert`] | 新属性 trie 插入（按 property context 路由） |
//! | [`relabel`] | 不可读 context 的 `ContextNode::context_` 改写 |
//! | [`synth`] | 不可读 context 的进程内合成属性区（S4/S5） |
//!
//! 对外只暴露 [`apply_cow_spoof`]、[`apply_unreadable_ctx_synth`]、[`unmap_prop_areas`]。

mod anon_name;
mod area;
mod inplace;
mod insert;
mod mappings;
mod patch;
mod relabel;
mod serial;
mod symbols;
mod synth;

pub use self::inplace::unmap_prop_areas;
pub use self::synth::apply_unreadable_ctx_synth;

use std::collections::{HashMap, HashSet};

use log::{info, warn};

use crate::prop_backend::PropBackend;

use self::{
    area::seal_area_clones,
    insert::{context_area_path, cow_patch_new, sys_prop_available},
    mappings::collect_prop_area_mappings,
    patch::cow_patch_existing,
    serial::prepare_serial_area,
    symbols::sys_prop_find,
};

// ── 入口 ───────────────────────────────────────────────────────────────────

/// 对目标进程的所有属性应用 COW 伪造。
///
/// - 已有属性：COW remap + 原地 patch
/// - 不存在属性：在对应 prop_area 的 COW 映射中插入 trie 节点 + prop_info
///   （例外：`config::is_patch_only_key` 认定的「本机可能不存在」的自动展开变体
///   —— dlkm 分区副本、营销名族、OPPO 别名 —— 只做「已存在才补齐」，不存在时
///   跳过不插入；`explicit` 里用户 `custom_props` 显式写出的 key 不受此门控）
///
/// `explicit`：`config::build_merged_property_map_with_explicit` 返回的显式 key 集合。
///
/// 返回本进程无法落地的属性列表（area 空间不足等）。**严格 COW 不再降级
/// companion resetprop**：调用方只记录 WARN，需要全局生效时由用户显式开启
/// `companion_resetprop`（见 src/lib.rs）。
pub fn apply_cow_spoof(
    prop_map: &HashMap<String, String>,
    explicit: &HashSet<String>,
) -> anyhow::Result<Vec<(String, String)>> {
    let mut unfound: Vec<(String, String)> = Vec::new();

    if prop_map.is_empty() {
        return Ok(unfound);
    }

    let find_fn = match sys_prop_find() {
        Some(f) => f,
        None => {
            anyhow::bail!("__system_property_find not available (dlsym failed)");
        }
    };

    // 长值不预过滤：ksu_props ≥ ddb6ee7 的 `update()` 原地支持任意长度的新值
    // （≥ PROP_VALUE_MAX 时内部 allocate 新 long buffer 并改写 prop_info 的
    // long offset），inline→long、long→更长都直接成功；只有全新属性走
    // emplace(long) 插入路径。
    let filtered: Vec<(&str, &str)> = prop_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    // 预热：逐 key 调用 find()，触发 bionic 对尚未映射 context area 的
    // 惰性映射（本进程域允许的 area 在此刻完成映射；被 SELinux 拒绝的
    // context 返回 null，内核 audit 自动去重）。之后再收集映射快照，
    // 确保快照覆盖本进程全部可映射的 area。
    //
    // sys_prop 先于预热初始化：context 路由与映射快照均依赖其内部
    // PropertyContext 状态，提前初始化保证后续地址一致。
    let _ = sys_prop_available();
    for (key, _) in &filtered {
        if let Ok(ckey) = std::ffi::CString::new(*key) {
            unsafe { find_fn(ckey.as_ptr()) };
        }
    }
    let mappings = collect_prop_area_mappings();

    // 把被 patch 的属性区迁移到进程私有副本，原始 /dev/__properties__/*
    // 映射保持 r--s（消除 maps 里 rw-p 的指纹）。
    // 解析失败 → 降级回原地 COW（maps 会重新出现 rw-p），日志说明原因。
    let backend = match PropBackend::resolve_cached() {
        Ok(b) => Some(b),
        Err(e) => {
            warn!(
                "prop backend resolve failed: {e:#}; falling back to in-place COW \
                 (/dev/__properties__/* will show as rw-p in maps)"
            );
            None
        }
    };

    // 预初始化 serial area（供所有 update() 调用共享）
    let mut serial_pa = match prepare_serial_area(&mappings, backend.is_some()) {
        Ok(pa) => Some(pa),
        Err(e) => {
            warn!("Failed to prepare serial area: {e}, patches will use fallback");
            None
        }
    };

    let mut cow_patched = 0usize;
    let mut cow_inserted = 0usize;
    let mut cow_skipped = 0usize;
    let mut cow_patch_only_skipped = 0usize;

    for (key, value) in &filtered {
        // Context 路由目标 area 未映射进本进程（预热后仍未映射 = SELinux
        // 拒绝，如 build_bootimage_prop 仅 shell/update_engine 可读）⇒
        // 属性对本进程不可观察：真实值与伪装值对 app 内检测代码均为
        // unknown，无泄漏面，静默跳过（不进 unfound）。
        if let Some(path) = context_area_path(key)
            && !mappings.iter().any(|m| m.path == path)
        {
            info!("COW skip '{key}': routed area {path} unmapped in process (unobservable)");
            cow_skipped += 1;
            continue;
        }

        match cow_patch_existing(
            find_fn,
            key,
            value,
            &mappings,
            serial_pa.as_deref_mut(),
            backend,
        ) {
            Ok(true) => cow_patched += 1,
            Ok(false) => {
                // 「本机可能不存在」的 key（dlkm 分区副本、OEM 营销名族与别名）只在部分
                // ROM 上导出，且这些 key 是模块为覆盖前缀路由**自动展开**出来的。本进程
                // find() 不到 = 该 key 在此 ROM 上不存在（或本进程读不到）：此时 app 读到
                // 真值与伪装值同为 unknown，插入零收益，却会在 __system_property_foreach /
                // getprop 里留下真实设备没有的可枚举「幽灵属性」⇒ 只做「已存在才补齐」。
                //
                // 例外：用户在 custom_props 里显式写出的 key（`explicit`）是明确指令，
                // 照写不误（例如在非 OPPO 机型上伪装 OPPO 别名）。
                if crate::config::is_patch_only_key(key) && !explicit.contains(*key) {
                    info!("COW skip '{key}': absent on this ROM (patch-only key)");
                    cow_patch_only_skipped += 1;
                } else {
                    // 属性不存在 → 尝试在私有副本中插入新 trie 节点
                    match cow_patch_new(key, value, &mappings, backend) {
                        Ok(true) => {
                            if explicit.contains(*key) {
                                info!("COW insert '{key}' (explicit custom_prop)");
                            }
                            cow_inserted += 1;
                        }
                        Ok(false) => {
                            unfound.push((key.to_string(), value.to_string()));
                        }
                        Err(e) => {
                            warn!("COW insert failed for '{key}': {e}");
                            unfound.push((key.to_string(), value.to_string()));
                        }
                    }
                }
            }
            Err(e) => warn!("COW patch failed for '{key}': {e}"),
        }
    }

    // 全部 patch 完成：副本降回只读，形态与正常共享只读属性区一致。
    seal_area_clones();

    if cow_patched > 0 || cow_inserted > 0 || cow_patch_only_skipped > 0 {
        info!(
            "COW spoof: {cow_patched} patched, {cow_inserted} inserted, {cow_skipped} skipped (unobservable), {cow_patch_only_skipped} skipped (absent on ROM), {} total",
            filtered.len()
        );
    }

    self::inplace::clear_ranges();
    Ok(unfound)
}
