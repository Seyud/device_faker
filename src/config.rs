use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub package: String,
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
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub apps: Vec<AppConfig>,
}

impl Config {
    pub fn from_toml(content: &str) -> Result<Self> {
        Ok(toml::from_str(content)?)
    }

    pub fn get_app_config(&self, package_name: &str) -> Option<&AppConfig> {
        self.apps.iter().find(|app| app.package == package_name)
    }

    /// 构建系统属性映射（用于 SystemProperties Hook）
    pub fn build_property_map(app_config: &AppConfig) -> HashMap<String, String> {
        let mut map = HashMap::new();

        // 只添加配置中存在的属性
        if let Some(manufacturer) = &app_config.manufacturer {
            map.insert("ro.product.manufacturer".to_string(), manufacturer.clone());
        }
        if let Some(brand) = &app_config.brand {
            map.insert("ro.product.brand".to_string(), brand.clone());
        }
        if let Some(marketname) = &app_config.marketname {
            map.insert("ro.product.marketname".to_string(), marketname.clone());
        }
        if let Some(model) = &app_config.model {
            map.insert("ro.product.model".to_string(), model.clone());
        }
        if let Some(name) = &app_config.name {
            map.insert("ro.product.name".to_string(), name.clone());
        }

        map
    }
}
