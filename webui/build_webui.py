#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (c) 2025-2026 Seyud
# Device Faker: https://github.com/Seyud/device_faker

"""
Device Faker WebUI 构建脚本
这个脚本自动化了 Vue.js 项目的常见构建任务
"""

import os
import sys
import subprocess
import argparse
import logging
from pathlib import Path
from typing import Optional


def configure_utf8_console() -> None:
    """优先将控制台输出切换为 UTF-8，避免 Windows 下 emoji 输出导致构建脚本崩溃。"""
    for stream_name in ("stdout", "stderr"):
        stream = getattr(sys, stream_name, None)
        if stream is not None and hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")


configure_utf8_console()

# 设置日志
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


def write_console_line(line: str) -> None:
    """稳健地向控制台输出一行，避免 Windows 控制台编码导致构建脚本崩溃。"""
    text = f"{line}\n"
    try:
        sys.stdout.write(text)
    except UnicodeEncodeError:
        encoding = sys.stdout.encoding or "utf-8"
        safe_text = text.encode(encoding, errors="replace").decode(encoding, errors="replace")
        sys.stdout.write(safe_text)
    sys.stdout.flush()

class WebUIBuilder:
    """WebUI 项目构建器"""
    
    def __init__(self, project_dir: Optional[str] = None):
        """初始化构建器
        
        Args:
            project_dir: 项目目录路径，默认为当前目录
        """
        self.project_dir = Path(project_dir) if project_dir else Path(__file__).parent
        # 根据 vite.config.ts 检测实际构建输出目录
        vite_config = self.project_dir / "vite.config.ts"
        if vite_config.exists():
            # 默认输出到 ../module/webroot
            self.dist_dir = self.project_dir / ".." / "module" / "webroot"
        else:
            self.dist_dir = self.project_dir / "dist"
        self.node_modules = self.project_dir / "node_modules"
        self.package_json = self.project_dir / "package.json"
        self.node_path = None  # Node.js 路径
        
        if not self.package_json.exists():
            raise FileNotFoundError(f"在 {self.project_dir} 中找不到 package.json")
    
    def check_node_npm(self) -> bool:
        """检查 Node.js 和 npm 是否已安装"""
        # 直接使用 D:\nodejs 路径
        node_path = r"D:\nodejs"
        node_exe = os.path.join(node_path, "node.exe")
        npm_exe = os.path.join(node_path, "npm.cmd")  # Windows 下 npm 是 cmd 文件
        
        if os.path.exists(node_exe) and os.path.exists(npm_exe):
            logger.info(f"在 {node_path} 找到 Node.js")
            try:
                node_version = subprocess.run(
                    [node_exe, "--version"], 
                    capture_output=True, 
                    text=True, 
                    check=True
                ).stdout.strip()
                npm_version = subprocess.run(
                    [npm_exe, "--version"], 
                    capture_output=True, 
                    text=True, 
                    check=True
                ).stdout.strip()
                logger.info(f"Node.js 版本: {node_version}")
                logger.info(f"npm 版本: {npm_version}")
                self.node_path = node_path  # 保存 Node.js 路径
                return True
            except subprocess.CalledProcessError as e:
                logger.error(f"检查 Node.js 版本失败: {e}")
                return False
        
        logger.error(f"未找到 Node.js 或 npm，请确保 Node.js 已正确安装在：{node_path}")
        return False
    
    def install_dependencies(self) -> bool:
        """安装 npm 依赖"""
        if self.node_modules.exists():
            logger.info("依赖已存在，跳过安装")
            return True
        
        logger.info("正在安装 npm 依赖...")
        return self.run_command(["npm", "install"], "npm 依赖安装")
    
    def run_command(self, command: list, description: str, use_npm: bool = True) -> bool:
        """执行命令并返回是否成功，实时显示输出
        
        Args:
            command: 要执行的命令列表
            description: 命令描述
            use_npm: 是否使用npm路径
        """
        logger.info(f"正在执行: {description}")
        
        # 如果需要使用 npm 且有指定的 node_path，则使用完整路径
        if use_npm and self.node_path:
            npm_path = os.path.join(self.node_path, "npm.cmd")
            if command[0] == "npm":
                command[0] = npm_path
        
        try:
            # 使用 Popen 实现实时输出
            process = subprocess.Popen(
                command,
                cwd=self.project_dir,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,  # 合并 stderr 到 stdout
                text=True,
                encoding='utf-8',
                errors='ignore',
                bufsize=1,  # 行缓冲
                universal_newlines=True
            )
            
            # 确保 stdout 不为 None
            if process.stdout is None:
                logger.error(f"{description} 失败: 无法获取输出流")
                return False
            
            # 实时读取并输出
            while True:
                line = process.stdout.readline()
                if line:
                    write_console_line(line.rstrip())
                elif process.poll() is not None:
                    # 进程已结束，读取剩余输出
                    remaining = process.stdout.read()
                    if remaining:
                        for extra_line in remaining.splitlines():
                            write_console_line(extra_line)
                    break
            
            return_code = process.returncode
            
            if return_code == 0:
                logger.info(f"{description} 完成")
            else:
                logger.error(f"{description} 失败，返回码: {return_code}")
            
            return return_code == 0
            
        except FileNotFoundError:
            logger.error(f"{description} 失败: 找不到命令 {command[0]}")
            return False
        except Exception as e:
            logger.error(f"{description} 失败: {e}")
            return False
    
    def dev(self) -> None:
        """启动开发服务器"""
        if not self._ensure_dependencies():
            return
        logger.info("启动开发服务器...")
        
        # 使用正确的 npm 路径
        if self.node_path:
            npm_path = os.path.join(self.node_path, "npm.cmd")
            command = [npm_path, "run", "dev"]
        else:
            command = ["npm", "run", "dev"]
        
        try:
            subprocess.run(command, cwd=self.project_dir)
        except KeyboardInterrupt:
            logger.info("开发服务器已停止")
    
    def build(self) -> bool:
        """构建生产版本"""
        if not self.check_node_npm():
            return False

        logger.info("开始构建生产版本...")

        # 步骤 0: 依赖一致性检查与安装（复现 CI 的 npm ci，提前发现 lock 不同步）
        # CI 第 8 步用 `npm ci` 严格校验 package.json 与 package-lock.json 是否同步，
        # 本地预先跑一遍可在提交前暴露 "Missing ... from lock file" 之类的问题。
        logger.info("=== 依赖一致性检查 (npm ci) ===")
        ci_check = self.run_command(
            ["npm", "ci", "--no-audit", "--no-fund"],
            "npm ci 依赖一致性校验",
            use_npm=True,
        )
        if not ci_check:
            logger.error("❌ 依赖一致性检查失败：package.json 与 package-lock.json 不同步")
            logger.error("   请在 webui 目录运行 `npm install` 更新 lock 文件后再提交，避免 CI 失败")
            return False
        logger.info("✅ 依赖一致性检查通过")

        # 步骤 1: TypeScript 类型检查（强制检查，失败时停止构建）
        logger.info("=== TypeScript 类型检查 ===")
        type_check_success = self.run_command(["npm", "run", "type-check"], "TypeScript 类型检查", use_npm=True)
        if not type_check_success:
            logger.error("❌ TypeScript 类型检查失败，构建终止")
            return False
        logger.info("✅ TypeScript 类型检查通过")
        
        # 步骤 2: 代码检查（失败时自动修复后重试）
        logger.info("=== 代码检查 ===")
        lint_success = self.run_command(["npm", "run", "lint"], "代码检查", use_npm=True)
        if not lint_success:
            logger.warning("❌ 代码检查失败，尝试自动修复...")
            # 尝试自动修复
            fix_success = self.run_command(["npm", "run", "lint:fix"], "代码检查自动修复", use_npm=True)
            if not fix_success:
                logger.error("❌ 自动修复失败，构建终止")
                return False
            
            # 重新检查
            logger.info("=== 重新检查代码 ===")
            lint_success = self.run_command(["npm", "run", "lint"], "代码检查（修复后）", use_npm=True)
            if not lint_success:
                logger.error("❌ 修复后代码检查仍失败，构建终止")
                return False
        
        logger.info("✅ 代码检查通过")
        
        # 步骤 3: 构建生产版本
        logger.info("=== 构建生产版本 ===")
        success = self.run_command(["npm", "run", "build"], "构建生产版本", use_npm=True)
        
        if success and self.dist_dir.exists():
            dist_size = self._get_directory_size(self.dist_dir)
            logger.info(f"构建完成！输出目录: {self.dist_dir}")
            logger.info(f"构建产物大小: {dist_size}")
        else:
            logger.error("构建失败")
        
        return success
    

    def lint(self, fix: bool = False) -> bool:
        """运行代码检查
        
        Args:
            fix: 是否自动修复问题
        """
        command = ["npm", "run", "lint"]
        if fix:
            command = ["npm", "run", "lint:fix"]
        
        return self.run_command(command, "代码检查" + (" (自动修复)" if fix else ""), use_npm=True)
    
    def format(self, check: bool = False) -> bool:
        """格式化代码
        
        Args:
            check: 是否只检查格式，不实际修改
        """
        command = ["npm", "run", "format"]
        if check:
            command = ["npm", "run", "format:check"]
        
        return self.run_command(command, "代码格式化" + (" (检查)" if check else ""), use_npm=True)
    
    def type_check(self) -> bool:
        """运行 TypeScript 类型检查"""
        return self.run_command(["npm", "run", "type-check"], "TypeScript 类型检查", use_npm=True)
    
    def clean(self) -> None:
        """清理构建产物"""
        if self.dist_dir.exists():
            import shutil
            shutil.rmtree(self.dist_dir)
            logger.info("已清理构建产物")
        
        if self.node_modules.exists():
            import shutil
            # 只在明确要求清理依赖时才清理
            logger.info("注意：node_modules 未删除，如需清理请使用 --clean-deps 参数")
    
    def clean_all(self) -> None:
        """清理所有生成的文件"""
        import shutil
        
        # 清理构建产物
        if self.dist_dir.exists():
            shutil.rmtree(self.dist_dir)
            logger.info("已清理构建产物")
        
        # 清理 node_modules
        if self.node_modules.exists():
            shutil.rmtree(self.node_modules)
            logger.info("已清理依赖")
        
        # 清理其他可能的缓存文件
        cache_dirs = [".vite", ".cache", "coverage"]
        for cache_dir in cache_dirs:
            cache_path = self.project_dir / cache_dir
            if cache_path.exists():
                shutil.rmtree(cache_path)
                logger.info(f"已清理 {cache_dir}")
    
    def _ensure_dependencies(self) -> bool:
        """确保依赖已安装"""
        if not self.check_node_npm():
            return False
        
        if not self.node_modules.exists():
            return self.install_dependencies()
        
        return True
    
    def _get_directory_size(self, directory: Path) -> str:
        """获取目录大小"""
        total_size = 0
        for file_path in directory.rglob('*'):
            if file_path.is_file():
                total_size += file_path.stat().st_size
        
        # 转换为更友好的单位
        for unit in ['B', 'KB', 'MB', 'GB']:
            if total_size < 1024:
                return f"{total_size:.1f} {unit}"
            total_size /= 1024
        return f"{total_size:.1f} TB"
def main():
    """主函数"""
    parser = argparse.ArgumentParser(description="Device Faker WebUI 构建脚本")
    parser.add_argument(
        "command",
        nargs='?',  # 可选参数
        default='build',  # 默认执行构建
        choices=["dev", "build", "lint", "lint:fix", "format", "format:check", "type-check", "clean", "clean:all"],
        help="要执行的命令（不指定时默认执行构建）"
    )
    parser.add_argument(
        "--project-dir",
        type=str,
        help="项目目录路径（默认为当前目录）"
    )
    parser.add_argument(
        "--skip-deps-check",
        action="store_true",
        help="跳过依赖检查"
    )
    
    args = parser.parse_args()
    
    try:
        builder = WebUIBuilder(args.project_dir)
        
        # 默认执行完整构建流程（类似 build_android.py）
        if args.command == "build":
            write_console_line("机型伪装 WebUI 构建脚本")
            write_console_line("=" * 50)
            
            write_console_line("")
            write_console_line("=== 执行构建流程 ===")
            if not builder.build():
                write_console_line("❌ 构建失败")
                sys.exit(1)
            
            write_console_line("")
            write_console_line("=" * 50)
            write_console_line("✅ 构建完成！")
            write_console_line(f"构建产物位于 {builder.dist_dir} 目录")
            write_console_line("")
            write_console_line("构建产物已集成到模块中")
            write_console_line("=" * 50)
        
        elif args.command == "dev":
            builder.dev()

        elif args.command == "lint":
            success = builder.lint()
            sys.exit(0 if success else 1)
        elif args.command == "lint:fix":
            success = builder.lint(fix=True)
            sys.exit(0 if success else 1)
        elif args.command == "format":
            success = builder.format()
            sys.exit(0 if success else 1)
        elif args.command == "format:check":
            success = builder.format(check=True)
            sys.exit(0 if success else 1)
        elif args.command == "type-check":
            success = builder.type_check()
            sys.exit(0 if success else 1)
        elif args.command == "clean":
            builder.clean()
        elif args.command == "clean:all":
            builder.clean_all()

    
    except KeyboardInterrupt:
        logger.info("操作被用户中断")
        sys.exit(1)
    except Exception as e:
        logger.error(f"执行过程中发生错误: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
