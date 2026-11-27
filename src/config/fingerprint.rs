// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

//! 构建指纹解析：`brand/product/device:release/id/incremental:type/tags`。

/// Android 构建指纹格式：brand/product/device:release/id/incremental:type/tags
pub(super) struct FingerprintParts {
    pub(super) brand: String,
    pub(super) product: String,
    pub(super) device: String,
    pub(super) release: String,
    pub(super) build_id: String,
    pub(super) incremental: String,
    pub(super) build_type: String,
    pub(super) tags: String,
}

impl FingerprintParts {
    /// 解析标准格式指纹；字段数不符或存在空字段时返回 None（视为未配置指纹，
    /// 不做任何补全）。
    pub(super) fn parse(fingerprint: &str) -> Option<Self> {
        let (identity, build) = fingerprint.split_once(':')?;
        let (version, variant) = build.split_once(':')?;

        let mut identity_parts = identity.split('/');
        let brand = identity_parts.next()?;
        let product = identity_parts.next()?;
        let device = identity_parts.next()?;
        if identity_parts.next().is_some() {
            return None;
        }

        let mut version_parts = version.split('/');
        let release = version_parts.next()?;
        let build_id = version_parts.next()?;
        let incremental = version_parts.next()?;
        if version_parts.next().is_some() {
            return None;
        }

        let mut variant_parts = variant.split('/');
        let build_type = variant_parts.next()?;
        let tags = variant_parts.next()?;
        if variant_parts.next().is_some()
            || [
                brand,
                product,
                device,
                release,
                build_id,
                incremental,
                build_type,
                tags,
            ]
            .iter()
            .any(|part| part.is_empty())
        {
            return None;
        }

        Some(Self {
            brand: brand.to_string(),
            product: product.to_string(),
            device: device.to_string(),
            release: release.to_string(),
            build_id: build_id.to_string(),
            incremental: incremental.to_string(),
            build_type: build_type.to_string(),
            tags: tags.to_string(),
        })
    }
}
