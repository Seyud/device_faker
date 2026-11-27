// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

//! 合并配置 → 系统属性表 / 删除列表。

use std::collections::{HashMap, HashSet};

use super::{
    families::{
        delete_build_family, delete_product_family, expand_known_custom_property,
        insert_build_family, insert_product_family,
    },
    fields::{NumberSetting, field_value},
    merged::MergedAppConfig,
    schema::Config,
};

impl Config {
    /// 构建合并配置的系统属性映射
    /// 空字符串会被忽略，不会添加到映射中
    /// __DELETE__ 标记的属性会被记录到 delete_props 中
    /// 构建属性表，同时返回 `custom_props` 里**逐字显式写出**的 key 集合。
    ///
    /// 显式 key 不受 [`super::families::is_patch_only_key`] 的「本机不存在就不写」门控：用户在
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
            // 营销名只「已存在才补齐」（见 [`super::families::is_patch_only_key`]）：真机上没有的营销名副本
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
