use std::{collections::HashMap, sync::Mutex};

/// 用于恢复真实属性值的 native_get 原始函数签名。
pub type OriginalNativeGet = unsafe extern "C" fn(
    env: *mut jni::sys::JNIEnv,
    class: jni::sys::jclass,
    key: jni::sys::jstring,
    def: jni::sys::jstring,
) -> jni::sys::jstring;

pub static FAKE_PROPS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
pub static IS_FULL_MODE: Mutex<bool> = Mutex::new(false);
pub static ACTIVE_RESET_SESSION: Mutex<Option<ActiveResetSession>> = Mutex::new(None);
pub static ORIGINAL_NATIVE_GET: Mutex<Option<OriginalNativeGet>> = Mutex::new(None);

#[derive(Clone)]
pub struct ActiveResetSession {
    pub package: String,
    pub backups: HashMap<String, String>,
}
