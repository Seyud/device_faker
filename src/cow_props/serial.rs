//! `properties_serial` 的进程私有载体。
//!
//! `MmapPropArea::update()` 需要一个可写的 `serial_pa` 来 bump 全局 serial；
//! 用匿名区承载这次 bump，真实 `/dev/__properties__/properties_serial` 保持 `r--s`。

use prop_rs_android::mmap_prop_area::MmapPropArea;

use super::area::open_area;
use super::inplace::ensure_prop_area_private;
use super::mappings::PropAreaMapping;

// ── Serial area ──────────────────────────────────────────────────────────

/// 准备 `MmapPropArea::update()` 需要的 `serial_pa`。
///
/// - `private = true`（私有副本路径）：用一个**进程私有匿名区**承载 serial bump，
///   真实 `/dev/__properties__/properties_serial` 映射保持 `r--s`（不再原地 COW）。
///   bump 只影响本进程，与原先「COW remap 后 bump 落在私有副本」语义一致；
///   进程内 `__system_property_wait_any` 读到的仍是真实全局 serial，
///   不会因为我们的写入被伪唤醒。
/// - 降级：沿用原地 COW remap。
pub(super) fn prepare_serial_area(
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
