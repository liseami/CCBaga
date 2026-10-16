#!/usr/bin/env python3
# [INPUT]: 本机 macOS 架构或明确的架构名称、待打包 Mach-O/应用包。
# [OUTPUT]: arm64/x86_64 的一致命名、更新平台键与全包架构校验。
# [POS]: macOS 引擎、启动器、开发安装和发布的共同架构边界，不改变产品功能。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import argparse
from pathlib import Path
import platform
import subprocess

MAGIC = {b'\xfe\xed\xfa\xce', b'\xce\xfa\xed\xfe', b'\xfe\xed\xfa\xcf', b'\xcf\xfa\xed\xfe',
         b'\xca\xfe\xba\xbe', b'\xbe\xba\xfe\xca', b'\xca\xfe\xba\xbf', b'\xbf\xba\xfe\xca'}


def architecture(machine=None):
    machine = machine or platform.machine()
    aliases = {'arm64': 'arm64', 'aarch64': 'arm64', 'x86_64': 'x86_64', 'AMD64': 'x86_64'}
    if machine not in aliases:
        raise RuntimeError(f'Unsupported macOS architecture: {machine}')
    return aliases[machine]


def updater_target(arch=None):
    return {'arm64': 'darwin-aarch64', 'x86_64': 'darwin-x86_64'}[architecture(arch)]


def check_binary(path, arch=None):
    expected = architecture(arch)
    arches = subprocess.check_output(['lipo', '-archs', str(path)], text=True).split()
    if expected not in arches:
        raise RuntimeError(f'Missing {expected} code in {path}: {arches}')
    return arches


def check_bundle(app, arch=None):
    app = app.resolve()
    binaries = {}
    for path in app.rglob('*'):
        if path.is_symlink():
            if not path.exists() or not path.resolve().is_relative_to(app):
                raise RuntimeError(f'Invalid engine symlink: {path}')
        elif path.is_file():
            with path.open('rb') as stream:
                magic = stream.read(4)
            if magic in MAGIC:
                binaries[path.relative_to(app).as_posix()] = check_binary(path, arch)
    if not binaries:
        raise RuntimeError(f'No Mach-O executable in {app}')
    return binaries


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', type=Path)
    args = parser.parse_args()
    if args.check:
        check_bundle(args.check)
    else:
        print(architecture())
