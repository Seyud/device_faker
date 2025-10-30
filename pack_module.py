#!/usr/bin/env python3
"""
打包 Magisk 模块脚本
将 module/ 目录打包为可安装的 ZIP 文件
"""

import os
import zipfile
import datetime

def create_magisk_module_zip():
    """创建 Magisk 模块 ZIP 包"""
    project_root = os.path.dirname(os.path.abspath(__file__))
    module_dir = os.path.join(project_root, "module")
    output_dir = os.path.join(project_root, "output")
    
    # 创建 output 目录（如果不存在）
    os.makedirs(output_dir, exist_ok=True)
    
    # 生成带时间戳的 ZIP 文件名
    timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    zip_filename = f"device_faker_{timestamp}.zip"
    zip_path = os.path.join(output_dir, zip_filename)
    
    print(f"开始打包 Magisk 模块...")
    print(f"输出文件: {zip_filename}")
    
    with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zipf:
        # 遍历 module 目录
        for root, dirs, files in os.walk(module_dir):
            for file in files:
                file_path = os.path.join(root, file)
                # 计算相对路径（相对于 module 目录）
                arcname = os.path.relpath(file_path, module_dir)
                
                # 跳过 list.txt（已废弃）
                if arcname == "list.txt":
                    continue
                
                zipf.write(file_path, arcname)
                print(f"  添加: {arcname}")
    
    # 获取文件大小
    file_size = os.path.getsize(zip_path)
    size_mb = file_size / (1024 * 1024)
    
    print(f"\n✅ 打包完成！")
    print(f"📦 文件: output/{zip_filename}")
    print(f"📏 大小: {size_mb:.2f} MB")
    print(f"\n请将此 ZIP 文件通过root管理器安装")

if __name__ == "__main__":
    create_magisk_module_zip()
