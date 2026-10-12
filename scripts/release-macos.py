#!/usr/bin/env python3
# [INPUT]: 双应用源代码、外部 Developer ID、Apple API 与更新签名凭据、哈希锁定的 dmgbuild 与 macOS 发布工具。
# [OUTPUT]: 公证并装订票据的双应用 DMG、带版本签名的主程序更新归档/清单、SHA256SUMS 和实际签名/公证记录。
# [POS]: 发布唯一顺序入口；浏览器票据先进入宿主资源；票据网络失败限次重试，仍要求验收通过；更新私钥只传给签名子进程。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
from datetime import datetime, timezone
import tarfile
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
import dmgbuild

ROOT = Path(__file__).resolve().parent.parent
BROWSER = ROOT / 'browser-runtime/Browser.app'
HOST = ROOT / 'src-tauri/target/release/bundle/macos/CCBaga-A.app'
SIGNING_KEYS = ['TAURI_SIGNING_PRIVATE_KEY', 'TAURI_SIGNING_PRIVATE_KEY_PATH', 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD']
BUILD_ENV = {key: value for key, value in os.environ.items() if key not in SIGNING_KEYS and not key.startswith("APPLE_")}


def run(*args, env=None):
    subprocess.run([str(x) for x in args], cwd=ROOT, env=env if env is not None else BUILD_ENV, check=True)


def staple(action, target):
    # ----- 仅重试票据服务的网络错误；无效票据/签名仍立即失败 -----
    command = ['xcrun', 'stapler', action, str(target)]
    for attempt in range(3):
        result = subprocess.run(command, cwd=ROOT, env=BUILD_ENV, capture_output=True, text=True)
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        if result.returncode == 0:
            return
        network = 'NSURLErrorDomain' in result.stdout + result.stderr
        if result.returncode != 68 or not network or attempt == 2:
            raise subprocess.CalledProcessError(result.returncode, command)
        time.sleep(3 * (attempt + 1))


def main():
    required = ['CC_SIGN_IDENTITY', 'APPLE_API_KEY', 'APPLE_API_ISSUER', 'APPLE_API_KEY_PATH']
    for key in required:
        if not os.environ.get(key):
            raise SystemExit(f'Missing environment variable: {key}')
    signing_env = BUILD_ENV | {key: os.environ[key] for key in SIGNING_KEYS if key in os.environ}
    if not any(signing_env.get(key) for key in SIGNING_KEYS[:2]):
        raise SystemExit('Missing updater signing private key (external path or secret content)')
    # ----- 第三方 DMG 工具的子进程也不继承更新私钥 -----
    for key in SIGNING_KEYS:
        os.environ.pop(key, None)
    identity = os.environ['CC_SIGN_IDENTITY']
    if identity == '-':
        raise SystemExit('Public releases require a Developer ID Application identity')
    if not Path(os.environ['APPLE_API_KEY_PATH']).is_file():
        raise SystemExit('Apple API key file does not exist')
    version = os.environ.get('CC_RELEASE_VERSION', json.loads((ROOT / 'package.json').read_text())['version'])
    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', version):
        raise SystemExit('Release version must have three numeric components')
    output = ROOT / 'releases'
    output.mkdir(exist_ok=True)
    records = []
    auth = ['--key', os.environ['APPLE_API_KEY_PATH'], '--key-id', os.environ['APPLE_API_KEY'], '--issuer', os.environ['APPLE_API_ISSUER']]

    def notarize(archive, ticket_target):
        print(f'Notarizing {archive.name}', flush=True)
        result = subprocess.run(['xcrun', 'notarytool', 'submit', str(archive), *auth, '--no-s3-acceleration', '--wait', '--output-format', 'json'], cwd=ROOT, check=True, capture_output=True, text=True, env=BUILD_ENV)
        record = json.loads(result.stdout)
        records.append({'artifact': archive.name, 'id': record['id'], 'status': record.get('status')})
        (output / 'notarization.json').write_text(json.dumps(records, indent=2) + '\n')
        if record.get('status') != 'Accepted':
            log = output / f"notary-{record['id']}.json"
            run('xcrun', 'notarytool', 'log', record['id'], *auth, log)
            raise SystemExit(f'Apple rejected {archive.name}; inspect {log}')
        print(f"Accepted: {record['id']}", flush=True)
        staple('staple', ticket_target)
        staple('validate', ticket_target)

    run('sh', 'scripts/make-icons.sh')
    run('sh', 'scripts/package-browser.sh')
    notarize(ROOT / 'browser-runtime/Browser.zip', BROWSER)
    # ----- 先嵌入浏览器票据，再封住宿主资源；禁止再次品牌化或重签浏览器 -----
    (ROOT / 'browser-runtime/Browser.zip').unlink()
    run('ditto', '-c', '-k', '--keepParent', BROWSER, ROOT / 'browser-runtime/Browser.zip')
    build_env = BUILD_ENV.copy()
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
        # ----- 更新只签最终装订后的宿主字节；公钥留在应用，私钥不进入构建 -----
        update_archive = output / f'CCBaga-{version}-macos-arm64.app.tar.gz'
        with tarfile.open(update_archive, 'w:gz', dereference=False) as bundle:
            bundle.add(HOST, arcname=HOST.name)
        unpacked = work / 'updater-verification'
        unpacked.mkdir()
        with tarfile.open(update_archive, 'r:gz') as bundle:
            bundle.extractall(unpacked, filter='tar')
        run('codesign', '--verify', '--deep', '--strict', unpacked / HOST.name)
        staple('validate', unpacked / HOST.name)
        run('spctl', '--assess', '--type', 'execute', unpacked / HOST.name)
        subprocess.run(['bun', 'run', 'tauri', 'signer', 'sign', '--app-version', version, str(update_archive)], cwd=ROOT, check=True, capture_output=True, env=signing_env)
        signature = Path(str(update_archive) + '.sig')
        endpoint = json.loads((ROOT / 'src-tauri/tauri.conf.json').read_text())['plugins']['updater']['endpoints'][0]
        repository = endpoint.split('/releases/')[0]
        manifest = output / 'latest.json'
        manifest.write_text(json.dumps({
            'version': version,
            'notes': 'CCBaga-A update. For both applications, download the DMG from GitHub Releases.',
            'pub_date': datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
            'platforms': {'darwin-aarch64': {
                'url': f'{repository}/releases/download/v{version}/{update_archive.name}',
                'signature': signature.read_text().strip()
            }}
        }, indent=2) + '\n')
        verify_env = build_env | {'CC_UPDATE_MANIFEST': str(manifest), 'CC_UPDATE_ARCHIVE': str(update_archive)}
        run('cargo', 'test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', '--features', 'custom-protocol',
            'updater_tests::published_archive_matches_embedded_key_and_signed_version', '--', '--ignored', env=verify_env)
        stage = work / 'CCBaga'
        stage.mkdir()
        for app, name in [(HOST, 'CCBaga-A.app'), (BROWSER, 'CCBaga-B.app')]:
            run('codesign', '--verify', '--deep', '--strict', app)
            run('spctl', '--assess', '--type', 'execute', '--verbose=2', app)
            run('ditto', app, stage / name)
        for name in ['LICENSE', 'NOTICE.md']:
            run('ditto', ROOT / name, stage / name)
        (stage / 'Install.txt').write_text('Drag both apps into Applications. Requires Apple Silicon and macOS 12 or later.\n')
        dmg = output / f'CCBaga-{version}-macos-arm64.dmg'
        dmgbuild.build_dmg(str(dmg), 'CCBaga', settings_file=str(ROOT / 'scripts/dmg-settings.py'), defines={'stage': str(stage), 'root': str(ROOT)})
        run(sys.executable, 'scripts/verify-dmg.py', dmg)
        run('codesign', '--force', '--sign', identity, '--timestamp', dmg)
        notarize(dmg, dmg)
        run('spctl', '--assess', '--type', 'open', '--context', 'context:primary-signature', '--verbose=2', dmg)
    digests = []
    for artifact in [dmg, update_archive, signature, manifest]:
        with artifact.open('rb') as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        digests.append(f'{digest}  {artifact.name}\n')
    (output / 'SHA256SUMS').write_text(''.join(digests))
    print(f'Release verified: {dmg}', flush=True)


if __name__ == '__main__':
    main()
