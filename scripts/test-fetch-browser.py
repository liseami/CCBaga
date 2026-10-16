#!/usr/bin/env python3
# [INPUT]: 真实准备入口、临时引擎树与显式下载/摘要/平台工具替身。
# [OUTPUT]: 初始复制失败后的重试、既有引擎保护和清理失败路径回归。
# [POS]: 不下载或运行真实引擎，不挂载磁盘映像，不操作用户应用。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import contextlib
import importlib.util
import io
import os
import json
import platform
import sys
from pathlib import Path
import plistlib
import shutil
import struct
import subprocess
import tarfile
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import zipfile

spec = importlib.util.spec_from_file_location('fetch_browser', Path(__file__).with_name('fetch-browser.py'))
browser = importlib.util.module_from_spec(spec)
spec.loader.exec_module(browser)
COPYTREE = shutil.copytree
RMTREE = shutil.rmtree
RUN_TOOL = subprocess.run
ACTUAL_SYSTEM = platform.system()
ALL_PLATFORM_FIXTURES = '--all-platform-fixtures' in sys.argv
NATIVE_DITTO = ACTUAL_SYSTEM == 'Darwin' and not ALL_PLATFORM_FIXTURES
DIGESTS = {
    'Linux': '70d239830332e5820aa34dfcb284161cac0429eee25da642830afe04bda717f4',
    'Windows': '9ef3f471b7a6641b4224532522b29141ce3746e27d55788d88e2fd951f362579',
    'Darwin': browser.SHA256,
}
TARGETS = {'Linux': 'LinuxBrowser', 'Windows': 'WindowsBrowser', 'Darwin': 'Browser.app'}


class PreparationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        (self.root / 'browser-runtime').mkdir()
        self.volume = self.root / 'mounted-fixture'
        app = self.volume / 'Chromium.app'
        (app / 'Contents/MacOS').mkdir(parents=True)
        (app / 'Contents/Resources').mkdir()
        (app / 'Contents/MacOS/chrome').write_bytes(b'\xcf\xfa\xed\xfe fixture executable')
        (app / 'Contents/MacOS/chrome').chmod(0o755)
        (app / 'Contents/Resources/icudtl.dat').write_bytes(b'complete resource')
        (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
            'CFBundleExecutable': 'chrome', 'CFBundleShortVersionString': browser.VERSION,
        }))
        self.interrupt = False
        self.downloads = 0
        self.detaches = 0

    def copy(self, source, destination, *args, **kwargs):
        source, destination = Path(source), Path(destination)
        if self.interrupt:
            destination.mkdir()
            binary = 'Contents/MacOS/chrome' if (source / 'Contents').exists() else ('chrome.exe' if (source / 'chrome.exe').exists() else 'chrome')
            (destination / binary).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source / binary, destination / binary)
            raise OSError('fixture interrupted before copying required resources')
        return COPYTREE(source, destination, *args, **kwargs)

    def run_tool(self, command, **kwargs):
        if command[0] in ('curl', 'curl.exe'):
            self.downloads += 1
            archive = Path(command[command.index('--output') + 1])
            if archive.suffix == '.xz':
                with tarfile.open(archive, 'w:xz') as tar:
                    for name, data in [('engine/chrome', b'\x7fELF\x02' + b'\0' * 13 + b'\x3e\0'), ('engine/icudtl.dat', b'complete resource')]:
                        entry = tarfile.TarInfo(name)
                        entry.size, entry.mode = len(data), 0o755 if name.endswith('chrome') else 0o644
                        tar.addfile(entry, io.BytesIO(data))
                    link = tarfile.TarInfo('engine/resource-link')
                    link.type, link.linkname = tarfile.SYMTYPE, 'icudtl.dat'
                    tar.addfile(link)
            elif archive.suffix == '.zip':
                binary = bytearray(70)
                binary[:2], binary[60:64], binary[64:70] = b'MZ', struct.pack('<I', 64), b'PE\0\0\x64\x86'
                with zipfile.ZipFile(archive, 'w') as zipped:
                    zipped.writestr('engine/chrome.exe', binary)
                    zipped.writestr('engine/icudtl.dat', b'complete resource')
            else:
                archive.write_bytes(b'fixture DMG, mounting is explicitly doubled')
        elif command[0] == 'ditto':
            if NATIVE_DITTO and not self.interrupt:
                return RUN_TOOL(command, **kwargs)
            try:
                self.copy(command[1], command[2], symlinks=True)
            except OSError as error:
                raise subprocess.CalledProcessError(1, command) from error
        elif command[:2] == ['hdiutil', 'detach']:
            self.detaches += 1
        else:
            self.fail(f'Unexpected tool: {command}')
        return SimpleNamespace(returncode=0)

    def output_tool(self, command, **kwargs):
        if command[:2] == ['hdiutil', 'attach']:
            return plistlib.dumps({'system-entities': [{'mount-point': str(self.volume)}]})
        if command[0] == 'lipo':
            return 'arm64\n'
        if command[0] == 'powershell' or command[-1] == '--version':
            return 'Chromium ' + browser.VERSION
        self.fail(f'Unexpected output tool: {command}')

    @contextlib.contextmanager
    def platform_fixture(self, system):
        with contextlib.ExitStack() as stack:
            stack.enter_context(patch.dict(os.environ, {'CC_MACOS_ENGINE_URL': '', 'CC_MACOS_ENGINE_SHA256': ''}))
            stack.enter_context(patch.object(browser, 'ROOT', self.root))
            stack.enter_context(patch.object(browser.platform, 'system', return_value=system))
            stack.enter_context(patch.object(browser.platform, 'machine', return_value='arm64' if system == 'Darwin' else 'x86_64'))
            stack.enter_context(patch.object(browser.subprocess, 'run', side_effect=self.run_tool))
            stack.enter_context(patch.object(browser.subprocess, 'check_output', side_effect=self.output_tool))
            stack.enter_context(patch.object(browser.hashlib, 'file_digest', return_value=SimpleNamespace(hexdigest=lambda: DIGESTS[system])))
            stack.enter_context(patch.object(browser.shutil, 'copytree', side_effect=self.copy))
            yield self.root / 'browser-runtime' / TARGETS[system]

    def interrupted_then_retry(self, system):
        with self.platform_fixture(system) as target:
            self.interrupt = True
            with self.assertRaises((OSError, subprocess.CalledProcessError)):
                browser.main([])
            self.assertFalse(target.exists(), 'An interrupted copy must not publish a partial engine')
            self.assertEqual(list(target.parent.glob('.*-stage-*')), [])
            self.interrupt = False
            browser.main([])
            resource = target / ('Contents/Resources/icudtl.dat' if system == 'Darwin' else 'icudtl.dat')
            self.assertEqual(resource.read_bytes(), b'complete resource')
            self.assertEqual(self.downloads, 2)
            if system == 'Darwin':
                self.assertEqual(self.detaches, 2)
            else:
                self.assertEqual((target / 'version.txt').read_text(), browser.VERSION + '\n')
            if system == 'Linux':
                self.assertTrue(os.access(target / 'chrome', os.X_OK))
                self.assertTrue((target / 'resource-link').is_symlink())
                self.assertEqual(os.readlink(target / 'resource-link'), 'icudtl.dat')

    def existing_target(self, system):
        with self.platform_fixture(system) as target:
            target.mkdir()
            marker = target / 'existing'
            marker.write_bytes(b'preserve existing engine')
            with self.assertRaisesRegex(SystemExit, 'already exists'):
                browser.main([])
            self.assertEqual(marker.read_bytes(), b'preserve existing engine')
            self.assertEqual(list(target.iterdir()), [marker])
            self.assertEqual(self.downloads, 0)

    def cleanup_failure(self, system):
        def cleanup(path, *args, **kwargs):
            if '-stage-' in Path(path).name:
                raise OSError('fixture cleanup denied before removal')
            return RMTREE(path, *args, **kwargs)
        with self.platform_fixture(system) as target, patch.object(browser.shutil, 'rmtree', side_effect=cleanup):
            self.interrupt = True
            with self.assertRaises((RuntimeError, OSError, subprocess.CalledProcessError)) as failure:
                browser.main([])
            self.assertIsInstance(failure.exception, RuntimeError)
            self.assertIn('Could not remove engine staging directory at', str(failure.exception))
            self.assertFalse(target.exists())
            stages = list(target.parent.glob('.*-stage-*'))
            self.assertEqual(len(stages), 1)
            self.assertIn(str(stages[0]), str(failure.exception))
            binary = 'Contents/MacOS/chrome' if system == 'Darwin' else ('chrome.exe' if system == 'Windows' else 'chrome')
            self.assertTrue((stages[0] / 'engine' / binary).is_file())
            if system == 'Darwin':
                self.assertEqual(self.detaches, 1)

    def complete_preparation(self, system):
        with self.platform_fixture(system) as target:
            browser.main([])
            resource = target / ('Contents/Resources/icudtl.dat' if system == 'Darwin' else 'icudtl.dat')
            self.assertEqual(resource.read_bytes(), b'complete resource')
            self.assertEqual(self.downloads, 1)
            if system == 'Darwin':
                self.assertEqual(self.detaches, 1)
            else:
                self.assertEqual((target / 'version.txt').read_text(), browser.VERSION + '\n')

    def test_linux_complete_preparation(self): self.complete_preparation('Linux')
    def test_windows_complete_preparation(self): self.complete_preparation('Windows')
    def test_macos_complete_preparation(self): self.complete_preparation('Darwin')
    def test_linux_interrupted_preparation_can_retry(self): self.interrupted_then_retry('Linux')
    def test_windows_interrupted_preparation_can_retry(self): self.interrupted_then_retry('Windows')
    def test_macos_interrupted_preparation_can_retry(self): self.interrupted_then_retry('Darwin')
    def test_linux_existing_engine_is_untouched(self): self.existing_target('Linux')
    def test_windows_existing_engine_is_untouched(self): self.existing_target('Windows')
    def test_macos_existing_engine_is_untouched(self): self.existing_target('Darwin')
    def test_linux_cleanup_failure_reports_retained_stage(self): self.cleanup_failure('Linux')
    def test_windows_cleanup_failure_reports_retained_stage(self): self.cleanup_failure('Windows')
    def test_macos_cleanup_failure_reports_retained_stage(self): self.cleanup_failure('Darwin')


if __name__ == '__main__':
    prefix = {'Linux': 'linux', 'Windows': 'windows', 'Darwin': 'macos'}[ACTUAL_SYSTEM]
    names = unittest.defaultTestLoader.getTestCaseNames(PreparationTests)
    selected = names if ALL_PLATFORM_FIXTURES else [name for name in names if name.startswith(f'test_{prefix}_')]
    expected_count = 12 if ALL_PLATFORM_FIXTURES else 4
    assert len(selected) == expected_count, selected
    print(json.dumps({'actual_system': ACTUAL_SYSTEM, 'native_ditto': NATIVE_DITTO, 'all_platform_fixture_mode': ALL_PLATFORM_FIXTURES, 'test_inventory': selected}), flush=True)
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(PreparationTests(name) for name in selected))
    sys.exit(not result.wasSuccessful())
