#!/usr/bin/env python3
# [INPUT]: Browser.app 或显式 --app 候选包、CC_SIGN_IDENTITY 与仅供 Chromium Renderer/GPU 的 JIT entitlement。
# [OUTPUT]: 内层 Mach-O、Helper 应用、Framework 与浏览器外壳按依赖顺序签名，正式证书启用 Hardened Runtime/可信时间戳；本机临时签名清除 runtime，避免无 Team ID 的 Framework 被库验证拒绝。
# [POS]: 浏览器品牌化后的签名边界；不使用深层自动重签，不读取 Apple 私钥，公证由发布脚本独立执行。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import argparse
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
APP = ROOT / 'browser-runtime/Browser.app'
JIT = ROOT / 'browser-runtime/entitlements.plist'
IDENTITY = os.environ.get('CC_SIGN_IDENTITY', '-')
MAGIC = {b'\xfe\xed\xfa\xce', b'\xce\xfa\xed\xfe', b'\xfe\xed\xfa\xcf', b'\xcf\xfa\xed\xfe', b'\xca\xfe\xba\xbe', b'\xbe\xba\xfe\xca'}


def sign(path, jit=False):
    args = ['/usr/bin/codesign', '--force', '--sign', IDENTITY, '--options', '0' if IDENTITY == '-' else 'runtime']
    args += ['--timestamp=none'] if IDENTITY == '-' else ['--timestamp']
    if jit:
        args += ['--entitlements', str(JIT)]
    if path.name in ['Chromium', 'CCBrowser']:
        args += ['--identifier', 'ai.local.requirement-french.browser']
    subprocess.run([*args, str(path)], check=True)


def main(app=None):
    app = app if app is not None else APP
    if not (app / 'Contents/MacOS/Chromium').is_file():
        raise SystemExit(f'Missing {app}/Contents/MacOS/Chromium')
    binaries, bundles = [], []
    for base, directories, files in os.walk(app, followlinks=False):
        directories[:] = [name for name in directories if not (Path(base) / name).is_symlink()]
        for name in directories:
            path = Path(base) / name
            if path.suffix in ['.app', '.framework']:
                bundles.append(path)
        for name in files:
            path = Path(base) / name
            if path.is_symlink():
                continue
            with path.open('rb') as stream:
                if stream.read(4) in MAGIC:
                    binaries.append(path)
    # ----- 先签叶子，再封装容器；仅 Renderer/GPU 获得 V8/SwiftShader JIT 权限 -----
    for path in sorted(binaries, key=lambda p: len(p.parts), reverse=True):
        sign(path)
    for path in sorted(bundles, key=lambda p: len(p.parts), reverse=True):
        sign(path, path.name in ['Chromium Helper (Renderer).app', 'Chromium Helper (GPU).app'])
    sign(app)
    subprocess.run(['/usr/bin/codesign', '--verify', '--deep', '--strict', str(app)], check=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--app', type=Path, default=APP)
    main(parser.parse_args().app)
