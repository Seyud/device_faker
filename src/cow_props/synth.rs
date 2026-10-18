//! 不可读 context 的进程内合成属性区（S4/S5）。
//!
//! 背景见 `AGENTS.md`「不可读属性区：进程内合成区 + `context_` 改写」。
//!
//! 有些属性区文件对 app 域不可读（如 HyperOS 的 `vendor_default_prop`），bionic 惰性
//! 映射失败 ⇒ 节点 `prop_area_` 永远 NULL ⇒ `SystemProperties.get` 恒空。对**用户显式
//! 写出**（`custom_props`）的这类 key：合成一块空 prop_area、写入键值、挂到路由到该
//! context 的节点上，并把节点的 `context_` 换成 app 可读的 context 名，使 bionic
//! specialize 后的 SELinux 复查不再回收它。

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use log::{info, warn};
use prop_rs_android::mmap_prop_area::MmapPropArea;

use crate::prop_backend::PropBackend;

use super::anon_name::name_anon_vma;
use super::area::readonly_area;
use super::insert::context_area_path;
use super::mappings::{PropAreaMapping, collect_prop_area_mappings, path_is_hidden};
use super::relabel::readable_context_label;
use super::symbols::{FnSystemPropertyFind, sys_prop_find};

// ── 不可读 context 的进程内合成属性区（S4）────────────────────────────────
//
// 背景：有些属性区的**文件**对 app 域不可读（SELinux 拒绝），例如 HyperOS 上
// `ro.vendor.*` 路由到的 `vendor_default_prop`。bionic 惰性 mmap 该区时
// `ContextNode::prop_area_` 永远是 NULL ⇒ `__system_property_find` 返回 NULL，
// `SystemProperties.get` 恒为空串。
//
// 这种「存在性」检测（如 Maple 的 `oplus_property_consistency`：品牌属 OPPO
// 家族时要求 `ro.vendor.oplus.regionmark` 非空）靠任何**写真实区**的方案都过不了：
// companion 的全局 resetprop 确实写进了文件（root 可读），但 app 进程依旧读不到；
// 严格 COW 也会因为「路由区未映射」直接跳过。
//
// 修法：对**用户显式要求**（`custom_props`）且本进程读不到的 key，在进程内
// 合成一个与 stock 同构的属性区（128B 头 + trie，匿名映射），`emplace` 键值后
// 把路由到该区的那个 context 节点指过去。于是 `SystemProperties.get`、
// `__system_property_find`、`__system_property_foreach` 在本进程内都能读到，
// 而真实属性区文件、全局属性、SELinux 策略一概不动。

/// 一个进程内合成的属性区（无真实文件来源）。
struct SynthArea {
    /// 被合成取代的 context 路由路径（`/dev/__properties__/...`）
    path: String,
    start: usize,
    end: usize,
}

thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static SYNTH_AREAS: RefCell<Vec<SynthArea>> = const { RefCell::new(Vec::new()) };
}

/// 对「路由区在本进程不可读」的显式属性合成进程内匿名属性区。
///
/// - 只有 `explicit`（`custom_props` 里逐字写出的 key）参与：模块自动展开的
///   分区变体不合成，避免在 `__system_property_foreach` 里造出真机没有的幽灵属性。
/// - `hide_patterns`（`hide_maps` 配置）命中的区跳过：用户显式要求隐藏的映射
///   不应被合成区"复活"。
/// - 任一环节失败都只是降级（记 WARN），不影响既有 COW / companion 行为。
///
/// 返回成功变得可读的属性个数。
pub fn apply_unreadable_ctx_synth(
    prop_map: &HashMap<String, String>,
    explicit: &HashSet<String>,
    hide_patterns: &[String],
) -> anyhow::Result<usize> {
    if explicit.is_empty() || prop_map.is_empty() {
        return Ok(0);
    }
    let Some(find_fn) = sys_prop_find() else {
        return Ok(0);
    };

    // 诊断：hook 时刻本进程的 SELinux 域。zygote 域 ⇒ 此刻仍能读 app 不可读的属性区，
    // 因此「此刻 find 得到」不能作为「app 可读」的判据。
    if let Ok(dom) = std::fs::read_to_string("/proc/self/attr/current") {
        info!(
            "unreadable-context synth: hook domain = {dom}, pid={}",
            std::process::id()
        );
    }

    // 1) 预热：让 bionic 对每个显式 key 做一次惰性映射尝试。之后 maps 快照才能
    //    区分「可读但尚未映射」与「本进程永远映射不了（SELinux 拒绝）」。
    for key in explicit.iter() {
        if !prop_map.contains_key(key) {
            continue;
        }
        if let Ok(ckey) = std::ffi::CString::new(key.as_str()) {
            unsafe { find_fn(ckey.as_ptr()) };
        }
    }

    // 2) 候选：显式 key && 本进程 find 不到 && 路由区未映射 && 未被 hide_maps 命中
    let mappings = collect_prop_area_mappings();
    let mut groups: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for (key, value) in prop_map {
        if !explicit.contains(key) {
            continue;
        }
        if find_key(find_fn, key).is_some() {
            continue; // 本进程已可读：正常路径（已有属性/私有副本）已覆盖
        }
        let Some(path) = context_area_path(key) else {
            continue;
        };
        if path_is_hidden(&path, hide_patterns) {
            info!("unreadable-context synth: skip '{key}' ({path} hidden by hide_maps)");
            continue;
        }
        if mappings.iter().any(|m| m.path == path) {
            log::debug!(
                "unreadable-context synth: '{key}' routed area mapped ({path}), normal path"
            );
            continue; // 区已映射进本进程 ⇒ 交给常规 COW 插入
        }
        match groups.iter_mut().find(|(p, _)| *p == path) {
            Some((_, items)) => items.push((key.clone(), value.clone())),
            None => groups.push((path, vec![(key.clone(), value.clone())])),
        }
    }

    if groups.is_empty() {
        return Ok(0);
    }
    info!(
        "unreadable-context synth: {} context(s), {} explicit prop(s) route to app-unreadable areas",
        groups.len(),
        groups.iter().map(|(_, items)| items.len()).sum::<usize>()
    );

    // 3) 解析 bionic 后端（拿 context 节点数组）。失败则整体降级。
    let backend = match PropBackend::resolve_cached() {
        Ok(b) => b,
        Err(e) => {
            warn!("unreadable-context synth: prop backend resolve failed: {e:#}");
            return Ok(0);
        }
    };

    // 4) 模板区：只用来取 pa_size（stock 属性区尺寸），取最大的已映射属地区。
    let Some(template) = pick_synth_template(&mappings) else {
        warn!("unreadable-context synth: no usable template prop area mapped");
        return Ok(0);
    };
    let Some(template_area) = readonly_area(template) else {
        warn!("unreadable-context synth: template area unusable");
        return Ok(0);
    };

    let mut total = 0usize;
    for (path, items) in &groups {
        match synth_and_attach(find_fn, backend, &template_area, template, path, items) {
            Ok(n) => total += n,
            Err(e) => warn!("unreadable-context synth failed for {path}: {e:#}"),
        }
    }
    if total > 0 {
        SYNTH_AREAS.with(|c| {
            for a in c.borrow().iter() {
                info!(
                    "unreadable-context synth area: {path} [{start:#x}-{end:#x}]",
                    path = a.path,
                    start = a.start,
                    end = a.end
                );
            }
        });
        info!("unreadable-context synth: {total} explicit prop(s) made readable in-process");
    }
    Ok(total)
}

/// 调一次 `__system_property_find`，返回非空指针（0 = 找不到）。
fn find_key(find_fn: FnSystemPropertyFind, key: &str) -> Option<usize> {
    let ckey = std::ffi::CString::new(key).ok()?;
    let p = unsafe { find_fn(ckey.as_ptr()) } as usize;
    if p == 0 { None } else { Some(p) }
}

/// 选一个已映射的属性区当合成区模板（只要 pa_size）。
///
/// 排除 `properties_serial`（serial 专用布局）与 `property_info`（不是 prop_area，
/// `MmapPropArea::new` 会因 magic/version 校验失败）。
fn pick_synth_template(mappings: &[PropAreaMapping]) -> Option<&PropAreaMapping> {
    mappings
        .iter()
        .filter(|m| m.end - m.start >= 0x1000)
        .filter(|m| !m.path.ends_with("/properties_serial"))
        .filter(|m| !m.path.ends_with("/property_info"))
        .max_by_key(|m| m.end - m.start)
}

/// 为一个 context 路径合成属性区、写入键值、挂到路由到它的 context 节点上。
fn synth_and_attach(
    find_fn: FnSystemPropertyFind,
    backend: &PropBackend,
    template_area: &MmapPropArea,
    template: &PropAreaMapping,
    path: &str,
    items: &[(String, String)],
) -> anyhow::Result<usize> {
    let size = template.end - template.start;

    // 4.1 造一个空属性区（magic/version/bytes_used 已初始化，trie 根为空）
    let mut area =
        MmapPropArea::new_anon_from(template_area).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let start = area.as_ptr() as usize;

    // 4.2 灌入键值（inline / long 由 emplace 内部处理）
    let mut inserted: Vec<String> = Vec::new();
    for (key, value) in items {
        match area.emplace(key, value.as_bytes(), 0) {
            Ok(()) => match area.find(key) {
                Ok(Some(_)) => inserted.push(key.clone()),
                other => {
                    warn!("unreadable-context synth: '{key}' not found after emplace ({other:?})")
                }
            },
            Err(e) => warn!("unreadable-context synth: emplace '{key}' failed: {e:?}"),
        }
    }
    if inserted.is_empty() {
        // area 随 MmapMut drop 自动 munmap
        anyhow::bail!("no property could be emplaced into synthesized area");
    }

    // 4.3 找路由到这个 context 的 NULL 节点：逐个挂上合成区并用 find 验证。
    //     bionic 的负结果不缓存（每次 find 都会重试惰性映射），所以逐节点试是安全的。
    let probe = std::ffi::CString::new(inserted[0].as_str())?;
    let mut attached: Option<usize> = None;
    for idx in 0..backend.node_count() {
        if backend.node_area(idx) != 0 {
            continue;
        }
        backend.set_node_area(idx, start);
        let got = unsafe { find_fn(probe.as_ptr()) } as usize;
        if got >= start && got < start + size {
            attached = Some(idx);
            break;
        }
        backend.set_node_area(idx, 0);
    }

    let Some(idx) = attached else {
        anyhow::bail!("no NULL context node routes '{}' ({path})", inserted[0]);
    };
    // bionic specialize 后会按 `ContextNode::context_` 做 SELinux 可读性复查，
    // 不可读就 munmap 掉 `prop_area_`（真机实测会连我们的合成区一起回收）。
    // 改成可读 context 名后复查通过，合成区得以存活；路由仍由 property_info 决定。
    backend.set_node_context(idx, readable_context_label());

    // 4.4 泄漏映射（context 节点长期持有），命名成中性 VMA 后 seal 成只读。
    //     命名必须在仍可写时做（RO 后 PR_SET 会变成 [anon:<fault>]）。
    std::mem::forget(area);
    name_anon_vma(start as *mut libc::c_void, size);
    let _ = unsafe { libc::mprotect(start as *mut libc::c_void, size, libc::PROT_READ) };

    SYNTH_AREAS.with(|c| {
        c.borrow_mut().push(SynthArea {
            path: path.to_string(),
            start,
            end: start + size,
        })
    });

    info!(
        "unreadable-context synth: {} prop(s) -> context node #{idx} @{start:#x} size={size:#x} ({path}) pid={}",
        inserted.len(),
        std::process::id()
    );
    Ok(inserted.len())
}
