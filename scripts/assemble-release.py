#!/usr/bin/env python3
# [INPUT]: 各平台验收产物、完整覆盖目标版本的内部 SHA-256 清单与 macOS/Windows 更新签名。
# [OUTPUT]: 七个完整安装器、两个 macOS 更新归档、三平台更新 latest.json 和总校验清单的公开白名单。
# [POS]: 发布任务的白名单装配层；内部签名文件、引擎地图和公证记录留在 CI，不混入下载列表。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import hashlib
import json
import os
from pathlib import Path
import re
from macos_arch import updater_target

ROOT=Path(__file__).resolve().parent.parent
OUT=ROOT/'releases'

def main():
    version=os.environ['RELEASE_VERSION']
    if not re.fullmatch(r'\d+\.\d+\.\d+',version):raise SystemExit('Invalid release version')
    macos = ['arm64', 'x86_64']
    dmgs = [f'CCBaga-{version}-macos-{arch}.dmg' for arch in macos]
    archives = [f'CCBaga-{version}-macos-{arch}.app.tar.gz' for arch in macos]
    windows_installer = f'CCBaga-{version}-windows-x86_64-setup.exe'
    linux = [f'CCBaga-{version}-linux-x86_64.{ext}' for ext in ['deb','rpm','AppImage','pkg.tar.zst']]
    installers = dmgs + [windows_installer] + linux
    required = {f'macOS-{arch}-SHA256SUMS': {dmg, archive, archive+'.sig', f'latest-macos-{arch}.json'}
                for arch, dmg, archive in zip(macos, dmgs, archives)}
    required.update({'Linux-SHA256SUMS': set(linux[:3]), 'Arch-SHA256SUMS': {linux[3]},
                     'Windows-SHA256SUMS': {windows_installer}})
    for name,artifacts in required.items():
        verified=set()
        for line in (OUT/name).read_text(encoding='utf-8').splitlines():
            digest,filename=line.split('  ',1)
            if Path(filename).name!=filename or not re.fullmatch(r'[0-9a-f]{64}',digest):raise SystemExit('Unsafe digest entry')
            with (OUT/filename).open('rb') as stream:
                if hashlib.file_digest(stream,'sha256').hexdigest()!=digest:raise SystemExit('Artifact digest mismatch')
            verified.add(filename)
        if not artifacts <= verified:raise SystemExit('Missing verified artifact in '+name)
    manifests = [json.loads((OUT/f'latest-macos-{arch}.json').read_text(encoding='utf-8')) for arch in macos]
    manifests.append(json.loads((OUT/'latest-windows.json').read_text(encoding='utf-8')))
    manifest = dict(manifests[0])
    manifest['platforms'] = {}
    for entry in manifests:
        if entry['version'] != version:raise SystemExit('Mixed release versions')
        if manifest['platforms'].keys() & entry['platforms'].keys():raise SystemExit('Duplicate updater target')
        manifest['platforms'].update(entry['platforms'])
    expected = {updater_target(arch): archive for arch, archive in zip(macos, archives)}
    expected['windows-x86_64'] = windows_installer
    if set(manifest['platforms'])!=set(expected):raise SystemExit('Unexpected updater target')
    for platform,filename in expected.items():
        entry=manifest['platforms'][platform]
        url=f'https://github.com/liseami/CCBaga/releases/download/v{version}/{filename}'
        if entry['url']!=url or entry['signature']!=(OUT/(filename+'.sig')).read_text(encoding='utf-8').strip():raise SystemExit('Updater URL/signature mismatch')
    manifest['notes']='macOS updates CCBaga-A; Windows updates both applications. Save unfinished browser work before updating.'
    (OUT/'latest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    public=installers+archives+['latest.json']
    digests=[]
    for filename in public:
        with (OUT/filename).open('rb') as stream:digests.append(f"{hashlib.file_digest(stream,'sha256').hexdigest()}  {filename}\n")
    (OUT/'SHA256SUMS').write_text(''.join(digests),encoding='utf-8')
    public.append('SHA256SUMS')
    (ROOT/'release-files.txt').write_text(''.join('releases/'+name+'\n' for name in public),encoding='utf-8')
    labels=['macOS Apple Silicon · A + B','macOS Intel · A + B','Windows x64 · A + B','Linux DEB','Linux RPM','Linux AppImage','Arch/pacman']
    notes=['CCBaga combines recoverable local Claude identities, CLI management, real environment checks and an isolated companion browser.\n','\n| Platform | Installer |\n| --- | --- |\n']
    for label,filename in zip(labels,installers):notes.append(f'| {label} | [Download](https://github.com/liseami/CCBaga/releases/download/v{version}/{filename}) |\n')
    notes+=['\nEach installer contains both applications. On macOS drag both apps into Applications. On Windows run the setup installer; SmartScreen may show a warning because the installer has no Authenticode certificate. Organization policy can restrict installation.\n','\nSidebar updates verify signed downloads: macOS updates A, Windows updates A and B. Linux uses its package manager. The archive and latest.json are used by automatic updates; ordinary installation only needs the platform installer. SHA256SUMS covers public downloads.\n','\n[Usage](https://github.com/liseami/CCBaga#readme) · [Network Skill](https://github.com/liseami/CCBaga/tree/main/skills/ccbaga-network)\n']
    (ROOT/'release-notes.md').write_text(''.join(notes),encoding='utf-8')
    print(f'{len(public)} public files assembled; internal verification material excluded.')

if __name__=='__main__':main()
