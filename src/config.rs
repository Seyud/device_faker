// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

//! 配置模型：TOML schema、按包合并、属性表生成。
//!
//! 2026-11-28 拆分（原单文件 1000+ 行）：按职责分层，避免上帝文件。
//!
//! | 模块 | 职责 |
//! | --- | --- |
//! | [`schema`] | `Config` / `DeviceTemplate` / `AppConfig` 的 serde 定义与「包 → 模板」查找 |
//! | [`fields`] | 单字段取值归一化（`__EMPTY__` / `__DELETE__`）、`NumberSetting`、dpi 合法区间 |
//! | [`fingerprint`] | 构建指纹解析（其余身份字段的补全依据） |
//! | [`families`] | 属性族（分区副本）展开/删除与 [`is_patch_only_key`] 门控 |
//! | [`merged`] | [`MergedAppConfig`]（模板/直配合并结果）、指纹补全、attestation 侧 Build 字段派生 |
//! | [`props`] | 合并配置 → 系统属性表 / 删除列表 |
//!
//! 拆分是纯搬家：函数体、常量与日志文本均未改，只补了模块文档与跨模块可见性
//! （`pub(super)`），对外 API 仍是 [`Config`] / [`MergedAppConfig`] / [`NumberSetting`] /
//! [`is_patch_only_key`] / [`DPI_MIN`] / [`DPI_MAX`]。

mod families;
mod fields;
mod fingerprint;
mod merged;
mod props;
mod schema;

pub use self::{
    families::is_patch_only_key,
    fields::{DPI_MAX, DPI_MIN, NumberSetting},
    merged::MergedAppConfig,
    schema::Config,
};
