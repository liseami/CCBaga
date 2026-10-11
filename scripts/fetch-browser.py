#!/usr/bin/env python3
# [INPUT]: 固定上游 Fingerprint Chromium DMG URL 与发布者提供的 SHA-256、macOS hdiutil/ditto。
# [OUTPUT]: 校验来源与架构后恢复被 Git 排除的 browser-runtime/Browser.app。
# [POS]: 干净检出的引擎准备边界；保留上游完整资源与许可，不下载随时间变化的 latest。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import hashlib
from pathlib import Path
import plistlib
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
VERSION = '148.0.7778.215'
URL = f'https://github.com/adryfish/fingerprint-chromium/releases/download/{VERSION}/ungoogled-chromium_{VERSION}-1.1_macos.dmg'
SHA256 = 'b72f091e2e1a7583eed389c4b8e3534ed355e568af8c8bbf8fc30a25e23ca679'


def main():
    target = ROOT / 'browser-runtime/Browser.app'
    if target.exists():
        raise SystemExit('Browser.app already exists; refusing to overwrite a prepared engine')
    with tempfile.TemporaryDirectory(prefix='cctool-engine-') as temporary:
        image = Path(temporary) / 'browser.dmg'
        subprocess.run(['curl', '--fail', '--location', '--retry', '3', '--output', str(image), URL], check=True)
        with image.open('rb') as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != SHA256:
                raise SystemExit('Upstream browser SHA-256 mismatch')
        mounted = plistlib.loads(subprocess.check_output(['hdiutil', 'attach', '-readonly', '-nobrowse', '-plist', str(image)]))
        volumes = [Path(item['mount-point']) for item in mounted['system-entities'] if 'mount-point' in item]
        try:
            apps = [app for volume in volumes for app in volume.glob('*.app')]
            if len(apps) != 1:
                raise SystemExit('Expected exactly one browser app in upstream DMG')
            metadata = plistlib.loads((apps[0] / 'Contents/Info.plist').read_bytes())
            if metadata['CFBundleShortVersionString'] != VERSION:
                raise SystemExit('Upstream browser version mismatch')
            binary = apps[0] / 'Contents/MacOS' / metadata['CFBundleExecutable']
            if 'arm64' not in subprocess.check_output(['lipo', '-archs', str(binary)], text=True).split():
                raise SystemExit('Upstream browser does not contain Apple Silicon code')
            subprocess.run(['ditto', str(apps[0]), str(target)], check=True)
        finally:
            for volume in volumes:
                subprocess.run(['hdiutil', 'detach', str(volume)], check=True)
    print(f'Fingerprint Chromium {VERSION}: SHA-256 and arm64 verified')


if __name__ == '__main__':
    main()
