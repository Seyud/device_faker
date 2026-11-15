//! 运行时 GPU 伪装模块（OpenGL ES + Vulkan）
//!
//! - 不替换或修改系统/Vendor 任何驱动 `.so`（保留 `sq2-vulkan-policy.conf` 等路由机制）；
//! - 在 `pre_app_specialize` 中保存配置并为 Mesa 驱动预置 `force_gl_*` 环境变量；
//! - 在 `post_app_specialize` 中仅对配置了 GPU 伪装的进程保留模块并挂载 Zygisk PLT Hook +
//!   `dlopen`/`android_dlopen_ext`/`dlsym`/`eglGetProcAddress`/`vkGetInstanceProcAddr` 动态链路追踪。

use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::ResolvedGpuConfig;

static GPU_SPOOF_ACTIVE: AtomicBool = AtomicBool::new(false);

/// 当前进程是否启用了运行时 GPU 伪装（用于决定是否跳过 `DlCloseModuleLibrary`）。
pub fn is_active() -> bool {
    GPU_SPOOF_ACTIVE.load(Ordering::Acquire)
}

/// 在 `pre_app_specialize` 阶段准备 GPU 伪装状态（纯内存存储 + Mesa 环境变量）。
pub fn prepare_in_pre_specialize(cfg: &ResolvedGpuConfig, debug_log: bool) {
    #[cfg(target_os = "android")]
    android_impl::prepare(cfg, debug_log);
    #[cfg(not(target_os = "android"))]
    let _ = (cfg, debug_log);
    GPU_SPOOF_ACTIVE.store(true, Ordering::Release);
}

/// 在 `post_app_specialize` 阶段安装全局 PLT Hook；若未启用 GPU 伪装则立即返回 `false`。
#[cfg(target_os = "android")]
pub fn install_in_post_specialize(
    _api: &mut zygisk_api::api::ZygiskApi<zygisk_api::api::V4>,
) -> bool {
    if !is_active() {
        return false;
    }
    android_impl::install();
    true
}

#[cfg(not(target_os = "android"))]
pub fn install_in_post_specialize(
    _api: &mut zygisk_api::api::ZygiskApi<zygisk_api::api::V4>,
) -> bool {
    false
}

#[cfg(target_os = "android")]
mod android_impl {
    use std::ffi::{CStr, CString, c_char, c_int, c_void};
    use std::ptr::{self, NonNull};
    use std::sync::{
        Mutex, OnceLock,
        atomic::{AtomicPtr, Ordering},
    };

    use log::info;

    use crate::config::ResolvedGpuConfig;

    const GL_VENDOR: u32 = 0x1F00;
    const GL_RENDERER: u32 = 0x1F01;
    const GL_VERSION: u32 = 0x1F02;

    const VK_MAX_PHYSICAL_DEVICE_NAME_SIZE: usize = 256;

    const PT_LOAD: u32 = 1;
    const PT_DYNAMIC: u32 = 2;
    const PT_GNU_RELRO: u32 = 0x6474_e552;

    const DT_NULL: i64 = 0;
    const DT_PLTRELSZ: i64 = 2;
    const DT_STRTAB: i64 = 5;
    const DT_SYMTAB: i64 = 6;
    const DT_RELA: i64 = 7;
    const DT_RELASZ: i64 = 8;
    const DT_PLTREL: i64 = 20;
    const DT_JMPREL: i64 = 23;
    const DT_ANDROID_RELA: i64 = 0x6000_0010;
    const DT_ANDROID_RELASZ: i64 = 0x6000_0011;

    const R_AARCH64_ABS64: u32 = 257;
    const R_AARCH64_GLOB_DAT: u32 = 1025;
    const R_AARCH64_JUMP_SLOT: u32 = 1026;

    #[repr(C)]
    struct VkPhysicalDevicePropertiesPrefix {
        api_version: u32,
        driver_version: u32,
        vendor_id: u32,
        device_id: u32,
        device_type: u32,
        device_name: [u8; VK_MAX_PHYSICAL_DEVICE_NAME_SIZE],
    }

    #[repr(C)]
    struct VkPhysicalDeviceProperties2Prefix {
        s_type: u32,
        p_next: *mut c_void,
        properties: VkPhysicalDevicePropertiesPrefix,
    }

    struct ActiveGpuConfig {
        renderer: Option<CString>,
        vendor: Option<CString>,
        gl_version: Option<CString>,
        vendor_id: Option<u32>,
        device_id: Option<u32>,
        driver_version: Option<u32>,
        debug_log: bool,
    }

    static GPU_CONFIG: OnceLock<ActiveGpuConfig> = OnceLock::new();
    static GOT_PATCH_LOCK: Mutex<()> = Mutex::new(());

    static ORIG_DLOPEN: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static ORIG_ANDROID_DLOPEN_EXT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static ORIG_DLSYM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static LOADER_DLOPEN: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static LOADER_ANDROID_DLOPEN_EXT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static LOADER_DLSYM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static ORIG_GL_GET_STRING: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static ORIG_EGL_GET_PROC_ADDRESS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static ORIG_VK_GET_INSTANCE_PROC_ADDR: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static ORIG_VK_GET_DEVICE_PROC_ADDR: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static ORIG_VK_GET_PROPS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static ORIG_VK_GET_PROPS2: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static ORIG_VK_GET_PROPS2_KHR: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

    type FnDlopen = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_void;
    type FnAndroidDlopenExt =
        unsafe extern "C" fn(*const c_char, c_int, *const c_void) -> *mut c_void;
    type FnDlsym = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
    type FnLoaderDlopen = unsafe extern "C" fn(*const c_char, c_int, *const c_void) -> *mut c_void;
    type FnLoaderAndroidDlopenExt =
        unsafe extern "C" fn(*const c_char, c_int, *const c_void, *const c_void) -> *mut c_void;
    type FnLoaderDlsym =
        unsafe extern "C" fn(*mut c_void, *const c_char, *const c_void) -> *mut c_void;
    type FnGlGetString = unsafe extern "C" fn(u32) -> *const c_char;
    type FnEglGetProcAddress = unsafe extern "C" fn(*const c_char) -> *mut c_void;
    type FnVkGetInstanceProcAddr = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
    type FnVkGetDeviceProcAddr = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
    type FnVkGetPhysicalDeviceProperties =
        unsafe extern "C" fn(*mut c_void, *mut VkPhysicalDevicePropertiesPrefix);
    type FnVkGetPhysicalDeviceProperties2 =
        unsafe extern "C" fn(*mut c_void, *mut VkPhysicalDeviceProperties2Prefix);

    #[derive(Clone, Copy)]
    struct HookSpec {
        name: &'static CStr,
        replacement: *const c_void,
        orig_slot: &'static AtomicPtr<c_void>,
    }

    // HookSpec 只含静态 CStr、函数指针与全局 AtomicPtr，跨线程只读安全。
    unsafe impl Sync for HookSpec {}

    const HOOK_SPECS: &[HookSpec] = &[
        HookSpec {
            name: c"dlopen",
            replacement: hooked_dlopen as *const c_void,
            orig_slot: &ORIG_DLOPEN,
        },
        HookSpec {
            name: c"android_dlopen_ext",
            replacement: hooked_android_dlopen_ext as *const c_void,
            orig_slot: &ORIG_ANDROID_DLOPEN_EXT,
        },
        HookSpec {
            name: c"dlsym",
            replacement: hooked_dlsym as *const c_void,
            orig_slot: &ORIG_DLSYM,
        },
        HookSpec {
            name: c"glGetString",
            replacement: hooked_gl_get_string as *const c_void,
            orig_slot: &ORIG_GL_GET_STRING,
        },
        HookSpec {
            name: c"eglGetProcAddress",
            replacement: hooked_egl_get_proc_address as *const c_void,
            orig_slot: &ORIG_EGL_GET_PROC_ADDRESS,
        },
        HookSpec {
            name: c"vkGetInstanceProcAddr",
            replacement: hooked_vk_get_instance_proc_addr as *const c_void,
            orig_slot: &ORIG_VK_GET_INSTANCE_PROC_ADDR,
        },
        HookSpec {
            name: c"vkGetDeviceProcAddr",
            replacement: hooked_vk_get_device_proc_addr as *const c_void,
            orig_slot: &ORIG_VK_GET_DEVICE_PROC_ADDR,
        },
        HookSpec {
            name: c"vkGetPhysicalDeviceProperties",
            replacement: hooked_vk_get_physical_device_properties as *const c_void,
            orig_slot: &ORIG_VK_GET_PROPS,
        },
        HookSpec {
            name: c"vkGetPhysicalDeviceProperties2",
            replacement: hooked_vk_get_physical_device_properties2 as *const c_void,
            orig_slot: &ORIG_VK_GET_PROPS2,
        },
        HookSpec {
            name: c"vkGetPhysicalDeviceProperties2KHR",
            replacement: hooked_vk_get_physical_device_properties2_khr as *const c_void,
            orig_slot: &ORIG_VK_GET_PROPS2_KHR,
        },
    ];

    fn to_cstring(s: Option<&str>) -> Option<CString> {
        s.and_then(|v| CString::new(v).ok())
    }

    fn set_env_cstring(key: &CStr, value: &CStr) {
        unsafe {
            libc::setenv(key.as_ptr(), value.as_ptr(), 1);
        }
    }

    pub fn prepare(cfg: &ResolvedGpuConfig, debug_log: bool) {
        let active = ActiveGpuConfig {
            renderer: to_cstring(cfg.renderer.as_deref()),
            vendor: to_cstring(cfg.vendor.as_deref()),
            gl_version: to_cstring(cfg.gl_version.as_deref()),
            vendor_id: cfg.vendor_id,
            device_id: cfg.device_id,
            driver_version: cfg.driver_version,
            debug_log,
        };

        // Mesa/Turnip/Zink 在初始化期检查这两个环境变量，提前注入不影响非 Mesa 驱动。
        if let Some(renderer) = &active.renderer {
            set_env_cstring(c"force_gl_renderer", renderer);
        }
        if let Some(vendor) = &active.vendor {
            set_env_cstring(c"force_gl_vendor", vendor);
        }

        let _ = GPU_CONFIG.set(active);
    }

    fn logcat_info(msg: &str) {
        if !GPU_CONFIG.get().is_some_and(|c| c.debug_log) {
            return;
        }
        let Ok(c_msg) = CString::new(msg) else {
            return;
        };
        let fn_ptr = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"__android_log_write".as_ptr()) };
        if !fn_ptr.is_null() {
            type FnLogWrite = unsafe extern "C" fn(c_int, *const c_char, *const c_char) -> c_int;
            let write_fn = unsafe { std::mem::transmute::<*mut c_void, FnLogWrite>(fn_ptr) };
            // ANDROID_LOG_INFO = 4
            unsafe {
                write_fn(4, c"device_faker_gpu".as_ptr(), c_msg.as_ptr());
            }
        }
    }

    pub fn install() {
        // 先用我们自身的 libc import 地址保底初始化 loader 原始指针
        record_orig(
            &ORIG_DLOPEN,
            libc::dlopen as *mut c_void,
            hooked_dlopen as *const c_void,
        );
        record_orig(
            &ORIG_DLSYM,
            libc::dlsym as *mut c_void,
            hooked_dlsym as *const c_void,
        );

        // 先解析 bionic __loader_* 与 android_dlopen_ext 原始地址，再全量修补已加载 ELF 的 GOT/PLT，
        // 避免 Zygisk Next plt_hook_commit 在部分未导入全部符号的 SO 上整批失败。
        ensure_loader_originals();
        let patched_elfs = patch_all_loaded_elf_gots();

        if let Some(cfg) = GPU_CONFIG.get() {
            let msg = format!(
                "GPU spoof GOT hooks installed across {} mapped ELFs (renderer={:?}, vendor={:?}, vendor_id={:?}, loader_dlopen={}, loader_ext={}, loader_dlsym={})",
                patched_elfs,
                cfg.renderer.as_ref().map(|c| c.to_string_lossy()),
                cfg.vendor.as_ref().map(|c| c.to_string_lossy()),
                cfg.vendor_id.map(|id| format!("0x{id:04X}")),
                !LOADER_DLOPEN.load(Ordering::Acquire).is_null(),
                !LOADER_ANDROID_DLOPEN_EXT.load(Ordering::Acquire).is_null(),
                !LOADER_DLSYM.load(Ordering::Acquire).is_null(),
            );
            info!("{msg}");
            logcat_info(&msg);
        }
    }

    fn find_self_info() -> Option<(usize, *const c_char)> {
        let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::dladdr(hooked_dlopen as *const c_void, &mut info) };
        if rc == 0 || info.dli_fbase.is_null() {
            return None;
        }
        Some((info.dli_fbase as usize, info.dli_fname))
    }

    fn record_orig(slot: &AtomicPtr<c_void>, candidate: *mut c_void, self_fn: *const c_void) {
        if !candidate.is_null() && !std::ptr::eq(candidate as *const c_void, self_fn) {
            let _ = slot.compare_exchange(
                ptr::null_mut(),
                candidate,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }

    fn lookup_hook_spec(sym: &str) -> Option<&'static HookSpec> {
        HOOK_SPECS.iter().find(|spec| spec.name.to_str() == Ok(sym))
    }

    fn ensure_loader_originals() {
        let dlsym_fn: FnDlsym = match NonNull::new(ORIG_DLSYM.load(Ordering::Acquire)) {
            Some(p) => unsafe { std::mem::transmute::<*mut c_void, FnDlsym>(p.as_ptr()) },
            None => libc::dlsym,
        };
        if ORIG_ANDROID_DLOPEN_EXT.load(Ordering::Acquire).is_null() {
            let ext = unsafe { dlsym_fn(libc::RTLD_DEFAULT, c"android_dlopen_ext".as_ptr()) };
            record_orig(
                &ORIG_ANDROID_DLOPEN_EXT,
                ext,
                hooked_android_dlopen_ext as *const c_void,
            );
        }
        // 解析 bionic __loader_* 入口，调用时显式透传真实 caller_addr，保留链接器命名空间上下文
        for (slot, sym) in [
            (&LOADER_DLOPEN, c"__loader_dlopen"),
            (&LOADER_ANDROID_DLOPEN_EXT, c"__loader_android_dlopen_ext"),
            (&LOADER_DLSYM, c"__loader_dlsym"),
        ] {
            if slot.load(Ordering::Acquire).is_null() {
                let ptr = resolve_symbol_fallback(sym, Some(c"libdl.so"));
                if !ptr.is_null() {
                    let _ = slot.compare_exchange(
                        ptr::null_mut(),
                        ptr,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    );
                }
            }
        }
    }

    fn resolve_symbol_fallback(sym: &CStr, fallback_lib: Option<&CStr>) -> *mut c_void {
        let dlsym_fn: FnDlsym = match NonNull::new(ORIG_DLSYM.load(Ordering::Acquire)) {
            Some(p) => unsafe { std::mem::transmute::<*mut c_void, FnDlsym>(p.as_ptr()) },
            None => libc::dlsym,
        };
        let ptr = unsafe { dlsym_fn(libc::RTLD_DEFAULT, sym.as_ptr()) };
        if !ptr.is_null() {
            return ptr;
        }
        if let Some(lib_name) = fallback_lib {
            let dlopen_fn: FnDlopen = match NonNull::new(ORIG_DLOPEN.load(Ordering::Acquire)) {
                Some(p) => unsafe { std::mem::transmute::<*mut c_void, FnDlopen>(p.as_ptr()) },
                None => libc::dlopen,
            };
            let handle =
                unsafe { dlopen_fn(lib_name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_NOLOAD) };
            if !handle.is_null() {
                return unsafe { dlsym_fn(handle, sym.as_ptr()) };
            }
        }
        ptr::null_mut()
    }

    #[cfg(target_arch = "aarch64")]
    #[unsafe(naked)]
    unsafe extern "C" fn hooked_dlopen(_filename: *const c_char, _flags: c_int) -> *mut c_void {
        core::arch::naked_asm!(
            "hint #36", // bti jc
            "mov x2, x30",
            "b {inner}",
            inner = sym hooked_dlopen_with_caller,
        );
    }

    #[cfg(not(target_arch = "aarch64"))]
    unsafe extern "C" fn hooked_dlopen(filename: *const c_char, flags: c_int) -> *mut c_void {
        unsafe { hooked_dlopen_with_caller(filename, flags, ptr::null()) }
    }

    unsafe extern "C" fn hooked_dlopen_with_caller(
        filename: *const c_char,
        flags: c_int,
        caller_addr: *const c_void,
    ) -> *mut c_void {
        let loader_ptr = LOADER_DLOPEN.load(Ordering::Acquire);
        let handle = if !loader_ptr.is_null() && !caller_addr.is_null() {
            let loader_fn =
                unsafe { std::mem::transmute::<*mut c_void, FnLoaderDlopen>(loader_ptr) };
            unsafe { loader_fn(filename, flags, caller_addr) }
        } else {
            let orig_ptr = ORIG_DLOPEN.load(Ordering::Acquire);
            let orig_fn: FnDlopen = if !orig_ptr.is_null() {
                unsafe { std::mem::transmute::<*mut c_void, FnDlopen>(orig_ptr) }
            } else {
                libc::dlopen
            };
            unsafe { orig_fn(filename, flags) }
        };
        if !handle.is_null() {
            patch_all_loaded_elf_gots();
        }
        handle
    }

    #[cfg(target_arch = "aarch64")]
    #[unsafe(naked)]
    unsafe extern "C" fn hooked_android_dlopen_ext(
        _filename: *const c_char,
        _flags: c_int,
        _extinfo: *const c_void,
    ) -> *mut c_void {
        core::arch::naked_asm!(
            "hint #36", // bti jc
            "mov x3, x30",
            "b {inner}",
            inner = sym hooked_android_dlopen_ext_with_caller,
        );
    }

    #[cfg(not(target_arch = "aarch64"))]
    unsafe extern "C" fn hooked_android_dlopen_ext(
        filename: *const c_char,
        flags: c_int,
        extinfo: *const c_void,
    ) -> *mut c_void {
        unsafe { hooked_android_dlopen_ext_with_caller(filename, flags, extinfo, ptr::null()) }
    }

    unsafe extern "C" fn hooked_android_dlopen_ext_with_caller(
        filename: *const c_char,
        flags: c_int,
        extinfo: *const c_void,
        caller_addr: *const c_void,
    ) -> *mut c_void {
        let loader_ptr = LOADER_ANDROID_DLOPEN_EXT.load(Ordering::Acquire);
        let handle = if !loader_ptr.is_null() && !caller_addr.is_null() {
            let loader_fn =
                unsafe { std::mem::transmute::<*mut c_void, FnLoaderAndroidDlopenExt>(loader_ptr) };
            unsafe { loader_fn(filename, flags, extinfo, caller_addr) }
        } else {
            let mut orig_ptr = ORIG_ANDROID_DLOPEN_EXT.load(Ordering::Acquire);
            if orig_ptr.is_null() {
                orig_ptr = resolve_symbol_fallback(c"android_dlopen_ext", Some(c"libdl.so"));
                record_orig(
                    &ORIG_ANDROID_DLOPEN_EXT,
                    orig_ptr,
                    hooked_android_dlopen_ext as *const c_void,
                );
            }
            if orig_ptr.is_null() {
                return ptr::null_mut();
            }
            let orig_fn =
                unsafe { std::mem::transmute::<*mut c_void, FnAndroidDlopenExt>(orig_ptr) };
            unsafe { orig_fn(filename, flags, extinfo) }
        };
        if !handle.is_null() {
            patch_all_loaded_elf_gots();
        }
        handle
    }

    #[cfg(target_arch = "aarch64")]
    #[unsafe(naked)]
    unsafe extern "C" fn hooked_dlsym(_handle: *mut c_void, _symbol: *const c_char) -> *mut c_void {
        core::arch::naked_asm!(
            "hint #36", // bti jc
            "mov x2, x30",
            "b {inner}",
            inner = sym hooked_dlsym_with_caller,
        );
    }

    #[cfg(not(target_arch = "aarch64"))]
    unsafe extern "C" fn hooked_dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void {
        unsafe { hooked_dlsym_with_caller(handle, symbol, ptr::null()) }
    }

    unsafe extern "C" fn hooked_dlsym_with_caller(
        handle: *mut c_void,
        symbol: *const c_char,
        caller_addr: *const c_void,
    ) -> *mut c_void {
        let loader_ptr = LOADER_DLSYM.load(Ordering::Acquire);
        let real = if !loader_ptr.is_null() && !caller_addr.is_null() {
            let loader_fn =
                unsafe { std::mem::transmute::<*mut c_void, FnLoaderDlsym>(loader_ptr) };
            unsafe { loader_fn(handle, symbol, caller_addr) }
        } else {
            let orig_ptr = ORIG_DLSYM.load(Ordering::Acquire);
            let orig_fn: FnDlsym = if !orig_ptr.is_null() {
                unsafe { std::mem::transmute::<*mut c_void, FnDlsym>(orig_ptr) }
            } else {
                libc::dlsym
            };
            unsafe { orig_fn(handle, symbol) }
        };
        if real.is_null() || symbol.is_null() {
            return real;
        }
        if let Ok(sym) = unsafe { CStr::from_ptr(symbol) }.to_str()
            && let Some(spec) = lookup_hook_spec(sym)
        {
            record_orig(spec.orig_slot, real, spec.replacement);
            return spec.replacement as *mut c_void;
        }
        real
    }

    unsafe extern "C" fn hooked_gl_get_string(name: u32) -> *const c_char {
        if let Some(cfg) = GPU_CONFIG.get() {
            match name {
                GL_RENDERER => {
                    if let Some(renderer) = &cfg.renderer {
                        return renderer.as_ptr();
                    }
                }
                GL_VENDOR => {
                    if let Some(vendor) = &cfg.vendor {
                        return vendor.as_ptr();
                    }
                }
                GL_VERSION => {
                    if let Some(version) = &cfg.gl_version {
                        return version.as_ptr();
                    }
                }
                _ => {}
            }
        }

        let mut orig_ptr = ORIG_GL_GET_STRING.load(Ordering::Acquire);
        if orig_ptr.is_null() {
            orig_ptr = resolve_symbol_fallback(c"glGetString", Some(c"libGLESv2.so"));
            record_orig(
                &ORIG_GL_GET_STRING,
                orig_ptr,
                hooked_gl_get_string as *const c_void,
            );
        }
        if orig_ptr.is_null() {
            return ptr::null();
        }
        let orig_fn = unsafe { std::mem::transmute::<*mut c_void, FnGlGetString>(orig_ptr) };
        unsafe { orig_fn(name) }
    }

    unsafe extern "C" fn hooked_egl_get_proc_address(procname: *const c_char) -> *mut c_void {
        let mut orig_ptr = ORIG_EGL_GET_PROC_ADDRESS.load(Ordering::Acquire);
        if orig_ptr.is_null() {
            orig_ptr = resolve_symbol_fallback(c"eglGetProcAddress", Some(c"libEGL.so"));
            record_orig(
                &ORIG_EGL_GET_PROC_ADDRESS,
                orig_ptr,
                hooked_egl_get_proc_address as *const c_void,
            );
        }
        if orig_ptr.is_null() {
            return ptr::null_mut();
        }
        let orig_fn = unsafe { std::mem::transmute::<*mut c_void, FnEglGetProcAddress>(orig_ptr) };
        let real = unsafe { orig_fn(procname) };
        if real.is_null() || procname.is_null() {
            return real;
        }
        if let Ok(sym) = unsafe { CStr::from_ptr(procname) }.to_str()
            && let Some(spec) = lookup_hook_spec(sym)
        {
            record_orig(spec.orig_slot, real, spec.replacement);
            return spec.replacement as *mut c_void;
        }
        real
    }

    unsafe extern "C" fn hooked_vk_get_instance_proc_addr(
        instance: *mut c_void,
        p_name: *const c_char,
    ) -> *mut c_void {
        let mut orig_ptr = ORIG_VK_GET_INSTANCE_PROC_ADDR.load(Ordering::Acquire);
        if orig_ptr.is_null() {
            orig_ptr = resolve_symbol_fallback(c"vkGetInstanceProcAddr", Some(c"libvulkan.so"));
            record_orig(
                &ORIG_VK_GET_INSTANCE_PROC_ADDR,
                orig_ptr,
                hooked_vk_get_instance_proc_addr as *const c_void,
            );
        }
        if orig_ptr.is_null() {
            return ptr::null_mut();
        }
        let orig_fn =
            unsafe { std::mem::transmute::<*mut c_void, FnVkGetInstanceProcAddr>(orig_ptr) };
        let real = unsafe { orig_fn(instance, p_name) };
        if real.is_null() || p_name.is_null() {
            return real;
        }
        if let Ok(sym) = unsafe { CStr::from_ptr(p_name) }.to_str()
            && let Some(spec) = lookup_hook_spec(sym)
        {
            record_orig(spec.orig_slot, real, spec.replacement);
            return spec.replacement as *mut c_void;
        }
        real
    }

    unsafe extern "C" fn hooked_vk_get_device_proc_addr(
        device: *mut c_void,
        p_name: *const c_char,
    ) -> *mut c_void {
        let mut orig_ptr = ORIG_VK_GET_DEVICE_PROC_ADDR.load(Ordering::Acquire);
        if orig_ptr.is_null() {
            orig_ptr = resolve_symbol_fallback(c"vkGetDeviceProcAddr", Some(c"libvulkan.so"));
            record_orig(
                &ORIG_VK_GET_DEVICE_PROC_ADDR,
                orig_ptr,
                hooked_vk_get_device_proc_addr as *const c_void,
            );
        }
        if orig_ptr.is_null() {
            return ptr::null_mut();
        }
        let orig_fn =
            unsafe { std::mem::transmute::<*mut c_void, FnVkGetDeviceProcAddr>(orig_ptr) };
        let real = unsafe { orig_fn(device, p_name) };
        if real.is_null() || p_name.is_null() {
            return real;
        }
        if let Ok(sym) = unsafe { CStr::from_ptr(p_name) }.to_str()
            && let Some(spec) = lookup_hook_spec(sym)
        {
            record_orig(spec.orig_slot, real, spec.replacement);
            return spec.replacement as *mut c_void;
        }
        real
    }

    fn patch_vk_props_prefix(props: &mut VkPhysicalDevicePropertiesPrefix) {
        let Some(cfg) = GPU_CONFIG.get() else {
            return;
        };
        if let Some(driver_version) = cfg.driver_version {
            props.driver_version = driver_version;
        }
        if let Some(vendor_id) = cfg.vendor_id {
            props.vendor_id = vendor_id;
        }
        if let Some(device_id) = cfg.device_id {
            props.device_id = device_id;
        }
        if let Some(renderer) = &cfg.renderer {
            let bytes = renderer.as_bytes();
            let copy_len = bytes.len().min(VK_MAX_PHYSICAL_DEVICE_NAME_SIZE - 1);
            props.device_name.fill(0);
            props.device_name[..copy_len].copy_from_slice(&bytes[..copy_len]);
        }
    }

    unsafe extern "C" fn hooked_vk_get_physical_device_properties(
        physical_device: *mut c_void,
        p_properties: *mut VkPhysicalDevicePropertiesPrefix,
    ) {
        let mut orig_ptr = ORIG_VK_GET_PROPS.load(Ordering::Acquire);
        if orig_ptr.is_null() {
            orig_ptr =
                resolve_symbol_fallback(c"vkGetPhysicalDeviceProperties", Some(c"libvulkan.so"));
            record_orig(
                &ORIG_VK_GET_PROPS,
                orig_ptr,
                hooked_vk_get_physical_device_properties as *const c_void,
            );
        }
        if !orig_ptr.is_null() {
            let orig_fn = unsafe {
                std::mem::transmute::<*mut c_void, FnVkGetPhysicalDeviceProperties>(orig_ptr)
            };
            unsafe { orig_fn(physical_device, p_properties) };
            if !p_properties.is_null() {
                patch_vk_props_prefix(unsafe { &mut *p_properties });
            }
        }
    }

    unsafe extern "C" fn hooked_vk_get_physical_device_properties2(
        physical_device: *mut c_void,
        p_properties: *mut VkPhysicalDeviceProperties2Prefix,
    ) {
        let mut orig_ptr = ORIG_VK_GET_PROPS2.load(Ordering::Acquire);
        if orig_ptr.is_null() {
            orig_ptr =
                resolve_symbol_fallback(c"vkGetPhysicalDeviceProperties2", Some(c"libvulkan.so"));
            record_orig(
                &ORIG_VK_GET_PROPS2,
                orig_ptr,
                hooked_vk_get_physical_device_properties2 as *const c_void,
            );
        }
        if !orig_ptr.is_null() {
            let orig_fn = unsafe {
                std::mem::transmute::<*mut c_void, FnVkGetPhysicalDeviceProperties2>(orig_ptr)
            };
            unsafe { orig_fn(physical_device, p_properties) };
            if !p_properties.is_null() {
                patch_vk_props_prefix(unsafe { &mut (*p_properties).properties });
            }
        }
    }

    unsafe extern "C" fn hooked_vk_get_physical_device_properties2_khr(
        physical_device: *mut c_void,
        p_properties: *mut VkPhysicalDeviceProperties2Prefix,
    ) {
        let mut orig_ptr = ORIG_VK_GET_PROPS2_KHR.load(Ordering::Acquire);
        if orig_ptr.is_null() {
            orig_ptr = resolve_symbol_fallback(
                c"vkGetPhysicalDeviceProperties2KHR",
                Some(c"libvulkan.so"),
            );
            record_orig(
                &ORIG_VK_GET_PROPS2_KHR,
                orig_ptr,
                hooked_vk_get_physical_device_properties2_khr as *const c_void,
            );
        }
        if !orig_ptr.is_null() {
            let orig_fn = unsafe {
                std::mem::transmute::<*mut c_void, FnVkGetPhysicalDeviceProperties2>(orig_ptr)
            };
            unsafe { orig_fn(physical_device, p_properties) };
            if !p_properties.is_null() {
                patch_vk_props_prefix(unsafe { &mut (*p_properties).properties });
            }
        }
    }

    // ── ELF64 GOT/PLT 动态补丁（捕获 dlopen/android_dlopen_ext 后才加载的 SO）───────

    fn patch_all_loaded_elf_gots() -> usize {
        let Ok(_guard) = GOT_PATCH_LOCK.lock() else {
            return 0;
        };
        let self_base = find_self_info().map(|(base, _)| base).unwrap_or(0);
        let mut ctx = DlIterContext {
            self_base,
            visited: 0,
        };
        unsafe {
            libc::dl_iterate_phdr(
                Some(dl_iter_callback),
                (&mut ctx as *mut DlIterContext).cast(),
            );
        }
        ctx.visited
    }

    struct DlIterContext {
        self_base: usize,
        visited: usize,
    }

    #[cfg(target_pointer_width = "64")]
    unsafe extern "C" fn dl_iter_callback(
        info: *mut libc::dl_phdr_info,
        _size: libc::size_t,
        data: *mut c_void,
    ) -> c_int {
        let Some(info_ref) = (unsafe { info.as_ref() }) else {
            return 0;
        };
        let Some(ctx) = (unsafe { (data as *mut DlIterContext).as_mut() }) else {
            return 0;
        };

        let base = info_ref.dlpi_addr as usize;
        if base == ctx.self_base || info_ref.dlpi_phdr.is_null() || info_ref.dlpi_phnum == 0 {
            return 0;
        }
        if !info_ref.dlpi_name.is_null()
            && let Ok(name) = unsafe { CStr::from_ptr(info_ref.dlpi_name) }.to_str()
            && (name.contains("[vdso]") || name == "linux-vdso.so.1")
        {
            return 0;
        }

        ctx.visited += 1;
        let phdrs =
            unsafe { std::slice::from_raw_parts(info_ref.dlpi_phdr, info_ref.dlpi_phnum as usize) };
        unsafe { patch_elf64_gots(base, phdrs) };
        0
    }

    #[cfg(target_pointer_width = "32")]
    unsafe extern "C" fn dl_iter_callback(
        _info: *mut libc::dl_phdr_info,
        _size: libc::size_t,
        _data: *mut c_void,
    ) -> c_int {
        0
    }

    #[cfg(target_pointer_width = "64")]
    fn resolve_elf_ptr(base: usize, val: usize) -> usize {
        if val == 0 {
            0
        } else if base != 0 && val >= base {
            val
        } else {
            base.wrapping_add(val)
        }
    }

    #[cfg(target_pointer_width = "64")]
    fn addr_in_pt_load(base: usize, phdrs: &[libc::Elf64_Phdr], addr: usize, len: usize) -> bool {
        if addr == 0 || len == 0 {
            return false;
        }
        let Some(end) = addr.checked_add(len) else {
            return false;
        };
        phdrs.iter().any(|ph| {
            if ph.p_type != PT_LOAD {
                return false;
            }
            let seg_start = base.wrapping_add(ph.p_vaddr as usize);
            let Some(seg_end) = seg_start.checked_add(ph.p_memsz as usize) else {
                return false;
            };
            addr >= seg_start && end <= seg_end
        })
    }

    #[cfg(target_pointer_width = "64")]
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Elf64Dyn {
        d_tag: i64,
        d_un: u64,
    }

    #[cfg(target_pointer_width = "64")]
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Elf64Sym {
        st_name: u32,
        st_info: u8,
        st_other: u8,
        st_shndx: u16,
        st_value: u64,
        st_size: u64,
    }

    #[cfg(target_pointer_width = "64")]
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Elf64Rela {
        r_offset: u64,
        r_info: u64,
        r_addend: i64,
    }

    #[cfg(target_pointer_width = "64")]
    unsafe fn patch_elf64_gots(base: usize, phdrs: &[libc::Elf64_Phdr]) {
        let Some(dyn_ph) = phdrs.iter().find(|ph| ph.p_type == PT_DYNAMIC) else {
            return;
        };
        let dyn_addr = base.wrapping_add(dyn_ph.p_vaddr as usize);
        let dyn_len = dyn_ph.p_memsz as usize;
        if dyn_len < std::mem::size_of::<Elf64Dyn>()
            || !addr_in_pt_load(base, phdrs, dyn_addr, dyn_len)
        {
            return;
        }

        let dyn_count = dyn_len / std::mem::size_of::<Elf64Dyn>();
        let dyns = unsafe { std::slice::from_raw_parts(dyn_addr as *const Elf64Dyn, dyn_count) };

        let mut strtab_val = 0usize;
        let mut symtab_val = 0usize;
        let mut jmprel_val = 0usize;
        let mut pltrelsz = 0usize;
        let mut pltrel_type = DT_RELA;
        let mut rela_val = 0usize;
        let mut relasz = 0usize;
        let mut android_rela_val = 0usize;
        let mut android_relasz = 0usize;

        for d in dyns {
            if d.d_tag == DT_NULL {
                break;
            }
            match d.d_tag {
                DT_STRTAB => strtab_val = d.d_un as usize,
                DT_SYMTAB => symtab_val = d.d_un as usize,
                DT_JMPREL => jmprel_val = d.d_un as usize,
                DT_PLTRELSZ => pltrelsz = d.d_un as usize,
                DT_PLTREL => pltrel_type = d.d_un as i64,
                DT_RELA => rela_val = d.d_un as usize,
                DT_RELASZ => relasz = d.d_un as usize,
                DT_ANDROID_RELA => android_rela_val = d.d_un as usize,
                DT_ANDROID_RELASZ => android_relasz = d.d_un as usize,
                _ => {}
            }
        }

        let strtab = resolve_elf_ptr(base, strtab_val);
        let symtab = resolve_elf_ptr(base, symtab_val);
        if strtab == 0 || symtab == 0 {
            return;
        }
        if !addr_in_pt_load(base, phdrs, strtab, 1)
            || !addr_in_pt_load(base, phdrs, symtab, std::mem::size_of::<Elf64Sym>())
        {
            return;
        }

        let relro_range = phdrs.iter().find(|ph| ph.p_type == PT_GNU_RELRO).map(|ph| {
            let start = base.wrapping_add(ph.p_vaddr as usize);
            (start, start.saturating_add(ph.p_memsz as usize))
        });

        // 1. DT_JMPREL
        if pltrel_type == DT_RELA {
            unsafe {
                patch_rela_table(
                    base,
                    phdrs,
                    strtab,
                    symtab,
                    relro_range,
                    jmprel_val,
                    pltrelsz,
                );
            }
        }

        // 2. DT_RELA
        unsafe {
            patch_rela_table(base, phdrs, strtab, symtab, relro_range, rela_val, relasz);
        }

        // 3. DT_ANDROID_RELA (APS2)
        if android_rela_val != 0 && android_relasz > 4 {
            let packed = resolve_elf_ptr(base, android_rela_val);
            if addr_in_pt_load(base, phdrs, packed, android_relasz) {
                let bytes =
                    unsafe { std::slice::from_raw_parts(packed as *const u8, android_relasz) };
                if bytes.starts_with(b"APS2") {
                    unsafe {
                        decode_and_patch_aps2(&bytes[4..], base, phdrs, strtab, symtab, relro_range)
                    };
                }
            }
        }
    }

    #[cfg(target_pointer_width = "64")]
    unsafe fn patch_rela_table(
        base: usize,
        phdrs: &[libc::Elf64_Phdr],
        strtab: usize,
        symtab: usize,
        relro_range: Option<(usize, usize)>,
        table_val: usize,
        table_bytes: usize,
    ) {
        if table_val == 0 || table_bytes < std::mem::size_of::<Elf64Rela>() {
            return;
        }
        let table_ptr = resolve_elf_ptr(base, table_val);
        if !addr_in_pt_load(base, phdrs, table_ptr, table_bytes) {
            return;
        }
        let count = table_bytes / std::mem::size_of::<Elf64Rela>();
        let relas = unsafe { std::slice::from_raw_parts(table_ptr as *const Elf64Rela, count) };
        for rela in relas {
            unsafe { try_patch_rela(base, phdrs, strtab, symtab, relro_range, rela) };
        }
    }

    #[cfg(target_pointer_width = "64")]
    unsafe fn try_patch_rela(
        base: usize,
        phdrs: &[libc::Elf64_Phdr],
        strtab: usize,
        symtab: usize,
        relro_range: Option<(usize, usize)>,
        rela: &Elf64Rela,
    ) {
        let r_type = (rela.r_info & 0xffff_ffff) as u32;
        if r_type != R_AARCH64_JUMP_SLOT
            && r_type != R_AARCH64_GLOB_DAT
            && r_type != R_AARCH64_ABS64
        {
            return;
        }
        let sym_idx = (rela.r_info >> 32) as usize;
        if sym_idx == 0 {
            return;
        }
        let sym_addr = symtab.wrapping_add(sym_idx.saturating_mul(std::mem::size_of::<Elf64Sym>()));
        if !addr_in_pt_load(base, phdrs, sym_addr, std::mem::size_of::<Elf64Sym>()) {
            return;
        }
        let sym = unsafe { &*(sym_addr as *const Elf64Sym) };
        let name_addr = strtab.wrapping_add(sym.st_name as usize);
        if !addr_in_pt_load(base, phdrs, name_addr, 1) {
            return;
        }
        let Ok(sym_name) = unsafe { CStr::from_ptr(name_addr as *const c_char) }.to_str() else {
            return;
        };
        let Some(spec) = lookup_hook_spec(sym_name) else {
            return;
        };

        let got_slot = base.wrapping_add(rela.r_offset as usize);
        if !addr_in_pt_load(base, phdrs, got_slot, std::mem::size_of::<usize>()) {
            return;
        }
        let slot_ptr = got_slot as *mut usize;
        let cur_val = unsafe { ptr::read_volatile(slot_ptr) };
        let target_val = spec.replacement as usize;
        if cur_val == target_val {
            return;
        }
        record_orig(spec.orig_slot, cur_val as *mut c_void, spec.replacement);

        unsafe { write_got_slot(slot_ptr, target_val, relro_range) };
    }

    #[cfg(target_pointer_width = "64")]
    unsafe fn write_got_slot(
        slot_ptr: *mut usize,
        new_val: usize,
        relro_range: Option<(usize, usize)>,
    ) {
        let addr = slot_ptr as usize;
        let in_relro = relro_range.is_some_and(|(s, e)| addr >= s && addr < e);
        if !in_relro {
            unsafe { ptr::write_volatile(slot_ptr, new_val) };
            return;
        }

        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(4096) as usize;
        let page_start = addr & !(page_size - 1);
        let page_end = (addr + std::mem::size_of::<usize>() + page_size - 1) & !(page_size - 1);
        let mlen = page_end - page_start;
        let rc = unsafe {
            libc::mprotect(
                page_start as *mut c_void,
                mlen,
                libc::PROT_READ | libc::PROT_WRITE,
            )
        };
        if rc == 0 {
            unsafe { ptr::write_volatile(slot_ptr, new_val) };
            unsafe { libc::mprotect(page_start as *mut c_void, mlen, libc::PROT_READ) };
        }
    }

    #[cfg(target_pointer_width = "64")]
    unsafe fn decode_and_patch_aps2(
        data: &[u8],
        base: usize,
        phdrs: &[libc::Elf64_Phdr],
        strtab: usize,
        symtab: usize,
        relro_range: Option<(usize, usize)>,
    ) {
        const RELOCATION_GROUPED_BY_INFO_FLAG: i64 = 1 << 0;
        const RELOCATION_GROUPED_BY_OFFSET_DELTA_FLAG: i64 = 1 << 1;
        const RELOCATION_GROUPED_BY_ADDEND_FLAG: i64 = 1 << 2;
        const RELOCATION_GROUP_HAS_ADDEND_FLAG: i64 = 1 << 3;

        let mut pos = 0usize;
        let Some(num_relocs) = read_sleb128(data, &mut pos) else {
            return;
        };
        let Some(mut offset) = read_sleb128(data, &mut pos) else {
            return;
        };
        let mut info = 0i64;
        let mut addend = 0i64;

        let mut remaining = num_relocs;
        while remaining > 0 {
            let Some(group_size) = read_sleb128(data, &mut pos) else {
                return;
            };
            let Some(group_flags) = read_sleb128(data, &mut pos) else {
                return;
            };
            let mut group_offset_delta = 0i64;
            if (group_flags & RELOCATION_GROUPED_BY_OFFSET_DELTA_FLAG) != 0 {
                let Some(d) = read_sleb128(data, &mut pos) else {
                    return;
                };
                group_offset_delta = d;
            }
            if (group_flags & RELOCATION_GROUPED_BY_INFO_FLAG) != 0 {
                let Some(i) = read_sleb128(data, &mut pos) else {
                    return;
                };
                info = i;
            }
            if (group_flags & RELOCATION_GROUP_HAS_ADDEND_FLAG) != 0
                && (group_flags & RELOCATION_GROUPED_BY_ADDEND_FLAG) != 0
            {
                let Some(a) = read_sleb128(data, &mut pos) else {
                    return;
                };
                addend += a;
            } else if (group_flags & RELOCATION_GROUP_HAS_ADDEND_FLAG) == 0 {
                addend = 0;
            }

            for _ in 0..group_size {
                if (group_flags & RELOCATION_GROUPED_BY_OFFSET_DELTA_FLAG) != 0 {
                    offset += group_offset_delta;
                } else {
                    let Some(d) = read_sleb128(data, &mut pos) else {
                        return;
                    };
                    offset += d;
                }
                if (group_flags & RELOCATION_GROUPED_BY_INFO_FLAG) == 0 {
                    let Some(i) = read_sleb128(data, &mut pos) else {
                        return;
                    };
                    info = i;
                }
                if (group_flags & RELOCATION_GROUP_HAS_ADDEND_FLAG) != 0
                    && (group_flags & RELOCATION_GROUPED_BY_ADDEND_FLAG) == 0
                {
                    let Some(a) = read_sleb128(data, &mut pos) else {
                        return;
                    };
                    addend += a;
                }

                let rela = Elf64Rela {
                    r_offset: offset as u64,
                    r_info: info as u64,
                    r_addend: addend,
                };
                unsafe { try_patch_rela(base, phdrs, strtab, symtab, relro_range, &rela) };
            }
            remaining -= group_size;
        }
    }

    #[cfg(target_pointer_width = "64")]
    fn read_sleb128(data: &[u8], pos: &mut usize) -> Option<i64> {
        let mut result: i64 = 0;
        let mut shift: u32 = 0;
        loop {
            let byte = *data.get(*pos)?;
            *pos += 1;
            result |= ((byte & 0x7f) as i64) << shift;
            shift += 7;
            if (byte & 0x80) == 0 {
                if shift < 64 && (byte & 0x40) != 0 {
                    result |= !0i64 << shift;
                }
                return Some(result);
            }
            if shift >= 64 {
                return None;
            }
        }
    }
}
