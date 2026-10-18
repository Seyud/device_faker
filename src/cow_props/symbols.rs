//! bionic 属性符号解析。

/// `__system_property_find` 的函数签名。
pub(super) type FnSystemPropertyFind =
    unsafe extern "C" fn(*const libc::c_char) -> *const libc::c_void;

/// bionic `PROP_VALUE_MAX`：inline 值上限（更长的值走 long 布局）。
pub(super) const PROP_VALUE_MAX: usize = 92;

// ── bionic 符号加载 ────────────────────────────────────────────────────────

pub(super) fn sys_prop_find() -> Option<FnSystemPropertyFind> {
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"__system_property_find".as_ptr()) };
    if sym.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute::<*mut libc::c_void, FnSystemPropertyFind>(sym) })
    }
}
