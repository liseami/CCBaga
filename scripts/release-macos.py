#!/usr/bin/env python3
# [INPUT]: 双应用源代码、外部 Developer ID 与 Apple API 凭据环境变量、macOS 发布工具。
# [OUTPUT]: 公证并装订票据的双应用 DMG、SHA256SUMS 和实际签名/公证记录。
# [POS]: 发布唯一顺序入口；浏览器票据先进入宿主资源，签名后的包不再修改内容。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
BROWSER = ROOT / 'browser-runtime/Browser.app'
HOST = ROOT / 'src-tauri/target/release/bundle/macos/CC清理器.app'


def run(*args, env=None):
    subprocess.run([str(x) for x in args], cwd=ROOT, env=env, check=True)


def main():
    required = ['CC_SIGN_IDENTITY', 'APPLE_API_KEY', 'APPLE_API_ISSUER', 'APPLE_API_KEY_PATH']
    for key in required:
        if not os.environ.get(key):
            raise SystemExit(f'Missing environment variable: {key}')
    identity = os.environ['CC_SIGN_IDENTITY']
    if identity == '-':
        raise SystemExit('Public releases require a Developer ID Application identity')
    if not Path(os.environ['APPLE_API_KEY_PATH']).is_file():
        raise SystemExit('Apple API key file does not exist')
    version = os.environ.get('CC_RELEASE_VERSION', json.loads((ROOT / 'package.json').read_text())['version'])
    if not __import__('re').fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', version):
        raise SystemExit('Release version must have three numeric components')
    output = ROOT / 'releases'
    output.mkdir(exist_ok=True)
    records = []
    auth = ['--key', os.environ['APPLE_API_KEY_PATH'], '--key-id', os.environ['APPLE_API_KEY'], '--issuer', os.environ['APPLE_API_ISSUER']]

    def notarize(archive, ticket_target):
        print(f'Notarizing {archive.name}', flush=True)
        result = subprocess.run(['xcrun', 'notarytool', 'submit', str(archive), *auth, '--no-s3-acceleration', '--wait', '--output-format', 'json'], cwd=ROOT, check=True, capture_output=True, text=True)
        record = json.loads(result.stdout)
        records.append({'artifact': archive.name, 'id': record['id'], 'status': record.get('status')})
        (output / 'notarization.json').write_text(json.dumps(records, indent=2) + '\n')
        if record.get('status') != 'Accepted':
            log = output / f"notary-{record['id']}.json"
            run('xcrun', 'notarytool', 'log', record['id'], *auth, log)
            raise SystemExit(f'Apple rejected {archive.name}; inspect {log}')
        print(f"Accepted: {record['id']}", flush=True)
        run('xcrun', 'stapler', 'staple', ticket_target)
        run('xcrun', 'stapler', 'validate', ticket_target)

    run('sh', 'scripts/make-icons.sh')
    run('sh', 'scripts/package-browser.sh')
    notarize(ROOT / 'browser-runtime/Browser.zip', BROWSER)
    # ----- 先嵌入浏览器票据，再封住宿主资源；禁止再次品牌化或重签浏览器 -----
    (ROOT / 'browser-runtime/Browser.zip').unlink()
    run('ditto', '-c', '-k', '--keepParent', BROWSER, ROOT / 'browser-runtime/Browser.zip')
    build_env = os.environ.copy()
    for key in required[1:]:
        build_env.pop(key, None)
    build_env['MACOSX_DEPLOYMENT_TARGET'] = '12.0'
    config = json.dumps({'version': version, 'build': {'beforeBuildCommand': 'bun run build'}, 'bundle': {'macOS': {'signingIdentity': identity}}})
    run('bun', 'run', 'tauri', 'build', '--bundles', 'app', '--features', 'custom-protocol', '--config', config, env=build_env)
    with tempfile.TemporaryDirectory(prefix='cctool-release-') as temporary:
        work = Path(temporary)
        archive = work / 'CCCleaner.zip'
        run('ditto', '-c', '-k', '--keepParent', HOST, archive)
        notarize(archive, HOST)
        stage = work / 'CCTool'
        stage.mkdir()
        for app in [HOST, BROWSER]:
            run('codesign', '--verify', '--deep', '--strict', app)
            run('spctl', '--assess', '--type', 'execute', '--verbose=2', app)
            run('ditto', app, stage / app.name)
        (stage / 'Applications').symlink_to('/Applications')
        for name in ['LICENSE', 'NOTICE.md']:
            run('ditto', ROOT / name, stage / name)
        (stage / 'Install.txt').write_text('Drag both apps into Applications. Requires Apple Silicon and macOS 12 or later.\n')
        dmg = output / f'CCTool-{version}-macos-arm64.dmg'
        run('hdiutil', 'create', '-volname', 'CCTool', '-srcfolder', stage, '-format', 'UDZO', '-ov', dmg)
        run('codesign', '--force', '--sign', identity, '--timestamp', dmg)
        notarize(dmg, dmg)
        run('spctl', '--assess', '--type', 'open', '--context', 'context:primary-signature', '--verbose=2', dmg)
    with dmg.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    (output / 'SHA256SUMS').write_text(f'{digest}  {dmg.name}\n')
    print(f'Release verified: {dmg}', flush=True)


if __name__ == '__main__':
    main()
