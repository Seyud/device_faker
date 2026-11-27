// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

//! 合并后的应用配置：模板/直配合并 + 指纹补全。

use std::collections::{HashMap, HashSet};

use super::{
    fields::{NumberSetting, fill_profile_default, has_profile_value, valid_dpi},
    fingerprint::FingerprintParts,
    schema::Config,
};

impl Config {
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
            // ro.build.version.incremental 一致（原先只进 custom_props 会漏掉伪装）
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
