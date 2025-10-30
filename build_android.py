#!/usr/bin/env python3

import os
import subprocess
import sys
import shutil

def check_ndk_path():
    ndk_path = "D:/android-ndk"
    if not os.path.isdir(ndk_path):
        print("未找到NDK路径，请确保NDK安装在D:/android-ndk")
        sys.exit(1)
    print("NDK路径检查通过")

def add_android_target():
    print("添加Android 64位目标...")
    try:
        # 确保在项目根目录执行rustup命令
        project_root = os.path.dirname(os.path.abspath(__file__))
        subprocess.run(["rustup", "target", "add", "aarch64-linux-android"], check=True, cwd=project_root)
    except subprocess.CalledProcessError as e:
        print(f"添加Android目标失败: {e}")
        sys.exit(1)

def run_fmt_and_clippy():
    # 确保在项目根目录执行cargo命令
    project_root = os.path.dirname(os.path.abspath(__file__))
    
    print("检查代码格式...")
    
    # 先检查是否需要格式化
    fmt_check_result = subprocess.run(["cargo", "fmt", "--", "--check"], 
                                    cwd=project_root, 
                                    capture_output=True, 
                                    text=True)
    
    if fmt_check_result.returncode == 0:
        print("代码格式检查通过，无需格式化")
    else:
        print("检测到代码格式问题，正在格式化...")
        try:
            subprocess.run(["cargo", "fmt"], check=True, cwd=project_root)
            print("代码格式化完成")
        except subprocess.CalledProcessError as e:
            print(f"代码格式化失败: {e}")
            if e.stderr:
                print(f"错误详情: {e.stderr}")
            sys.exit(1)
    
    # 运行clippy检查
    print("运行 clippy 检查...")
    try:
        subprocess.run(["cargo", "clippy", "--target", "aarch64-linux-android", "--", "-D", "warnings"], check=True, cwd=project_root)
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
        # 确保在项目根目录执行cargo命令
        project_root = os.path.dirname(os.path.abspath(__file__))
        subprocess.run(["cargo", "build", "--target", "aarch64-linux-android", "--release"], 
                      check=True, cwd=project_root)
    except subprocess.CalledProcessError as e:
        print(f"构建Android版本失败: {e}")
        sys.exit(1)

def copy_binary_to_output():
    print("将构建的二进制文件复制到module文件夹...")
    try:
        project_root = os.path.dirname(os.path.abspath(__file__))
        
        # 固定的库名称（从 Cargo.toml 中的 [lib] name 读取）
        lib_name = "libzygisk.so"
        
        source_path = os.path.join(project_root, "target", "aarch64-linux-android", "release", lib_name)
        output_dir = os.path.join(project_root, "module", "zygisk")
        
        # 检查源文件是否存在
        if not os.path.exists(source_path):
            print(f"错误：找不到构建的库文件: {source_path}")
            print("请确保构建成功完成")
            
            # 显示可能的文件名
            release_dir = os.path.dirname(source_path)
            if os.path.exists(release_dir):
                print(f"在 {release_dir} 中找到的文件：")
                for file in os.listdir(release_dir):
                    if not file.endswith('.d'):
                        print(f"  - {file}")
            sys.exit(1)
        
        # 创建 module/zygisk 目录（如果不存在）
        os.makedirs(output_dir, exist_ok=True)
        
        # Zygisk 规范: 直接使用 ABI 名称作为文件名
        # 例如: zygisk/arm64-v8a.so
        dest_path = os.path.join(output_dir, "arm64-v8a.so")
        shutil.copy2(source_path, dest_path)
        print(f"✅ 库文件已复制到 module/zygisk/arm64-v8a.so")
    except Exception as e:
        print(f"复制二进制文件失败: {e}")
        sys.exit(1)

def main():
    print("机型伪装 Zygisk 模块构建脚本 (仅64位)")
    print("=" * 50)

    check_ndk_path()
    
    print("\n=== 构建 Rust 原生库 ===")
    add_android_target()
    run_fmt_and_clippy()
    build_android()
    copy_binary_to_output()
    
    print("\n" + "=" * 50)
    print("✅ 构建完成！")
    print(f"模块文件位于 module/ 目录")
    print("✓ Native 库: module/zygisk/arm64-v8a.so")
    print("\n请将 module/ 目录打包为 ZIP 文件后通过 Magisk 安装")
    print("=" * 50)

if __name__ == "__main__":
    main()