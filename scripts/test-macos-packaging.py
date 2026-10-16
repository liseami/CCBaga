#!/usr/bin/env python3
# [INPUT]: 双架构引擎导入、架构命名、临时 ZIP/Mach-O 样本与 macOS 原生 ditto。
# [OUTPUT]: Intel 输入摘要、嵌套架构/真实探针前置、链接越界与更新目标一致性回归。
# [POS]: 三平台离线打包检查，不下载引擎、不运行真实浏览器或修改用户应用。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import hashlib
import importlib.util
from pathlib import Path
import plistlib
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile
from macos_arch import architecture, updater_target

spec = importlib.util.spec_from_file_location('fetch_browser', Path(__file__).with_name('fetch-browser.py'))
browser = importlib.util.module_from_spec(spec)
spec.loader.exec_module(browser)


class MacosPackagingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / 'browser-runtime').mkdir()
        self.archive = self.root / 'intel.zip'
        self.main = 'Chromium.app/Contents/MacOS/Chromium'
        self.helper = 'Chromium.app/Contents/Frameworks/Engine.framework/Versions/A/Engine'
        with zipfile.ZipFile(self.archive, 'w') as zipped:
            for name in [self.main, self.helper]:
                zipped.writestr(name, b'\xcf\xfa\xed\xfe fixture')
            zipped.writestr('Chromium.app/Contents/Info.plist', plistlib.dumps({
                'CFBundleShortVersionString': browser.VERSION, 'CFBundleExecutable': 'Chromium'}))
        self.digest = hashlib.sha256(self.archive.read_bytes()).hexdigest()
        self.calls = []
        self.addCleanup(patch.stopall)
        patch.object(browser, 'ROOT', self.root).start()
        patch.object(browser.platform, 'machine', return_value='x86_64').start()
        patch.object(browser.subprocess, 'check_output', return_value='x86_64\n').start()
        patch.object(browser.subprocess, 'run', side_effect=self.run_tool).start()

    def run_tool(self, command, **kwargs):
        self.calls.append(command)
        if command[:3] == ['ditto', '-x', '-k']:
            with zipfile.ZipFile(command[3]) as zipped:
                zipped.extractall(command[4])
        elif command[0] == 'ditto':
            shutil.copytree(command[1], command[2], symlinks=True)
        elif any(str(value).endswith('verify-macos-intel-engine.py') for value in command):
            return
        else:
            self.fail(f'Unexpected subprocess: {command}')

    def test_updater_targets_keep_both_native_architectures_distinct(self):
        self.assertEqual((architecture('aarch64'), updater_target('arm64')), ('arm64', 'darwin-aarch64'))
        self.assertEqual((architecture('AMD64'), updater_target('x86_64')), ('x86_64', 'darwin-x86_64'))
        with self.assertRaisesRegex(RuntimeError, 'Unsupported macOS architecture'):
            architecture('i386')

    def test_intel_input_runs_fingerprint_acceptance_before_publishing(self):
        browser.fetch_macos(self.archive, self.digest)
        target = self.root / 'browser-runtime/Browser.app'
        self.assertTrue((target / 'Contents/MacOS/Chromium').exists())
        self.assertTrue(any(str(value).endswith('verify-macos-intel-engine.py') for value in self.calls[-2]))
        self.assertEqual(self.calls[-1][0], 'ditto')

    def test_digest_mismatch_prevents_extraction_or_publication(self):
        with self.assertRaisesRegex(SystemExit, 'SHA-256 mismatch'):
            browser.fetch_macos(self.archive, '0' * 64)
        self.assertEqual(self.calls, [])
        self.assertFalse((self.root / 'browser-runtime/Browser.app').exists())

    def test_arm_helper_in_intel_input_is_rejected(self):
        def arches(command, **kwargs):
            return 'arm64\n' if command[-1].endswith('Engine') else 'x86_64\n'
        with patch.object(browser.subprocess, 'check_output', side_effect=arches):
            with self.assertRaisesRegex(RuntimeError, 'Missing x86_64 code'):
                browser.fetch_macos(self.archive, self.digest)
        self.assertEqual(len(self.calls), 1, 'Only extraction may run before architecture validation')
        self.assertFalse((self.root / 'browser-runtime/Browser.app').exists())

    def test_failed_fingerprint_probe_does_not_publish_an_engine(self):
        real_fixture = self.run_tool
        def run(command, **kwargs):
            if any(str(value).endswith('verify-macos-intel-engine.py') for value in command):
                raise subprocess.CalledProcessError(1, command)
            return real_fixture(command, **kwargs)
        with patch.object(browser.subprocess, 'run', side_effect=run):
            with self.assertRaises(subprocess.CalledProcessError):
                browser.fetch_macos(self.archive, self.digest)
        self.assertFalse((self.root / 'browser-runtime/Browser.app').exists())

    def test_zip_paths_and_symlink_descendants_are_rejected(self):
        cases = [('../escape', None, None),
                 ('Chromium.app/Contents/link', '/tmp/outside', None),
                 ('Chromium.app/Contents/link', '../../../../outside', None),
                 ('Chromium.app/Contents/link', 'target', 'Chromium.app/Contents/link/child')]
        for name, link, child in cases:
            with self.subTest(name=name, link=link, child=child):
                with zipfile.ZipFile(self.archive, 'w') as zipped:
                    if link:
                        info = zipfile.ZipInfo(name)
                        info.create_system = 3
                        info.external_attr = (stat.S_IFLNK | 0o777) << 16
                        zipped.writestr(info, link)
                    else:
                        zipped.writestr(name, 'escape')
                    if child:
                        zipped.writestr(child, 'escape')
                with self.assertRaises(SystemExit):
                    browser.validate_macos_zip(self.archive)

    def test_internal_framework_symlinks_remain_supported(self):
        with zipfile.ZipFile(self.archive, 'w') as zipped:
            info = zipfile.ZipInfo('Chromium.app/Contents/Frameworks/Engine.framework/Versions/Current')
            info.create_system = 3
            info.external_attr = (stat.S_IFLNK | 0o777) << 16
            zipped.writestr(info, 'A')
            zipped.writestr('Chromium.app/Contents/Frameworks/Engine.framework/Versions/A/Engine', 'fixture')
        browser.validate_macos_zip(self.archive)


@unittest.skipUnless(sys.platform == 'darwin', 'Native ditto/Finder metadata check requires macOS')
class NativeDittoTests(unittest.TestCase):
    def test_real_export_import_preserves_framework_links_and_finder_metadata(self):
        with tempfile.TemporaryDirectory(prefix='ccbaga-native-zip-') as temporary:
            root = Path(temporary).resolve()
            app = root / 'Chromium.app'
            framework = app / 'Contents/Frameworks/Engine.framework'
            (framework / 'Versions/A/Resources').mkdir(parents=True)
            (framework / 'Versions/A/Engine').write_bytes(b'fixture library, not executable')
            (framework / 'Versions/A/Resources/notice').write_text('fixture resource')
            (framework / 'Versions/Current').symlink_to('A')
            (framework / 'Engine').symlink_to('Versions/Current/Engine')
            (framework / 'Resources').symlink_to('Versions/Current/Resources')
            info = app / 'Contents/Info.plist'
            info.write_text('fixture metadata, not an executable app')
            finder_info = '54455854' + '00' * 28
            subprocess.run(['/usr/bin/xattr', '-wx', 'com.apple.FinderInfo', finder_info, str(info)], check=True)
            archive = root / 'engine.zip'
            subprocess.run(['ditto', '-c', '-k', '--keepParent', str(app), str(archive)], check=True)
            browser.validate_macos_zip(archive)
            extracted = root / 'extracted'
            subprocess.run(['ditto', '-x', '-k', str(archive), str(extracted)], check=True)
            actual = extracted / 'Chromium.app/Contents/Frameworks/Engine.framework'
            for link in ['Versions/Current', 'Engine', 'Resources']:
                self.assertTrue((actual / link).is_symlink(), link)
            self.assertEqual((actual / 'Resources/notice').read_text(), 'fixture resource')
            restored = subprocess.check_output(['/usr/bin/xattr', '-px', 'com.apple.FinderInfo', str(extracted / 'Chromium.app/Contents/Info.plist')], text=True)
            self.assertEqual(''.join(restored.split()).lower(), finder_info.lower())


if __name__ == '__main__':
    unittest.main()
