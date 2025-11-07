mod config;

use config::Config;
use jni::JNIEnv;
use jni::objects::{JClass, JString, JValue};
use jni::sys::JNINativeMethod;
use log::{error, info};
use std::collections::HashMap;
use std::sync::Mutex;
use zygisk_api::ZygiskModule;
use zygisk_api::api::v4::ZygiskOption;
use zygisk_api::api::{V4, ZygiskApi};

// 全局状态:存储当前应用的伪装属性
static FAKE_PROPS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

// 全局状态:标记是否使用了 full 模式（需要保持模块加载）
static IS_FULL_MODE: Mutex<bool> = Mutex::new(false);

// 原始 native_get 函数指针类型
type OriginalNativeGet = unsafe extern "C" fn(
    env: *mut jni::sys::JNIEnv,
    class: jni::sys::jclass,
    key: jni::sys::jstring,
    def: jni::sys::jstring,
) -> jni::sys::jstring;

// 存储原始 native_get 函数指针
static ORIGINAL_NATIVE_GET: Mutex<Option<OriginalNativeGet>> = Mutex::new(None);

#[derive(Default)]
struct MyModule;

impl ZygiskModule for MyModule {
    type Api = V4;

    fn on_load(&self, _api: ZygiskApi<V4>, _env: JNIEnv) {
        // 默认只记录错误，减少日志输出以提高隐蔽性
        // 可以通过配置文件的 debug = true 启用详细日志
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Error) // 默认只记录错误
                .with_tag("DeviceFaker"),
        );
    }

    fn pre_app_specialize(
        &self,
        mut api: ZygiskApi<V4>,
        mut env: JNIEnv,
        args: &mut <V4 as zygisk_api::raw::ZygiskRaw>::AppSpecializeArgs,
    ) {
        let mut inner = || -> anyhow::Result<()> {
            // 获取包名
            let package_name: String = env
                .get_string(args.nice_name)
                .map_err(|e| anyhow::anyhow!("Failed to get package name: {}", e))?
                .into();

            // 读取配置文件
            let config_path = "/data/adb/device_faker/config/config.toml";

            // 检查文件是否存在
            if !std::path::Path::new(config_path).exists() {
                api.set_option(ZygiskOption::DlCloseModuleLibrary);
                return Ok(());
            }

            let config_content = match std::fs::read_to_string(config_path) {
                Ok(content) => content,
                Err(e) => {
                    error!("Failed to read config: {}", e);
                    api.set_option(ZygiskOption::DlCloseModuleLibrary);
                    return Ok(());
                }
            };

            let config = Config::from_toml(&config_content)?;

            // 根据配置启用或禁用详细日志
            if config.debug {
                log::set_max_level(log::LevelFilter::Info);
                info!("Debug mode enabled");
                info!(
                    "Config loaded with {} apps and {} templates",
                    config.apps.len(),
                    config.templates.len()
                );
            }

            // 获取合并后的配置（会同时查找直接配置和模板配置）
            let merged_config = match config.get_merged_config(&package_name) {
                Some(cfg) => cfg,
                None => {
                    // 应用不在配置中
                    if config.debug {
                        info!("App {} not in config, unloading module", package_name);
                    }
                    api.set_option(ZygiskOption::DlCloseModuleLibrary);
                    return Ok(());
                }
            };

            // 获取工作模式
            let mode = &merged_config.mode;
            if config.debug {
                info!("Using mode: {} for app: {}", mode, package_name);
            }

            // Hook Build 类字段（两种模式都需要）
            Self::hook_build_fields(&mut env, &merged_config)?;
            if config.debug {
                info!("Build fields hooked successfully");
            }

            // 根据模式决定是否 Hook SystemProperties
            match mode.as_str() {
                "lite" => {
                    // 轻量模式：只修改 Build 类，完成后卸载模块
                    if config.debug {
                        info!("Lite mode: only Build fields hooked, unloading module");
                    }
                    *IS_FULL_MODE.lock().unwrap() = false;
                    api.set_option(ZygiskOption::DlCloseModuleLibrary);
                }
                "full" => {
                    // 完整模式：Hook Build + SystemProperties
                    if config.debug {
                        info!("Full mode: hooking SystemProperties");
                    }

                    // 标记为 full 模式，防止模块被卸载
                    *IS_FULL_MODE.lock().unwrap() = true;

                    // 构建属性映射表并存储
                    let prop_map = Config::build_merged_property_map(&merged_config);
                    if config.debug {
                        info!("Property map created with {} entries", prop_map.len());
                    }
                    *FAKE_PROPS.lock().unwrap() = Some(prop_map);

                    // Hook SystemProperties
                    Self::hook_system_properties(&mut api, &env)?;
                    if config.debug {
                        info!("SystemProperties hooked successfully, module will stay loaded");
                    }
                }
                _ => {
                    // 未知模式,回退到默认的 lite 模式
                    error!(
                        "Mode '{}' not fully supported, falling back to 'lite' mode",
                        mode
                    );
                    if config.debug {
                        info!("Lite mode (fallback): only Build fields hooked, unloading module");
                    }
                    *IS_FULL_MODE.lock().unwrap() = false;
                    api.set_option(ZygiskOption::DlCloseModuleLibrary);
                }
            }

            Ok(())
        };

        if let Err(e) = inner() {
            error!("Error: {:?}", e);
        }
    }

    fn post_app_specialize(
        &self,
        mut api: ZygiskApi<V4>,
        _env: JNIEnv,
        _args: &<V4 as zygisk_api::raw::ZygiskRaw>::AppSpecializeArgs,
    ) {
        // 只在非 full 模式下卸载模块
        // full 模式下，Hook 函数需要保持在内存中，否则会导致崩溃
        let is_full_mode = *IS_FULL_MODE.lock().unwrap();
        if !is_full_mode {
            // lite 模式：可以安全卸载
            api.set_option(ZygiskOption::DlCloseModuleLibrary);
        }
        // full 模式：不卸载，保持 Hook 函数可用
    }

    fn pre_server_specialize(
        &self,
        mut api: ZygiskApi<V4>,
        _env: JNIEnv,
        _args: &mut <V4 as zygisk_api::raw::ZygiskRaw>::ServerSpecializeArgs,
    ) {
        api.set_option(ZygiskOption::DlCloseModuleLibrary);
    }
}

impl MyModule {
    /// Hook Build 类的静态字段
    fn hook_build_fields(
        env: &mut JNIEnv,
        merged_config: &config::MergedAppConfig,
    ) -> anyhow::Result<()> {
        // 查找 android.os.Build 类
        let build_class = env
            .find_class("android/os/Build")
            .map_err(|e| anyhow::anyhow!("Failed to find Build class: {}", e))?;

        // 只修改配置中存在的字段
        if let Some(manufacturer) = &merged_config.manufacturer {
            Self::set_build_field(env, &build_class, "MANUFACTURER", manufacturer)?;
        }

        if let Some(brand) = &merged_config.brand {
            Self::set_build_field(env, &build_class, "BRAND", brand)?;
        }

        if let Some(model) = &merged_config.model {
            Self::set_build_field(env, &build_class, "MODEL", model)?;
        }

        if let Some(device) = &merged_config.device {
            Self::set_build_field(env, &build_class, "DEVICE", device)?;
        }

        if let Some(product) = &merged_config.product {
            Self::set_build_field(env, &build_class, "PRODUCT", product)?;
        }

        if let Some(fingerprint) = &merged_config.fingerprint {
            Self::set_build_field(env, &build_class, "FINGERPRINT", fingerprint)?;
        }

        Ok(())
    }

    /// 设置 Build 类的字段值
    fn set_build_field(
        env: &mut JNIEnv,
        build_class: &JClass,
        field_name: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        // 获取字段 ID
        let field_id = env
            .get_static_field_id(build_class, field_name, "Ljava/lang/String;")
            .map_err(|e| anyhow::anyhow!("Failed to get field ID for {}: {}", field_name, e))?;

        // 创建新的字符串值
        let new_value = env
            .new_string(value)
            .map_err(|e| anyhow::anyhow!("Failed to create string for {}: {}", value, e))?;

        // 设置静态字段
        env.set_static_field(build_class, field_id, JValue::Object(&new_value))
            .map_err(|e| anyhow::anyhow!("Failed to set field {}: {}", field_name, e))?;

        Ok(())
    }

    /// Hook SystemProperties.native_get 方法
    fn hook_system_properties(api: &mut ZygiskApi<V4>, env: &JNIEnv) -> anyhow::Result<()> {
        // 定义要 Hook 的 JNI 方法
        let mut methods = [JNINativeMethod {
            name: c"native_get".as_ptr() as *mut u8,
            signature: c"(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;".as_ptr()
                as *mut u8,
            fnPtr: native_get_hook as *mut std::ffi::c_void,
        }];

        // Hook SystemProperties 类的 native_get 方法
        use jni::strings::JNIStr;
        let class_name = unsafe { JNIStr::from_ptr(c"android/os/SystemProperties".as_ptr()) };

        unsafe {
            // 使用 unsafe_clone 来复制 JNIEnv
            api.hook_jni_native_methods(env.unsafe_clone(), class_name, &mut methods);
        }

        // 保存原始函数指针
        let original_fn_ptr = unsafe {
            std::mem::transmute::<*mut std::ffi::c_void, OriginalNativeGet>(methods[0].fnPtr)
        };
        *ORIGINAL_NATIVE_GET.lock().unwrap() = Some(original_fn_ptr);

        Ok(())
    }
}

zygisk_api::register_module!(MyModule);

/// SystemProperties.native_get Hook 函数
unsafe extern "C" fn native_get_hook(
    env: *mut jni::sys::JNIEnv,
    class: jni::sys::jclass,
    key: jni::sys::jstring,
    def: jni::sys::jstring,
) -> jni::sys::jstring {
    // 创建 JNIEnv 包装器
    let mut env_wrapper = match unsafe { JNIEnv::from_raw(env) } {
        Ok(e) => e,
        Err(_) => return def,
    };

    // 获取属性名
    let key_jstring = unsafe { JString::from_raw(key) };
    let key_str = match env_wrapper.get_string(&key_jstring) {
        Ok(s) => s,
        Err(_) => return def,
    };
    let key_string: String = key_str.into();

    // 检查是否需要伪装此属性
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

    // 如果有伪装值，返回；否则调用原始函数
    if let Some(fake_result) = result {
        return fake_result;
    }

    // 未匹配的属性：调用原始 native_get 函数获取真实值
    let original_native_get = ORIGINAL_NATIVE_GET.lock().unwrap();
    if let Some(orig_fn) = *original_native_get {
        return unsafe { orig_fn(env, class, key, def) };
    }

    // 如果原始函数不可用（不应该发生），返回默认值
    def
}
