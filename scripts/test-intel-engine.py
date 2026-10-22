#!/usr/bin/env python3
# [INPUT]: Engine recipe, locked metadata and disposable archive/cache/workspace fixtures.
# [OUTPUT]: Regressions for hash refusal, extraction boundaries, host/target selection and resumable stage claims.
# [POS]: Offline recipe tests; no downloads, compiler execution, apps or user data.
# [PROTOCOL]: Update this contract and scripts/CLAUDE.md when changing this file.
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('intel_engine',Path(__file__).with_name('build-intel-engine.py'))
build=importlib.util.module_from_spec(spec);spec.loader.exec_module(build)

class RecipeTests(unittest.TestCase):
    def setUp(self):
        self.temporary=tempfile.TemporaryDirectory();self.addCleanup(self.temporary.cleanup)
        self.root=Path(self.temporary.name).resolve();self.lock=json.loads(build.LOCK.read_text())

    def item(self,data=b'correct'):
        return {'filename':'fixed.tar.gz','url':'https://example.invalid/fixed.tar.gz','algorithm':'sha256','digest':hashlib.sha256(data).hexdigest()}

    def test_verified_cache_never_downloads(self):
        p=self.root/'fixed.tar.gz';p.write_bytes(b'correct')
        with patch.object(build.subprocess,'run') as run:
            self.assertEqual(build.fetch(self.item(),self.root),p);run.assert_not_called()

    def test_corrupt_cache_is_preserved_and_refused(self):
        p=self.root/'fixed.tar.gz';p.write_bytes(b'corrupt')
        with patch.object(build.subprocess,'run') as run:
            with self.assertRaisesRegex(RuntimeError,'checksum mismatch'):build.fetch(self.item(),self.root)
            run.assert_not_called()
        self.assertEqual(p.read_bytes(),b'corrupt')

    def test_bad_download_is_not_published(self):
        def download(args,**kw):Path(args[args.index('--output')+1]).write_bytes(b'bad network result')
        with patch.object(build.subprocess,'run',side_effect=download):
            with self.assertRaisesRegex(RuntimeError,'checksum mismatch'):build.fetch(self.item(),self.root)
        self.assertFalse((self.root/'fixed.tar.gz').exists())
        self.assertEqual(list(self.root.iterdir()),[])

    def archive(self,name,link=None):
        p=self.root/'input.tar'
        with tarfile.open(p,'w') as tar:
            info=tarfile.TarInfo(name)
            if link:
                info.type=tarfile.SYMTYPE;info.linkname=link;tar.addfile(info)
            else:
                data=b'fixture';info.size=len(data);tar.addfile(info,io.BytesIO(data))
        return p

    def test_archive_traversal_never_escapes(self):
        archive=self.archive('../escaped')
        with self.assertRaises(tarfile.FilterError):build.unpack(archive,'root',self.root/'dest')
        self.assertFalse((self.root/'escaped').exists());self.assertFalse((self.root/'dest').exists())

    def test_archive_escaping_symlink_refused(self):
        archive=self.archive('root/link','/outside')
        with self.assertRaises(tarfile.FilterError):build.unpack(archive,'root',self.root/'dest')
        self.assertFalse((self.root/'dest').exists())

    def test_archive_complete_directory_publication_and_no_merge(self):
        archive=self.archive('root/file')
        build.unpack(archive,'root',self.root/'dest')
        self.assertEqual((self.root/'dest/file').read_bytes(),b'fixture')
        with self.assertRaisesRegex(RuntimeError,'Refusing to merge'):build.unpack(archive,'root',self.root/'dest')

    def test_native_arm_host_still_targets_intel_stdlib(self):
        selected=build.select_downloads('arm64')
        self.assertIn('llvm_arm64',selected);self.assertIn('rust_arm64',selected);self.assertIn('rust_x86_64',selected)
        self.assertNotIn('llvm_x86_64',selected)
        self.assertEqual(build.select_downloads('x86_64').count('rust_x86_64'),1)

    def test_unknown_host_rejected(self):
        with self.assertRaises(RuntimeError):build.select_downloads('unsupported')

    def recipe(self,workspace=None):return build.Recipe(workspace or self.root/'owned',self.root/'cache',self.lock,'arm64')

    def test_workspace_with_whitespace_is_refused_before_writing(self):
        path=self.root/'unsafe space'
        with self.assertRaisesRegex(RuntimeError,'without whitespace'):self.recipe(path)
        self.assertFalse(path.exists())

    def test_unowned_nonempty_workspace_untouched(self):
        workspace=self.root/'existing';workspace.mkdir();(workspace/'keep').write_text('keep')
        with self.assertRaisesRegex(RuntimeError,'unowned'):self.recipe(workspace).owned()
        self.assertEqual((workspace/'keep').read_text(),'keep');self.assertFalse((workspace/build.MARKER).exists())

    def test_failed_preparation_cannot_report_completed(self):
        recipe=self.recipe();recipe.owned()
        def fail():raise RuntimeError('interrupted')
        with self.assertRaises(RuntimeError):recipe.stage('patches',fail)
        saved=json.loads((recipe.workspace/build.MARKER).read_text())
        self.assertEqual(saved['completed'],[]);self.assertEqual(saved['in_progress'],'patches')
        with self.assertRaisesRegex(RuntimeError,'interrupted'):self.recipe().owned()

    def test_compiler_interruption_can_resume(self):
        recipe=self.recipe();recipe.owned();recipe.state['in_progress']='compiling';build.write_json(recipe.workspace/build.MARKER,recipe.state)
        self.recipe().owned()

    def test_foreign_recipe_cannot_be_adopted(self):
        recipe=self.recipe();recipe.owned();recipe.state['recipe_sha256']='different';build.write_json(recipe.workspace/build.MARKER,recipe.state)
        with self.assertRaisesRegex(RuntimeError,'different recipe'):self.recipe().owned()

    def test_signing_and_compiler_overrides_do_not_leak(self):
        with patch.dict(build.os.environ,{'APPLE_ID':'fixture','TAURI_SIGNING_PRIVATE_KEY':'fixture','CARGO_TARGET_DIR':'outside','CC':'outside','TARGET':'outside'}):
            env=build.clean_environment()
        for key in ('APPLE_ID','TAURI_SIGNING_PRIVATE_KEY','CARGO_TARGET_DIR','CC','TARGET'):self.assertNotIn(key,env)

if __name__=='__main__':unittest.main()
