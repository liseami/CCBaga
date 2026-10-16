#!/usr/bin/env python3
# [INPUT]: 本机 macOS arm64/x86_64 源码、匹配引擎与 Tauri debug 构建工具。
# [OUTPUT]: 临时签名、内嵌 UI 的双应用开发包，事务重装并启动。
# [POS]: 本地快速开发入口；不读取公证凭据、不生成 DMG、不发布，正式发布入口保持独立。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
from macos_arch import architecture, check_bundle

ROOT = Path(__file__).resolve().parent.parent

def development_env():
    return {key: value for key, value in os.environ.items()
            if not key.startswith(('APPLE_', 'TAURI_SIGNING_PRIVATE_KEY'))} | {
                'CC_SIGN_IDENTITY': '-', 'MACOSX_DEPLOYMENT_TARGET': '12.0'}

def main():
    if platform.system() != 'Darwin':
        raise SystemExit('This fast debug installer is for macOS; use docs/linux.md on Linux')
    architecture()
    check_bundle(ROOT / 'browser-runtime/Browser.app')
    config = json.dumps({'bundle': {'macOS': {'signingIdentity': '-'}}})
    env = development_env()
    subprocess.run(['bun', 'run', 'tauri', 'build', '--debug', '--bundles', 'app', '--features', 'custom-protocol', '--config', config], cwd=ROOT, env=env, check=True)
    subprocess.run([str(ROOT / 'browser-runtime/Browser.app/Contents/MacOS/Chromium'), '--version'], cwd=ROOT, env=env, check=True, capture_output=True)
    subprocess.run([sys.executable, 'scripts/install-apps.py', '--development'], cwd=ROOT, env=env, check=True)

if __name__ == '__main__':
    main()
