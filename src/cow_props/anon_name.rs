//! 匿名 VMA 命名（中性名 `linker_alloc`，含内核能力探测）。
//!
//! 名字**必须中性**：maps 里出现的任何模块相关字样（device_faker / zygisk / libdf /
//! prop-area / memfd…）本身就会变成新的指纹。用 `linker_alloc`——bionic/linker 本就
//! 大量产出 `r--p [anon:linker_alloc]`，同形易混。

use log::{info, warn};

/// 副本匿名映射的 VMA 名。**必须中性**：maps 里出现的任何模块相关字样
/// （device_faker / zygisk / libdf / prop-area / memfd …）本身就会变成新的指纹。
/// 用 linker_alloc——bionic/linker 本就大量产出 `r--p [anon:linker_alloc]`，同形易混。
pub(super) const CLONE_ANON_NAME: &std::ffi::CStr = c"linker_alloc";

/// 命名能力：0=未知，1=可用，2=不可用。
static ANON_NAME_OK: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// 尽力给副本打上中性 VMA 名；**探测失败则永久回退无名**（不 `uname`、不版本判断）。
///
/// 不能拿正式副本做试验：部分 OEM 4.x 内核上 `prctl` 看似成功，maps 却变成
/// `[anon:<fault>]`，且清名抹不掉——比 stock 无名更扎眼。先用一次性探针映射试。
pub(super) fn name_anon_vma(ptr: *mut libc::c_void, size: usize) {
    use std::sync::atomic::Ordering;
    match ANON_NAME_OK.load(Ordering::Relaxed) {
        1 => {
            set_anon_name(ptr, size);
        }
        2 => {}
        _ => {
            let ok = probe_anon_naming();
            info!("VMA anon naming probe: ok={ok}");
            if ok {
                ANON_NAME_OK.store(1, Ordering::Relaxed);
                set_anon_name(ptr, size);
            } else {
                ANON_NAME_OK.store(2, Ordering::Relaxed);
                warn!("VMA anon naming unusable on this kernel; clones stay unnamed");
            }
        }
    }
}

/// 一次性探针：临时匿名页上试命名，核对 maps；失败则丢弃，不碰真实副本。
fn probe_anon_naming() -> bool {
    let size = 0x20000usize; // 与属性区副本同尺寸（128KiB）
    let page = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if page == libc::MAP_FAILED {
        return false;
    }
    let set_ok = set_anon_name(page, size);
    let name_ok = anon_vma_name_is(page, CLONE_ANON_NAME.to_str().unwrap_or(""));
    info!("VMA anon naming probe page={page:p} set_ok={set_ok} name_ok={name_ok}");
    unsafe { libc::munmap(page, size) };
    set_ok && name_ok
}

/// 设置 VMA 名；返回 prctl 是否成功。
///
/// 名字必须放在**不随 Zygisk DlClose 卸载**的堆上：部分内核对
/// `PR_SET_VMA_ANON_NAME` 不 `strndup` 而是持用户指针，so 卸载后
/// maps 读名 fault，显示成 `[anon:<fault>]`。
pub(super) fn set_anon_name(ptr: *mut libc::c_void, size: usize) -> bool {
    let ret = unsafe {
        libc::prctl(
            libc::PR_SET_VMA,
            libc::PR_SET_VMA_ANON_NAME,
            ptr as libc::c_ulong,
            size as libc::c_ulong,
            anon_name_ptr() as libc::c_ulong,
        )
    };
    ret == 0
}

/// 进程堆上的一次性名字缓冲（故意泄漏，活到进程结束）。
fn anon_name_ptr() -> *const u8 {
    static NAME: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let p = NAME.load(std::sync::atomic::Ordering::Relaxed);
    if p != 0 {
        return p as *const u8;
    }
    let mut buf = CLONE_ANON_NAME
        .to_bytes_with_nul()
        .to_vec()
        .into_boxed_slice();
    let np = buf.as_mut_ptr() as usize;
    std::mem::forget(buf);
    NAME.store(np, std::sync::atomic::Ordering::Relaxed);
    np as *const u8
}

/// 核对 `/proc/self/maps` 里该 VMA 的名字是否等于 `want`。
pub(super) fn anon_vma_name_is(ptr: *const libc::c_void, want: &str) -> bool {
    let start = ptr as usize;
    let Ok(maps) = std::fs::read_to_string("/proc/self/maps") else {
        return false;
    };
    let prefix = format!("{start:x}-");
    for line in maps.lines() {
        if !line.starts_with(&prefix) {
            continue;
        }
        if let Some(pos) = line.rfind("[anon:") {
            let name = line[pos + 6..].trim_end_matches(']');
            return name == want;
        }
        return false;
    }
    false
}
