//! 构建期用 rsbinder-aidl 把前台观察相关的 AIDL 编译成 Rust 绑定。
//!
//! - `IUidObserver`：系统侧回调我们的接口（onUidStateChanged 等），事务码与
//!   真实 AOSP `android.app.IUidObserver` 一致（1..=6）。
//! - `IActivityManager`：最小代理，`registerUidObserver` 必须保持在接口第 2 位
//!   （code=2），与真实 IActivityManager 的 native 侧稳定区段一致。

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("cargo:rerun-if-changed=aidl/android/app/IUidObserver.aidl");
    println!("cargo:rerun-if-changed=aidl/android/app/IActivityManager.aidl");
    // 版本 banner 要嵌短 git hash + 构建时间：切 commit / 打 tag / 改源码后必须
    // 重跑本脚本，否则 banner 里的 hash 与时间会停留在上一次构建（实测：只改
    // src/ 时 build.rs 不会重跑，时间戳不更新）。
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");

    emit_build_ident();

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");

    // 两个接口同属 android.app 包，一次生成避免重复 pub mod android::app。
    // 必须 async support（rsbinder 0.5.3 的 declare_binder_interface! 在 async
    // feature 下才生成 BnX::new_binder 同步服务；见 Cargo.toml 注释）。
    rsbinder_aidl::Builder::new()
        .include_dir(PathBuf::from("aidl"))
        .set_async_support(true)
        .source(PathBuf::from("aidl/android/app/IUidObserver.aidl"))
        .source(PathBuf::from("aidl/android/app/IActivityManager.aidl"))
        .output(PathBuf::from(&out_dir).join("fg_binder.rs"))
        .generate()
        .expect("failed to generate fg binder bindings");
}

/// 注入构建标识：短 git hash（工作区脏则加 `-dirty`）+ 构建时间（epoch 秒）。
fn emit_build_ident() {
    let hash =
        git_output(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let dirty = git_output(&["status", "--porcelain", "--untracked-files=no"])
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let ident = if dirty { format!("{hash}-dirty") } else { hash };
    println!("cargo:rustc-env=DF_GIT_HASH={ident}");

    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=DF_BUILD_EPOCH={epoch}");
}

/// 跑 git 并取 stdout（trim 后）。git 不可用 / 非 git 源码树时返回 `None`。
fn git_output(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
