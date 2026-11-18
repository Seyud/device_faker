// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use walkdir::WalkDir;
use zip::ZipArchive;

const MANUFACTURER_KEYS: &[&str] = &[
    "ro.product.manufacturer",
    "ro.product.system.manufacturer",
    "ro.product.vendor.manufacturer",
    "ro.product.odm.manufacturer",
];
const BRAND_KEYS: &[&str] = &[
    "ro.product.brand",
    "ro.product.system.brand",
    "ro.product.vendor.brand",
    "ro.product.odm.brand",
];
const MODEL_KEYS: &[&str] = &[
    "ro.product.model",
    "ro.product.system.model",
    "ro.product.vendor.model",
    "ro.product.odm.model",
];
const MARKETNAME_KEYS: &[&str] = &["ro.product.marketname", "ro.product.vendor.marketname"];
const NAME_KEYS: &[&str] = &[
    "ro.product.name",
    "ro.product.system.name",
    "ro.product.vendor.name",
    "ro.product.odm.name",
];
const DEVICE_KEYS: &[&str] = &[
    "ro.product.device",
    "ro.product.system.device",
    "ro.product.vendor.device",
    "ro.product.odm.device",
];
const PRODUCT_KEYS: &[&str] = &[
    "ro.product.product",
    "ro.product.system.product",
    "ro.product.vendor.product",
    "ro.product.odm.product",
];
const FINGERPRINT_KEYS: &[&str] = &[
    "ro.build.fingerprint",
    "ro.system.build.fingerprint",
    "ro.vendor.build.fingerprint",
    "ro.product.build.fingerprint",
];
const BUILD_ID_KEYS: &[&str] = &[
    "ro.build.id",
    "ro.system.build.id",
    "ro.vendor.build.id",
    "ro.product.build.id",
];
const CHARACTERISTICS_KEYS: &[&str] = &[
    "ro.build.characteristics",
    "ro.system.build.characteristics",
    "ro.vendor.build.characteristics",
    "ro.product.build.characteristics",
];
const ANDROID_VERSION_KEYS: &[&str] = &[
    "ro.build.version.release",
    "ro.system.build.version.release",
    "ro.vendor.build.version.release",
    "ro.product.build.version.release",
];
const SDK_INT_KEYS: &[&str] = &[
    "ro.build.version.sdk",
    "ro.system.build.version.sdk",
    "ro.vendor.build.version.sdk",
    "ro.product.build.version.sdk",
];
const HARDWARE_KEYS: &[&str] = &["ro.hardware"];
const BOARD_KEYS: &[&str] = &["ro.product.board"];
const SECURITY_PATCH_KEYS: &[&str] = &[
    "ro.build.version.security_patch",
    "ro.system.build.version.security_patch",
    "ro.vendor.build.version.security_patch",
    "ro.product.build.version.security_patch",
    // vendor 分区独有的裸名副本（无 .version. 段）。
    "ro.vendor.build.security_patch",
];

#[derive(Debug, Serialize)]
struct OutputConfig {
    templates: BTreeMap<String, DeviceTemplateToml>,
}

#[derive(Debug, Default, Serialize)]
struct DeviceTemplateToml {
    packages: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    brand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    marketname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    product: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hardware: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    board: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    build_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_patch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    characteristics: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    android_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sdk_int: Option<u32>,
}

impl DeviceTemplateToml {
    fn has_payload(&self) -> bool {
        self.manufacturer.is_some()
            || self.brand.is_some()
            || self.marketname.is_some()
            || self.model.is_some()
            || self.name.is_some()
            || self.device.is_some()
            || self.product.is_some()
            || self.hardware.is_some()
            || self.board.is_some()
            || self.fingerprint.is_some()
            || self.build_id.is_some()
            || self.security_patch.is_some()
            || self.characteristics.is_some()
            || self.android_version.is_some()
            || self.sdk_int.is_some()
    }
}

fn parse_getprop_line(line: &str) -> Option<(String, String)> {
    let (key_part, value_part) = line.split_once("]: [")?;
    let key = key_part.strip_prefix('[')?.trim();
    let value = value_part.strip_suffix(']')?.trim();

    if key.is_empty() {
        return None;
    }

    Some((key.to_string(), value.to_string()))
}

fn parse_property_text(content: &str) -> BTreeMap<String, String> {
    let mut properties = BTreeMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = parse_getprop_line(trimmed) {
            properties.insert(key, value);
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            let normalized_key = key.trim();
            if normalized_key.is_empty() {
                continue;
            }

            properties.insert(normalized_key.to_string(), value.trim().to_string());
        }
    }

    properties
}

fn read_non_empty_property(properties: &BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        properties
            .get(*key)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn parse_sdk_int(properties: &BTreeMap<String, String>) -> Option<u32> {
    read_non_empty_property(properties, SDK_INT_KEYS).and_then(|value| value.parse::<u32>().ok())
}

fn derive_template_name(properties: &BTreeMap<String, String>) -> String {
    read_non_empty_property(properties, MARKETNAME_KEYS)
        .or_else(|| read_non_empty_property(properties, MODEL_KEYS))
        .or_else(|| read_non_empty_property(properties, NAME_KEYS))
        .or_else(|| read_non_empty_property(properties, DEVICE_KEYS))
        .unwrap_or_else(|| "generated_template".to_string())
}

fn build_template(properties: &BTreeMap<String, String>) -> DeviceTemplateToml {
    let name = read_non_empty_property(properties, NAME_KEYS)
        .or_else(|| read_non_empty_property(properties, DEVICE_KEYS));
    let device = read_non_empty_property(properties, DEVICE_KEYS).or_else(|| name.clone());

    DeviceTemplateToml {
        packages: Vec::new(),
        manufacturer: read_non_empty_property(properties, MANUFACTURER_KEYS),
        brand: read_non_empty_property(properties, BRAND_KEYS),
        marketname: read_non_empty_property(properties, MARKETNAME_KEYS),
        model: read_non_empty_property(properties, MODEL_KEYS),
        name,
        device,
        product: read_non_empty_property(properties, PRODUCT_KEYS),
        hardware: read_non_empty_property(properties, HARDWARE_KEYS),
        board: read_non_empty_property(properties, BOARD_KEYS),
        fingerprint: read_non_empty_property(properties, FINGERPRINT_KEYS),
        build_id: read_non_empty_property(properties, BUILD_ID_KEYS),
        security_patch: read_non_empty_property(properties, SECURITY_PATCH_KEYS),
        characteristics: read_non_empty_property(properties, CHARACTERISTICS_KEYS),
        android_version: read_non_empty_property(properties, ANDROID_VERSION_KEYS),
        sdk_int: parse_sdk_int(properties),
    }
}

fn convert_property_map_to_toml(properties: &BTreeMap<String, String>) -> Result<String> {
    let template = build_template(properties);
    if !template.has_payload() {
        bail!("no supported device properties found in input");
    }

    let template_name = derive_template_name(properties);
    let mut templates = BTreeMap::new();
    templates.insert(template_name, template);

    let config = OutputConfig { templates };
    let serialized = toml::to_string_pretty(&config).context("failed to serialize TOML output")?;

    Ok(format!("# Generated by device_faker_cli\n{serialized}"))
}

fn convert_property_text_to_output(content: &str, output: &str) -> Result<()> {
    let properties = parse_property_text(content);
    let toml_output = convert_property_map_to_toml(&properties)?;
    fs::write(output, toml_output).context("failed to write output file")?;
    Ok(())
}

pub fn convert_props_config(input: &str, output: &str) -> Result<()> {
    println!("Converting property file {} to {}", input, output);
    let content = fs::read_to_string(input).context("failed to read input file")?;
    convert_property_text_to_output(&content, output)
}

pub fn dump_current_device_config(output: &str) -> Result<()> {
    println!("Dumping current device properties to {}", output);

    let command_output = Command::new("getprop")
        .output()
        .context("failed to execute getprop")?;

    if !command_output.status.success() {
        let stderr = String::from_utf8_lossy(&command_output.stderr);
        bail!("getprop failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8(command_output.stdout).context("getprop output is not UTF-8")?;
    convert_property_text_to_output(&stdout, output)
}

fn create_temp_dir() -> Result<PathBuf> {
    let base = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = base.join(format!("device_faker_cli_{}", nanos));
    fs::create_dir_all(&dir).context("failed to create temporary directory")?;
    Ok(dir)
}

fn cleanup_dir(path: &Path) {
    if let Err(err) = fs::remove_dir_all(path) {
        eprintln!(
            "Warning: failed to clean temp dir {}: {}",
            path.display(),
            err
        );
    }
}

fn extract_zip_to_dir(zip_path: &str, dest: &Path) -> Result<()> {
    let zip_file = File::open(zip_path).context("failed to open ZIP file")?;
    let mut archive = ZipArchive::new(zip_file).context("failed to read ZIP archive")?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).context("failed to read ZIP entry")?;
        let Some(rel_path) = entry.enclosed_name().map(|path| path.to_owned()) else {
            continue;
        };

        let out_path = dest.join(rel_path);
        if entry.name()?.ends_with('/') {
            fs::create_dir_all(&out_path).context("failed to create directory from ZIP")?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).context("failed to create parent directory")?;
        }

        let mut outfile = File::create(&out_path).context("failed to create extracted file")?;
        std::io::copy(&mut entry, &mut outfile).context("failed to write extracted file")?;
    }

    Ok(())
}

fn find_system_prop(root: &Path) -> Result<PathBuf> {
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if entry.file_type().is_file()
            && entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("system.prop")
        {
            return Ok(entry.into_path());
        }
    }

    Err(anyhow!("system.prop not found in ZIP archive"))
}

pub fn convert_zip_config(input_zip: &str, output: &str) -> Result<()> {
    println!("Converting config from ZIP {} to {}", input_zip, output);

    let temp_dir = create_temp_dir()?;

    let result = (|| -> Result<()> {
        extract_zip_to_dir(input_zip, &temp_dir)?;
        let system_prop_path = find_system_prop(&temp_dir)?;
        let content =
            fs::read_to_string(&system_prop_path).context("failed to read system.prop")?;
        convert_property_text_to_output(&content, output)?;
        Ok(())
    })();

    cleanup_dir(&temp_dir);
    result
}

#[cfg(test)]
mod tests {
    use super::{
        build_template, convert_property_map_to_toml, derive_template_name, parse_property_text,
    };

    #[test]
    fn parses_system_prop_format() {
        let properties = parse_property_text(
            r#"
            # comment
            ro.product.manufacturer = Xiaomi
            ro.product.model=2210132G
            ro.build.version.sdk = 34
            "#,
        );

        assert_eq!(
            properties
                .get("ro.product.manufacturer")
                .map(String::as_str),
            Some("Xiaomi")
        );
        assert_eq!(
            properties.get("ro.product.model").map(String::as_str),
            Some("2210132G")
        );
        assert_eq!(
            properties.get("ro.build.version.sdk").map(String::as_str),
            Some("34")
        );
    }

    #[test]
    fn parses_getprop_output_format() {
        let properties = parse_property_text(
            r#"
            [ro.product.model]: [Pixel 9 Pro]
            [ro.product.manufacturer]: [Google]
            [ro.build.version.sdk]: [35]
            "#,
        );

        assert_eq!(
            properties.get("ro.product.model").map(String::as_str),
            Some("Pixel 9 Pro")
        );
        assert_eq!(
            properties
                .get("ro.product.manufacturer")
                .map(String::as_str),
            Some("Google")
        );
        assert_eq!(
            properties.get("ro.build.version.sdk").map(String::as_str),
            Some("35")
        );
    }

    #[test]
    fn builds_template_with_extended_fields() {
        let properties = parse_property_text(
            r#"
            ro.product.manufacturer=Xiaomi
            ro.product.brand=Xiaomi
            ro.product.marketname=Xiaomi 15 Pro
            ro.product.model=25010PN30C
            ro.product.name=haotian
            ro.product.device=haotian
            ro.product.product=haotian
            ro.product.board=kalama
            ro.build.fingerprint=Xiaomi/haotian/haotian:15/AP4A.250205.002/123456:user/release-keys
            ro.build.id=AP4A.250205.002
            ro.build.version.security_patch=2025-06-05
            ro.build.characteristics=nosdcard
            ro.build.version.release=15
            ro.build.version.sdk=35
            "#,
        );

        let template = build_template(&properties);
        assert_eq!(template.manufacturer.as_deref(), Some("Xiaomi"));
        assert_eq!(template.brand.as_deref(), Some("Xiaomi"));
        assert_eq!(template.marketname.as_deref(), Some("Xiaomi 15 Pro"));
        assert_eq!(template.model.as_deref(), Some("25010PN30C"));
        assert_eq!(template.name.as_deref(), Some("haotian"));
        assert_eq!(template.device.as_deref(), Some("haotian"));
        assert_eq!(template.product.as_deref(), Some("haotian"));
        assert_eq!(template.board.as_deref(), Some("kalama"));
        assert_eq!(
            template.fingerprint.as_deref(),
            Some("Xiaomi/haotian/haotian:15/AP4A.250205.002/123456:user/release-keys")
        );
        assert_eq!(template.build_id.as_deref(), Some("AP4A.250205.002"));
        assert_eq!(template.security_patch.as_deref(), Some("2025-06-05"));
        assert_eq!(template.characteristics.as_deref(), Some("nosdcard"));
        assert_eq!(template.android_version.as_deref(), Some("15"));
        assert_eq!(template.sdk_int, Some(35));
    }

    #[test]
    fn security_patch_prefers_version_keys_over_vendor_bare_name() {
        // 标准 key 与 vendor 裸名副本同时存在时，取标准 key
        let properties = parse_property_text(
            r#"
            ro.build.version.security_patch=2025-06-05
            ro.vendor.build.security_patch=2024-12-01
            "#,
        );
        let template = build_template(&properties);
        assert_eq!(template.security_patch.as_deref(), Some("2025-06-05"));

        // 只有裸名副本的机型（无 .version. 变体）也能导入。
        let properties = parse_property_text(
            r#"
            ro.product.manufacturer=Xiaomi
            ro.product.model=25010PN30C
            ro.vendor.build.security_patch=2024-12-01
            "#,
        );
        let template = build_template(&properties);
        assert_eq!(template.security_patch.as_deref(), Some("2024-12-01"));
    }

    #[test]
    fn derives_template_name_from_best_available_property() {
        let properties = parse_property_text(
            r#"
            ro.product.marketname=REDMAGIC 10 Pro
            ro.product.model=NX123J
            "#,
        );

        assert_eq!(derive_template_name(&properties), "REDMAGIC 10 Pro");
    }

    #[test]
    fn serializes_output_as_templates_table() {
        let properties = parse_property_text(
            r#"
            [ro.product.manufacturer]: [Google]
            [ro.product.brand]: [google]
            [ro.product.model]: [Pixel 9]
            [ro.product.name]: [tokay]
            [ro.build.version.sdk]: [35]
            "#,
        );

        let toml_output = convert_property_map_to_toml(&properties).unwrap();
        let parsed: toml::Value = toml::from_str(&toml_output).unwrap();

        let templates = parsed.get("templates").unwrap().as_table().unwrap();
        let pixel = templates.get("Pixel 9").unwrap().as_table().unwrap();

        assert_eq!(pixel.get("manufacturer").unwrap().as_str(), Some("Google"));
        assert_eq!(pixel.get("brand").unwrap().as_str(), Some("google"));
        assert_eq!(pixel.get("name").unwrap().as_str(), Some("tokay"));
        assert_eq!(pixel.get("sdk_int").unwrap().as_integer(), Some(35));
        assert_eq!(pixel.get("packages").unwrap().as_array().unwrap().len(), 0);
    }
}
