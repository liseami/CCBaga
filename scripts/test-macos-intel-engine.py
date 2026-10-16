#!/usr/bin/env python3
# [INPUT]: Intel 源码构建/验收入口与临时文件、架构/浏览器观测替身。
# [OUTPUT]: 架构混装、普通 Chromium/失效图形开关冒充、错误源码复用/半套 SDK 补丁与凭据继承的拒绝回归。
# [POS]: 三平台离线检查；不下载源码、不执行真实引擎，不替代 Intel 原生编译与执行。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import importlib.util
import json
import os
import plistlib
import subprocess
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


builder = load('intel_builder', 'build-macos-intel-engine.py')
verifier = load('intel_verifier', 'verify-macos-intel-engine.py')
signer = load('intel_signer', 'sign-browser.py')


class IntelEngineTests(unittest.TestCase):
    def test_low_disk_space_stops_before_spawning_a_build_child(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(builder, 'BUILD_VOLUME', Path(temporary)), patch.object(builder.shutil, 'disk_usage', return_value=SimpleNamespace(free=9*1024**3)), patch.object(builder.subprocess, 'Popen') as child:
            with self.assertRaisesRegex(RuntimeError, 'preserve 10 GiB free'):
                builder.run('must-not-start')
            child.assert_not_called()

    def test_disk_pressure_stops_only_the_isolated_build_process_group(self):
        process = SimpleNamespace(pid=12345)
        process.wait = Mock(side_effect=[subprocess.TimeoutExpired('fixture', 2), 0])
        process.poll = Mock(return_value=None)
        with tempfile.TemporaryDirectory() as temporary, patch.object(builder, 'BUILD_VOLUME', Path(temporary)), patch.object(builder.shutil, 'disk_usage', side_effect=[SimpleNamespace(free=40*1024**3), SimpleNamespace(free=9*1024**3)]), patch.object(builder.subprocess, 'Popen', return_value=process) as child, patch.object(builder.os, 'killpg', create=True) as stop:
            with self.assertRaisesRegex(RuntimeError, 'preserve 10 GiB free'):
                builder.run('build-fixture')
            self.assertTrue(child.call_args.kwargs['start_new_session'])
            stop.assert_called_once_with(12345, builder.signal.SIGTERM)

    def test_xcode_16_sdk_15_2_meets_the_pinned_source_minimum(self):
        def output(command, **kwargs):
            if command[0] == 'python3':
                return json.dumps([[3, 13], builder.sys.prefix])
            if command[0] == 'sysctl':
                return ''
            if command[0] == 'xcrun':
                return '15.2\n'
            if command[0] == 'xcodebuild':
                return b'Xcode 16.2\n'
            self.fail(f'Unexpected environment mutation: {command}')
        with tempfile.TemporaryDirectory() as temporary, patch.object(builder.platform, 'system', return_value='Darwin'), patch.object(builder.platform, 'machine', return_value='x86_64'), patch.object(builder.sys, 'version_info', (3, 13, 13)), patch.object(builder.shutil, 'which', return_value='/fixture/tool'), patch.object(builder.shutil, 'disk_usage', return_value=SimpleNamespace(free=200*1024**3)), patch.object(builder.importlib.util, 'find_spec', return_value=object()), patch.object(builder.subprocess, 'check_output', side_effect=output):
            builder.preflight(Path(temporary))

    def test_all_nested_binaries_require_intel_code(self):
        with tempfile.TemporaryDirectory() as temporary:
            app = Path(temporary).resolve() / 'Chromium.app'
            paths = ['Contents/MacOS/Chromium', 'Contents/Frameworks/Engine.framework/Versions/A/Engine',
                     'Contents/Frameworks/Engine.framework/Versions/A/Helpers/Renderer.app/Contents/MacOS/Renderer']
            for name in paths:
                binary = app / name
                binary.parent.mkdir(parents=True, exist_ok=True)
                binary.write_bytes(b'\xcf\xfa\xed\xfe')
            def architecture(command, **kwargs):
                return 'arm64\n' if command[-1].endswith('Renderer') else 'x86_64\n'
            with patch.object(verifier.subprocess, 'check_output', side_effect=architecture):
                with self.assertRaisesRegex(RuntimeError, 'Missing x86_64 code.*Renderer'):
                    verifier.architectures(app)
            with patch.object(verifier.subprocess, 'check_output', return_value='x86_64\n'):
                self.assertEqual(len(verifier.architectures(app)), 3)

    @unittest.skipIf(os.name == 'nt', 'Symlink fixture requires POSIX')
    def test_engine_symlinks_cannot_escape_the_bundle(self):
        with tempfile.TemporaryDirectory() as temporary:
            app = Path(temporary).resolve() / 'Chromium.app'
            app.mkdir()
            outside = app.parent / 'outside'
            outside.write_bytes(b'\xcf\xfa\xed\xfe')
            (app / 'escaped').symlink_to(outside)
            with self.assertRaisesRegex(RuntimeError, 'Invalid engine symlink'):
                verifier.architectures(app)

    def test_stock_browser_cannot_pass_by_printing_its_version(self):
        expected = {'cores': 7, 'timezone': 'Asia/Tokyo', 'language': 'en-US', 'platform': 'MacIntel'}
        for observed in [expected | {'cores': 4}, expected | {'timezone': 'UTC'}, expected | {'language': 'fr-FR'}]:
            with self.subTest(observed=observed):
                document = '<pre id="ccbaga-probe">' + json.dumps(observed) + '</pre>'
                with self.assertRaisesRegex(RuntimeError, 'Fingerprint switches did not take effect'):
                    verifier.validate_probe(document, expected)
        document = '<pre id="ccbaga-probe">' + json.dumps(expected) + '</pre>'
        self.assertEqual(verifier.validate_probe(document, expected), expected)
        with self.assertRaises(ValueError):
            verifier.validate_probe('<pre id="ccbaga-probe"></pre>', expected)

    def test_rosetta_cannot_be_reported_as_native_intel(self):
        with patch.object(verifier.platform, 'system', return_value='Darwin'), patch.object(verifier.platform, 'machine', return_value='x86_64'), patch.object(verifier.subprocess, 'check_output', return_value='1\n'):
            with self.assertRaisesRegex(RuntimeError, 'Rosetta execution cannot substitute'):
                verifier.verify(Path('/not-an-intel-artifact'))

    def test_engine_cannot_raise_the_arm_version_minimum_macos_requirement(self):
        with tempfile.TemporaryDirectory() as temporary:
            app = Path(temporary) / 'Chromium.app'
            (app / 'Contents').mkdir(parents=True)
            (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
                'CFBundleShortVersionString': builder.SOURCES['version'],
                'CFBundleExecutable': 'Chromium', 'LSMinimumSystemVersion': '13.0'}))
            with patch.object(verifier.platform, 'system', return_value='Darwin'), patch.object(verifier.platform, 'machine', return_value='x86_64'), patch.object(verifier.subprocess, 'check_output', return_value=''):
                with self.assertRaisesRegex(RuntimeError, 'retain macOS 12 runtime support'):
                    verifier.verify(app)

    def test_unowned_or_interrupted_work_is_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            marker = work / 'user-file'
            marker.write_text('preserve')
            with self.assertRaisesRegex(SystemExit, 'empty dedicated work directory'):
                builder.build(work, 2, False)
            builder.save_state(work / 'ccbaga-engine-build.json', 'preparing')
            with self.assertRaisesRegex(SystemExit, 'preparation was interrupted'):
                builder.build(work, 2, False)
            self.assertEqual(marker.read_text(), 'preserve')

    def test_source_pin_mismatch_is_rejected_before_commands(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(builder, 'run') as command:
            work = Path(temporary)
            (work / 'ccbaga-engine-build.json').write_text(json.dumps(builder.SOURCES | {'fingerprint': 'another-revision', 'phase': 'built'}))
            with self.assertRaisesRegex(SystemExit, 'another source version'):
                builder.build(work, 2, False)
            command.assert_not_called()

    def test_source_download_cache_exists_before_upstream_retrieval(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(builder, 'checkout'):
            work = Path(temporary)
            def first_command(*args, **kwargs):
                self.assertTrue((work / 'macos-builder/build/download_cache').is_dir())
                raise RuntimeError('stop before network retrieval')
            with patch.object(builder, 'run', side_effect=first_command):
                with self.assertRaisesRegex(RuntimeError, 'stop before network retrieval'):
                    builder.build(work, 2, True)

    def test_empty_submodule_directory_can_be_initialized(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(builder, 'run') as command, patch.object(builder.subprocess, 'check_output', return_value=builder.SOURCES['fingerprint'] + '\n'):
            builder.checkout('https://example.invalid/repo.git', builder.SOURCES['fingerprint'], Path(temporary), {})
            self.assertEqual(command.call_args_list[0].args[:2], ('git', 'init'))

    def test_build_environment_removes_signing_secrets(self):
        with patch.dict(os.environ, {'APPLE_API_KEY': 'fixture', 'TAURI_SIGNING_PRIVATE_KEY': 'fixture', 'CC_SIGN_IDENTITY': 'fixture-certificate'}):
            env = builder.build_environment()
        self.assertNotIn('APPLE_API_KEY', env)
        self.assertNotIn('TAURI_SIGNING_PRIVATE_KEY', env)
        self.assertEqual(env['CC_SIGN_IDENTITY'], '-')

    def test_candidate_signing_does_not_touch_the_installed_engine(self):
        with tempfile.TemporaryDirectory() as temporary:
            app = Path(temporary) / 'Candidate.app'
            binary = app / 'Contents/MacOS/Chromium'
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b'\xcf\xfa\xed\xfe')
            with patch.object(signer, 'sign') as sign, patch.object(signer.subprocess, 'run'):
                signer.main(app)
            self.assertEqual([call.args[0] for call in sign.call_args_list], [binary, app])


class GraphicsProbeTests(unittest.TestCase):
    def sample(self, marker):
        return {'pixels': str(marker) * 8, 'png': str(marker + 1) * 8, 'pngValid': True,
                'textWidth': 150 + marker / 1000, 'rect': [25 + marker / 1000, 31, 131.5, 55.25],
                'clientRect': [25, 31 + marker / 1000, 131.5, 55.25]}

    def document(self, samples):
        return '<pre id="ccbaga-probe">' + json.dumps({'samples': samples}) + '</pre>'

    def test_graphics_observations_must_be_complete_and_stable(self):
        sample = self.sample(1)
        self.assertEqual(verifier.validate_graphics(self.document([sample, sample])), sample)
        with self.assertRaisesRegex(RuntimeError, 'unstable'):
            verifier.validate_graphics(self.document([sample, self.sample(2)]))
        for malformed in [sample | {'pixels': ''}, sample | {'pngValid': False},
                          sample | {'rect': []}, sample | {'clientRect': [0, 0, -1, 2]},
                          sample | {'textWidth': float('nan')}, sample | {'textWidth': True}]:
            with self.subTest(malformed=malformed):
                with self.assertRaises(RuntimeError):
                    verifier.validate_graphics(self.document([malformed, malformed]))
        with self.assertRaisesRegex(RuntimeError, 'Missing Canvas'):
            verifier.validate_graphics('<pre id="ccbaga-probe">{"error":"no Canvas"}</pre>')

    def test_each_graphics_control_must_differ_from_other_seed_and_native(self):
        first, second, native = self.sample(1), self.sample(3), self.sample(5)
        observations = [first, second, native, native]
        self.assertEqual(verifier.compare_graphics(observations), observations)
        for key in ['pixels', 'png', 'textWidth', 'rect', 'clientRect']:
            for broken in [second | {key: first[key]}, second | {key: native[key]}]:
                with self.subTest(key=key, broken=broken):
                    with self.assertRaisesRegex(RuntimeError, 'did not take effect'):
                        verifier.compare_graphics([first, broken, native, native])

    def test_stock_or_seed_dependent_native_graphics_are_refused(self):
        sample = self.sample(1)
        with self.assertRaisesRegex(RuntimeError, 'did not take effect'):
            verifier.compare_graphics([sample, sample, sample, sample])
        with self.assertRaisesRegex(RuntimeError, 'Native graphics must ignore seed changes'):
            verifier.compare_graphics([sample, self.sample(3), self.sample(5), self.sample(7)])

    def test_graphics_modes_reuse_the_product_disable_switches(self):
        sample = self.sample(1)
        for native, expected in [(False, 'font,audio,gpu'), (True, 'font,audio,gpu,canvas,clientrects')]:
            with patch.object(verifier, 'run_probe', return_value=self.document([sample, sample])) as run:
                self.assertEqual(verifier.graphics_probe(Path('/fixture/Chromium'), 1001, native), sample)
            self.assertEqual(run.call_args.args[-1], [f'--disable-spoofing={expected}'])
            self.assertEqual(run.call_args.args[-2], verifier.GRAPHICS_PAGE)


@unittest.skipUnless(builder.shutil.which('patch'), 'SDK patch fixtures require the patch command')
class SourcePatchTests(unittest.TestCase):
    def fixture(self, root, states):
        source = root / 'source'
        source.mkdir()
        changes = []
        for name, state in states.items():
            (source / name).write_text(f'{state} {name}\n')
            changes.append(f'--- a/{name}\n+++ b/{name}\n@@ -1 +1 @@\n-before {name}\n+after {name}\n')
        patch_file = root / 'sdk-compat.patch'
        patch_file.write_text(''.join(changes))
        return source, patch_file

    def test_source_patch_is_applied_once_and_records_its_digest(self):
        with tempfile.TemporaryDirectory() as temporary:
            source, patch_file = self.fixture(Path(temporary), {'one.cc': 'before', 'two.cc': 'before'})
            with patch.object(builder, 'SOURCE_PATCHES', [patch_file]), patch.object(builder, 'BUILD_VOLUME', None):
                records = builder.apply_source_patches(source, builder.build_environment())
                self.assertEqual(builder.apply_source_patches(source, builder.build_environment()), records)
            self.assertEqual(records, [{'file': patch_file.name, 'sha256': builder.hashlib.sha256(patch_file.read_bytes()).hexdigest()}])
            self.assertEqual({p.name: p.read_text() for p in source.iterdir()}, {'one.cc': 'after one.cc\n', 'two.cc': 'after two.cc\n'})

    def test_partially_applied_patch_is_refused_without_mutation(self):
        with tempfile.TemporaryDirectory() as temporary:
            source, patch_file = self.fixture(Path(temporary), {'one.cc': 'after', 'two.cc': 'before'})
            before = {p.name: p.read_bytes() for p in source.iterdir()}
            with patch.object(builder, 'SOURCE_PATCHES', [patch_file]), patch.object(builder, 'BUILD_VOLUME', None):
                with self.assertRaisesRegex(SystemExit, 'does not match the pinned tree'):
                    builder.apply_source_patches(source, builder.build_environment())
            self.assertEqual({p.name: p.read_bytes() for p in source.iterdir()}, before)

    def test_different_source_is_refused_without_mutation(self):
        with tempfile.TemporaryDirectory() as temporary:
            source, patch_file = self.fixture(Path(temporary), {'one.cc': 'unrelated', 'two.cc': 'unrelated'})
            before = {p.name: p.read_bytes() for p in source.iterdir()}
            with patch.object(builder, 'SOURCE_PATCHES', [patch_file]), patch.object(builder, 'BUILD_VOLUME', None):
                with self.assertRaisesRegex(SystemExit, 'does not match the pinned tree'):
                    builder.apply_source_patches(source, builder.build_environment())
            self.assertEqual({p.name: p.read_bytes() for p in source.iterdir()}, before)


if __name__ == '__main__':
    unittest.main()
