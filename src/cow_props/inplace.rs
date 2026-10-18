//! 降级路径：原地 COW remap（`PropBackend::resolve` 失败时）与 `hide_maps` 卸载。

use std::cell::RefCell;

use log::info;

use super::mappings::PropAreaMapping;

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

// ── 降级路径：原地 COW remap ───────────────────────────────────────────────

/// 确保 `prop_ptr` 所在的 `/dev/__properties__/*` 映射已被原地 COW remap。
///
/// ⚠️ 这条路径会让该映射在 `/proc/self/maps` 里从 `r--s` 变成 `rw-p`，
/// 仅在后端解析失败（`PropBackend::resolve` 返回 Err）时使用。
pub(super) fn ensure_prop_area_private(
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

/// 清空本线程的原地 COW 范围缓存（一次 Apply 结束后调用）。
pub(super) fn clear_ranges() {
    COW_RANGES.with(|r| r.borrow_mut().clear());
}
