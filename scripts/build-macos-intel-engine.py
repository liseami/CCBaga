#!/usr/bin/env python3
# [INPUT]: Intel Mac、完整 Xcode/macOS SDK 15+、独立 APFS 工作目录、两个固定提交的公开源码与 macOS 兼容补丁。
# [OUTPUT]: 可恢复编译的 x86_64 Fingerprint Chromium 候选、包含本地补丁摘要的来源记录；通过真实架构/指纹观测后才输出 ZIP 和摘要。
# [POS]: Intel 引擎的独立源码构建入口；不替换现有 Browser.app，不签正式证书或发布。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
SOURCES = {
    'version': '148.0.7778.215',
    'builder': '6a00dfe08591a15cfb0aed0edd292ba789d9a0d7',
    'fingerprint': '13b89eae304123f0710f2d33fd0816a2f61d7ffc',
    'architecture': 'x86_64',
}
BUILD_VOLUME = None
SPACE_FLOOR_GIB = 10
SOURCE_PATCHES = [ROOT / 'scripts/macos-sdk15-fileport.patch',
                  ROOT / 'scripts/macos-sdk15-coregraphics.patch',
                  ROOT / 'scripts/macos-sdk15-appkit.patch',
                  ROOT / 'scripts/macos-sdk15-pasteboard.patch',
                  ROOT / 'scripts/macos-headless-shutdown.patch']


def check_build_space():
    if BUILD_VOLUME and shutil.disk_usage(BUILD_VOLUME).free < SPACE_FLOOR_GIB * 1024 ** 3:
        raise RuntimeError(f'Build stopped to preserve {SPACE_FLOOR_GIB} GiB free on {BUILD_VOLUME}; sources and cached downloads are retained')


def run(*args, cwd=None, env=None):
    command = [str(arg) for arg in args]
    if BUILD_VOLUME is None:
        subprocess.run(command, cwd=cwd, env=env, check=True)
        return
    check_build_space()
    process = subprocess.Popen(command, cwd=cwd, env=env, start_new_session=True)
    try:
        while True:
            try:
                result = process.wait(timeout=2)
                if result:
                    raise subprocess.CalledProcessError(result, command)
                return
            except subprocess.TimeoutExpired:
                check_build_space()
    finally:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()


def build_environment():
    # Neither downloaded build tools nor the candidate browser inherit signing secrets.
    return {key: value for key, value in os.environ.items()
            if not key.startswith(('APPLE_', 'TAURI_SIGNING_PRIVATE_KEY', 'CC_SIGN_'))} | {
                'CC_SIGN_IDENTITY': '-', 'MACOSX_DEPLOYMENT_TARGET': '12.0',
                'LC_ALL': 'en_US.UTF-8', 'LANG': 'en_US.UTF-8'}


def preflight(work, minimum_free_gib=100):
    errors = []
    if platform.system() != 'Darwin' or platform.machine() != 'x86_64':
        errors.append('Use a native Intel Mac for the initial build and execution checks')
    if not (3, 11) <= sys.version_info[:2] <= (3, 13):
        errors.append('Use Python 3.11–3.13 (the pinned upstream tools do not support Python 3.14)')
    elif shutil.which('python3'):
        python = json.loads(subprocess.check_output(['python3', '-c', 'import json,sys; print(json.dumps([list(sys.version_info[:2]), sys.prefix]))'], text=True))
        if python != [list(sys.version_info[:2]), sys.prefix]:
            errors.append('Activate the selected Python environment so upstream python3 commands use the same interpreter')
    if (ROOT / 'browser-runtime/version.txt').read_text().strip() != SOURCES['version']:
        errors.append('The engine version changed; review and repin both source commits first')
    if ' ' in str(work):
        errors.append('Chromium build paths must not contain spaces')
    for tool in ['git', 'greadlink', 'ninja', 'node', 'python3', 'patch', 'xcrun', 'xcodebuild', 'ditto', 'lipo']:
        if not shutil.which(tool):
            errors.append(f'Missing build tool: {tool}')
    for module in ['socks', 'httplib2']:
        if importlib.util.find_spec(module) is None:
            errors.append(f'Missing Python dependency: {module} (install PySocks and httplib2 in a venv)')
    if platform.system() == 'Darwin':
        try:
            if subprocess.check_output(['sysctl', '-in', 'sysctl.proc_translated'], text=True).strip() == '1':
                errors.append('Rosetta execution is not native Intel acceptance; use Intel hardware')
            sdk = subprocess.check_output(['xcrun', '--sdk', 'macosx', '--show-sdk-version'], text=True).strip()
            if tuple(map(int, sdk.split('.')[:2])) < (15, 0):
                errors.append(f'macOS SDK 15+ is required by the pinned source; selected SDK is {sdk}. Set DEVELOPER_DIR to an existing full Xcode installation')
            subprocess.check_output(['xcodebuild', '-version'], stderr=subprocess.STDOUT)
        except (OSError, ValueError, subprocess.CalledProcessError):
            errors.append('Select a full Xcode installation; Command Line Tools alone are insufficient')
    ancestor = work
    while not ancestor.exists():
        ancestor = ancestor.parent
    # Reserve a working budget before source download; this is not a peak-space guarantee.
    minimum_gib = SPACE_FLOOR_GIB if (work / 'ccbaga-engine-build.json').exists() else minimum_free_gib
    if shutil.disk_usage(ancestor).free < minimum_gib * 1024 ** 3:
        errors.append(f'Reserve at least {minimum_gib} GiB free on the build volume before starting or resuming')
    if errors:
        raise SystemExit('Intel engine prerequisites:\n- ' + '\n- '.join(errors))


def checkout(url, revision, target, env):
    if not (target / '.git').exists():
        if target.exists() and any(target.iterdir()):
            raise SystemExit(f'Refusing existing non-checkout files at {target}')
        run('git', 'init', target, env=env)
        run('git', '-C', target, 'fetch', '--depth=1', url, revision, env=env)
        run('git', '-C', target, 'checkout', '--detach', revision, env=env)
    actual = subprocess.check_output(['git', '-C', str(target), 'rev-parse', 'HEAD'], text=True).strip()
    if actual != revision:
        raise SystemExit(f'Refusing a different source checkout at {target}')
    run('git', '-C', target, 'diff', '--exit-code', '--', '.', ':!ungoogled-chromium', env=env)
    run('git', '-C', target, 'diff', '--cached', '--exit-code', env=env)


def save_state(path, phase):
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(SOURCES | {'phase': phase}, indent=2) + '\n')
    temporary.replace(path)


def apply_source_patches(source, env):
    records = []
    for patch in SOURCE_PATCHES:
        # --batch can silently change direction for individual files in a mixed tree.
        # --force honors the requested direction so a partial application is refused.
        command = ['patch', '--force', '-F0', '-p1', '-i', str(patch)]
        forward = subprocess.run(command + ['--forward', '--dry-run'], cwd=source, env=env, capture_output=True, text=True)
        if forward.returncode == 0:
            run(*command, '--forward', cwd=source, env=env)
        else:
            reverse = subprocess.run(command + ['--reverse', '--dry-run'], cwd=source, env=env, capture_output=True, text=True)
            if reverse.returncode:
                raise SystemExit(f'Source compatibility patch does not match the pinned tree: {patch.name}\n{forward.stdout}{forward.stderr}')
        records.append({'file': patch.name, 'sha256': hashlib.sha256(patch.read_bytes()).hexdigest()})
    return records


def build(work, jobs, prepare_only, cache_dir=None):
    global BUILD_VOLUME
    state = work / 'ccbaga-engine-build.json'
    if state.exists():
        metadata = json.loads(state.read_text())
        if any(metadata.get(key) != value for key, value in SOURCES.items()):
            raise SystemExit('Work directory belongs to another source version; use a new directory')
        phase = metadata['phase']
    else:
        if work.exists() and any(work.iterdir()):
            raise SystemExit('Use an empty dedicated work directory; existing files are preserved')
        work.mkdir(parents=True, exist_ok=True)
        phase = 'checkout'
        save_state(state, phase)
    if phase == 'preparing':
        raise SystemExit('Source preparation was interrupted. Use a new work directory; partial sources are preserved for inspection')
    BUILD_VOLUME = work
    env = build_environment()
    builder = work / 'macos-builder'
    core = builder / 'ungoogled-chromium'
    checkout('https://github.com/ungoogled-software/ungoogled-chromium-macos.git', SOURCES['builder'], builder, env)
    checkout('https://github.com/adryfish/fingerprint-chromium.git', SOURCES['fingerprint'], core, env)
    source = builder / 'build/src'
    output = source / 'out/Default'
    if phase == 'checkout':
        cache = builder / 'build/download_cache'
        cache.parent.mkdir(parents=True, exist_ok=True)
        if cache_dir and not cache.exists():
            cache_dir.mkdir(parents=True, exist_ok=True)
            cache.symlink_to(cache_dir, target_is_directory=True)
        cache.mkdir(parents=True, exist_ok=True)
        save_state(state, 'preparing')
        run('bash', builder / 'retrieve_and_unpack_resource.sh', '-d', '-g', 'x86_64', env=env)
        run(sys.executable, core / 'utils/prune_binaries.py', source, core / 'pruning.list', env=env)
        run(sys.executable, core / 'utils/patches.py', 'apply', source, core / 'patches', builder / 'patches', env=env)
        run(sys.executable, core / 'utils/domain_substitution.py', 'apply', '-r', core / 'domain_regex.list', '-f', core / 'domain_substitution.list', source, env=env)
        output.mkdir(parents=True, exist_ok=True)
        flags = (core / 'flags.gn').read_text() + '\n' + (builder / 'flags.macos.gn').read_text()
        # No debug symbols or Chrome PGO downloads; retain the upstream release configuration.
        (output / 'args.gn').write_text(flags + '\ntarget_cpu = "x64"\nsymbol_level = 0\n')
        run('bash', builder / 'retrieve_and_unpack_resource.sh', '-p', 'x86_64', env=env)
        run(sys.executable, source / 'tools/gn/bootstrap/bootstrap.py', '-o', 'out/Default/gn', '--skip-generate-buildfiles', cwd=source, env=env)
        run(sys.executable, source / 'tools/rust/build_bindgen.py', '--skip-test', cwd=source, env=env)
        run(output / 'gn', 'gen', 'out/Default', '--fail-on-unused-args', cwd=source, env=env)
        save_state(state, 'prepared')
    source_patches = apply_source_patches(source, env)
    if prepare_only:
        print(f'Source prepared at {source}; this command does not compile or accept a candidate')
        return
    # Repeating Ninja resumes an interrupted compile without reapplying patches or deleting out/.
    run('ninja', '-C', output, '-j', jobs, 'chrome', env=env)
    app = output / 'Chromium.app'
    run(sys.executable, ROOT / 'scripts/sign-browser.py', '--app', app, env=env)
    generations = work / 'artifacts'
    generations.mkdir(exist_ok=True)
    # Keep retries separate so an interrupted export cannot mix new and old provenance.
    artifacts = Path(tempfile.mkdtemp(prefix='.candidate-', dir=generations))
    report = artifacts / 'engine-verification.json'
    run(sys.executable, ROOT / 'scripts/verify-macos-intel-engine.py', app, '--report', report, env=env)
    archive = artifacts / f'FingerprintChromium-{SOURCES["version"]}-macos-x86_64.zip'
    run('ditto', '-c', '-k', '--keepParent', app, archive, env=env)
    with archive.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    provenance = SOURCES | {
        'archive': archive.name, 'sha256': digest,
        'sdk': subprocess.check_output(['xcrun', '--show-sdk-version'], text=True).strip(),
        'gn_flags': (output / 'args.gn').read_text(),
        'source_patches': source_patches,
        'verification': json.loads(report.read_text()),
        'status': 'development-candidate; full CCBaga integration and release acceptance pending',
    }
    (artifacts / 'engine-source.json').write_text(json.dumps(provenance, indent=2) + '\n')
    (artifacts / 'SHA256SUMS').write_text(f'{digest}  {archive.name}\n')
    shutil.copy2(core / 'LICENSE', artifacts / 'LICENSE.fingerprint-chromium')
    for patch in SOURCE_PATCHES:
        shutil.copy2(patch, artifacts / patch.name)
    final = generations / f'{SOURCES["version"]}-{artifacts.name.removeprefix(".candidate-")}'
    artifacts.rename(final)
    save_state(state, 'built')
    print(f'Intel engine candidate verified: {final / archive.name}\nNot installed or published')


def main():
    parser = argparse.ArgumentParser(description='Build the pinned Intel fingerprint engine in a dedicated work directory')
    parser.add_argument('--work-dir', type=Path, required=True)
    parser.add_argument('--jobs', type=int, default=2)
    parser.add_argument('--cache-dir', type=Path, help='Separate verified-download cache; source and output remain in the work directory')
    parser.add_argument('--minimum-free-gib', type=int, default=100, help='Initial workspace budget (default 100); all commands retain a 10 GiB free-space floor')
    parser.add_argument('--check', action='store_true', help='Check prerequisites without creating or downloading anything')
    parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error('--jobs must be positive')
    if args.minimum_free_gib < SPACE_FLOOR_GIB:
        parser.error(f'--minimum-free-gib must be at least {SPACE_FLOOR_GIB}')
    work = args.work_dir.expanduser().resolve()
    preflight(work, args.minimum_free_gib)
    if args.check:
        print('Intel engine prerequisites passed; no source downloaded or compiled')
    else:
        build(work, args.jobs, args.prepare_only, args.cache_dir.expanduser().resolve() if args.cache_dir else None)


if __name__ == '__main__':
    main()
