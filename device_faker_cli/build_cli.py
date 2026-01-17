#!/usr/bin/env python3

import os
import subprocess
import sys
import shutil

def check_ndk_path():
    ndk_path = "D:/android-ndk"
    if not os.path.isdir(ndk_path):
        print("未找到NDK路径,请确保NDK安装在D:/android-ndk")
        sys.exit(1)
    print("NDK路径检查通过")

def check_upx_path():
    upx_path = "D:/upx"
    if not os.path.isdir(upx_path):
        print("未找到UPX路径,请确保UPX安装在D:/upx")
        sys.exit(1)
    upx_exe_path = os.path.join(upx_path, "upx.exe")
    if not os.path.isfile(upx_exe_path):
        print(f"未找到UPX可执行文件: {upx_exe_path}")
        sys.exit(1)
    print("UPX路径检查通过")

def add_android_target():
    print("添加Android 64位目标...")
    try:
        # 脚本在 device_faker_cli/ 子目录，向上一级到 workspace root
        script_dir = os.path.dirname(os.path.abspath(__file__))
        workspace_root = os.path.dirname(script_dir)
        subprocess.run(["rustup", "target", "add", "aarch64-linux-android"], check=True, cwd=workspace_root)
    except subprocess.CalledProcessError as e:
        print(f"添加Android目标失败: {e}")
        sys.exit(1)

def run_fmt_and_clippy():
    # 脚本在 device_faker_cli/ 子目录，向上一级到 workspace root
    script_dir = os.path.dirname(os.path.abspath(__file__))
    workspace_root = os.path.dirname(script_dir)
    
    print("检查代码格式...")
    
    # 检查当前包的格式（在子目录中运行）
    fmt_check_result = subprocess.run(["cargo", "fmt", "--", "--check"], 
                                    cwd=script_dir, 
                                    capture_output=True, 
                                    text=True)
    
    if fmt_check_result.returncode == 0:
        print("代码格式检查通过，无需格式化")
    else:
        print("检测到代码格式问题，正在格式化...")
        try:
            subprocess.run(["cargo", "fmt"], check=True, cwd=script_dir)
            print("代码格式化完成")
        except subprocess.CalledProcessError as e:
            print(f"代码格式化失败: {e}")
            if e.stderr:
                print(f"错误详情: {e.stderr}")
            sys.exit(1)
    
    # 运行clippy检查（直接检查当前包）
    print("运行 clippy 检查...")
    try:
        subprocess.run(["cargo", "clippy", "--target", "aarch64-linux-android", "--", "-D", "warnings"], 
                      check=True, cwd=script_dir)
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
        # 脚本在 device_faker_cli/ 子目录，直接构建当前包
        script_dir = os.path.dirname(os.path.abspath(__file__))
        workspace_root = os.path.dirname(script_dir)
        subprocess.run(["cargo", "build", "--target", "aarch64-linux-android", "--release"], 
                      check=True, cwd=script_dir)
    except subprocess.CalledProcessError as e:
        print(f"构建Android版本失败: {e}")
        sys.exit(1)

def compress_with_upx():
    print("使用UPX压缩二进制文件...")
    try:
        # 独立项目：target 目录位于当前包目录
        script_dir = os.path.dirname(os.path.abspath(__file__))
        
        binary_name = "device_faker_cli"
        binary_path = os.path.join(script_dir, "target", "aarch64-linux-android", "release", binary_name)
        
        upx_exe_path = os.path.join("D:/upx", "upx.exe")
        
        # 运行UPX压缩，直接替换原文件
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
        # 独立项目：target 目录位于当前包目录
        script_dir = os.path.dirname(os.path.abspath(__file__))
        workspace_root = os.path.dirname(script_dir)
        
        binary_name = "device_faker_cli"
        
        source_path = os.path.join(script_dir, "target", "aarch64-linux-android", "release", binary_name)
        output_dir = os.path.join(workspace_root, "module", "bin")
        
        # 检查源文件是否存在
        if not os.path.exists(source_path):
            print(f"错误：找不到构建的二进制文件: {source_path}")
            print("请确保构建成功完成")
            
            # 显示可能的文件名
            release_dir = os.path.dirname(source_path)
            if os.path.exists(release_dir):
                print(f"在 {release_dir} 中找到的文件：")
                for file in os.listdir(release_dir):
                    if not file.endswith('.d'):
                        print(f"  - {file}")
            sys.exit(1)
        
        # 创建 module/bin 目录（如果不存在）
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

    check_ndk_path()
    check_upx_path()
    
    print("\n=== 构建 Rust CLI 工具 ===")
    add_android_target()
    run_fmt_and_clippy()
    build_android()
    compress_with_upx()
    copy_binary_to_output()
    
    print("\n" + "=" * 50)
    print("✅ 构建完成！")
    print(f"CLI工具文件位于 ../module/bin/ 目录")
    print("✓ 二进制文件: ../module/bin/device_faker_cli")
    print("\n请将 module/ 目录打包为 ZIP 文件后通过root管理器安装")
    print("=" * 50)

if __name__ == "__main__":
    main()
