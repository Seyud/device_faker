#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (c) 2025-2026 Seyud
# Device Faker: https://github.com/Seyud/device_faker

import os
import shutil
import subprocess
import sys
import tomllib

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
LOCAL_CONFIG = os.path.join(SCRIPT_DIR, "build_config.local.toml")
SAMPLE_CONFIG = os.path.join(SCRIPT_DIR, "build_config.sample.toml")


def load_build_config():
    """优先读本地真实配置，否则退回 sample。"""
    for path in (LOCAL_CONFIG, SAMPLE_CONFIG):
        if not os.path.isfile(path):
            continue
        with open(path, "rb") as f:
            cfg = tomllib.load(f)
        source = os.path.basename(path)
        print(f"已加载构建配置: {source}")
        return cfg
    print("未找到 build_config.local.toml 或 build_config.sample.toml")
    print(f"请复制 {SAMPLE_CONFIG} 为 build_config.local.toml 并填写本机路径")
    sys.exit(1)


def require_dir(cfg, key, label):
    path = cfg.get(key, "")
    if not path:
        print(f"构建配置缺少 {key}（{label}）")
        sys.exit(1)
    path = os.path.normpath(path)
    if not os.path.isdir(path):
        print(f"未找到{label}路径: {path}")
        print("请在 build_config.local.toml 中修改该路径")
        sys.exit(1)
    return path


def check_ndk_path(cfg):
    path = require_dir(cfg, "ndk_path", "NDK")
    print(f"NDK路径检查通过: {path}")
    return path


def check_upx_path(cfg):
    path = require_dir(cfg, "upx_path", "UPX")
    upx_exe_path = os.path.join(path, "upx.exe")
    if not os.path.isfile(upx_exe_path):
        print(f"未找到UPX可执行文件: {upx_exe_path}")
        sys.exit(1)
    print(f"UPX路径检查通过: {path}")
    return upx_exe_path


def add_android_target():
    print("添加Android 64位目标...")
    try:
        subprocess.run(["rustup", "target", "add", "aarch64-linux-android"], check=True, cwd=SCRIPT_DIR)
    except subprocess.CalledProcessError as e:
        print(f"添加Android目标失败: {e}")
        sys.exit(1)


def run_fmt_and_clippy():
    print("检查代码格式...")

    fmt_check_result = subprocess.run(
        ["cargo", "fmt", "--", "--check"],
        cwd=SCRIPT_DIR,
        capture_output=True,
        text=True,
    )

    if fmt_check_result.returncode == 0:
        print("代码格式检查通过，无需格式化")
    else:
        print("检测到代码格式问题，正在格式化...")
        try:
            subprocess.run(["cargo", "fmt"], check=True, cwd=SCRIPT_DIR)
            print("代码格式化完成")
        except subprocess.CalledProcessError as e:
            print(f"代码格式化失败: {e}")
            if e.stderr:
                print(f"错误详情: {e.stderr}")
            sys.exit(1)

    print("运行 clippy 检查...")
    try:
        subprocess.run(
            ["cargo", "clippy", "--target", "aarch64-linux-android", "--", "-D", "warnings"],
            check=True,
            cwd=SCRIPT_DIR,
        )
        print("clippy 检查通过")
    except subprocess.CalledProcessError as e:
        print(f"clippy检查失败: {e}")
        if e.stderr:
            print(f"错误详情: {e.stderr}")
        print("请修复上述clippy警告后重新运行")
        sys.exit(1)


def build_android():
    print("构建Android 64位版本...")
    try:
        subprocess.run(
            ["cargo", "build", "--target", "aarch64-linux-android", "--release"],
            check=True,
            cwd=SCRIPT_DIR,
        )
    except subprocess.CalledProcessError as e:
        print(f"构建Android版本失败: {e}")
        sys.exit(1)


def compress_with_upx(upx_exe_path):
    print("使用UPX压缩二进制文件...")
    try:
        binary_path = os.path.join(
            SCRIPT_DIR, "target", "aarch64-linux-android", "release", "device_faker_cli"
        )
        subprocess.run([upx_exe_path, binary_path], check=True)
        print("UPX压缩完成")
    except subprocess.CalledProcessError as e:
        print(f"UPX压缩失败: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"UPX压缩过程中发生错误: {e}")
        sys.exit(1)


def copy_binary_to_output():
    print("将构建的二进制文件复制到module文件夹...")
    try:
        workspace_root = os.path.dirname(SCRIPT_DIR)
        binary_name = "device_faker_cli"
        source_path = os.path.join(
            SCRIPT_DIR, "target", "aarch64-linux-android", "release", binary_name
        )
        output_dir = os.path.join(workspace_root, "module", "bin")

        if not os.path.exists(source_path):
            print(f"错误：找不到构建的二进制文件: {source_path}")
            print("请确保构建成功完成")
            release_dir = os.path.dirname(source_path)
            if os.path.exists(release_dir):
                print(f"在 {release_dir} 中找到的文件：")
                for file in os.listdir(release_dir):
                    if not file.endswith(".d"):
                        print(f"  - {file}")
            sys.exit(1)

        os.makedirs(output_dir, exist_ok=True)
        dest_path = os.path.join(output_dir, binary_name)
        shutil.copy2(source_path, dest_path)
        print(f"✅ 二进制文件已复制到 module/bin/{binary_name}")
    except Exception as e:
        print(f"复制二进制文件失败: {e}")
        sys.exit(1)


def main():
    print("Device Faker CLI 构建脚本 (仅64位)")
    print("=" * 50)

    cfg = load_build_config()
    check_ndk_path(cfg)
    upx_exe_path = check_upx_path(cfg)

    print("\n=== 构建 Rust CLI 工具 ===")
    add_android_target()
    run_fmt_and_clippy()
    build_android()
    compress_with_upx(upx_exe_path)
    copy_binary_to_output()

    print("\n" + "=" * 50)
    print("✅ 构建完成！")
    print("CLI工具文件位于 ../module/bin/ 目录")
    print("✓ 二进制文件: ../module/bin/device_faker_cli")
    print("\n请将 module/ 目录打包为 ZIP 文件后通过root管理器安装")
    print("=" * 50)


if __name__ == "__main__":
    main()
