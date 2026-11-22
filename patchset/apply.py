#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
CC Switch 定制补丁一键应用与构建工具
用于基于官方 v4.0.20 源码干净应用定制补丁集。
"""

import os
import sys
import subprocess
import shutil

BASE_REPO = "https://github.com/farion1231/cc-switch.git"
BASE_TAG = "v4.0.20"
BUILD_DIR = "cc-switch-custom-build"

def run(cmd, cwd=None):
    print(f"==> 执行: {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=cwd)
    if r.returncode != 0:
        print(f"错误: 命令执行失败 (exit code {r.returncode})", file=sys.stderr)
        sys.exit(r.returncode)

def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    patch_file = os.path.join(root, "patchset", "changes.diff")

    if not os.path.exists(patch_file):
        print(f"错误: 未找到补丁文件 {patch_file}", file=sys.stderr)
        sys.exit(1)

    target_dir = os.path.join(os.getcwd(), BUILD_DIR)
    if not os.path.exists(target_dir):
        print(f"==> 克隆上游仓库: {BASE_REPO} (tag: {BASE_TAG})")
        run(["git", "clone", "--depth", "1", "--branch", BASE_TAG, BASE_REPO, target_dir])
    else:
        print(f"==> 使用已有构建目录: {target_dir}")

    print("==> 应用增量补丁集...")
    run(["git", "apply", "--whitespace=nowarn", patch_file], cwd=target_dir)

    print("==> 补丁应用成功！")
    print(f"源码目录就绪: {target_dir}")
    print("你可以进入该目录执行编译：")
    print("  cd cc-switch-custom-build")
    print("  pnpm install")
    print("  pnpm tauri build --no-bundle")

if __name__ == "__main__":
    main()
