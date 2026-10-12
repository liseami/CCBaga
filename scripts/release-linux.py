#!/usr/bin/env python3
# [INPUT]: Ubuntu x86_64、固定引擎、版本、Tauri Linux 配置、Node.js 标准管道身份 UI/Skill 离线测试与标准 rpmbuild。
# [OUTPUT]: 验收后的 DEB/RPM/AppImage 和独立 SHA256 清单。
# [POS]: 无 Apple/更新私钥的 Linux 发布执行层；提取真实产物后逐一核对，不修改用户系统。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import shutil
import shlex
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('payload', ROOT / 'scripts/verify-linux-package.py')
payload = importlib.util.module_from_spec(spec)
spec.loader.exec_module(payload)


def run(*args, **kwargs):
    subprocess.run([str(a) for a in args], check=True, **kwargs)


def build_rpm(deb, version):
    # ----- DEB/RPM 共用 /usr 文件树；Linux 更新仅走外部包管理器 -----
    # Tauri RPM 在 CI 停滞，改用原生 rpmbuild；不执行安装钩子或剥离引擎。
    # ponytail: 沿用 DEB 宿主类型提示；启用 Linux 应用内更新前必须补齐 RPM 类型标注。
    config = json.loads((ROOT / 'src-tauri/tauri.linux.conf.json').read_text())['bundle']['linux']['rpm']
    with tempfile.TemporaryDirectory(prefix='ccbaga-rpm-') as temporary:
        work = Path(temporary)
        tree = work / 'payload'
        run('dpkg-deb', '--extract', deb, tree)
        for name in ['BUILD', 'BUILDROOT', 'RPMS', 'SOURCES', 'SPECS', 'SRPMS']:
            (work / name).mkdir()
        files = ['/usr/bin/requirement-french', '/usr/lib/CCBaga-A']
        if (tree / 'usr/libexec/CCBaga-B').is_dir():
            files.append('/usr/libexec/CCBaga-B')
        files.extend('/' + str(path.relative_to(tree)) for path in sorted((tree / 'usr/share').rglob('*')) if path.is_file() or path.is_symlink())
        requires = '\n'.join('Requires: ' + value for value in config['depends'])
        spec = work / 'SPECS/ccbaga.spec'
        spec.write_text(f'''Name: ccbaga-a
Version: {version}
Release: 1
Summary: CCBaga native maintenance and companion browser
License: MIT
URL: https://github.com/liseami/CCBaga
BuildArch: x86_64
AutoReqProv: no
{requires}

%description
CCBaga-A with its isolated CCBaga-B companion browser.

%prep
%build
%install
mkdir -p "%{{buildroot}}"
cp -a {shlex.quote(str(tree / 'usr'))} "%{{buildroot}}/"

%files
''' + '\n'.join(json.dumps(path) for path in files) + '\n')
        run('rpmbuild', '-bb', spec, '--target', 'x86_64', '--define', f'_topdir {work}',
            '--define', '__os_install_post %{nil}', '--define', '_build_id_links none',
            '--define', '_binary_payload w3.zstdio', timeout=300)
        candidates = list((work / 'RPMS/x86_64').glob('*.rpm'))
        if len(candidates) != 1:
            raise RuntimeError('Expected one native RPM artifact')
        folder = ROOT / 'src-tauri/target/release/bundle/rpm'
        folder.mkdir(exist_ok=True)
        shutil.copy2(candidates[0], folder / candidates[0].name)


def main():
    version = os.environ['CC_RELEASE_VERSION']
    if platform.system() != 'Linux' or platform.machine() != 'x86_64' or not re.fullmatch(r'\d+\.\d+\.\d+', version):
        raise SystemExit('Linux x86_64 and a numeric release version are required')
    if any(key.startswith('APPLE_') or key.startswith('TAURI_SIGNING_PRIVATE_KEY') for key in os.environ):
        raise SystemExit('Linux builds must not receive Apple or updater credentials')
    run('python3', ROOT / 'skills/ccbaga-network/scripts/test_network.py')
    run('bash', '-n', ROOT / 'skills/ccbaga-network/scripts/setup-vps.sh')
    run('bun', 'test')
    run('bun', ROOT / 'scripts/test-skill-install.ts')
    run('bun', 'run', 'build')
    run('bun', 'run', 'test:identities')
    run('cargo', 'test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', '--features', 'custom-protocol')
    run('python3', 'scripts/test-linux.py')
    run('python3', 'scripts/test-rpm.py')
    build_env = os.environ | {'NO_STRIP': 'true', 'APPIMAGE_EXTRACT_AND_RUN': '1'}
    run('bun', 'run', 'tauri', 'build', '--verbose', '--bundles', 'deb,appimage', '--features', 'custom-protocol', '--config', json.dumps({'version': version}), env=build_env)
    debs = list((ROOT / 'src-tauri/target/release/bundle/deb').glob('*.deb'))
    if len(debs) != 1:
        raise RuntimeError('Expected one DEB artifact')
    build_rpm(debs[0], version)
    output = ROOT / 'releases'
    output.mkdir(exist_ok=True)
    manifests = []
    engine = ROOT / 'browser-runtime/LinuxBrowser/chrome'
    for kind, extension in [('deb', 'deb'), ('rpm', 'rpm'), ('appimage', 'AppImage')]:
        candidates = list((ROOT / f'src-tauri/target/release/bundle/{kind}').glob(f'*.{extension}'))
        if len(candidates) != 1:
            raise RuntimeError(f'Expected one {kind} artifact')
        source = candidates[0]
        with tempfile.TemporaryDirectory(prefix='ccbaga-linux-verify-') as temporary:
            root = Path(temporary)
            if kind == 'deb':
                run('dpkg-deb', '--extract', source, root)
                assert subprocess.check_output(['dpkg-deb', '-f', str(source), 'Version'], text=True).strip() == version
            elif kind == 'rpm':
                run('bsdtar', '-xf', source, '-C', root)
                assert subprocess.check_output(['rpm', '-qp', '--queryformat', '%{VERSION}', str(source)], text=True).strip() == version
            else:
                run(source, '--appimage-extract', cwd=root, stdout=subprocess.DEVNULL)
                root /= 'squashfs-root'
            payload.verify(root, engine)
        target = output / f'CCBaga-{version}-linux-x86_64.{extension}'
        shutil.copy2(source, target)
        manifests.append(f'{payload.digest(target)}  {target.name}\n')
    (output / 'Linux-SHA256SUMS').write_text(''.join(manifests))


if __name__ == '__main__':
    main()
