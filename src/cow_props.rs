//! COW 属性伪造引擎。
//!
//! - 已有属性：bionic `__system_property_find()` 定位所属属性区 → 取该区的
//!   **进程私有副本** → 在副本上原地 patch
//! - 不存在属性：在同一个私有副本中用 `MmapPropArea::emplace()` 插入 trie 节点
//!   （不依赖 companion resetprop，per-process 隔离零驻留）
//!
//! 迁移点（`ContextNode.prop_area_`）由 `prop_backend` 在运行期解析得出，不硬编码偏移。
//! 解析失败时**降级回原地 COW**（maps 会重新出现 `rw-p`），并打 WARN 说明原因。
//!
//! # 实现说明
//!
//! 副本通过 `MmapPropArea`（ksu_props）操作：
//! - `transmute((ptr, len))` → `MmapMut` 构造 `MmapPropArea`（MmapMut = `{ptr, len}` on Unix）
//! - `ManuallyDrop` 防止 `MmapPropArea` drop → `MmapMut` drop → munmap（副本需保持存活）
//! - `emplace()` 内部 bump allocator 分配 trie 节点 + prop_info，Release store 发布指针

use std::{cell::RefCell, collections::HashMap};

use log::{info, warn};
use prop_rs_android::mmap_prop_area::{MmapPropArea, PROP_INFO_LONG_FLAG};

use crate::prop_backend::PropBackend;

// ── bionic 类型定义 ────────────────────────────────────────────────────────

type FnSystemPropertyFind = unsafe extern "C" fn(*const libc::c_char) -> *const libc::c_void;

const PROP_VALUE_MAX: usize = 92;

// ── COW 范围缓存（per-thread，避免重复 remap 同一区域）────────────────────

struct PropRange {
    start: usize,
    end: usize,
}

thread_local! {
    // 已用 const {} 包裹，此 nightly 的 lint 仍误报（bug），allow 压制。
    #[allow(clippy::missing_const_for_thread_local)]
    static COW_RANGES: RefCell<Vec<PropRange>> = const { RefCell::new(Vec::new()) };
}

// ── 前缀 → area 路径缓存（per-thread，首次遍历后记住正确的 area）──────────

thread_local! {
    // HashMap::new 在此 toolchain 上非 const fn，无法按 clippy 建议包成 const。
    #[allow(clippy::missing_const_for_thread_local)]
    static PREFIX_AREA_CACHE: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

// ── 属性区私有副本────────────────────────────────────────────────────

/// 一个真实属性区的进程私有副本。副本随进程存活，**不回收**
/// （bionic 的 context 节点长期指向它，提前 munmap 会悬空）。
struct AreaClone {
    /// 原始 `/dev/__properties__/*` 映射起始地址
    orig_start: usize,
    clone_start: usize,
    clone_end: usize,
}

thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static AREA_CLONES: RefCell<Vec<AreaClone>> = const { RefCell::new(Vec::new()) };
}

/// 副本 memfd 的名字。**必须中性**：maps 里出现的任何模块相关字样
/// （device_faker / zygisk / libdf …）本身就会变成新的指纹。
const CLONE_MEMFD_NAME: &std::ffi::CStr = c"prop-area";

// ── bionic 符号加载 ────────────────────────────────────────────────────────

fn sys_prop_find() -> Option<FnSystemPropertyFind> {
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"__system_property_find".as_ptr()) };
    if sym.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute::<*mut libc::c_void, FnSystemPropertyFind>(sym) })
    }
}

// ── 入口 ───────────────────────────────────────────────────────────────────

/// 对目标进程的所有属性应用 COW 伪造。
///
/// - 已有属性：COW remap + 原地 patch
/// - 不存在属性：在对应 prop_area 的 COW 映射中插入 trie 节点 + prop_info
///
/// 返回仍未能处理的属性列表（映射找不到或空间不足），供 companion resetprop 兜底。
pub fn apply_cow_spoof(
    prop_map: &HashMap<String, String>,
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
    let backend = match PropBackend::resolve() {
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
            backend.as_ref(),
        ) {
            Ok(true) => cow_patched += 1,
            Ok(false) => {
                // 属性不存在 → 尝试在私有副本中插入新 trie 节点
                match cow_patch_new(key, value, &mappings, backend.as_ref()) {
                    Ok(true) => cow_inserted += 1,
                    Ok(false) => {
                        unfound.push((key.to_string(), value.to_string()));
                    }
                    Err(e) => {
                        warn!("COW insert failed for '{key}': {e}");
                        unfound.push((key.to_string(), value.to_string()));
                    }
                }
            }
            Err(e) => warn!("COW patch failed for '{key}': {e}"),
        }
    }

    // 全部 patch 完成：副本降回只读，形态与正常共享只读属性区一致。
    seal_area_clones();

    if cow_patched > 0 || cow_inserted > 0 {
        info!(
            "COW spoof: {cow_patched} patched, {cow_inserted} inserted, {cow_skipped} skipped (unobservable), {} total",
            filtered.len()
        );
    }

    COW_RANGES.with(|r| r.borrow_mut().clear());
    Ok(unfound)
}

// ── 已有属性：COW patch ────────────────────────────────────────────────────

/// 判断路径是否为 build 相关的 prop_area。
fn is_build_area(path: &str) -> bool {
    path.contains("build_prop")
        || path.contains("build_odm_prop")
        || path.contains("build_vendor_prop")
        || path.contains("default_prop")
}

fn cow_patch_existing(
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
                info!("COW cross-area: '{key}' not found in {p}", p = mapping.path);
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
                info!(
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

/// munmap `/dev/__properties__/*` 中路径匹配指定模式的映射（`hide_maps` 配置项）。
/// 这些属性值为空，munmap 不影响任何功能。
///
/// **不会**卸载任何真实属性区映射（真实映射保留，副本另行建立）；
/// 这里只处理用户显式配置的 `hide_maps` 模式。因此被 `hide_maps` 清掉的区不会再
/// 参与后续克隆（`collect_prop_area_mappings` 已看不到它），行为与改动前一致。
pub fn unmap_prop_areas(patterns: &[String]) {
    if patterns.is_empty() {
        return;
    }

    let Ok(maps) = std::fs::read_to_string("/proc/self/maps") else {
        return;
    };

    for line in maps.lines() {
        if !line.contains("/dev/__properties__/") {
            continue;
        }
        if !patterns.iter().any(|p| line.contains(p.as_str())) {
            continue;
        }

        let mut ws = line.split_whitespace();
        let Some(range) = ws.next() else { continue };

        let Some((start_s, end_s)) = range.split_once('-') else {
            continue;
        };
        let Ok(start) = usize::from_str_radix(start_s, 16) else {
            continue;
        };
        let Ok(end) = usize::from_str_radix(end_s, 16) else {
            continue;
        };

        let size = end - start;
        let ret = unsafe { libc::munmap(start as *mut libc::c_void, size) };
        if ret == 0 {
            info!("Unmapped prop area: {range}");
        }
    }
}

// ── 映射收集 ───────────────────────────────────────────────────────────────

struct PropAreaMapping {
    start: usize,
    end: usize,
    path: String,
    offset: u64,
}

fn collect_prop_area_mappings() -> Vec<PropAreaMapping> {
    let Ok(maps) = std::fs::read_to_string("/proc/self/maps") else {
        return vec![];
    };

    let mut result = vec![];
    for line in maps.lines() {
        let mut ws = line.split_whitespace();
        let Some(range) = ws.next() else { continue };
        let Some(_perms) = ws.next() else { continue };
        let Some(off_str) = ws.next() else { continue };
        let Some(_dev) = ws.next() else { continue };
        let Some(_inode) = ws.next() else { continue };
        let Some(path) = ws.next() else { continue };

        if !path.starts_with("/dev/__properties__/") {
            continue;
        }

        let Some((start_s, end_s)) = range.split_once('-') else {
            continue;
        };
        let Ok(start) = usize::from_str_radix(start_s, 16) else {
            continue;
        };
        let Ok(end) = usize::from_str_radix(end_s, 16) else {
            continue;
        };
        let Ok(offset) = u64::from_str_radix(off_str, 16) else {
            continue;
        };

        result.push(PropAreaMapping {
            start,
            end,
            path: path.to_string(),
            offset,
        });
    }
    result
}

// ── 可写属性区：私有副本 / 降级原地 COW ─────────────────────────────────

/// 构造某个可写属性区上的 `MmapPropArea` 视图（`ManuallyDrop`：区随进程存活）。
///
/// 调用方必须保证 `addr` 指向一个内容自洽的 `prop_area`（真实
/// `/dev/__properties__/<ctx>` 映射，或本模块的逐字节副本）——`MmapPropArea::new`
/// 校验失败时会 drop 掉 `MmapMut`（即 munmap 掉这块映射，而它并不归我们所有）。
/// 现有调用点只传真实属性区与自身副本，`properties_serial` 同样是合法的 prop_area；
/// `property_info` 不是，但它被 `is_build_area` / `context_area_path` 过滤在外。
fn open_area(addr: usize, size: usize) -> anyhow::Result<std::mem::ManuallyDrop<MmapPropArea>> {
    use memmap2::MmapMut;

    let mmap_mut =
        unsafe { std::mem::transmute::<(*mut u8, usize), MmapMut>((addr as *mut u8, size)) };
    Ok(std::mem::ManuallyDrop::new(MmapPropArea::new(mmap_mut)?))
}

/// 只读探测某个真实属性区（不 COW、不克隆）。
///
/// 用于「这个区里到底有没有这个 key」的预判：先探测再决定是否值得建私有副本，
/// 避免为无关的区白白克隆（每个副本都会让该区在本进程内冻结成快照）。
fn readonly_area(mapping: &PropAreaMapping) -> Option<std::mem::ManuallyDrop<MmapPropArea>> {
    open_area(mapping.start, mapping.end - mapping.start).ok()
}

/// `addr` 所属的**原始**属性区起始地址（若 `addr` 落在私有副本里，映射回原区）。
fn original_area_start(addr: usize, mappings: &[PropAreaMapping]) -> Option<usize> {
    let from_clone = AREA_CLONES.with(|c| {
        c.borrow()
            .iter()
            .find(|cl| addr >= cl.clone_start && addr < cl.clone_end)
            .map(|cl| cl.orig_start)
    });
    from_clone.or_else(|| {
        mappings
            .iter()
            .find(|m| addr >= m.start && addr < m.end)
            .map(|m| m.start)
    })
}

/// 取包含 `addr` 的可写属性区，返回 `(区域起始地址, 大小)`。
fn writable_area_for(
    addr: usize,
    mappings: &[PropAreaMapping],
    backend: Option<&PropBackend>,
) -> anyhow::Result<(usize, usize)> {
    if let Some(range) = clone_range_containing(addr) {
        return Ok(range);
    }
    let mapping = mappings
        .iter()
        .find(|m| addr >= m.start && addr < m.end)
        .ok_or_else(|| anyhow::anyhow!("prop_info at {addr:#x} not in any prop area mapping"))?;
    writable_area_for_mapping(mapping, mappings, backend)
}

/// 取某个**原始**属性区对应的可写区（已有副本直接复用，否则新建）。
fn writable_area_for_mapping(
    mapping: &PropAreaMapping,
    mappings: &[PropAreaMapping],
    backend: Option<&PropBackend>,
) -> anyhow::Result<(usize, usize)> {
    let size = mapping.end - mapping.start;
    if let Some(range) = clone_range_of(mapping.start) {
        return Ok(range);
    }
    match backend {
        // 整块克隆 + 迁移 context 节点指针，原映射不动
        Some(b) => {
            // 同一份属性区文件在本进程里可能被映射多次（活跃/非活跃两套
            // ContextsSerialized 各映射一遍）。只有被 context 节点引用的那一份
            // 才值得克隆 —— 否则克隆出来也没有指针指向它，白 mmap/munmap 一轮。
            if !b.references_area(mapping.start) {
                anyhow::bail!(
                    "no context node references {:#x} ({path})",
                    mapping.start,
                    path = mapping.path
                );
            }
            Ok((create_area_clone(mapping, b)?, size))
        }
        // 降级：原地 COW remap（maps 里该区会变成 rw-p）
        None => {
            ensure_prop_area_private(mapping.start as *const u8, mappings)?;
            Ok((mapping.start, size))
        }
    }
}

/// 已有副本按原区起始地址复用（顺带确保可写——`seal_area_clones` 之后需要）。
fn clone_range_of(orig_start: usize) -> Option<(usize, usize)> {
    AREA_CLONES
        .with(|c| {
            c.borrow()
                .iter()
                .find(|cl| cl.orig_start == orig_start)
                .map(|cl| (cl.clone_start, cl.clone_end - cl.clone_start))
        })
        .map(|(start, size)| {
            make_writable(start, size);
            (start, size)
        })
}

/// 已有副本按地址区间复用。
fn clone_range_containing(addr: usize) -> Option<(usize, usize)> {
    AREA_CLONES
        .with(|c| {
            c.borrow()
                .iter()
                .find(|cl| addr >= cl.clone_start && addr < cl.clone_end)
                .map(|cl| (cl.clone_start, cl.clone_end - cl.clone_start))
        })
        .map(|(start, size)| {
            make_writable(start, size);
            (start, size)
        })
}

fn make_writable(addr: usize, size: usize) {
    let ret = unsafe {
        libc::mprotect(
            addr as *mut libc::c_void,
            size,
            libc::PROT_READ | libc::PROT_WRITE,
        )
    };
    if ret != 0 {
        warn!(
            "mprotect(RW) on prop area clone @{addr:#x} failed: {err}",
            err = std::io::Error::last_os_error()
        );
    }
}

/// patch 全部完成：把副本降回只读，形态与正常共享只读属性区一致（`r--s`）。
fn seal_area_clones() {
    AREA_CLONES.with(|c| {
        for cl in c.borrow().iter() {
            let size = cl.clone_end - cl.clone_start;
            let ret = unsafe {
                libc::mprotect(cl.clone_start as *mut libc::c_void, size, libc::PROT_READ)
            };
            if ret != 0 {
                warn!(
                    "mprotect(RO) on prop area clone @{:#x} failed: {err}",
                    cl.clone_start,
                    err = std::io::Error::last_os_error()
                );
            }
        }
    });
}

/// 把真实属性区整块复制到进程私有映射，并让 bionic 的 context 节点指向副本。
///
/// 原始 `/dev/__properties__/*` 映射**不做任何改动**（权限形态保持 `r--s`）。
fn create_area_clone(mapping: &PropAreaMapping, backend: &PropBackend) -> anyhow::Result<usize> {
    let size = mapping.end - mapping.start;
    let fd = unsafe { libc::memfd_create(CLONE_MEMFD_NAME.as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        anyhow::bail!(
            "memfd_create failed: {err}",
            err = std::io::Error::last_os_error()
        );
    }
    let result = build_area_clone(fd, mapping, size, backend);
    unsafe { libc::close(fd) };
    result
}

fn build_area_clone(
    fd: i32,
    mapping: &PropAreaMapping,
    size: usize,
    backend: &PropBackend,
) -> anyhow::Result<usize> {
    if unsafe { libc::ftruncate(fd, size as libc::off_t) } != 0 {
        anyhow::bail!(
            "ftruncate({size}) failed: {err}",
            err = std::io::Error::last_os_error()
        );
    }

    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        anyhow::bail!(
            "mmap clone failed: {err}",
            err = std::io::Error::last_os_error()
        );
    }

    // 整块逐字节复制（header / trie / 全部 prop_info）。副本必须与原区完全一致，
    // 否则 bionic 的 trie 遍历会读到错误节点。
    unsafe { std::ptr::copy_nonoverlapping(mapping.start as *const u8, ptr as *mut u8, size) };

    let clone = ptr as usize;
    let nodes = backend.repoint_area(mapping.start, clone);
    if nodes == 0 {
        // 没有任何 context 节点引用这个区（正常情况下已被 `references_area`
        // 预检挡掉），迁移无意义 —— 回滚，避免留下一个无人使用的私有映射。
        unsafe { libc::munmap(ptr, size) };
        anyhow::bail!("no context node references {:#x}", mapping.start);
    }

    AREA_CLONES.with(|c| {
        c.borrow_mut().push(AreaClone {
            orig_start: mapping.start,
            clone_start: clone,
            clone_end: clone + size,
        })
    });

    info!(
        "cloned prop area {path} [{start:#x}-{end:#x}] -> [{clone:#x}-{cend:#x}] ({nodes} context node(s) repointed)",
        path = mapping.path,
        start = mapping.start,
        end = mapping.end,
        cend = clone + size,
    );
    Ok(clone)
}

// ── 降级路径：原地 COW remap ───────────────────────────────────────────────

/// 确保 `prop_ptr` 所在的 `/dev/__properties__/*` 映射已被原地 COW remap。
///
/// ⚠️ 这条路径会让该映射在 `/proc/self/maps` 里从 `r--s` 变成 `rw-p`，
/// 仅在后端解析失败（`PropBackend::resolve` 返回 Err）时使用。
fn ensure_prop_area_private(
    prop_ptr: *const u8,
    mappings: &[PropAreaMapping],
) -> anyhow::Result<()> {
    let addr = prop_ptr as usize;

    // 缓存命中检查
    let cached = COW_RANGES.with(|r| {
        r.borrow()
            .iter()
            .any(|range| addr >= range.start && addr < range.end)
    });
    if cached {
        return Ok(());
    }

    // 找到包含 prop_ptr 的映射
    let mapping = mappings
        .iter()
        .find(|m| addr >= m.start && addr < m.end)
        .ok_or_else(|| {
            anyhow::anyhow!("prop_info at {addr:#x} not in any /dev/__properties__ mapping")
        })?;

    let size = mapping.end - mapping.start;

    let cpath = std::ffi::CString::new(mapping.path.as_str())
        .map_err(|_| anyhow::anyhow!("invalid path: {path}", path = mapping.path))?;
    let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        anyhow::bail!(
            "open({path}): {err}",
            path = mapping.path,
            err = std::io::Error::last_os_error()
        );
    }

    let ret = unsafe {
        libc::mmap(
            mapping.start as *mut libc::c_void,
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_FIXED,
            fd,
            mapping.offset as libc::off_t,
        )
    };
    unsafe { libc::close(fd) };

    if ret == libc::MAP_FAILED {
        anyhow::bail!(
            "mmap COW remap failed for {path}: {err}",
            path = mapping.path,
            err = std::io::Error::last_os_error()
        );
    }

    COW_RANGES.with(|r| {
        r.borrow_mut().push(PropRange {
            start: mapping.start,
            end: mapping.end,
        });
    });

    info!(
        "COW remapped {path} [{start:#x}-{end:#x}]",
        path = mapping.path,
        start = mapping.start,
        end = mapping.end
    );
    Ok(())
}

// ── Serial area ──────────────────────────────────────────────────────────

/// 准备 `MmapPropArea::update()` 需要的 `serial_pa`。
///
/// - `private = true`（私有副本路径）：用一个**进程私有匿名区**承载 serial bump，
///   真实 `/dev/__properties__/properties_serial` 映射保持 `r--s`（不再原地 COW）。
///   bump 只影响本进程，与原先「COW remap 后 bump 落在私有副本」语义一致；
///   进程内 `__system_property_wait_any` 读到的仍是真实全局 serial，
///   不会因为我们的写入被伪唤醒。
/// - 降级：沿用原地 COW remap。
fn prepare_serial_area(
    mappings: &[PropAreaMapping],
    private: bool,
) -> anyhow::Result<std::mem::ManuallyDrop<MmapPropArea>> {
    if !private {
        return cow_serial_area(mappings);
    }

    let serial_mapping = mappings
        .iter()
        .find(|m| m.path.ends_with("/properties_serial"))
        .ok_or_else(|| anyhow::anyhow!("properties_serial mapping not found"))?;

    // 只读访问原区（仅用于取 pa_size），实际写入落在匿名副本上。
    let src = open_area(
        serial_mapping.start,
        serial_mapping.end - serial_mapping.start,
    )?;
    Ok(std::mem::ManuallyDrop::new(MmapPropArea::new_anon_from(
        &src,
    )?))
}

/// 找到 `properties_serial` mapping 并 COW remap，构造 `MmapPropArea`（降级路径）。
///
/// `MmapPropArea::update()` 需要 `serial_pa` 来 bump global area serial + futex wake。
/// COW-remap 后 bump 只影响当前进程的私有副本，不会错误通知其他进程。
fn cow_serial_area(
    mappings: &[PropAreaMapping],
) -> anyhow::Result<std::mem::ManuallyDrop<MmapPropArea>> {
    use memmap2::MmapMut;

    let serial_mapping = mappings
        .iter()
        .find(|m| m.path.ends_with("/properties_serial"))
        .ok_or_else(|| anyhow::anyhow!("properties_serial mapping not found"))?;

    ensure_prop_area_private(serial_mapping.start as *const u8, mappings)?;

    let size = serial_mapping.end - serial_mapping.start;
    let ptr = serial_mapping.start as *mut u8;
    let mmap_mut = unsafe { std::mem::transmute::<(*mut u8, usize), MmapMut>((ptr, size)) };
    let area = MmapPropArea::new(mmap_mut)?;
    Ok(std::mem::ManuallyDrop::new(area))
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
fn sys_prop_available() -> bool {
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
fn context_area_path(key: &str) -> Option<String> {
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
fn cow_patch_new(
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
