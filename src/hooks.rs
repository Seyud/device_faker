use std::ffi::{CStr, CString};

use anyhow::Context;
use jni::{
    JNIEnv,
    objects::{JClass, JString, JValue},
    strings::JNIStr,
    sys::JNINativeMethod,
};
use zygisk_api::api::{V4, ZygiskApi};

use crate::{
    config::MergedAppConfig,
    state::{FAKE_PROPS, ORIGINAL_NATIVE_GET, OriginalNativeGet},
};

static mut ORIGINAL_SYSTEM_PROPERTY_GET: Option<
    unsafe extern "C" fn(*const libc::c_char, *mut libc::c_char) -> libc::c_int,
> = None;

/// 根据合并配置 Hook android.os.Build 的静态字段。
pub fn hook_build_fields(env: &mut JNIEnv, merged_config: &MergedAppConfig) -> anyhow::Result<()> {
    let build_class = env
        .find_class("android/os/Build")
        .context("Failed to find Build class")?;

    if let Some(manufacturer) = &merged_config.manufacturer
        && !manufacturer.is_empty()
    {
        set_build_field(env, &build_class, "MANUFACTURER", manufacturer)?;
    }

    if let Some(brand) = &merged_config.brand
        && !brand.is_empty()
    {
        set_build_field(env, &build_class, "BRAND", brand)?;
    }

    if let Some(model) = &merged_config.model
        && !model.is_empty()
    {
        set_build_field(env, &build_class, "MODEL", model)?;
    }

    if let Some(device) = &merged_config.device
        && !device.is_empty()
    {
        set_build_field(env, &build_class, "DEVICE", device)?;
    }

    if let Some(product) = &merged_config.product
        && !product.is_empty()
    {
        set_build_field(env, &build_class, "PRODUCT", product)?;
    }

    if let Some(fingerprint) = &merged_config.fingerprint
        && !fingerprint.is_empty()
    {
        set_build_field(env, &build_class, "FINGERPRINT", fingerprint)?;
    }

    hook_version_fields(env, &build_class, merged_config)?;

    Ok(())
}

fn hook_version_fields(
    env: &mut JNIEnv,
    _build_class: &JClass,
    merged_config: &MergedAppConfig,
) -> anyhow::Result<()> {
    let version_class = env
        .find_class("android/os/Build$VERSION")
        .context("Failed to find Build.VERSION class")?;

    if let Some(android_version) = &merged_config.android_version
        && !android_version.is_empty()
    {
        set_build_field(env, &version_class, "RELEASE", android_version)?;
    }

    if let Some(sdk_int) = merged_config.sdk_int {
        set_build_int_field(env, &version_class, "SDK_INT", sdk_int as i32)?;
    }

    Ok(())
}

fn set_build_field(
    env: &mut JNIEnv,
    build_class: &JClass,
    field_name: &str,
    value: &str,
) -> anyhow::Result<()> {
    let field_id = env
        .get_static_field_id(build_class, field_name, "Ljava/lang/String;")
        .with_context(|| format!("Failed to get field ID for {field_name}"))?;

    let new_value = env
        .new_string(value)
        .with_context(|| format!("Failed to create string for {value}"))?;

    env.set_static_field(build_class, field_id, JValue::Object(&new_value))
        .with_context(|| format!("Failed to set field {field_name}"))?;

    Ok(())
}

fn set_build_int_field(
    env: &mut JNIEnv,
    build_class: &JClass,
    field_name: &str,
    value: i32,
) -> anyhow::Result<()> {
    let field_id = env
        .get_static_field_id(build_class, field_name, "I")
        .with_context(|| format!("Failed to get field ID for {field_name}"))?;

    env.set_static_field(build_class, field_id, JValue::Int(value))
        .with_context(|| format!("Failed to set field {field_name}"))?;

    Ok(())
}

/// Hook SystemProperties.native_get 以截获属性查询。
pub fn hook_system_properties(api: &mut ZygiskApi<V4>, env: &JNIEnv) -> anyhow::Result<()> {
    let mut methods = [JNINativeMethod {
        name: c"native_get".as_ptr() as *mut u8,
        signature: c"(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;".as_ptr() as *mut u8,
        fnPtr: native_get_hook as *mut std::ffi::c_void,
    }];

    let class_name = unsafe { JNIStr::from_ptr(c"android/os/SystemProperties".as_ptr()) };

    unsafe {
        api.hook_jni_native_methods(env.unsafe_clone(), class_name, &mut methods);
    }

    let original_fn_ptr = unsafe {
        std::mem::transmute::<*mut std::ffi::c_void, OriginalNativeGet>(methods[0].fnPtr)
    };
    *ORIGINAL_NATIVE_GET.lock().unwrap() = Some(original_fn_ptr);

    Ok(())
}

/// 为 Hook 提供的 SystemProperties.native_get 替身实现。
pub unsafe extern "C" fn native_get_hook(
    env: *mut jni::sys::JNIEnv,
    class: jni::sys::jclass,
    key: jni::sys::jstring,
    def: jni::sys::jstring,
) -> jni::sys::jstring {
    let mut env_wrapper = match unsafe { JNIEnv::from_raw(env) } {
        Ok(e) => e,
        Err(_) => return def,
    };

    let key_jstring = unsafe { JString::from_raw(key) };
    let key_str = match env_wrapper.get_string(&key_jstring) {
        Ok(s) => s,
        Err(_) => return def,
    };
    let key_string: String = key_str.into();

    let result = {
        let fake_props = FAKE_PROPS.lock().unwrap();
        if let Some(props) = fake_props.as_ref() {
            props.get(&key_string).and_then(|fake_value| {
                env_wrapper
                    .new_string(fake_value)
                    .ok()
                    .map(|s| s.into_raw())
            })
        } else {
            None
        }
    };

    if let Some(fake_result) = result {
        return fake_result;
    }

    let original_native_get = ORIGINAL_NATIVE_GET.lock().unwrap();
    if let Some(orig_fn) = *original_native_get {
        return unsafe { orig_fn(env, class, key, def) };
    }

    def
}

unsafe extern "C" fn my_system_property_get(
    name: *const libc::c_char,
    value: *mut libc::c_char,
) -> libc::c_int {
    if name.is_null() || value.is_null() {
        return 0;
    }

    let name_str = match unsafe { CStr::from_ptr(name).to_str() } {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let result = {
        let fake_props = FAKE_PROPS.lock().unwrap();
        if let Some(props) = fake_props.as_ref() {
            props.get(name_str).map(|fake_value| {
                let len = std::cmp::min(fake_value.len(), 91);
                unsafe {
                    std::ptr::copy(fake_value.as_ptr() as *const libc::c_char, value, len);
                    value.add(len).write(b'\0');
                }
                len as libc::c_int
            })
        } else {
            None
        }
    };

    if let Some(len) = result {
        return len;
    }

    unsafe {
        if let Some(orig_fn) = ORIGINAL_SYSTEM_PROPERTY_GET {
            return orig_fn(name, value);
        }
    }

    0
}

pub fn hook_native_property_get(api: &mut ZygiskApi<V4>) -> anyhow::Result<()> {
    let symbol = CString::new("__system_property_get").unwrap();
    #[allow(clippy::missing_transmute_annotations)]
    unsafe {
        let mut original: *const () = std::ptr::null();
        api.plt_hook_register(
            0,
            0,
            &symbol,
            my_system_property_get as *const (),
            &mut original,
        );
        let _ = api.plt_hook_commit();

        ORIGINAL_SYSTEM_PROPERTY_GET = Some(std::mem::transmute(original));
    }

    Ok(())
}
