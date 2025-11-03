mod config;

use config::Config;
use jni::JNIEnv;
use log::error;
use std::collections::HashMap;
use std::sync::Mutex;
use std::{fs::File, io::Read};
use zygisk_rs::{register_zygisk_module, Api, AppSpecializeArgs, Module, ServerSpecializeArgs};

// 全局状态:存储当前应用的伪装属性
static FAKE_PROPS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

// 原始 native_get 函数指针类型
type OriginalNativeGet = unsafe extern "C" fn(
    env: *mut jni_sys::JNIEnv,
    class: jni_sys::jclass,
    key: jni_sys::jstring,
    def: jni_sys::jstring,
) -> jni_sys::jstring;

// 存储原始 native_get 函数指针
static ORIGINAL_NATIVE_GET: Mutex<Option<OriginalNativeGet>> = Mutex::new(None);

struct MyModule {
    api: Api,
    env: JNIEnv<'static>,
}

impl Module for MyModule {
    fn new(api: Api, env: *mut jni_sys::JNIEnv) -> Self {
        // 初始化日志，使用 Error 级别减少日志输出，防止留痕
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Error) // 只记录错误，提高隐蔽性
                .with_tag("DeviceFaker"),
        );

        let env = unsafe { JNIEnv::from_raw(env.cast()).unwrap() };

        Self { api, env }
    }
    fn pre_app_specialize(&mut self, args: &mut AppSpecializeArgs) {
        let mut inner = || -> anyhow::Result<()> {
            // 获取包名
            let package_name = self
                .env
                .get_string(unsafe {
                    (args.nice_name as *mut jni_sys::jstring as *mut ()
                        as *const jni::objects::JString<'_>)
                        .as_ref()
                        .unwrap()
                })?
                .to_string_lossy()
                .to_string();

            // 读取配置文件
            let config_path = "/data/adb/device_faker/config/config.toml";

            // 检查文件是否存在
            if !std::path::Path::new(config_path).exists() {
                self.api
                    .set_option(zygisk_rs::ModuleOption::DlcloseModuleLibrary);
                return Ok(());
            }

            let mut config_file = match File::open(config_path) {
                Ok(file) => file,
                Err(e) => {
                    error!("Failed to open config: {}", e);
                    self.api
                        .set_option(zygisk_rs::ModuleOption::DlcloseModuleLibrary);
                    return Ok(());
                }
            };

            let mut config_content = String::new();
            config_file.read_to_string(&mut config_content)?;

            let config = Config::from_toml(&config_content)?;

            // 查找当前应用的配置
            let app_config = match config.get_app_config(&package_name) {
                Some(cfg) => cfg,
                None => {
                    // 应用不在配置中，立即卸载模块
                    self.api
                        .set_option(zygisk_rs::ModuleOption::DlcloseModuleLibrary);
                    return Ok(());
                }
            };

            // 构建属性映射表（用于 SystemProperties Hook）
            let prop_map = Config::build_property_map(app_config);

            // 保存到全局变量
            *FAKE_PROPS.lock().unwrap() = Some(prop_map);

            // Hook Build 类字段
            self.hook_build_fields(app_config)?;

            // Hook SystemProperties.native_get
            self.hook_system_properties()?;

            // 完成伪装后立即卸载模块，减少在内存中的存在时间
            self.api
                .set_option(zygisk_rs::ModuleOption::DlcloseModuleLibrary);

            Ok(())
        };

        if let Err(e) = inner() {
            error!("Error: {:?}", e);
        }
    }

    fn post_app_specialize(&mut self, _args: &AppSpecializeArgs) {
        // 额外的隐身策略：在 post_app_specialize 阶段再次确保模块卸载
        // 即使 pre_app_specialize 中已经卸载，这里再次调用以提高隐蔽性
        self.api
            .set_option(zygisk_rs::ModuleOption::DlcloseModuleLibrary);
    }

    fn pre_server_specialize(&mut self, _args: &mut ServerSpecializeArgs) {
        self.api
            .set_option(zygisk_rs::ModuleOption::DlcloseModuleLibrary);
    }

    fn post_server_specialize(&mut self, _args: &ServerSpecializeArgs) {}
}

impl MyModule {
    /// Hook Build 类的静态字段
    fn hook_build_fields(&mut self, app_config: &config::AppConfig) -> anyhow::Result<()> {
        // 查找 android.os.Build 类
        let build_class = self.env.find_class("android/os/Build")?;

        // 只修改配置中存在的字段
        if let Some(manufacturer) = &app_config.manufacturer {
            self.set_build_field(&build_class, "MANUFACTURER", manufacturer)?;
        }

        if let Some(brand) = &app_config.brand {
            self.set_build_field(&build_class, "BRAND", brand)?;
        }

        if let Some(model) = &app_config.model {
            self.set_build_field(&build_class, "MODEL", model)?;
        }

        if let Some(name) = &app_config.name {
            self.set_build_field(&build_class, "PRODUCT", name)?;
            self.set_build_field(&build_class, "DEVICE", name)?;
        }

        Ok(())
    }

    /// 设置 Build 类的字段值
    fn set_build_field(
        &mut self,
        build_class: &jni::objects::JClass,
        field_name: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        let field_id =
            self.env
                .get_static_field_id(build_class, field_name, "Ljava/lang/String;")?;

        let new_value = self.env.new_string(value)?;

        self.env.set_static_field(
            build_class,
            field_id,
            jni::objects::JValue::Object(&new_value),
        )?;

        Ok(())
    }

    /// Hook SystemProperties.native_get 方法
    fn hook_system_properties(&mut self) -> anyhow::Result<()> {
        // 定义要 Hook 的 JNI 方法
        let mut methods = [jni_sys::JNINativeMethod {
            name: c"native_get".as_ptr() as *mut u8,
            signature: c"(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;".as_ptr()
                as *mut u8,
            fnPtr: native_get_hook as *mut std::ffi::c_void,
        }];

        // Hook SystemProperties 类的 native_get 方法
        // 获取原始 JNIEnv 指针
        let env_ptr = self.env.get_raw() as *mut jni_sys::JNIEnv;

        self.api
            .hook_jni_native_methods(env_ptr, "android/os/SystemProperties", &mut methods);

        // 保存原始函数指针
        let original_fn_ptr = unsafe {
            std::mem::transmute::<*mut std::ffi::c_void, OriginalNativeGet>(methods[0].fnPtr)
        };
        *ORIGINAL_NATIVE_GET.lock().unwrap() = Some(original_fn_ptr);

        Ok(())
    }
}

register_zygisk_module!(MyModule);

/// SystemProperties.native_get Hook 函数
unsafe extern "C" fn native_get_hook(
    env: *mut jni_sys::JNIEnv,
    class: jni_sys::jclass,
    key: jni_sys::jstring,
    def: jni_sys::jstring,
) -> jni_sys::jstring {
    // 获取 JNI 函数表
    let jni_funcs = (**env).v1_4;

    // 获取属性名
    let key_cstr = (jni_funcs.GetStringUTFChars)(env, key, std::ptr::null_mut());
    if key_cstr.is_null() {
        return def;
    }

    let key_str = match std::ffi::CStr::from_ptr(key_cstr).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            (jni_funcs.ReleaseStringUTFChars)(env, key, key_cstr);
            return def;
        }
    };

    (jni_funcs.ReleaseStringUTFChars)(env, key, key_cstr);

    // 检查是否需要伪装此属性
    let fake_props = FAKE_PROPS.lock().unwrap();
    if let Some(props) = fake_props.as_ref() {
        if let Some(fake_value) = props.get(&key_str) {
            // 返回伪装的值
            let fake_cstr = std::ffi::CString::new(fake_value.as_str()).unwrap();
            return (jni_funcs.NewStringUTF)(env, fake_cstr.as_ptr());
        }
    }

    // 未匹配的属性：调用原始 native_get 函数获取真实值
    let original_native_get = ORIGINAL_NATIVE_GET.lock().unwrap();
    if let Some(orig_fn) = *original_native_get {
        return orig_fn(env, class, key, def);
    }

    // 如果原始函数不可用（不应该发生），返回默认值
    def
}
