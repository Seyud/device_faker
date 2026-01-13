use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

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
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub characteristics: Option<String>,
    /// 是否为匹配的应用强制执行 FORCE_DENYLIST_UNMOUNT（默认继承全局设置）
    #[serde(default)]
    pub force_denylist_unmount: Option<bool>,
    /// 模板的工作模式（可选）
    #[serde(default)]
    pub mode: Option<String>,
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
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub characteristics: Option<String>,
    /// 是否为该应用强制执行 FORCE_DENYLIST_UNMOUNT（默认继承全局设置）
    #[serde(default)]
    pub force_denylist_unmount: Option<bool>,
    /// 工作模式：
    /// - "lite": 只修改 Build 类（轻量模式，可卸载模块）
    /// - "full": Build + SystemProperties Hook（完整模式，不可卸载）
    /// - "resetprop": 使用 resetprop 工具修改属性（需要 Root，不可卸载）
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// 全局默认模式："lite", "full" 或 "resetprop"
    #[serde(default = "default_mode")]
    pub default_mode: String,
    /// 是否默认启用 FORCE_DENYLIST_UNMOUNT（避免模块挂载痕迹）
    #[serde(default)]
    pub default_force_denylist_unmount: bool,
    /// 是否启用调试日志（默认关闭以提高隐蔽性）
    #[serde(default)]
    pub debug: bool,
    /// 机型设备模板定义
    #[serde(default)]
    pub templates: HashMap<String, DeviceTemplate>,
    /// 应用配置
    #[serde(default)]
    pub apps: Vec<AppConfig>,
}

fn default_mode() -> String {
    "lite".to_string() // 默认使用轻量模式，增强隐蔽性
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
    pub fn find_template_for_package(&self, package_name: &str) -> Option<&DeviceTemplate> {
        self.templates
            .values()
            .find(|template| template.packages.iter().any(|pkg| pkg == package_name))
    }

    /// 获取应用的最终配置（优先查找直接配置，其次查找模板的 packages 列表）
    pub fn get_merged_config(&self, package_name: &str) -> Option<MergedAppConfig> {
        // 优先查找直接配置的应用
        if let Some(app) = self.get_app_config(package_name) {
            return Some(MergedAppConfig {
                manufacturer: app.manufacturer.clone(),
                brand: app.brand.clone(),
                marketname: app.marketname.clone(),
                model: app.model.clone(),
                name: app.name.clone(),
                device: app.device.clone(),
                product: app.product.clone(),
                fingerprint: app.fingerprint.clone(),
                characteristics: app.characteristics.clone(),
                force_denylist_unmount: app
                    .force_denylist_unmount
                    .unwrap_or(self.default_force_denylist_unmount),
                mode: app
                    .mode
                    .clone()
                    .unwrap_or_else(|| self.default_mode.clone()),
            });
        }

        // 如果没有直接配置，查找模板的 packages 列表
        if let Some(template) = self.find_template_for_package(package_name) {
            return Some(MergedAppConfig {
                manufacturer: template.manufacturer.clone(),
                brand: template.brand.clone(),
                marketname: template.marketname.clone(),
                model: template.model.clone(),
                name: template.name.clone(),
                device: template.device.clone(),
                product: template.product.clone(),
                fingerprint: template.fingerprint.clone(),
                characteristics: template.characteristics.clone(),
                force_denylist_unmount: template
                    .force_denylist_unmount
                    .unwrap_or(self.default_force_denylist_unmount),
                mode: template
                    .mode
                    .clone()
                    .unwrap_or_else(|| self.default_mode.clone()),
            });
        }

        None
    }

    /// 构建合并配置的系统属性映射
    /// 注意：仅用于 full 模式的 SystemProperties Hook 和 resetprop 模式
    /// 空字符串会被忽略，不会添加到映射中
    pub fn build_merged_property_map(merged: &MergedAppConfig) -> HashMap<String, String> {
        let mut map = HashMap::new();

        if let Some(manufacturer) = &merged.manufacturer
            && !manufacturer.is_empty()
        {
            map.insert("ro.product.manufacturer".to_string(), manufacturer.clone());
        }
        if let Some(brand) = &merged.brand
            && !brand.is_empty()
        {
            map.insert("ro.product.brand".to_string(), brand.clone());
        }
        if let Some(marketname) = &merged.marketname
            && !marketname.is_empty()
        {
            map.insert("ro.product.marketname".to_string(), marketname.clone());
        }
        if let Some(model) = &merged.model
            && !model.is_empty()
        {
            map.insert("ro.product.model".to_string(), model.clone());
        }
        if let Some(name) = &merged.name
            && !name.is_empty()
        {
            map.insert("ro.product.name".to_string(), name.clone());
        }
        if let Some(device) = &merged.device
            && !device.is_empty()
        {
            map.insert("ro.product.device".to_string(), device.clone());
        } else if let Some(name) = &merged.name
            && !name.is_empty()
        {
            // Fallback to name if device is not set (legacy behavior)
            map.insert("ro.product.device".to_string(), name.clone());
        }

        if let Some(fingerprint) = &merged.fingerprint
            && !fingerprint.is_empty()
        {
            map.insert("ro.build.fingerprint".to_string(), fingerprint.clone());
        }

        if let Some(characteristics) = &merged.characteristics
            && !characteristics.is_empty()
        {
            map.insert(
                "ro.build.characteristics".to_string(),
                characteristics.clone(),
            );
        }

        map
    }

    /// 构建用于 resetprop 模式的系统属性映射（排除 characteristics）
    /// resetprop 模式暂时不支持 characteristics
    pub fn build_merged_property_map_for_resetprop(
        merged: &MergedAppConfig,
    ) -> HashMap<String, String> {
        let mut map = Self::build_merged_property_map(merged);
        map.remove("ro.build.characteristics");
        map
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
    pub fingerprint: Option<String>,
    pub characteristics: Option<String>,
    pub force_denylist_unmount: bool,
    pub mode: String,
}
