//! 属性区私有副本：建立、迁移 bionic context 节点、seal。
//!
//! 副本用 `MAP_PRIVATE|MAP_ANONYMOUS` 建立并逐字节复制，seal 后形态与 stock
//! 共享只读属性区一致（`r--p [anon:linker_alloc]` 或无名）。
//! 原始 `/dev/__properties__/*` 映射**不做任何改动**。

use std::cell::RefCell;

use log::{info, warn};
use prop_rs_android::mmap_prop_area::MmapPropArea;

use crate::prop_backend::PropBackend;

use super::anon_name::{CLONE_ANON_NAME, anon_vma_name_is, name_anon_vma, set_anon_name};
use super::inplace::ensure_prop_area_private;
use super::mappings::PropAreaMapping;
use super::relabel::readable_context_label;

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

// ── 可写属性区：私有副本 / 降级原地 COW ─────────────────────────────────

/// 构造某个可写属性区上的 `MmapPropArea` 视图（`ManuallyDrop`：区随进程存活）。
///
/// 调用方必须保证 `addr` 指向一个内容自洽的 `prop_area`（真实
/// `/dev/__properties__/<ctx>` 映射，或本模块的逐字节副本）——`MmapPropArea::new`
/// 校验失败时会 drop 掉 `MmapMut`（即 munmap 掉这块映射，而它并不归我们所有）。
/// 现有调用点只传真实属性区与自身副本，`properties_serial` 同样是合法的 prop_area；
/// `property_info` 不是，但它被 `is_build_area` / `context_area_path` 过滤在外。
pub(super) fn open_area(
    addr: usize,
    size: usize,
) -> anyhow::Result<std::mem::ManuallyDrop<MmapPropArea>> {
    use memmap2::MmapMut;

    let mmap_mut =
        unsafe { std::mem::transmute::<(*mut u8, usize), MmapMut>((addr as *mut u8, size)) };
    Ok(std::mem::ManuallyDrop::new(MmapPropArea::new(mmap_mut)?))
}

/// 只读探测某个真实属性区（不 COW、不克隆）。
///
/// 用于「这个区里到底有没有这个 key」的预判：先探测再决定是否值得建私有副本，
/// 避免为无关的区白白克隆（每个副本都会让该区在本进程内冻结成快照）。
pub(super) fn readonly_area(
    mapping: &PropAreaMapping,
) -> Option<std::mem::ManuallyDrop<MmapPropArea>> {
    open_area(mapping.start, mapping.end - mapping.start).ok()
}

/// `addr` 所属的**原始**属性区起始地址（若 `addr` 落在私有副本里，映射回原区）。
pub(super) fn original_area_start(addr: usize, mappings: &[PropAreaMapping]) -> Option<usize> {
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
pub(super) fn writable_area_for(
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
pub(super) fn writable_area_for_mapping(
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

/// patch 全部完成：尽量 seal 成只读；命名与 mprotect 在部分内核上互相打架，
/// 最终形态是 `r--p [anon:linker_alloc]` 或退回 `rw-p [anon:linker_alloc]`（stock 同有）。
pub(super) fn seal_area_clones() {
    AREA_CLONES.with(|c| {
        for cl in c.borrow().iter() {
            let size = cl.clone_end - cl.clone_start;
            let addr = cl.clone_start as *mut libc::c_void;
            // 必须在仍可写时命名（RO 后 PR_SET 会变成 [anon:<fault>]）
            name_anon_vma(addr, size);
            let named_rw = anon_vma_name_is(addr, CLONE_ANON_NAME.to_str().unwrap_or(""));
            let _ = unsafe { libc::mprotect(addr, size, libc::PROT_READ) };
            let named_ro = anon_vma_name_is(addr, CLONE_ANON_NAME.to_str().unwrap_or(""));
            info!("seal clone @{addr:p} size={size:#x} named_rw={named_rw} named_ro={named_ro}");
            if !named_ro {
                let _ = unsafe { libc::mprotect(addr, size, libc::PROT_READ | libc::PROT_WRITE) };
                set_anon_name(addr, size);
                let named_rw2 = anon_vma_name_is(addr, CLONE_ANON_NAME.to_str().unwrap_or(""));
                info!("seal clone @{addr:p} fallback rw named={named_rw2}");
            }
        }
    });
}

/// 把真实属性区整块复制到进程私有匿名映射，并让 bionic 的 context 节点指向副本。
///
/// 原始 `/dev/__properties__/*` 映射**不做任何改动**（权限形态保持 `r--s`）。
/// 副本用 MAP_PRIVATE|MAP_ANONYMOUS：seal 后呈 `r--p`，再视内核能力命名成
/// `[anon:linker_alloc]`（≥5.17）或保持无名（空路径，与 stock 无名 `r--p` 同形）。
/// （不用 MAP_SHARED|ANON——那会被内核挂成 `/dev/zero (deleted)`，反而非典型；
/// 也不 bind-mount 伪路径——maple 等 mountinfo 扫描直接命中。）
pub(super) fn create_area_clone(
    mapping: &PropAreaMapping,
    backend: &PropBackend,
) -> anyhow::Result<usize> {
    let size = mapping.end - mapping.start;
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        anyhow::bail!(
            "anonymous mmap clone failed: {err}",
            err = std::io::Error::last_os_error()
        );
    }

    // 命名放在 seal 之后（部分内核 mprotect 会丢掉 VMA name）。
    build_area_clone(ptr, mapping, size, backend)
}

fn build_area_clone(
    ptr: *mut libc::c_void,
    mapping: &PropAreaMapping,
    size: usize,
    backend: &PropBackend,
) -> anyhow::Result<usize> {
    // 整块逐字节复制（header / trie / 全部 prop_info）。副本必须与原区完全一致，
    // 否则 bionic 的 trie 遍历会读到错误节点。
    unsafe { std::ptr::copy_nonoverlapping(mapping.start as *const u8, ptr as *mut u8, size) };

    let clone = ptr as usize;
    let nodes = backend.repoint_area_and_relabel(mapping.start, clone, readable_context_label());
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
