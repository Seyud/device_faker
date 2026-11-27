// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

//! 属性族（分区副本）展开与「本机不存在就不写」门控。

use std::collections::HashMap;

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

/// 为 `ro.product.<suffix>` 写入全部分区副本（bionic prefix routing 可能读取任一副本）。
///
/// 写入侧的「本机不存在就不写」由 [`is_patch_only_key`] 统一门控（COW 与 companion
/// 两条路径各自做存在性判断），涵盖：
/// - `system_dlkm` / `vendor_dlkm` / `odm_dlkm` 三个 dlkm 分区副本（只在部分 ROM 上存在）；
/// - `suffix == "marketname"` 的整族——OEM 营销名通常只有 base 与 odm/vendor 等少数副本，
///   其余分区副本在多数 ROM 上并不存在。
pub(super) fn insert_product_family(map: &mut HashMap<String, String>, suffix: &str, value: &str) {
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
pub(super) fn insert_build_family(map: &mut HashMap<String, String>, suffix: &str, value: &str) {
    map.insert(format!("ro.build.{suffix}"), value.to_string());
    for partition in BUILD_PARTITION_PREFIXES {
        map.insert(format!("ro.{partition}.build.{suffix}"), value.to_string());
    }
}

pub(super) fn delete_product_family(delete_props: &mut Vec<String>, suffix: &str) {
    delete_props.push(format!("ro.product.{suffix}"));
    for partition in PRODUCT_PARTITION_PREFIXES {
        delete_props.push(format!("ro.product.{partition}.{suffix}"));
    }
}

pub(super) fn delete_build_family(delete_props: &mut Vec<String>, suffix: &str) {
    delete_props.push(format!("ro.build.{suffix}"));
    for partition in BUILD_PARTITION_PREFIXES {
        delete_props.push(format!("ro.{partition}.build.{suffix}"));
    }
}

/// custom_props 中已知属性族的 key 自动整族展开；未知 key 不展开，
/// 随后的精确 key 写入仍具有最高优先级。
pub(super) fn expand_known_custom_property(
    map: &mut HashMap<String, String>,
    key: &str,
    value: &str,
) {
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
