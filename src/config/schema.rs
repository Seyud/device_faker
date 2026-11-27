// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

//! config.toml 的 serde schema 与「包 → 模板」查找。

use std::collections::HashMap;

use anyhow::Result;
use indexmap::IndexMap;
use serde::Deserialize;

use super::fields::NumberSetting;

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
}
