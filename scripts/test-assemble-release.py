#!/usr/bin/env python3
# [INPUT]: 真实发布装配入口与临时平台产物/校验清单，无真实安装器或签名材料。
# [OUTPUT]: 双架构 macOS 的完整十一文件交付、缺失摘要与混用更新目标拒绝的离线回归。
# [POS]: 发布合并边界的标准库测试，不发布、不下载、不操作用户文件。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('assembly', Path(__file__).with_name('assemble-release.py'))
assembly = importlib.util.module_from_spec(spec)
spec.loader.exec_module(assembly)


class ReleaseAssemblyTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='ccbaga-assembly-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.out = self.root / 'releases'
        self.out.mkdir()
        self.version = '1.2.3'
        prefix = f'CCBaga-{self.version}'
        self.dmg = prefix + '-macos-arm64.dmg'
        self.archive = prefix + '-macos-arm64.app.tar.gz'
        self.intel_dmg = prefix + '-macos-x86_64.dmg'
        self.intel_archive = prefix + '-macos-x86_64.app.tar.gz'
        self.windows = prefix + '-windows-x86_64-setup.exe'
        self.linux = [prefix + '-linux-x86_64.' + ext for ext in ['deb', 'rpm', 'AppImage']]
        self.arch = prefix + '-linux-x86_64.pkg.tar.zst'
        for name in [self.dmg, self.archive, self.intel_dmg, self.intel_archive, self.windows, *self.linux, self.arch]:
            (self.out / name).write_bytes(b'disposable verified artifact')
        for name, platform, manifest in [(self.archive, 'darwin-aarch64', 'latest-macos-arm64.json'),
                                          (self.intel_archive, 'darwin-x86_64', 'latest-macos-x86_64.json'),
                                          (self.windows, 'windows-x86_64', 'latest-windows.json')]:
            signature = 'disposable-signature-' + platform
            (self.out / (name + '.sig')).write_text(signature + '\n')
            (self.out / manifest).write_text(json.dumps({'version': self.version, 'platforms': {
                platform: {'url': f'https://github.com/liseami/CCBaga/releases/download/v{self.version}/{name}',
                           'signature': signature}}}))
        self.lists = {'macOS-arm64-SHA256SUMS': [self.dmg, self.archive, self.archive + '.sig', 'latest-macos-arm64.json'],
                      'macOS-x86_64-SHA256SUMS': [self.intel_dmg, self.intel_archive, self.intel_archive + '.sig', 'latest-macos-x86_64.json'],
                      'Linux-SHA256SUMS': self.linux,
                      'Arch-SHA256SUMS': [self.arch], 'Windows-SHA256SUMS': [self.windows]}
        self.write_lists()
        self.addCleanup(patch.stopall)
        patch.object(assembly, 'ROOT', self.root).start()
        patch.object(assembly, 'OUT', self.out).start()
        patch.dict(os.environ, {'RELEASE_VERSION': self.version}).start()

    def write_lists(self):
        for name, artifacts in self.lists.items():
            lines = [hashlib.sha256((self.out / artifact).read_bytes()).hexdigest() + '  ' + artifact + '\n'
                     for artifact in artifacts]
            (self.out / name).write_text(''.join(lines))

    def test_complete_inputs_publish_only_eleven_verified_assets(self):
        assembly.main()
        published = (self.root / 'release-files.txt').read_text().splitlines()
        expected = [self.dmg, self.intel_dmg, self.windows, *self.linux, self.arch, self.archive, self.intel_archive, 'latest.json', 'SHA256SUMS']
        self.assertEqual(published, ['releases/' + name for name in expected])
        manifest = json.loads((self.out / 'latest.json').read_text())
        self.assertEqual(set(manifest['platforms']), {'darwin-aarch64', 'darwin-x86_64', 'windows-x86_64'})
        self.assertTrue(manifest['platforms']['darwin-x86_64']['url'].endswith(self.intel_archive))
        for line in (self.out / 'SHA256SUMS').read_text().splitlines():
            digest, filename = line.split('  ', 1)
            self.assertEqual(digest, hashlib.sha256((self.out / filename).read_bytes()).hexdigest())

    def test_omitted_artifact_cannot_be_rehashed_and_published(self):
        for manifest, artifact in [('macOS-arm64-SHA256SUMS', self.dmg), ('macOS-x86_64-SHA256SUMS', self.intel_dmg),
                                   ('macOS-x86_64-SHA256SUMS', self.intel_archive), ('Windows-SHA256SUMS', self.windows),
                                   *[('Linux-SHA256SUMS', name) for name in self.linux],
                                   ('Arch-SHA256SUMS', self.arch)]:
            with self.subTest(manifest=manifest, artifact=artifact):
                originals = {name: (self.out / name).read_bytes() for name in [*self.lists, 'latest-macos-arm64.json', artifact]}
                self.lists[manifest].remove(artifact)
                self.write_lists()
                (self.out / artifact).write_bytes(b'unverified replacement after platform checks')
                original_latest = (self.out / 'latest-macos-arm64.json').read_bytes()
                try:
                    with self.assertRaisesRegex(SystemExit, 'Missing verified artifact'):
                        assembly.main()
                    self.assertFalse((self.root / 'release-files.txt').exists())
                    self.assertFalse((self.root / 'release-notes.md').exists())
                    self.assertFalse((self.out / 'latest.json').exists())
                    self.assertEqual((self.out / 'latest-macos-arm64.json').read_bytes(), original_latest)
                finally:
                    self.lists[manifest].append(artifact)
                    for name, data in originals.items():
                        (self.out / name).write_bytes(data)
                    for name in ['release-files.txt', 'release-notes.md']:
                        (self.root / name).unlink(missing_ok=True)

    def test_intel_updater_cannot_point_to_the_arm_archive(self):
        path = self.out / 'latest-macos-x86_64.json'
        manifest = json.loads(path.read_text())
        manifest['platforms']['darwin-x86_64']['url'] = manifest['platforms']['darwin-x86_64']['url'].replace(self.intel_archive, self.archive)
        path.write_text(json.dumps(manifest))
        self.write_lists()
        with self.assertRaisesRegex(SystemExit, 'Updater URL/signature mismatch'):
            assembly.main()
        self.assertFalse((self.root / 'release-files.txt').exists())
        self.assertFalse((self.out / 'latest.json').exists())


if __name__ == '__main__':
    unittest.main()
