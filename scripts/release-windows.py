#!/usr/bin/env python3
# [INPUT]: Windows x64 MSVC、锁定前端/Rust 依赖、固定引擎、Node.js 标准管道的隔离身份 UI 模拟与统一版本。
# [OUTPUT]: 经真实安装验证的单个 NSIS 安装器、内部载荷清单与校验摘要；构建不接触私钥。
# [POS]: Windows 发布构建边界；先验证再产出，更新签名由独立 CI 步骤完成。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

ROOT=Path(__file__).resolve().parent.parent
ENV={k:v for k,v in os.environ.items() if not k.startswith(('APPLE_','TAURI_SIGNING_PRIVATE_KEY'))}

def run(*args):
    subprocess.run(list(args),cwd=ROOT,env=ENV,check=True)

def main():
    if platform.system()!='Windows' or platform.machine().upper() not in ('AMD64','X86_64'):raise SystemExit('Windows x64 MSVC is required')
    version=os.environ.get('CC_RELEASE_VERSION',json.loads((ROOT/'package.json').read_text())['version'])
    phase=sys.argv[1] if len(sys.argv)>1 else 'all'
    if phase not in ['all','check','build','verify']:raise SystemExit('Invalid build phase')
    if phase=='verify':
        run(sys.executable,'scripts/verify-windows-installer.py',str(ROOT/'releases'/f'CCBaga-{version}-windows-x86_64-setup.exe'),str(ROOT/'releases/Windows-engine-manifest.json'))
        return
    if phase in ['all','check']:
        run('bun','test');run('bun','run','build')
        checks=[
            ['bun','run','test:identities'],
            ['bun','scripts/test-skill-install.ts'],
            [sys.executable,'skills/ccbaga-network/scripts/test_network.py'],
            ['cargo','test','--locked','--manifest-path','src-tauri/Cargo.toml','--features','custom-protocol'],
            [sys.executable,'scripts/test-windows.py'],
        ]
        failed=[]
        for command in checks:
            result=subprocess.run(command,cwd=ROOT,env=ENV)
            if result.returncode:failed.append(command[0])
        if failed:raise SystemExit('Windows checks failed: '+', '.join(failed))
        if phase=='check':return
    run('bun','run','tauri','build','--bundles','nsis','--features','custom-protocol','--config',json.dumps({'version':version}))
    installers=list((ROOT/'src-tauri/target/release/bundle/nsis').glob('*-setup.exe'))
    if len(installers)!=1:raise SystemExit('Expected exactly one NSIS installer')
    output=ROOT/'releases';output.mkdir(exist_ok=True)
    installer=output/f'CCBaga-{version}-windows-x86_64-setup.exe'
    shutil.copyfile(installers[0],installer)
    engine=ROOT/'browser-runtime/WindowsBrowser'
    manifest={str(p.relative_to(engine)).replace('\\','/'):hashlib.file_digest(p.open('rb'),'sha256').hexdigest() for p in engine.rglob('*') if p.is_file()}
    (output/'Windows-engine-manifest.json').write_text(json.dumps(manifest,sort_keys=True)+'\n')
    if phase=='all':run(sys.executable,'scripts/verify-windows-installer.py',str(installer),str(output/'Windows-engine-manifest.json'))
    digest=hashlib.file_digest(installer.open('rb'),'sha256').hexdigest()
    (output/'Windows-SHA256SUMS').write_text(f'{digest}  {installer.name}\n')
    print('Windows NSIS installer and unchanged companion payload verified.')

if __name__=='__main__':main()
