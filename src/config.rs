use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

/// 机型设备信息模板
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
    /// 工作模式：
    /// - "lite": 只修改 Build 类（轻量模式，可卸载模块）
    /// - "full": Build + SystemProperties Hook（完整模式，不可卸载）
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// 全局默认模式："lite" 或 "full"
    #[serde(default = "default_mode")]
    pub default_mode: String,
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
                mode: template
                    .mode
                    .clone()
                    .unwrap_or_else(|| self.default_mode.clone()),
            });
        }

        None
    }

    /// 构建合并配置的系统属性映射
    pub fn build_merged_property_map(merged: &MergedAppConfig) -> HashMap<String, String> {
        let mut map = HashMap::new();

        if let Some(manufacturer) = &merged.manufacturer {
            map.insert("ro.product.manufacturer".to_string(), manufacturer.clone());
        }
        if let Some(brand) = &merged.brand {
            map.insert("ro.product.brand".to_string(), brand.clone());
        }
        if let Some(marketname) = &merged.marketname {
            map.insert("ro.product.marketname".to_string(), marketname.clone());
        }
        if let Some(model) = &merged.model {
            map.insert("ro.product.model".to_string(), model.clone());
        }
        if let Some(name) = &merged.name {
            map.insert("ro.product.name".to_string(), name.clone());
        }
        if let Some(device) = &merged.device {
            map.insert("ro.product.device".to_string(), device.clone());
        }
        if let Some(product) = &merged.product {
            map.insert("ro.build.product".to_string(), product.clone());
        }
        if let Some(fingerprint) = &merged.fingerprint {
            map.insert("ro.build.fingerprint".to_string(), fingerprint.clone());
        }

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
    pub mode: String,
}
