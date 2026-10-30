mod zip_ext;

use std::{
    fs::{self},
    path::{Path, PathBuf},
    process::{self, Command},
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use fs_extra::{dir, file};
use time::{OffsetDateTime, UtcOffset};
use zip::{CompressionMethod, DateTime, write::FileOptions};
use zip_ext::zip_create_from_directory_with_options;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check the build of device_faker
    Check {
        /// Build in release mode (default: false)
        #[clap(short, long, default_value = "false")]
        release: bool,

        /// Print detailed output (default: false)
        #[clap(short, long, default_value = "false")]
        verbose: bool,
    },

    /// Build device_faker and device_faker_cli
    Build {
        /// Build in release mode (default: false)
        #[clap(short, long, default_value = "false")]
        release: bool,

        /// Print detailed output (default: false)
        #[clap(short, long, default_value = "false")]
        verbose: bool,
    },

    /// Clean build artifacts
    Clean,

    /// Format source code
    Format {
        /// Print detailed output (default: false)
        #[clap(short, long, default_value = "false")]
        verbose: bool,
    },

    /// Run the Clippy linter
    Lint {
        /// Automatically fix lint issues (default: false)
        #[clap(short, long, default_value = "false")]
        fix: bool,
    },

    /// Update project dependencies
    Update,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let Some(command) = cli.command else {
        eprintln!("No command specified. Use --help to see available commands.");
        process::exit(1);
    };

    match command {
        Commands::Check { release, verbose } => {
            check(release, verbose)?;
        }
        Commands::Build { release, verbose } => {
            build(release, verbose)?;
        }
        Commands::Clean => {
            clean()?;
        }
        Commands::Format { verbose } => {
            format(verbose)?;
        }
        Commands::Lint { fix } => {
            lint(fix)?;
        }
        Commands::Update => {
            update()?;
        }
    }

    Ok(())
}

/// 跑外部命令并在**非零退出码**时直接失败。
///
/// `Command::spawn()?.wait()?` 只能发现「进程起不来」；命令自己跑挂了（退出码非 0）
/// 会被静默忽略。2026-10-30 的 CI（run 41123564715）就是这样：`cargo ndk build` 因
/// `error[E0425]: cannot find function build_ident` 失败，退出码被丢掉，23 秒后才在
/// `file::copy(...).unwrap()` 处 panic——日志里只剩那句 unwrap，真正的 rustc 报错被
/// 埋在几十行 vite 输出之前，很容易误判成环境问题。
fn run_command(command: &mut Command, what: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to spawn {what}"))?;
    if !status.success() {
        bail!("{what} failed ({status})");
    }
    Ok(())
}

fn build(release: bool, verbose: bool) -> Result<()> {
    let temp_dir = temp_dir(release);

    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir)?;

    let mut cargo = cargo_ndk();
    let args = vec!["build", "--target", "aarch64-linux-android"];
    cargo.args(args);
    if release {
        cargo.arg("--release");
    }

    if verbose {
        cargo.arg("--verbose");
    }

    run_command(&mut cargo, "cargo ndk build")?;
    run_command(
        cargo.current_dir("device_faker_cli/"),
        "cargo build (device_faker_cli)",
    )?;

    // Build WebUI first so that module/webroot/ exists before copying
    build_webui()?;

    let module_dir = module_dir();
    dir::copy(
        &module_dir,
        &temp_dir,
        &dir::CopyOptions::new().overwrite(true).content_only(true),
    )
    .with_context(|| format!("failed to copy {} into build dir", module_dir.display()))?;
    if temp_dir.join(".gitignore").exists() {
        fs::remove_file(temp_dir.join(".gitignore")).context("failed to drop .gitignore")?;
    }
    let so_source = bin_path(release);
    if !so_source.exists() {
        bail!(
            "{} was not produced by the build (see the build output above)",
            so_source.display()
        );
    }
    file::copy(
        &so_source,
        temp_dir.join("zygisk/arm64-v8a.so"),
        &file::CopyOptions::new().overwrite(true),
    )
    .context("failed to stage zygisk/arm64-v8a.so")?;
    file::copy(
        cli_bin_path(release),
        temp_dir.join("bin/device_faker_cli"),
        &file::CopyOptions::new().overwrite(true),
    )
    .unwrap();

    let build_type = if release { "release" } else { "debug" };
    let package_path = Path::new("output").join(format!("device_faker-({build_type}).zip"));

    let local_offset = UtcOffset::from_hms(8, 0, 0).expect("UTC+8 offset should always be valid");

    zip_create_from_directory_with_options(&package_path, &temp_dir, |entry_path| {
        let mut options: FileOptions<'_, '_, ()> = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(9));

        if let Some(modified_time) = zip_entry_modified_time(entry_path, local_offset) {
            options = options.last_modified_time(modified_time);
        }

        options
    })
    .unwrap();

    println!("device_faker built successfully: {:?}", package_path);

    Ok(())
}

fn zip_entry_modified_time(entry_path: &Path, local_offset: UtcOffset) -> Option<DateTime> {
    let modified = fs::metadata(entry_path).ok()?.modified().ok()?;
    let modified = OffsetDateTime::from(modified).to_offset(local_offset);
    // zip 9 移除了 `TryFrom<OffsetDateTime> for DateTime`，只剩 PrimitiveDateTime 版本。
    // ZIP 条目只存墙上时间（无时区），因此取 date+time 与原语义完全一致。
    let primitive = time::PrimitiveDateTime::new(modified.date(), modified.time());

    DateTime::try_from(primitive).ok()
}

fn check(release: bool, verbose: bool) -> Result<()> {
    // check 不需要链接，因此不需要 cargo-ndk 设置的 Android linker。
    // 同时避免与 -Z build-std 冲突导致 duplicate core lang item。
    let mut cargo = Command::new("cargo");
    cargo.args([
        "+nightly",
        "check",
        "--target",
        "aarch64-linux-android",
        "-Z",
        "trim-paths",
    ]);

    if release {
        cargo.arg("--release");
    }

    if verbose {
        cargo.arg("--verbose");
    }

    run_command(&mut cargo, "cargo ndk check")?;

    Ok(())
}

fn clean() -> Result<()> {
    let temp_dir = temp_dir(false);
    let _ = fs::remove_dir_all(&temp_dir);

    run_command(Command::new("cargo").arg("clean"), "cargo clean")?;

    Ok(())
}

fn format(verbose: bool) -> Result<()> {
    let mut command = Command::new("cargo");
    command.args(["fmt", "--all"]);
    if verbose {
        command.arg("--verbose");
    }
    run_command(&mut command, "cargo fmt")?;

    Ok(())
}

fn lint(fix: bool) -> Result<()> {
    let command_builder = |fix: bool| {
        let mut command = cargo_ndk();
        command.arg("clippy");
        if fix {
            command.args(["--fix", "--allow-dirty", "--allow-staged", "--all"]);
        }
        command.args(["--target", "aarch64-linux-android"]);
        command
    };

    run_command(&mut command_builder(fix), "cargo clippy")?;
    run_command(
        command_builder(fix).arg("--release"),
        "cargo clippy --release",
    )?;

    Ok(())
}

fn update() -> Result<()> {
    run_command(
        Command::new("cargo").args(["update", "--recursive"]),
        "cargo update",
    )?;
    run_command(
        Command::new("cargo")
            .current_dir("xtask")
            .args(["update", "--recursive"]),
        "cargo update (xtask)",
    )?;

    Ok(())
}

fn module_dir() -> PathBuf {
    Path::new("module").to_path_buf()
}

fn temp_dir(release: bool) -> PathBuf {
    Path::new("output")
        .join(".temp")
        .join(if release { "release" } else { "debug" })
}

fn bin_path(release: bool) -> PathBuf {
    Path::new("target")
        .join("aarch64-linux-android")
        .join(if release { "release" } else { "debug" })
        .join("libzygisk.so")
}

fn cli_bin_path(release: bool) -> PathBuf {
    Path::new("device_faker_cli/target")
        .join("aarch64-linux-android")
        .join(if release { "release" } else { "debug" })
        .join("device_faker_cli")
}

fn cargo_ndk() -> Command {
    let mut command = Command::new("cargo");
    command
        .args(["+nightly", "ndk", "--platform", "31", "-t", "arm64-v8a"])
        .env("RUSTFLAGS", "-C default-linker-libraries");
    command
}

fn build_webui() -> Result<()> {
    let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let npm = || {
        let mut command = Command::new(npm_cmd);
        command.current_dir("webui");
        command
    };

    run_command(npm().arg("install"), "npm install")?;
    run_command(npm().args(["run", "build"]), "npm run build")?;

    Ok(())
}
