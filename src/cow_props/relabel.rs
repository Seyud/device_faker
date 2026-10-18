//! 把「app 域不可读」的 context 换成可读 context。
//!
//! bionic specialize 之后会走 `SystemProperties::Init()` 已初始化分支 →
//! `ContextsSerialized::ResetAccess()` → `ContextNode::ResetAccess()`：
//! `access("/dev/__properties__/<context_>", R_OK)` 失败就 `Unmap()` 掉
//! `prop_area_` 并置 `no_access_`。
//!
//! 判据用的是 `ContextNode::context_`（节点自带指针），**改 property_info 无效**。
//! 把它换成 app 域可读的 `default_prop` 后复查通过、私有副本/合成区得以存活；
//! 属性**路由**由 property_info 的 entry 表决定，本改动不碰路由。

/// app 域可直接读取的属性区 context 名对应的 `ContextNode::context_` 替换值。
///
/// `ContextNode::CheckAccess()` 用 `access("/dev/__properties__/<context_>", R_OK)`
/// 判断可读性；把不可读 context 的 `context_` 换成可读 context 名即可通过复查。
/// 必须在**堆上**构造并泄漏（bionic 只持有指针，模块 so 会被 DlClose 卸载）。
pub(super) fn readable_context_label() -> usize {
    use std::sync::atomic::Ordering;
    static PTR: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let p = PTR.load(Ordering::Relaxed);
    if p != 0 {
        return p;
    }
    let mut buf = b"u:object_r:default_prop:s0\0".to_vec().into_boxed_slice();
    let np = buf.as_mut_ptr() as usize;
    std::mem::forget(buf);
    PTR.store(np, Ordering::Relaxed);
    np
}
