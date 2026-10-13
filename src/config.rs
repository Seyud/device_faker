use std::collections::{HashMap, HashSet};

use anyhow::Result;
use indexmap::IndexMap;
use serde::Deserialize;

pub const DPI_MIN: u32 = 120;
pub const DPI_MAX: u32 = 640;

fn valid_dpi(value: Option<u32>) -> Option<u32> {
    value.filter(|dpi| (DPI_MIN..=DPI_MAX).contains(dpi))
}

/// `ro.product.*` 会按属性来源顺序读取不同分区的副本。模板字段必须同步这些副本，
/// 否则检测程序可以从另一个分区读回真机值。
///
/// `ro.product.<分区>.<字段>` 需要覆盖 9 个分区：system / system_ext / product / vendor /
/// odm / bootimage / system_dlkm / vendor_dlkm / odm_dlkm。前 6 个是 AOSP 长期导出的；
/// 后 3 个动态内核模块分区只在 Android 13+ 的部分 ROM 上导出，因此它们走
/// [`is_patch_only_key`] 的「已存在才补齐」语义，避免在不支持的 ROM 上造出幽灵属性。
const PRODUCT_PARTITION_PREFIXES: &[&str] = &[
    "odm",
    "vendor",
    "system",
    "system_ext",
    "product",
    "bootimage",
    "system_dlkm",
    "vendor_dlkm",
    "odm_dlkm",
];

/// 只在部分 ROM 上存在的动态内核模块分区（system_dlkm / vendor_dlkm / odm_dlkm）。
const PATCH_ONLY_PARTITIONS: &[&str] = &["system_dlkm", "vendor_dlkm", "odm_dlkm"];

/// OEM 营销名别名：`ro.vendor.oplus.market.name` 只在 OnePlus/OPPO 上存在。
const OEM_MARKETNAME_ALIASES: &[&str] = &["ro.vendor.oplus.market.name"];

/// 该属性是否属于「本机可能不存在」的 key，即只允许 **已存在才补齐**。
///
/// 覆盖三类：
/// 1. 动态内核模块分区副本：`ro.product.{system_dlkm,vendor_dlkm,odm_dlkm}.<字段>`
///    与 `ro.{...}.build.<字段>`（Android 13+ 部分 ROM 才有）；
/// 2. OEM 营销名族 `ro.product[.<分区>].marketname`：OEM 通常只导出 base 与少数几个
///    分区副本，其余分区副本在多数 ROM 上并不存在；
/// 3. OEM 营销名别名（[`OEM_MARKETNAME_ALIASES`]）。
///
/// 设备上不存在的 key 若被写入，会在 `__system_property_foreach` / `getprop` 里
/// 留下真实设备没有的可枚举「幽灵属性」（设备信息类检测可据此发现伪装），而对 app
/// 来说真值与伪装值同样是 unknown —— 写入零收益、纯增指纹，故一律跳过不写。
pub fn is_patch_only_key(key: &str) -> bool {
    if OEM_MARKETNAME_ALIASES.contains(&key) {
        return true;
    }

    // 营销名族：`ro.product.marketname` 与 `ro.product.<分区>.marketname`
    if key.starts_with("ro.product.") && key.ends_with(".marketname") {
        return true;
    }

    let rest = key
        .strip_prefix("ro.product.")
        .or_else(|| key.strip_prefix("ro."));
    rest.is_some_and(|rest| {
        rest.split_once('.')
            .is_some_and(|(partition, _)| PATCH_ONLY_PARTITIONS.contains(&partition))
    })
}

/// 新版 Android 还为动态内核模块分区导出独立构建信息。
const BUILD_PARTITION_PREFIXES: &[&str] = &[
    "system",
    "system_ext",
    "product",
    "vendor",
    "odm",
    "bootimage",
    "system_dlkm",
    "vendor_dlkm",
    "odm_dlkm",
];

/// Android 构建指纹格式：brand/product/device:release/id/incremental:type/tags
struct FingerprintParts {
    brand: String,
    product: String,
    device: String,
    release: String,
    build_id: String,
    incremental: String,
    build_type: String,
    tags: String,
}

impl FingerprintParts {
    /// 解析标准格式指纹；字段数不符或存在空字段时返回 None（视为未配置指纹，
    /// 不做任何补全）。
    fn parse(fingerprint: &str) -> Option<Self> {
        let (identity, build) = fingerprint.split_once(':')?;
        let (version, variant) = build.split_once(':')?;

        let mut identity_parts = identity.split('/');
        let brand = identity_parts.next()?;
        let product = identity_parts.next()?;
        let device = identity_parts.next()?;
        if identity_parts.next().is_some() {
            return None;
        }

        let mut version_parts = version.split('/');
        let release = version_parts.next()?;
        let build_id = version_parts.next()?;
        let incremental = version_parts.next()?;
        if version_parts.next().is_some() {
            return None;
        }

        let mut variant_parts = variant.split('/');
        let build_type = variant_parts.next()?;
        let tags = variant_parts.next()?;
        if variant_parts.next().is_some()
            || [
                brand,
                product,
                device,
                release,
                build_id,
                incremental,
                build_type,
                tags,
            ]
            .iter()
            .any(|part| part.is_empty())
        {
            return None;
        }

        Some(Self {
            brand: brand.to_string(),
            product: product.to_string(),
            device: device.to_string(),
            release: release.to_string(),
            build_id: build_id.to_string(),
            incremental: incremental.to_string(),
            build_type: build_type.to_string(),
            tags: tags.to_string(),
        })
    }
}

/// 数值型伪装字段（目前只有 `sdk_int`）的设置值。
///
/// 数值字段无法用类型本身表达字符串标记，因此单独建模：
/// - `sdk_int = 35` → [`NumberSetting::Value`]
/// - `sdk_int = "__DELETE__"` → [`NumberSetting::Delete`]（与字符串字段的
///   `__DELETE__` 语义一致，删除 `ro.build.version.sdk` 整族；SDK_INT 由系统属性
///   派生，删除后相关读取会失败，属高风险操作，仅供明确需要的用户使用）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberSetting {
    Value(u32),
    Delete,
}

impl NumberSetting {
    /// 需要写入的伪装值；`__DELETE__` 返回 None（走删除列表）。
    pub fn value(self) -> Option<u32> {
        match self {
            Self::Value(value) => Some(value),
            Self::Delete => None,
        }
    }

    pub fn is_delete(self) -> bool {
        matches!(self, Self::Delete)
    }
}

impl<'de> Deserialize<'de> for NumberSetting {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SettingVisitor;

        impl serde::de::Visitor<'_> for SettingVisitor {
            type Value = NumberSetting;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("整数伪装值或 \"__DELETE__\"")
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                u32::try_from(value)
                    .map(NumberSetting::Value)
                    .map_err(|_| E::invalid_value(serde::de::Unexpected::Signed(value), &self))
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                u32::try_from(value)
                    .map(NumberSetting::Value)
                    .map_err(|_| E::invalid_value(serde::de::Unexpected::Unsigned(value), &self))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                if value == "__DELETE__" {
                    Ok(NumberSetting::Delete)
                } else {
                    Err(E::invalid_value(serde::de::Unexpected::Str(value), &self))
                }
            }
        }

        deserializer.deserialize_any(SettingVisitor)
    }
}

fn has_profile_value(value: &Option<String>) -> bool {
    value.as_ref().is_some_and(|value| !value.is_empty())
}

fn fill_profile_default(target: &mut Option<String>, value: impl Into<String>) {
    if !has_profile_value(target) {
        *target = Some(value.into());
    }
}

/// 顶层字段的有效伪装值：未设置、空串或 `__DELETE__`（走删除列表）返回 None，
/// `__EMPTY__` 归一化为空字符串。
fn field_value(field: &Option<String>) -> Option<String> {
    match field.as_deref() {
        Some(value) if !value.is_empty() && value != "__DELETE__" => {
            Some(if value == "__EMPTY__" {
                String::new()
            } else {
                value.to_string()
            })
        }
        _ => None,
    }
}

/// 为 `ro.product.<suffix>` 写入全部分区副本（bionic prefix routing 可能读取任一副本）。
///
/// 写入侧的「本机不存在就不写」由 [`is_patch_only_key`] 统一门控（COW 与 companion
/// 两条路径各自做存在性判断），涵盖：
/// - `system_dlkm` / `vendor_dlkm` / `odm_dlkm` 三个 dlkm 分区副本（只在部分 ROM 上存在）；
/// - `suffix == "marketname"` 的整族——OEM 营销名通常只有 base 与 odm/vendor 等少数副本，
///   其余分区副本在多数 ROM 上并不存在。
fn insert_product_family(map: &mut HashMap<String, String>, suffix: &str, value: &str) {
    map.insert(format!("ro.product.{suffix}"), value.to_string());
    for partition in PRODUCT_PARTITION_PREFIXES {
        map.insert(
            format!("ro.product.{partition}.{suffix}"),
            value.to_string(),
        );
    }
}

/// 为 `ro.build.<suffix>` 写入全部分区副本（含动态内核模块分区；dlkm 分区同样按
/// 「已存在才补齐」处理，见 [`is_patch_only_key`]）。
fn insert_build_family(map: &mut HashMap<String, String>, suffix: &str, value: &str) {
    map.insert(format!("ro.build.{suffix}"), value.to_string());
    for partition in BUILD_PARTITION_PREFIXES {
        map.insert(format!("ro.{partition}.build.{suffix}"), value.to_string());
    }
}

fn delete_product_family(delete_props: &mut Vec<String>, suffix: &str) {
    delete_props.push(format!("ro.product.{suffix}"));
    for partition in PRODUCT_PARTITION_PREFIXES {
        delete_props.push(format!("ro.product.{partition}.{suffix}"));
    }
}

fn delete_build_family(delete_props: &mut Vec<String>, suffix: &str) {
    delete_props.push(format!("ro.build.{suffix}"));
    for partition in BUILD_PARTITION_PREFIXES {
        delete_props.push(format!("ro.{partition}.build.{suffix}"));
    }
}

/// custom_props 中已知属性族的 key 自动整族展开；未知 key 不展开，
/// 随后的精确 key 写入仍具有最高优先级。
fn expand_known_custom_property(map: &mut HashMap<String, String>, key: &str, value: &str) {
    match key {
        "ro.product.manufacturer" => insert_product_family(map, "manufacturer", value),
        "ro.product.brand" => insert_product_family(map, "brand", value),
        "ro.product.model" => insert_product_family(map, "model", value),
        "ro.product.name" => insert_product_family(map, "name", value),
        "ro.product.device" => insert_product_family(map, "device", value),
        "ro.product.marketname" => insert_product_family(map, "marketname", value),
        "ro.build.fingerprint" => insert_build_family(map, "fingerprint", value),
        "ro.build.id" => insert_build_family(map, "id", value),
        "ro.build.version.release" => insert_build_family(map, "version.release", value),
        "ro.build.version.sdk" => insert_build_family(map, "version.sdk", value),
        "ro.build.version.incremental" => insert_build_family(map, "version.incremental", value),
        "ro.build.type" => insert_build_family(map, "type", value),
        "ro.build.tags" => insert_build_family(map, "tags", value),
        _ => {}
    }
}

/// 机型模板
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceTemplate {
    /// 包名列表
    #[serde(default)]
    pub packages: Vec<String>,
    /// 设备信息
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub marketname: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub hardware: Option<String>,
    #[serde(default)]
    pub board: Option<String>,
    /// SoC 型号伪装（映射 Build.SOC_MODEL + ro.soc.model，单属性无分区副本）
    #[serde(default)]
    pub soc_model: Option<String>,
    #[serde(default)]
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub build_id: Option<String>,
    /// 系统版本号（映射 Build.DISPLAY + ro.build.display.id，单属性无分区副本）
    #[serde(default)]
    pub display_id: Option<String>,
    /// 构建增量号（映射 Build.VERSION.INCREMENTAL + ro.build.version.incremental 族）
    #[serde(default)]
    pub incremental: Option<String>,
    /// 安全补丁日期（映射 Build.VERSION.SECURITY_PATCH + ro.build.version.security_patch 族）
    #[serde(default)]
    pub security_patch: Option<String>,
    #[serde(default)]
    pub characteristics: Option<String>,
    /// Android 版本伪装（如 "15", "14"）
    #[serde(default)]
    pub android_version: Option<String>,
    /// SDK 版本伪装（如 35, 34）；`"__DELETE__"` 删除 `ro.build.version.sdk` 整族
    #[serde(default)]
    pub sdk_int: Option<NumberSetting>,
    /// 临时覆盖系统显示密度（120–640），由 companion 使用 `wm density` 应用
    #[serde(default)]
    pub dpi: Option<u32>,
    /// 自定义属性映射表
    #[serde(default)]
    pub custom_props: Option<HashMap<String, String>>,
    /// 是否为匹配的应用强制执行 FORCE_DENYLIST_UNMOUNT（默认继承全局设置）
    #[serde(default)]
    pub force_denylist_unmount: Option<bool>,
    /// 要从 /proc/self/maps 中清除的属性映射模式列表（默认继承全局设置）
    #[serde(default)]
    pub hide_maps: Option<Vec<String>>,
    /// 是否跳过 COW 属性伪造，所有属性直接交给 companion resetprop 处理
    /// true 时 getprop（独立进程）和进程内读取一致，适用于属性一致性对比检测
    #[serde(default)]
    pub companion_resetprop: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub package: String,
    /// 直接指定设备信息
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub marketname: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub hardware: Option<String>,
    #[serde(default)]
    pub board: Option<String>,
    /// SoC 型号伪装（映射 Build.SOC_MODEL + ro.soc.model，单属性无分区副本）
    #[serde(default)]
    pub soc_model: Option<String>,
    #[serde(default)]
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub build_id: Option<String>,
    /// 系统版本号（映射 Build.DISPLAY + ro.build.display.id，单属性无分区副本）
    #[serde(default)]
    pub display_id: Option<String>,
    /// 构建增量号（映射 Build.VERSION.INCREMENTAL + ro.build.version.incremental 族）
    #[serde(default)]
    pub incremental: Option<String>,
    /// 安全补丁日期（映射 Build.VERSION.SECURITY_PATCH + ro.build.version.security_patch 族）
    #[serde(default)]
    pub security_patch: Option<String>,
    #[serde(default)]
    pub characteristics: Option<String>,
    /// Android 版本伪装（如 "15", "14"）
    #[serde(default)]
    pub android_version: Option<String>,
    /// SDK 版本伪装（如 35, 34）；`"__DELETE__"` 删除 `ro.build.version.sdk` 整族
    #[serde(default)]
    pub sdk_int: Option<NumberSetting>,
    /// 临时覆盖系统显示密度（120–640），由 companion 使用 `wm density` 应用
    #[serde(default)]
    pub dpi: Option<u32>,
    /// 自定义属性映射表
    #[serde(default)]
    pub custom_props: Option<HashMap<String, String>>,
    /// 是否为该应用强制执行 FORCE_DENYLIST_UNMOUNT（默认继承全局设置）
    #[serde(default)]
    pub force_denylist_unmount: Option<bool>,
    /// 要从 /proc/self/maps 中清除的属性映射模式列表（默认继承全局设置）
    #[serde(default)]
    pub hide_maps: Option<Vec<String>>,
    /// 是否跳过 COW 属性伪造，所有属性直接交给 companion resetprop 处理
    /// true 时 getprop（独立进程）和进程内读取一致，适用于属性一致性对比检测
    #[serde(default)]
    pub companion_resetprop: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// 是否默认启用 FORCE_DENYLIST_UNMOUNT（避免模块挂载痕迹）
    #[serde(default)]
    pub default_force_denylist_unmount: bool,
    /// 是否启用调试日志（默认关闭以提高隐蔽性）
    #[serde(default)]
    pub debug: bool,
    /// 机型设备模板定义
    ///
    /// 用 IndexMap 而非 HashMap：保留 config.toml 中的书写顺序，使
    /// find_template_for_package 在「同一包名被多个模板声明」时命中确定的
    /// 那一个（书写顺序靠前者），而不是 HashMap 的进程级随机迭代顺序。
    #[serde(default)]
    pub templates: IndexMap<String, DeviceTemplate>,
    /// 应用配置
    #[serde(default)]
    pub apps: Vec<AppConfig>,
    /// 要从 /proc/self/maps 中清除的属性映射模式列表（默认不启用）
    #[serde(default)]
    pub default_hide_maps: Vec<String>,
}

impl Config {
    pub fn from_toml(content: &str) -> Result<Self> {
        Ok(toml::from_str(content)?)
    }

    /// 查找包名对应的应用配置（优先）或模板配置
    pub fn get_app_config(&self, package_name: &str) -> Option<&AppConfig> {
        self.apps.iter().find(|app| app.package == package_name)
    }

    /// 查找包名对应的模板（从模板的 packages 列表中查找）
    ///
    /// 多个模板声明同一包名时命中**书写顺序最先**的那一个。Config.templates 是
    /// IndexMap（配合 toml 的 preserve_order），顺序即 config.toml 中的出现顺序，
    /// 因此结果是确定的、跨进程一致的——不依赖 HashMap 的随机迭代顺序。
    pub fn find_template_for_package(&self, package_name: &str) -> Option<&DeviceTemplate> {
        self.templates
            .iter()
            .find(|(_, template)| template.packages.iter().any(|pkg| pkg == package_name))
            .map(|(_, template)| template)
    }

    /// 找出所有声明了该包名的模板名（按书写顺序）。
    ///
    /// 长度 > 1 表示配置存在歧义：只有第一个会真正生效，其余被静默丢弃。
    /// 用于诊断日志/告警，调用方可据此提示用户「一个包只应挂一个模板」。
    pub fn conflicting_templates_for_package(&self, package_name: &str) -> Vec<&str> {
        self.templates
            .iter()
            .filter(|(_, template)| template.packages.iter().any(|pkg| pkg == package_name))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// 获取应用的最终配置（优先查找直接配置，其次查找模板的 packages 列表）
    pub fn get_merged_config(&self, package_name: &str) -> Option<MergedAppConfig> {
        // 优先查找直接配置的应用
        if let Some(app) = self.get_app_config(package_name) {
            let mut merged = MergedAppConfig {
                manufacturer: app.manufacturer.clone(),
                brand: app.brand.clone(),
                marketname: app.marketname.clone(),
                model: app.model.clone(),
                name: app.name.clone(),
                device: app.device.clone(),
                product: app.product.clone(),
                hardware: app.hardware.clone(),
                board: app.board.clone(),
                soc_model: app.soc_model.clone(),
                fingerprint: app.fingerprint.clone(),
                build_id: app.build_id.clone(),
                display_id: app.display_id.clone(),
                incremental: app.incremental.clone(),
                security_patch: app.security_patch.clone(),
                characteristics: app.characteristics.clone(),
                android_version: app.android_version.clone(),
                sdk_int: app.sdk_int,
                dpi: valid_dpi(app.dpi),
                custom_props: app.custom_props.clone(),
                force_denylist_unmount: app
                    .force_denylist_unmount
                    .unwrap_or(self.default_force_denylist_unmount),
                hide_maps: app
                    .hide_maps
                    .clone()
                    .unwrap_or_else(|| self.default_hide_maps.clone()),
                companion_resetprop: app.companion_resetprop.unwrap_or(false),
                derived_props: HashSet::new(),
            };
            merged.complete_device_profile();
            return Some(merged);
        }

        // 如果没有直接配置，查找模板的 packages 列表
        if let Some(template) = self.find_template_for_package(package_name) {
            let mut merged = MergedAppConfig {
                manufacturer: template.manufacturer.clone(),
                brand: template.brand.clone(),
                marketname: template.marketname.clone(),
                model: template.model.clone(),
                name: template.name.clone(),
                device: template.device.clone(),
                product: template.product.clone(),
                hardware: template.hardware.clone(),
                board: template.board.clone(),
                soc_model: template.soc_model.clone(),
                fingerprint: template.fingerprint.clone(),
                build_id: template.build_id.clone(),
                display_id: template.display_id.clone(),
                incremental: template.incremental.clone(),
                security_patch: template.security_patch.clone(),
                characteristics: template.characteristics.clone(),
                android_version: template.android_version.clone(),
                sdk_int: template.sdk_int,
                dpi: valid_dpi(template.dpi),
                custom_props: template.custom_props.clone(),
                force_denylist_unmount: template
                    .force_denylist_unmount
                    .unwrap_or(self.default_force_denylist_unmount),
                hide_maps: template
                    .hide_maps
                    .clone()
                    .unwrap_or_else(|| self.default_hide_maps.clone()),
                companion_resetprop: template.companion_resetprop.unwrap_or(false),
                derived_props: HashSet::new(),
            };
            merged.complete_device_profile();
            return Some(merged);
        }

        None
    }

    /// 构建合并配置的系统属性映射
    /// 空字符串会被忽略，不会添加到映射中
    /// __DELETE__ 标记的属性会被记录到 delete_props 中
    /// 构建属性表，同时返回 `custom_props` 里**逐字显式写出**的 key 集合。
    ///
    /// 显式 key 不受 [`is_patch_only_key`] 的「本机不存在就不写」门控：用户在
    /// `custom_props` 里指名道姓写 `"ro.vendor.oplus.market.name" = "…"`（例如在非 OPPO
    /// 机型上伪装成 OPPO）就是要求写入该 key，写不写由用户决定。门控只针对**自动展开**出来的
    /// 分区/属性族变体——那是模块为覆盖 bionic 前缀路由而自作主张补的副本，真机上不存在
    /// 时会变成可枚举的幽灵属性。
    ///
    /// 注意：`custom_props."ro.product.marketname"` 这类**族头**只把该族头本身算作显式，
    /// 它展开出的分区副本仍走门控。
    pub fn build_merged_property_map_with_explicit(
        merged: &MergedAppConfig,
    ) -> (HashMap<String, String>, HashSet<String>) {
        let mut map = HashMap::new();
        let mut explicit: HashSet<String> = HashSet::new();

        // 分区副本同步：bionic prefix routing 可能从任一分区的 prop_area 读取，
        // 检测程序也可以直接指定分区变体读回真机值，必须整族写入。
        if let Some(manufacturer) = field_value(&merged.manufacturer) {
            insert_product_family(&mut map, "manufacturer", &manufacturer);
        }
        if let Some(brand) = field_value(&merged.brand) {
            insert_product_family(&mut map, "brand", &brand);
        }
        if let Some(marketname) = field_value(&merged.marketname) {
            // 营销名只「已存在才补齐」（见 [`is_patch_only_key`]）：真机上没有的营销名副本
            // （含下列 OPPO 别名在不支持该 prop 的 ROM 上）一律不写，避免造出可枚举幽灵属性。
            insert_product_family(&mut map, "marketname", &marketname);
            // OnePlus/OPPO 设备读 ro.vendor.oplus.market.name 而非 ro.product.marketname
            map.insert(
                "ro.vendor.oplus.market.name".to_string(),
                marketname.clone(),
            );
        }
        if let Some(model) = field_value(&merged.model) {
            insert_product_family(&mut map, "model", &model);
        }

        // Build.PRODUCT 的属性来源是 ro.product.name；product 与 name 已在
        // complete_device_profile 中同步为同一个产品代号，这里取非空值写入。
        if let Some(product) = field_value(&merged.product).or_else(|| field_value(&merged.name)) {
            insert_product_family(&mut map, "name", &product);
            map.insert("ro.build.product".to_string(), product.clone());
        }

        if let Some(device) = field_value(&merged.device) {
            insert_product_family(&mut map, "device", &device);
        }

        if let Some(hardware) = field_value(&merged.hardware) {
            map.insert("ro.hardware".to_string(), hardware);
        }

        // Build.BOARD 的属性来源是 ro.product.board（单一属性，AOSP 不导出分区
        // 副本——真实设备上只有这一个 key，插入不存在的分区变体会制造可被枚举
        // 检测的幽灵属性，故与 ro.hardware 同形态、不做整族展开）。
        if let Some(board) = field_value(&merged.board) {
            map.insert("ro.product.board".to_string(), board);
        }

        // Build.SOC_MODEL 的属性来源是 ro.soc.model（单一属性，同 hardware/board
        // 形态，无分区变体）。
        if let Some(soc_model) = field_value(&merged.soc_model) {
            map.insert("ro.soc.model".to_string(), soc_model);
        }

        if let Some(fingerprint) = field_value(&merged.fingerprint) {
            insert_build_family(&mut map, "fingerprint", &fingerprint);
        }

        if let Some(build_id) = field_value(&merged.build_id) {
            insert_build_family(&mut map, "id", &build_id);
        }

        // Build.DISPLAY 的属性来源是 ro.build.display.id（单一属性，同 hardware/board）
        if let Some(display_id) = field_value(&merged.display_id) {
            map.insert("ro.build.display.id".to_string(), display_id);
        }

        if let Some(incremental) = field_value(&merged.incremental) {
            insert_build_family(&mut map, "version.incremental", &incremental);
        }

        // Build.VERSION.SECURITY_PATCH 的属性来源是 ro.build.version.security_patch，
        // 各分区（system/vendor/product 等）均导出自己的副本，与 version.sdk 同族。
        if let Some(security_patch) = field_value(&merged.security_patch) {
            insert_build_family(&mut map, "version.security_patch", &security_patch);
            // vendor 分区另导出一个无 .version. 段的裸名副本
            map.insert("ro.vendor.build.security_patch".to_string(), security_patch);
        }

        if let Some(characteristics) = field_value(&merged.characteristics) {
            map.insert(
                "ro.build.characteristics".to_string(),
                characteristics.clone(),
            );
        }

        // Android 版本伪装属性
        if let Some(android_version) = field_value(&merged.android_version) {
            insert_build_family(&mut map, "version.release", &android_version);
        }

        if let Some(sdk_int) = merged.sdk_int.and_then(NumberSetting::value) {
            insert_build_family(&mut map, "version.sdk", &sdk_int.to_string());
        }

        // 自定义属性：已知属性族先整族展开，精确 key 随后覆盖（优先级最高）
        if let Some(custom_props) = &merged.custom_props {
            for (key, value) in custom_props {
                if value == "__DELETE__" {
                    continue;
                }
                let final_value = if value == "__EMPTY__" { "" } else { value };
                expand_known_custom_property(&mut map, key, final_value);
                // 用户显式写出的 key 本身（不含上面展开出的分区副本）不受
                // `is_patch_only_key` 门控，见函数文档；模块自行注入的派生键
                // （derived_props）不算显式。
                if !merged.derived_props.contains(key) {
                    explicit.insert(key.clone());
                }
            }
            for (key, value) in custom_props {
                if value == "__DELETE__" {
                    continue;
                }
                let final_value = if value == "__EMPTY__" { "" } else { value };
                map.insert(key.clone(), final_value.to_string());
            }
        }

        (map, explicit)
    }

    /// 构建需要删除的属性列表（用于 companion 模式）。
    /// 与 build_merged_property_map 的属性族展开保持对称：
    /// `__DELETE__` 删除整个属性族（含全部分区副本）。
    pub fn build_delete_props_list(merged: &MergedAppConfig) -> Vec<String> {
        let mut delete_props = Vec::new();

        if merged.brand.as_ref().is_some_and(|s| s == "__DELETE__") {
            delete_product_family(&mut delete_props, "brand");
        }
        if merged
            .manufacturer
            .as_ref()
            .is_some_and(|s| s == "__DELETE__")
        {
            delete_product_family(&mut delete_props, "manufacturer");
        }
        if merged.model.as_ref().is_some_and(|s| s == "__DELETE__") {
            delete_product_family(&mut delete_props, "model");
        }
        if merged.name.as_ref().is_some_and(|s| s == "__DELETE__") {
            delete_product_family(&mut delete_props, "name");
            delete_props.push("ro.build.product".to_string());
        }
        if merged.device.as_ref().is_some_and(|s| s == "__DELETE__") {
            delete_product_family(&mut delete_props, "device");
        }
        if merged
            .marketname
            .as_ref()
            .is_some_and(|s| s == "__DELETE__")
        {
            delete_product_family(&mut delete_props, "marketname");
            delete_props.push("ro.vendor.oplus.market.name".to_string());
        }
        if merged
            .fingerprint
            .as_ref()
            .is_some_and(|s| s == "__DELETE__")
        {
            delete_build_family(&mut delete_props, "fingerprint");
        }
        if merged.build_id.as_ref().is_some_and(|s| s == "__DELETE__") {
            delete_build_family(&mut delete_props, "id");
        }
        if merged
            .display_id
            .as_ref()
            .is_some_and(|s| s == "__DELETE__")
        {
            delete_props.push("ro.build.display.id".to_string());
        }
        if merged
            .incremental
            .as_ref()
            .is_some_and(|s| s == "__DELETE__")
        {
            delete_build_family(&mut delete_props, "version.incremental");
        }
        if merged
            .security_patch
            .as_ref()
            .is_some_and(|s| s == "__DELETE__")
        {
            delete_build_family(&mut delete_props, "version.security_patch");
            delete_props.push("ro.vendor.build.security_patch".to_string());
        }
        if merged
            .characteristics
            .as_ref()
            .is_some_and(|s| s == "__DELETE__")
        {
            delete_props.push("ro.build.characteristics".to_string());
        }
        if merged.hardware.as_ref().is_some_and(|s| s == "__DELETE__") {
            delete_props.push("ro.hardware".to_string());
        }
        if merged.board.as_ref().is_some_and(|s| s == "__DELETE__") {
            delete_props.push("ro.product.board".to_string());
        }
        if merged.soc_model.as_ref().is_some_and(|s| s == "__DELETE__") {
            delete_props.push("ro.soc.model".to_string());
        }
        if merged
            .android_version
            .as_ref()
            .is_some_and(|s| s == "__DELETE__")
        {
            delete_build_family(&mut delete_props, "version.release");
        }
        if merged.sdk_int.is_some_and(NumberSetting::is_delete) {
            delete_build_family(&mut delete_props, "version.sdk");
        }

        if let Some(custom_props) = &merged.custom_props {
            for (key, value) in custom_props {
                if value == "__DELETE__" {
                    delete_props.push(key.clone());
                }
            }
        }

        delete_props
    }
}

/// 合并后的应用配置（模板 + 直接配置）
#[derive(Debug, Clone)]
pub struct MergedAppConfig {
    pub manufacturer: Option<String>,
    pub brand: Option<String>,
    pub marketname: Option<String>,
    pub model: Option<String>,
    pub name: Option<String>,
    pub device: Option<String>,
    pub product: Option<String>,
    pub hardware: Option<String>,
    pub board: Option<String>,
    pub soc_model: Option<String>,
    pub fingerprint: Option<String>,
    pub build_id: Option<String>,
    /// 系统版本号 → Build.DISPLAY + ro.build.display.id
    pub display_id: Option<String>,
    /// 构建增量号 → Build.VERSION.INCREMENTAL + ro.build.version.incremental 族
    pub incremental: Option<String>,
    /// 安全补丁日期 → Build.VERSION.SECURITY_PATCH + ro.build.version.security_patch 族
    pub security_patch: Option<String>,
    pub characteristics: Option<String>,
    pub android_version: Option<String>,
    pub sdk_int: Option<NumberSetting>,
    /// 最终显示密度覆盖值，超出 120–640 的配置会被忽略
    pub dpi: Option<u32>,
    pub custom_props: Option<HashMap<String, String>>,
    pub force_denylist_unmount: bool,
    /// 要从 /proc/self/maps 中清除的属性映射模式列表
    pub hide_maps: Vec<String>,
    /// 是否跳过 COW，所有属性走 companion resetprop（默认 false）
    pub companion_resetprop: bool,
    /// 模块**自行注入** `custom_props` 的派生键（目前只有指纹解析出的
    /// `ro.build.type` / `ro.build.tags`，见 `complete_device_profile`）。
    /// 这些键不算「用户显式写出」，因此**不享受** patch-only 门控豁免
    /// （见 `build_merged_property_map_with_explicit` 的 `explicit` 集合）。
    pub derived_props: HashSet<String>,
}

impl MergedAppConfig {
    /// 用构建指纹补齐未显式填写的标准字段，并同步指纹内携带的构建信息。
    ///
    /// 显式模板字段始终优先；指纹只负责补空，不会覆盖用户主动设置的
    /// 品牌、设备代号等。指纹携带的 incremental/type/tags 以 custom_props
    /// 条目形式补入（不覆盖用户显式配置），随后按属性族展开生效。
    fn complete_device_profile(&mut self) {
        let parsed = self
            .fingerprint
            .as_deref()
            .and_then(FingerprintParts::parse);

        if let Some(parts) = &parsed {
            fill_profile_default(&mut self.brand, parts.brand.clone());
            fill_profile_default(&mut self.product, parts.product.clone());
            fill_profile_default(&mut self.device, parts.device.clone());
            fill_profile_default(&mut self.android_version, parts.release.clone());
            fill_profile_default(&mut self.build_id, parts.build_id.clone());
            // 指纹 incremental 同步进专用字段，保证 Build.VERSION.INCREMENTAL 与
            // ro.build.version.incremental 一致（原先只进 custom_props 会漏 hook）
            fill_profile_default(&mut self.incremental, parts.incremental.clone());
        }

        // Build.PRODUCT 的属性来源是 ro.product.name，而本项目的 product 与
        // name 语义相同（产品代号）。两者必须同步，否则 JNI 字段与系统属性
        // 会分别显示新旧值。
        if !has_profile_value(&self.product)
            && let Some(name) = self.name.as_ref().filter(|value| !value.is_empty())
        {
            self.product = Some(name.clone());
        }
        if !has_profile_value(&self.name)
            && let Some(product) = self.product.as_ref().filter(|value| !value.is_empty())
        {
            self.name = Some(product.clone());
        }
        if !has_profile_value(&self.device)
            && let Some(product) = self.product.as_ref().filter(|value| !value.is_empty())
        {
            self.device = Some(product.clone());
        }

        if let Some(parts) = parsed {
            let props = self.custom_props.get_or_insert_with(HashMap::new);
            // 指纹派生键：用户没显式写时才注入，并登记为「派生」（不受 patch-only
            // 门控豁免，理由见 MergedAppConfig::derived_props）。
            for (key, value) in [
                ("ro.build.type", parts.build_type),
                ("ro.build.tags", parts.tags),
            ] {
                if !props.contains_key(key) {
                    props.insert(key.to_string(), value);
                    self.derived_props.insert(key.to_string());
                }
            }
        }
    }
}
