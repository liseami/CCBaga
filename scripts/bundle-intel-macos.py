#!/usr/bin/env python3
# [INPUT]: Self-built Fingerprint Chromium 148 x86_64 app, locked CCBaga source and local toolchain.
# [OUTPUT]: Ad-hoc signed personal DMG containing A and B; never publishes or installs applications.
# [POS]: Personal Intel build entry. Official release/signing workflows remain separate.
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
VERSION = '148.0.7778.215'
MAGIC = {b'\xfe\xed\xfa\xce', b'\xce\xfa\xed\xfe', b'\xfe\xed\xfa\xcf', b'\xcf\xfa\xed\xfe', b'\xca\xfe\xba\xbe', b'\xbe\xba\xfe\xca', b'\xca\xfe\xba\xbf', b'\xbf\xba\xfe\xca'}
def run(*args, **kwargs):
    subprocess.run(list(map(str,args)), check=True, **kwargs)
def sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()
def verify_code(app):
    count = 0
    for base, dirs, files in os.walk(app, followlinks=False):
        dirs[:] = [d for d in dirs if not (Path(base)/d).is_symlink()]
        for name in files:
            path = Path(base)/name
            if path.is_symlink(): continue
            with path.open('rb') as f: magic = f.read(4)
            if magic in MAGIC:
                run('/usr/bin/lipo', path, '-verify_arch', 'x86_64')
                count += 1
    if not count: raise RuntimeError('No Mach-O code found in browser app')
    return count

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--browser-app', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--build-label', default='')
    args = parser.parse_args()
    if args.build_label and not re.fullmatch(r'[a-z0-9-]+', args.build_label):
        raise SystemExit('Build label must contain lowercase letters, digits or hyphens')
    if os.uname().sysname != 'Darwin': raise SystemExit('macOS build host required')
    app = args.browser_app.resolve()
    info = plistlib.loads((app/'Contents/Info.plist').read_bytes())
    if info['CFBundleShortVersionString'] != VERSION:
        raise SystemExit(f'Expected Fingerprint Chromium {VERSION}, got {info["CFBundleShortVersionString"]}')
    verify_code(app)
    binary = app/'Contents/MacOS'/info['CFBundleExecutable']
    run('node', ROOT/'scripts/test-intel-browser.cjs', binary, cwd=ROOT)
    target = ROOT/'browser-runtime/Browser.app'
    origin = ROOT/'browser-runtime/intel-origin.json'
    framework = app/'Contents/Frameworks/Chromium Framework.framework/Chromium Framework'
    expected = {'version':VERSION,'chromium_sha256':sha256(binary),'framework_sha256':sha256(framework),'architecture':'x86_64'}
    if target.exists():
        if not origin.exists() or json.loads(origin.read_text()) != expected:
            raise SystemExit('Prepared Browser.app has a different or unknown origin; refusing to overwrite')
        verify_code(target)
    else:
        run('/usr/bin/ditto', app, target)
        origin.write_text(json.dumps(expected,indent=2)+'\n')
    env = {k:v for k,v in os.environ.items() if not k.startswith(('APPLE_','TAURI_SIGNING_PRIVATE_KEY','PROD_MACOS_','MACOS_CERTIFICATE'))}
    env.update(CC_MACOS_ARCH='x86_64', CC_SIGN_IDENTITY='-', MACOSX_DEPLOYMENT_TARGET='12.0')
    run('bun', 'run', 'tauri', 'build', '--target', 'x86_64-apple-darwin', '--bundles', 'app', '--features', 'custom-protocol', '--config', ROOT/'src-tauri/tauri.macos.conf.json', cwd=ROOT, env=env)
    host = ROOT/'src-tauri/target/x86_64-apple-darwin/release/bundle/macos/CCBaga-A.app'
    if os.environ.get('CARGO_TARGET_DIR'):
        host = Path(os.environ['CARGO_TARGET_DIR'])/'x86_64-apple-darwin/release/bundle/macos/CCBaga-A.app'
    verify_code(host)
    verify_code(target)
    embedded = host/'Contents/Resources/browser-runtime/Browser.zip'
    if not embedded.is_file() or sha256(embedded) != sha256(ROOT/'browser-runtime/Browser.zip'):
        raise RuntimeError('A is missing the exact built Intel browser archive')
    for bundle in (host,target): run('/usr/bin/codesign','--verify','--deep','--strict',bundle)
    # A launches Chromium directly; validate that entry after branding and signing.
    # The upstream launcher may activate A even with a disposable profile; test the raw engine only.
    run('node', ROOT/'scripts/test-intel-browser.cjs', target/'Contents/MacOS/Chromium', cwd=ROOT)
    args.output.mkdir(parents=True,exist_ok=True)
    version = json.loads((ROOT/'package.json').read_text())['version']
    label = f'-{args.build_label}' if args.build_label else ''
    dmg = args.output.resolve()/f'CCBaga-{version}-macos-x86_64-personal{label}.dmg'
    if dmg.exists(): raise SystemExit('DMG already exists; refusing to replace it')
    with tempfile.TemporaryDirectory(prefix='ccbaga-intel-dmg-') as folder:
        stage = Path(folder)
        run('/usr/bin/ditto',host,stage/'CCBaga-A.app')
        run('/usr/bin/ditto',target,stage/'CCBaga-B.app')
        (stage/'Applications').symlink_to('/Applications')
        shutil.copyfile(ROOT/'LICENSE',stage/'LICENSE.CCBaga.txt')
        shutil.copyfile(ROOT/'NOTICE.md',stage/'NOTICE.md')
        (stage/'个人构建说明.txt').write_text('Intel x86_64 personal build. Both A and B are included.\nAd-hoc signed; not an official liseami release and not Apple notarized.\nUpdate by replacing both apps with a complete Intel installer.\n')
        run('/usr/bin/hdiutil','create','-volname','CCBaga Intel Personal','-srcfolder',stage,'-format','UDZO',dmg)
    mounted = plistlib.loads(subprocess.check_output(['/usr/bin/hdiutil','attach','-readonly','-nobrowse','-plist',str(dmg)]))
    volumes = [Path(e['mount-point']) for e in mounted['system-entities'] if 'mount-point' in e]
    try:
        if len(volumes) != 1: raise RuntimeError('Expected one DMG volume')
        volume = volumes[0]
        if {p.name for p in volume.glob('*.app')} != {'CCBaga-A.app','CCBaga-B.app'}:
            raise RuntimeError('DMG must contain both A and B')
        for name,identifier in [('CCBaga-A.app','ai.local.requirement-french'),('CCBaga-B.app','ai.local.requirement-french.browser')]:
            bundle = volume/name
            info = plistlib.loads((bundle/'Contents/Info.plist').read_bytes())
            if info['CFBundleIdentifier'] != identifier: raise RuntimeError('Wrong bundle identifier')
            verify_code(bundle)
            run('/usr/bin/codesign','--verify','--deep','--strict',bundle)
        if sha256(volume/'CCBaga-A.app/Contents/Resources/browser-runtime/Browser.zip') != sha256(embedded):
            raise RuntimeError('DMG embedded browser archive changed during packaging')
    finally:
        for volume in volumes: run('/usr/bin/hdiutil','detach',volume)
    with dmg.open('rb') as f: digest=hashlib.file_digest(f,'sha256').hexdigest()
    (args.output/'SHA256SUMS.txt').write_text(f'{digest}  {dmg.name}\n')
    print('Created personal A+B installer:',dmg)
if __name__ == '__main__': main()
