// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

//! 单字段取值归一化：字符串标记（`__EMPTY__` / `__DELETE__`）、数值型设置、dpi 合法区间。

use anyhow::Result;
use serde::Deserialize;

pub const DPI_MIN: u32 = 120;
pub const DPI_MAX: u32 = 640;

pub(super) fn valid_dpi(value: Option<u32>) -> Option<u32> {
    value.filter(|dpi| (DPI_MIN..=DPI_MAX).contains(dpi))
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

pub(super) fn has_profile_value(value: &Option<String>) -> bool {
    value.as_ref().is_some_and(|value| !value.is_empty())
}

pub(super) fn fill_profile_default(target: &mut Option<String>, value: impl Into<String>) {
    if !has_profile_value(target) {
        *target = Some(value.into());
    }
}

/// 顶层字段的有效伪装值：未设置、空串或 `__DELETE__`（走删除列表）返回 None，
/// `__EMPTY__` 归一化为空字符串。
pub(super) fn field_value(field: &Option<String>) -> Option<String> {
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
