//! device_faker_cli —— 开发/排障用的命令行工具（`convert` / `convert-props` / `dump-device`）。
//!
//! TODO(remove-cli): 自 2026-10-31 起**不再随模块打包**（`xtask` 不再拷贝、`customize.sh`
//! 不再 set_perm），因为 WebUI 的「ZIP / 属性文本 → 机型模板」转换已改成纯前端实现
//! （`webui/src/utils/propsToTemplate.ts` + `zipReader.ts`，A/B 与 CLI 输出逐字节一致）。
//! 保留本 crate 只是给开发者手动使用；若确认没人再从 adb shell 跑它，可以整体删除
//! （连带 `xtask` 的 TODO 注释、AGENTS.md 的 CLI 段落、`Cargo.toml` 的 exclude 条目）。

use anyhow::Result;
use argh::FromArgs;

mod converter;

/// Device Faker configuration tool
#[derive(FromArgs)]
struct Cli {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    /// Convert Magisk module ZIP to TOML configuration
    Convert(ConvertArgs),
    /// Convert a system.prop-style file to TOML configuration
    ConvertProps(ConvertPropsArgs),
    /// Read current device properties via getprop and export TOML configuration
    DumpDevice(DumpDeviceArgs),
}

/// Convert Magisk module ZIP to TOML configuration
#[derive(FromArgs)]
#[argh(subcommand, name = "convert")]
struct ConvertArgs {
    /// input ZIP file path
    #[argh(option, short = 'i', long = "input")]
    input: String,

    /// output file path
    #[argh(option, short = 'o', long = "output")]
    output: String,
}

/// Convert a system.prop-style file to TOML configuration
#[derive(FromArgs)]
#[argh(subcommand, name = "convert-props")]
struct ConvertPropsArgs {
    /// input system.prop or property text file path
    #[argh(option, short = 'i', long = "input")]
    input: String,

    /// output TOML file path
    #[argh(option, short = 'o', long = "output")]
    output: String,
}

/// Read current device properties via getprop and export TOML configuration
#[derive(FromArgs)]
#[argh(subcommand, name = "dump-device")]
struct DumpDeviceArgs {
    /// output TOML file path
    #[argh(option, short = 'o', long = "output")]
    output: String,
}

fn main() -> Result<()> {
    let cli: Cli = argh::from_env();

    match cli.command {
        Command::Convert(args) => {
            converter::convert_zip_config(&args.input, &args.output)?;
        }
        Command::ConvertProps(args) => {
            converter::convert_props_config(&args.input, &args.output)?;
        }
        Command::DumpDevice(args) => {
            converter::dump_current_device_config(&args.output)?;
        }
    }

    Ok(())
}
