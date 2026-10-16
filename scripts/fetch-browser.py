#!/usr/bin/env python3
# [INPUT]: 固定 arm64 DMG/Linux/Windows 引擎，或显式 SHA-256 的 Intel 源码构建 ZIP，以及平台工具。
# [OUTPUT]: 校验来源与架构后恢复被 Git 排除的 Browser.app、LinuxBrowser 或 WindowsBrowser，完整复制后才公开目标，失败清理暂存目录，不读取用户资料。
# [POS]: 干净检出的引擎准备边界；保留上游完整资源与许可，不下载随时间变化的 latest。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import argparse
import hashlib
import os
import platform
import tarfile
import zipfile
import shutil
from pathlib import Path, PurePosixPath
import plistlib
import subprocess
import tempfile
import sys
import re
from urllib.parse import urlsplit
from macos_arch import architecture, check_bundle

ROOT = Path(__file__).resolve().parent.parent
VERSION = '148.0.7778.215'
URL = f'https://github.com/adryfish/fingerprint-chromium/releases/download/{VERSION}/ungoogled-chromium_{VERSION}-1.1_macos.dmg'
SHA256 = 'b72f091e2e1a7583eed389c4b8e3534ed355e568af8c8bbf8fc30a25e23ca679'


def publish_engine(source, target, copy):
    """Copy beside the destination, then publish a complete prepared engine."""
    staging = Path(tempfile.mkdtemp(prefix=f'.{target.name}-stage-', dir=target.parent))
    pending = staging / 'engine'
    try:
        copy(source, pending)
        pending.rename(target)
    finally:
        try:
            shutil.rmtree(staging)
        except OSError as error:
            raise RuntimeError(f'Could not remove engine staging directory at {staging}') from error


def fetch_linux():
    if platform.machine() not in ('x86_64', 'AMD64'):
        raise SystemExit('The pinned upstream Linux engine only supports x86_64')
    target = ROOT / 'browser-runtime/LinuxBrowser'
    if target.exists():
        raise SystemExit('LinuxBrowser already exists; refusing to overwrite a prepared engine')
    url = f'https://github.com/adryfish/fingerprint-chromium/releases/download/{VERSION}/ungoogled-chromium-{VERSION}-1-x86_64_linux.tar.xz'
    digest = '70d239830332e5820aa34dfcb284161cac0429eee25da642830afe04bda717f4'
    with tempfile.TemporaryDirectory(prefix='ccbaga-engine-') as temporary:
        archive = Path(temporary) / 'browser.tar.xz'
        subprocess.run(['curl', '--fail', '--location', '--retry', '3', '--output', str(archive), url], check=True)
        with archive.open('rb') as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != digest:
                raise SystemExit('Upstream browser SHA-256 mismatch')
        extracted = Path(temporary) / 'extracted'
        with tarfile.open(archive) as tar:
            tar.extractall(extracted, filter='data')
        engines = list(extracted.glob('**/chrome'))
        if len(engines) != 1:
            raise SystemExit('Expected exactly one Linux Chromium executable')
        binary = engines[0]
        with binary.open('rb') as stream:
            elf = stream.read(20)
        if elf[:5] != b'\x7fELF\x02' or elf[18:20] != b'\x3e\x00':
            raise SystemExit('Expected an x86_64 ELF executable')
        actual = subprocess.check_output([str(binary), '--version'], text=True)
        if VERSION not in actual:
            raise SystemExit('Upstream browser version mismatch')
        (binary.parent / 'version.txt').write_text(VERSION + '\n')
        publish_engine(binary.parent, target, lambda source, pending: shutil.copytree(source, pending, symlinks=True))
    print(f'Fingerprint Chromium {VERSION}: Linux x86_64 SHA-256 verified')


def fetch_windows():
    if platform.machine() not in ('x86_64', 'AMD64'):
        raise SystemExit('The pinned upstream Windows engine only supports x86_64')
    target = ROOT / 'browser-runtime/WindowsBrowser'
    if target.exists():
        raise SystemExit('WindowsBrowser already exists; refusing to overwrite a prepared engine')
    url = f'https://github.com/adryfish/fingerprint-chromium/releases/download/{VERSION}/ungoogled-chromium_{VERSION}-1.1_windows_x64.zip'
    digest = '9ef3f471b7a6641b4224532522b29141ce3746e27d55788d88e2fd951f362579'
    with tempfile.TemporaryDirectory(prefix='ccbaga-win-engine-') as temporary:
        archive = Path(temporary)/'browser.zip'
        subprocess.run(['curl.exe','--fail','--location','--retry','3','--output',str(archive),url],check=True)
        with archive.open('rb') as stream:
            if hashlib.file_digest(stream,'sha256').hexdigest()!=digest: raise SystemExit('Upstream browser SHA-256 mismatch')
        extracted=Path(temporary)/'extracted'
        with zipfile.ZipFile(archive) as zipped:
            for item in zipped.infolist():
                path=Path(item.filename)
                if path.is_absolute() or '..' in path.parts or ':' in item.filename or '\\' in item.filename: raise SystemExit('Unsafe Windows engine archive path')
            zipped.extractall(extracted)
        engines = list(extracted.glob('**/chrome.exe'))
        if len(engines) != 1:
            raise SystemExit('Expected exactly one Windows Chromium executable')
        binary = engines[0]
        import struct, os
        with binary.open('rb') as stream:
            if stream.read(2)!=b'MZ':raise SystemExit('Expected a PE browser executable')
            stream.seek(0x3c); offset=struct.unpack('<I',stream.read(4))[0];stream.seek(offset)
            if stream.read(6)!=b'PE\0\0\x64\x86':raise SystemExit('Expected a Windows x64 browser executable')
        actual=subprocess.check_output(['powershell','-NoProfile','-NonInteractive','-Command','(Get-Item -LiteralPath $env:CC_ENGINE_EXE).VersionInfo.ProductVersion'],env=os.environ|{'CC_ENGINE_EXE':str(binary)},text=True)
        if VERSION not in actual:
            raise SystemExit('Upstream browser version mismatch')
        (binary.parent / 'version.txt').write_text(VERSION + '\n')
        publish_engine(binary.parent, target, shutil.copytree)
    print(f'Fingerprint Chromium {VERSION}: Windows x64 SHA-256 verified')


def validate_macos_zip(archive):
    with zipfile.ZipFile(archive) as zipped:
        links = set()
        paths = []
        for item in zipped.infolist():
            path = PurePosixPath(item.filename)
            if path.is_absolute() or '..' in path.parts or '\\' in item.filename:
                raise SystemExit('Unsafe macOS engine archive path')
            paths.append(path)
            if (item.external_attr >> 16) & 0o170000 == 0o120000:
                target = zipped.read(item).decode('utf-8')
                if PurePosixPath(target).is_absolute():
                    raise SystemExit('External macOS engine symlink')
                parts = list(path.parent.parts)
                for part in PurePosixPath(target).parts:
                    if part == '..':
                        if len(parts) <= 1:
                            raise SystemExit('External macOS engine symlink')
                        parts.pop()
                    elif part != '.':
                        parts.append(part)
                links.add(path)
        if any(parent in links for path in paths for parent in path.parents):
            raise SystemExit('Archive entry descends through a symlink')


def accept_macos_app(app, arch, target, report):
    metadata = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
    if metadata['CFBundleShortVersionString'] != VERSION:
        raise SystemExit('Upstream browser version mismatch')
    binary = app / 'Contents/MacOS' / metadata['CFBundleExecutable']
    check_bundle(app, arch)
    if not binary.is_file():
        raise SystemExit('Missing macOS browser executable')
    if arch == 'x86_64':
        subprocess.run([sys.executable, str(ROOT / 'scripts/verify-macos-intel-engine.py'), str(app), '--report', str(report)], check=True)
    publish_engine(app, target, lambda source, pending: subprocess.run(['ditto', str(source), str(pending)], check=True))


def fetch_macos(archive=None, digest=None, url=None):
    arch = architecture()
    if archive and url:
        raise SystemExit('Choose a local macOS archive or a download URL')
    if not archive and not url:
        if arch == 'arm64':
            url, digest = URL, SHA256
        else:
            raise SystemExit('Intel requires the accepted 148 engine ZIP and SHA-256. Build it with scripts/build-macos-intel-engine.py, then pass --macos-archive and --macos-sha256 (or configure CC_MACOS_ENGINE_URL / CC_MACOS_ENGINE_SHA256)')
    if not digest or not re.fullmatch(r'[0-9a-f]{64}', digest):
        raise SystemExit('An explicit lowercase SHA-256 is required for a macOS engine input')
    if url and (urlsplit(url).scheme != 'https' or urlsplit(url).username or urlsplit(url).password):
        raise SystemExit('The macOS engine URL must use HTTPS without embedded credentials')
    target = ROOT / 'browser-runtime/Browser.app'
    if target.exists():
        raise SystemExit('Browser.app already exists; refusing to overwrite a prepared engine')
    with tempfile.TemporaryDirectory(prefix='cctool-engine-') as temporary:
        image = Path(archive).resolve() if archive else Path(temporary) / ('browser.dmg' if arch == 'arm64' else 'browser.zip')
        if url:
            subprocess.run(['curl', '--fail', '--location', '--retry', '3', '--output', str(image), url], check=True)
        with image.open('rb') as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != digest:
                raise SystemExit('Upstream browser SHA-256 mismatch')
        if arch == 'x86_64':
            validate_macos_zip(image)
            extracted = Path(temporary) / 'extracted'
            subprocess.run(['ditto', '-x', '-k', str(image), str(extracted)], check=True)
            apps = list(extracted.glob('*.app'))
            if len(apps) != 1:
                raise SystemExit('Expected exactly one browser app in Intel engine ZIP')
            accept_macos_app(apps[0], arch, target, Path(temporary) / 'verification.json')
            print(f'Fingerprint Chromium {VERSION}: SHA-256, x86_64 and real fingerprint probes verified')
            return
        mounted = plistlib.loads(subprocess.check_output(['hdiutil', 'attach', '-readonly', '-nobrowse', '-plist', str(image)]))
        volumes = [Path(item['mount-point']) for item in mounted['system-entities'] if 'mount-point' in item]
        try:
            apps = [app for volume in volumes for app in volume.glob('*.app')]
            if len(apps) != 1:
                raise SystemExit('Expected exactly one browser app in upstream DMG')
            accept_macos_app(apps[0], arch, target, Path(temporary) / 'verification.json')
        finally:
            for volume in volumes:
                subprocess.run(['hdiutil', 'detach', str(volume)], check=True)
    print(f'Fingerprint Chromium {VERSION}: SHA-256 and arm64 verified')


def main(argv=None):
    parser = argparse.ArgumentParser(description='Prepare the pinned browser engine for the native platform')
    parser.add_argument('--macos-archive', type=Path)
    parser.add_argument('--macos-sha256', default=os.environ.get('CC_MACOS_ENGINE_SHA256') or None)
    parser.add_argument('--macos-url', default=os.environ.get('CC_MACOS_ENGINE_URL') or None)
    args = parser.parse_args(argv)
    if platform.system() == 'Windows':
        return fetch_windows()
    if platform.system() == 'Linux':
        return fetch_linux()
    if platform.system() == 'Darwin':
        return fetch_macos(args.macos_archive, args.macos_sha256, args.macos_url)
    raise SystemExit('Only macOS, Linux and Windows are supported')


if __name__ == '__main__':
    main()
