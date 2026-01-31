use std::fs;

use anyhow::{Context, Result};

// Gitee repository configuration - raw file base URL
const GITEE_RAW_BASE: &str = "https://gitee.com/Seyud/device_faker_config_mirror/raw/main";

/// Download template content from URL
fn download_template_content(url: &str) -> Result<String> {
    println!("Downloading template from: {}", url);

    let response = minreq::get(url)
        .with_header("User-Agent", "device_faker_cli/0.1.0")
        .with_timeout(30)
        .send()
        .context("Failed to download template")?;

    if response.status_code < 200 || response.status_code >= 300 {
        return Err(anyhow::anyhow!(
            "Download failed with status: {}",
            response.status_code
        ));
    }

    let content = response.as_str().context("Failed to read response")?;
    Ok(content.to_string())
}

/// Parse template name from source parameter
/// Supports:
/// - Direct URL: https://gitee.com/.../templates/common/example.toml
/// - Category/name: common/example
/// - Just name: example (defaults to common category)
fn parse_template_source(source: &str) -> Result<String> {
    // If it's a full URL, return as-is
    if source.starts_with("http://") || source.starts_with("https://") {
        return Ok(source.to_string());
    }

    // If it contains a slash, treat as category/name
    if source.contains('/') {
        let url = format!("{}/templates/{}.toml", GITEE_RAW_BASE, source);
        return Ok(url);
    }

    // Otherwise, assume it's in the common category
    let url = format!("{}/templates/common/{}.toml", GITEE_RAW_BASE, source);
    Ok(url)
}

/// Import device template from online source
///
/// Supports multiple source formats:
/// - Full URL: https://gitee.com/.../templates/common/example.toml
/// - Category/name: common/example
/// - Just name: example (defaults to common category)
pub fn import_template(source: &str, output: &str) -> Result<()> {
    println!("Importing template from: {}", source);

    // Parse source to get download URL
    let download_url = parse_template_source(source)?;

    // Download template content
    let content = download_template_content(&download_url)?;

    // Validate it's valid TOML
    let _parsed: toml::Value =
        toml::from_str(&content).context("Downloaded content is not valid TOML")?;

    // Write to output file
    fs::write(output, &content).context("Failed to write output file")?;

    println!("Template imported successfully to: {}", output);
    Ok(())
}
