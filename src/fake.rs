// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

use anyhow::Context;
use jni::{
    Env, EnvUnowned, jni_sig, jni_str,
    objects::{JClass, JValue},
    strings::JNIStr,
};
use log::debug;

use crate::config::{MergedAppConfig, NumberSetting};

/// Build 字段的有效伪装值：与 config::field_value 相同的归一化——
/// 未设置/空串/`__DELETE__` 返回 None（跳过伪装），`__EMPTY__` 归一化为空串。
fn field_str(field: &Option<String>) -> Option<&str> {
    match field.as_deref() {
        Some(value) if !value.is_empty() && value != "__DELETE__" => {
            Some(if value == "__EMPTY__" { "" } else { value })
        }
        _ => None,
    }
}

/// 根据合并配置伪装 android.os.Build 的静态字段。
pub fn fake_build_fields(
    env: &mut EnvUnowned,
    merged_config: &MergedAppConfig,
) -> anyhow::Result<()> {
    env.with_env(|jenv| -> Result<(), jni::errors::Error> {
        let build_class = jenv.find_class(jni_str!("android/os/Build"))?;

        if let Some(manufacturer) = field_str(&merged_config.manufacturer) {
            set_build_field(jenv, &build_class, jni_str!("MANUFACTURER"), manufacturer)
                .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;
        }

        if let Some(brand) = field_str(&merged_config.brand) {
            set_build_field(jenv, &build_class, jni_str!("BRAND"), brand)
                .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;
        }

        if let Some(model) = field_str(&merged_config.model) {
            set_build_field(jenv, &build_class, jni_str!("MODEL"), model)
                .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;
        }

        if let Some(device) = field_str(&merged_config.device) {
            set_build_field(jenv, &build_class, jni_str!("DEVICE"), device)
                .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;
        }

        if let Some(product) = field_str(&merged_config.product) {
            set_build_field(jenv, &build_class, jni_str!("PRODUCT"), product)
                .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;
        }

        // HARDWARE 字段
        if let Some(hardware) = field_str(&merged_config.hardware) {
            set_build_field(jenv, &build_class, jni_str!("HARDWARE"), hardware)
                .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;
        }

        // BOARD 字段
        if let Some(board) = field_str(&merged_config.board) {
            set_build_field(jenv, &build_class, jni_str!("BOARD"), board)
                .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;
        }

        // SOC_MODEL 字段（API 29+ 存在；Build.SOC_MODEL 的属性来源是 ro.soc.model）
        if let Some(soc_model) = field_str(&merged_config.soc_model) {
            set_build_field(jenv, &build_class, jni_str!("SOC_MODEL"), soc_model)
                .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;
        }

        if let Some(fingerprint) = field_str(&merged_config.fingerprint) {
            set_build_field(jenv, &build_class, jni_str!("FINGERPRINT"), fingerprint)
                .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;
        }

        if let Some(build_id) = field_str(&merged_config.build_id) {
            set_build_field(jenv, &build_class, jni_str!("ID"), build_id)
                .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;
        }

        // Build.DISPLAY 的属性来源是 ro.build.display.id（单一属性，无分区副本）
        if let Some(display_id) = field_str(&merged_config.display_id) {
            set_build_field(jenv, &build_class, jni_str!("DISPLAY"), display_id)
                .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;
        }

        fake_attestation_fields(jenv, &build_class, merged_config)
            .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;

        fake_version_fields(jenv, &build_class, merged_config)
            .map_err(|_e| jni::errors::Error::JniCall(jni::errors::JniError::Unknown))?;

        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    Ok(())
}

/// 伪装 Android 15+ 的 attestation 身份字段（`Build.*_FOR_ATTESTATION`）。
///
/// 这组字段由 zygote 预初始化 `android.os.Build` 时按 `ro.product.*_for_attestation`
/// （缺失时回落 `ro.product.vendor.*`）算好、fork 后继承；而该属性落在
/// `build_attestation_prop` 上下文——app 域连 area 都不映射，属性侧 COW 改不动 app 看到的
/// 字段值，因此只能在 Java 层改写。字段在本 ROM 上不存在（Android < 15 或 ROM 未导出）
/// 时静默跳过。
///
/// 值与公开身份字段同源：`MANUFACTURER` ← manufacturer、`BRAND` ← brand、
/// `DEVICE` ← device、`MODEL` ← model、`PRODUCT` ← product（回落 name）。
fn fake_attestation_fields(
    env: &mut Env,
    build_class: &JClass,
    merged_config: &MergedAppConfig,
) -> anyhow::Result<()> {
    set_attestation_field(
        env,
        build_class,
        jni_str!("MANUFACTURER_FOR_ATTESTATION"),
        field_str(&merged_config.manufacturer),
    )?;
    set_attestation_field(
        env,
        build_class,
        jni_str!("BRAND_FOR_ATTESTATION"),
        field_str(&merged_config.brand),
    )?;
    set_attestation_field(
        env,
        build_class,
        jni_str!("DEVICE_FOR_ATTESTATION"),
        field_str(&merged_config.device),
    )?;
    set_attestation_field(
        env,
        build_class,
        jni_str!("MODEL_FOR_ATTESTATION"),
        field_str(&merged_config.model),
    )?;
    set_attestation_field(
        env,
        build_class,
        jni_str!("PRODUCT_FOR_ATTESTATION"),
        field_str(&merged_config.product).or_else(|| field_str(&merged_config.name)),
    )?;

    Ok(())
}

/// 写入「本 ROM 可能不存在」的 attestation 字段：未配置值或字段缺失时只记 debug 日志并跳过。
///
/// `get_static_field_id` 内部会捕获并清除 `NoSuchFieldError`（jni crate 行为），
/// 因此探测失败不会给后续 JNI 调用留下挂起异常。
fn set_attestation_field(
    env: &mut Env,
    build_class: &JClass,
    field_name: &JNIStr,
    value: Option<&str>,
) -> anyhow::Result<()> {
    let Some(value) = value else {
        return Ok(());
    };

    if env
        .get_static_field_id(build_class, field_name, jni_sig!("Ljava/lang/String;"))
        .is_err()
    {
        debug!(
            "Build.{} absent on this ROM, attestation fake skipped",
            field_name.to_str()
        );
        return Ok(());
    }

    set_build_field(env, build_class, field_name, value)
}

fn fake_version_fields(
    env: &mut Env,
    _build_class: &JClass,
    merged_config: &MergedAppConfig,
) -> anyhow::Result<()> {
    let version_class = env
        .find_class(jni_str!("android/os/Build$VERSION"))
        .context("Failed to find Build.VERSION class")?;

    if let Some(android_version) = field_str(&merged_config.android_version) {
        set_build_field(env, &version_class, jni_str!("RELEASE"), android_version)?;
    }

    // `__DELETE__`（NumberSetting::Delete）不伪装该字段：与字符串字段的
    // `__DELETE__` 语义一致——只由 delete_props 列表在 companion 侧删除属性，
    // Java 层字段保持系统原值。
    if let Some(sdk_int) = merged_config.sdk_int.and_then(NumberSetting::value) {
        set_build_int_field(env, &version_class, jni_str!("SDK_INT"), sdk_int as i32)?;
    }

    // Build.VERSION.INCREMENTAL 的属性来源是 ro.build.version.incremental
    if let Some(incremental) = field_str(&merged_config.incremental) {
        set_build_field(env, &version_class, jni_str!("INCREMENTAL"), incremental)?;
    }

    // Build.VERSION.SECURITY_PATCH 的属性来源是 ro.build.version.security_patch
    if let Some(security_patch) = field_str(&merged_config.security_patch) {
        set_build_field(
            env,
            &version_class,
            jni_str!("SECURITY_PATCH"),
            security_patch,
        )?;
    }

    Ok(())
}

fn set_build_field(
    env: &mut Env,
    build_class: &JClass,
    field_name: &JNIStr,
    value: &str,
) -> anyhow::Result<()> {
    let _field_id = env
        .get_static_field_id(build_class, field_name, jni_sig!("Ljava/lang/String;"))
        .with_context(|| "Failed to get field ID".to_string())?;

    let new_value = env
        .new_string(value)
        .with_context(|| format!("Failed to create string for {value}"))?;

    env.set_static_field(
        build_class,
        field_name,
        jni_sig!("Ljava/lang/String;"),
        JValue::Object(&new_value),
    )
    .with_context(|| "Failed to set field".to_string())?;

    Ok(())
}

fn set_build_int_field(
    env: &mut Env,
    build_class: &JClass,
    field_name: &JNIStr,
    value: i32,
) -> anyhow::Result<()> {
    let _field_id = env
        .get_static_field_id(build_class, field_name, jni_sig!("I"))
        .with_context(|| "Failed to get field ID".to_string())?;

    env.set_static_field(build_class, field_name, jni_sig!("I"), JValue::Int(value))
        .with_context(|| "Failed to set field".to_string())?;

    Ok(())
}
