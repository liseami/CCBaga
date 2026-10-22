#!/usr/bin/env python3
# [INPUT]: intel-engine.lock.json, macOS/Xcode, Python 3.12-3.13, Ninja and a new owned workspace.
# [OUTPUT]: Pinned, resumable x86_64 Fingerprint Chromium source build and a provenance/behavior report.
# [POS]: External engine prerequisite for bundle-intel-macos.py; never installs apps or uses signing credentials.
# [PROTOCOL]: Update this contract and scripts/CLAUDE.md when changing this file.
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import shutil
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parent.parent
LOCK = Path(__file__).with_name('intel-engine.lock.json')
MARKER = '.ccbaga-intel-engine.json'
MAGIC = {bytes.fromhex(x) for x in ('feedface','cefaedfe','feedfacf','cffaedfe','cafebabe','bebafeca','cafebabf','bfbafeca')}


def digest(path, algorithm='sha256'):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, algorithm).hexdigest()


def write_json(path, value):
    pending = path.with_suffix(path.suffix + '.pending')
    pending.write_text(json.dumps(value, indent=2) + '\n')
    pending.replace(path)


def fetch(item, cache):
    """A corrupt existing cache is preserved, never silently accepted or overwritten."""
    name = item['filename']
    if Path(name).name != name or not item['url'].startswith('https://'):
        raise RuntimeError('Invalid locked download')
    cache.mkdir(parents=True, exist_ok=True)
    target = cache / name
    if target.exists():
        if not target.is_file() or target.is_symlink() or digest(target, item['algorithm']) != item['digest']:
            raise RuntimeError(f'Cached checksum mismatch; preserve and inspect {name}')
        return target
    with tempfile.TemporaryDirectory(prefix='.ccbaga-download-', dir=cache) as folder:
        pending = Path(folder) / name
        subprocess.run(['curl','--fail','--location','--retry','3','--output',str(pending),item['url']], check=True)
        if digest(pending, item['algorithm']) != item['digest']:
            raise RuntimeError(f'Download checksum mismatch: {name}')
        pending.replace(target)
    return target


def unpack(archive, root_name, destination, subtree=None):
    """Publish only a fully extracted directory, rejecting traversal and escaping links."""
    if destination.exists() or destination.is_symlink():
        raise RuntimeError(f'Refusing to merge an existing engine input: {destination}')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.ccbaga-unpack-', dir=destination.parent) as folder:
        staging = Path(folder)
        with tarfile.open(archive) as tar:
            tar.extractall(staging, filter='data')
        source = staging / root_name
        if subtree:
            source = source / subtree
        if not source.is_dir() or source.is_symlink() or not source.resolve().is_relative_to(staging.resolve()):
            raise RuntimeError('Locked archive root/subtree missing or unsafe')
        source.rename(destination)


def select_downloads(host):
    if host not in ('arm64','x86_64'):
        raise RuntimeError('Only native arm64 or x86_64 macOS build hosts are supported')
    return ['fingerprint','macos','chromium','bindgen',f'llvm_{host}',f'node_{host}',f'rust_{host}'] + (['rust_x86_64'] if host == 'arm64' else [])


def clean_environment():
    env = {k:v for k,v in os.environ.items() if not k.startswith(('APPLE_','TAURI_SIGNING_PRIVATE_KEY','PROD_MACOS_','MACOS_CERTIFICATE'))}
    for key in ('CC_SIGN_IDENTITY','CARGO_TARGET_DIR','CC','CXX','LD','CFLAGS','CXXFLAGS','CPPFLAGS','LDFLAGS','RUSTC','RUSTFLAGS','TARGET'):
        env.pop(key, None)
    return env


def source_contract(source, lock):
    for name, expected in lock['patched_source_sha256'].items():
        if digest(source / name) != expected:
            raise RuntimeError(f'Patched fingerprint source differs: {name}')
    if digest(source / 'out/Default/args.gn') != lock['gn_args_sha256']:
        raise RuntimeError('GN flags do not match the locked Intel recipe')
    builder = digest(source / 'tools/rust/build_bindgen.py')
    if builder not in lock['bindgen_builder_sha256'].values():
        raise RuntimeError('Unexpected patched bindgen builder')
    if digest(source / 'third_party/rust-toolchain-intermediate/bindgen-src/Cargo.lock') != lock['bindgen_lock_sha256']:
        raise RuntimeError('Bindgen dependency lock differs')
    return {'fingerprint_sources_match':True, 'gn_args_match':True, 'bindgen_lock_matches':True,
            'bindgen_cargo_locked':builder == lock['bindgen_builder_sha256']['cargo_locked']}


def verify_engine(source, lock, env, smoke=True):
    report = source_contract(source, lock)
    app = source / 'out/Default/Chromium.app'
    info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
    if info['CFBundleShortVersionString'] != lock['engine_version']:
        raise RuntimeError('Built engine version mismatch')
    count = 0
    for base, dirs, files in os.walk(app, followlinks=False):
        dirs[:] = [d for d in dirs if not (Path(base)/d).is_symlink()]
        for name in files:
            path = Path(base)/name
            if path.is_symlink():
                continue
            with path.open('rb') as stream:
                code = stream.read(4) in MAGIC
            if code:
                subprocess.run(['/usr/bin/lipo', str(path), '-verify_arch', 'x86_64'], check=True)
                count += 1
    if not count:
        raise RuntimeError('No built x86_64 Mach-O code found')
    binary = app / 'Contents/MacOS' / info['CFBundleExecutable']
    report.update(engine_version=info['CFBundleShortVersionString'], mach_o_files=count,
                  architecture='x86_64', chromium_sha256=digest(binary),
                  framework_sha256=digest(app/'Contents/Frameworks/Chromium Framework.framework/Chromium Framework'))
    if smoke:
        env = dict(env)
        native_node = source / ('third_party/node/mac_arm64/node-darwin-arm64/bin' if platform.machine()=='arm64' else 'third_party/node/mac/node-darwin-x64/bin')
        if (native_node/'node').is_file():
            env['PATH'] = str(native_node)+os.pathsep+env.get('PATH','')
        measured = subprocess.check_output(['node', str(ROOT/'scripts/test-intel-browser.cjs'), str(binary)], cwd=ROOT, env=env, text=True)
        report['fingerprint_behavior'] = json.loads(measured)
        if not report['fingerprint_behavior']['passed']:
            raise RuntimeError('Fingerprint behavior did not pass')
    return report


class Recipe:
    def __init__(self, workspace, cache, lock, host):
        self.workspace = workspace.resolve()
        if any(ch.isspace() for ch in str(self.workspace)):
            raise RuntimeError('Use a workspace path without whitespace for the pinned Rust compiler helpers')
        self.cache = cache.resolve()
        self.lock = lock
        self.host = host
        self.fingerprint = self.workspace/'fingerprint'
        self.macos = self.workspace/'macos'
        self.source = self.macos/'build/src'
        self.env = clean_environment()
        self.state = {'schema':1,'recipe_sha256':digest(LOCK),'host':host,'completed':[],'in_progress':None}

    def owned(self):
        marker = self.workspace/MARKER
        if marker.exists():
            if marker.is_symlink():
                raise RuntimeError('Linked workspace marker')
            saved = json.loads(marker.read_text())
            if any(saved.get(k) != self.state[k] for k in ('schema','recipe_sha256','host')):
                raise RuntimeError('Workspace belongs to a different recipe/host; use a new directory')
            if saved.get('in_progress') not in (None,'compiling'):
                raise RuntimeError('Preparation was interrupted; preserve it and use a new workspace with the same verified cache')
            self.state = saved
        else:
            if self.workspace.exists() and any(self.workspace.iterdir()):
                raise RuntimeError('Refusing an existing unowned nonempty workspace')
            self.workspace.mkdir(parents=True, exist_ok=True, mode=0o700)
            write_json(marker, self.state)

    def run(self, args, cwd=None):
        print('RUN', ' '.join(map(str,args)), flush=True)
        subprocess.run(list(map(str,args)), cwd=cwd or self.workspace, env=self.env, check=True)

    def stage(self, name, action):
        if name in self.state['completed']:
            return
        self.state['in_progress'] = name
        write_json(self.workspace/MARKER, self.state)
        print('STAGE', name, flush=True)
        action()
        self.state['completed'].append(name)
        self.state['in_progress'] = None
        write_json(self.workspace/MARKER, self.state)

    def inputs(self):
        result = {}
        for key in select_downloads(self.host):
            result[key] = fetch(self.lock['downloads'][key], self.cache)
        return result

    def prepare(self):
        self.owned()
        if 'source' not in self.state['completed'] and shutil.disk_usage(self.workspace).free < 60*1024**3:
            raise RuntimeError('Source preparation requires at least 60 GiB free; allow additional space for compilation')
        archives = self.inputs()
        downloads = self.lock['downloads']
        def extract(key, dest, subtree=None):
            unpack(archives[key], downloads[key]['root'], dest, subtree)
        def source():
            extract('fingerprint', self.fingerprint)
            extract('macos', self.macos)
            extract('chromium', self.source)
        self.stage('source', source)
        def patches():
            self.run([sys.executable,self.fingerprint/'utils/prune_binaries.py',self.source,self.fingerprint/'pruning.list'])
            self.run([sys.executable,self.fingerprint/'utils/patches.py','apply',self.source,self.fingerprint/'patches',self.macos/'patches'])
            self.run([sys.executable,self.fingerprint/'utils/domain_substitution.py','apply','-r',self.fingerprint/'domain_regex.list','-f',self.fingerprint/'domain_substitution.list',self.source])
            for name, expected in self.lock['patched_source_sha256'].items():
                if digest(self.source/name) != expected:
                    raise RuntimeError(f'Fresh patched source mismatch: {name}')
            builder = self.source/'tools/rust/build_bindgen.py'
            if digest(builder) != self.lock['bindgen_builder_sha256']['upstream_patched']:
                raise RuntimeError('Unexpected fresh bindgen builder; do not patch an unknown version')
            text = builder.read_text()
            needle = "        f'--target={rust_target}',\n"
            if text.count(needle) != 1:
                raise RuntimeError('Missing unique bindgen Cargo target')
            builder.write_text(text.replace(needle, needle+"        '--locked',\n"))
            if digest(builder) != self.lock['bindgen_builder_sha256']['cargo_locked']:
                raise RuntimeError('Locked bindgen patch mismatch')
        self.stage('patches', patches)
        def toolchain():
            third = self.source/'third_party'
            llvm = third/'llvm-build/Release+Asserts'
            rust = third/'rust-toolchain'
            # Match upstream replacement of vendor toolchains, confined to this owned source tree.
            node = third/('node/mac_arm64/node-darwin-arm64' if self.host=='arm64' else 'node/mac/node-darwin-x64')
            for path in (llvm,rust,node):
                if path.is_symlink():
                    raise RuntimeError('Linked vendor toolchain directory')
                if path.exists():
                    if not path.resolve().is_relative_to(self.source.resolve()):
                        raise RuntimeError('Toolchain outside owned source tree')
                    shutil.rmtree(path)
            extract('llvm_'+self.host, llvm)
            extract('rust_'+self.host, rust)
            node = third/('node/mac_arm64/node-darwin-arm64' if self.host=='arm64' else 'node/mac/node-darwin-x64')
            extract('node_'+self.host, node)
            native = 'aarch64-apple-darwin' if self.host=='arm64' else 'x86_64-apple-darwin'
            def link(target, path):
                path.parent.mkdir(parents=True, exist_ok=True)
                if path.exists() or path.is_symlink():
                    raise RuntimeError(f'Unexpected toolchain link target: {path}')
                path.symlink_to(os.path.relpath(target, path.parent))
            (rust/'bin').mkdir(exist_ok=True)
            for name,component in [('rustc','rustc'),('cargo','cargo'),('rustfmt','rustfmt-preview')]:
                link(rust/component/'bin'/name,rust/'bin'/name)
            link(rust/f'rust-std-{native}/lib/rustlib/{native}/lib',rust/f'rustc/lib/rustlib/{native}/lib')
            link(rust/'rustc/lib',rust/'rustfmt-preview/lib')
            link(llvm/'bin/llvm-install-name-tool',llvm/'bin/install_name_tool')
            if self.host=='arm64':
                extract('rust_x86_64',rust/'rustc/lib/rustlib/x86_64-apple-darwin','rust-std-x86_64-apple-darwin/lib/rustlib/x86_64-apple-darwin')
            extract('bindgen',third/'rust-toolchain-intermediate/bindgen-src')
            if digest(third/'rust-toolchain-intermediate/bindgen-src/Cargo.lock') != self.lock['bindgen_lock_sha256']:
                raise RuntimeError('Fresh bindgen lock mismatch')
        self.stage('toolchain', toolchain)
        node_bin = self.source/('third_party/node/mac_arm64/node-darwin-arm64/bin' if self.host=='arm64' else 'third_party/node/mac/node-darwin-x64/bin')
        self.env['PATH'] = str(node_bin)+os.pathsep+self.env.get('PATH','')
        out = self.source/'out/Default'
        out.mkdir(parents=True, exist_ok=True)
        flags = (self.fingerprint/'flags.gn').read_text()+'\n'+(self.macos/'flags.macos.gn').read_text()+'\n'+'\n'.join(self.lock['gn_overrides'])+'\n'
        if hashlib.sha256(flags.encode()).hexdigest() != self.lock['gn_args_sha256']:
            raise RuntimeError('Locked source flags mismatch')
        if (out/'args.gn').exists() and (out/'args.gn').read_text() != flags:
            raise RuntimeError('Existing GN flags differ; preserve them and use a new recipe workspace')
        if not (out/'args.gn').exists():
            (out/'args.gn').write_text(flags)
        self.stage('gn-bootstrap', lambda:self.run([sys.executable,self.source/'tools/gn/bootstrap/bootstrap.py','-o','out/Default/gn','--skip-generate-buildfiles','-j','2'],self.source))
        self.stage('bindgen', lambda:self.run([sys.executable,self.source/'tools/rust/build_bindgen.py','--skip-checkout','--skip-test'],self.source))
        self.stage('generate', lambda:self.run([out/'gn','gen',out,'--fail-on-unused-args'],self.source))
        source_contract(self.source,self.lock)
        return self.source

    def build(self, jobs):
        source = self.prepare()
        if shutil.disk_usage(self.workspace).free < 60*1024**3:
            raise RuntimeError('Compilation requires at least 60 GiB remaining; 200 GiB initial free space is recommended')
        self.state['in_progress'] = 'compiling'
        write_json(self.workspace/MARKER,self.state)
        self.run(['ninja','-C',source/'out/Default','-j',str(jobs),'chrome'],source)
        report = verify_engine(source,self.lock,self.env)
        self.state['in_progress'] = None
        if 'compiling' not in self.state['completed']:
            self.state['completed'].append('compiling')
        write_json(self.workspace/MARKER,self.state)
        return report


def preflight():
    if platform.system() != 'Darwin' or not ((3,12) <= sys.version_info[:2] <= (3,13)):
        raise RuntimeError('Use native macOS and Python 3.12 or 3.13')
    for tool in ('curl','patch','ninja','xcrun'):
        if not shutil.which(tool):
            raise RuntimeError('Missing prerequisite: '+tool)
    sdk = subprocess.check_output(['xcrun','--sdk','macosx','--show-sdk-version'],text=True).strip()
    ninja = subprocess.check_output(['ninja','--version'],text=True).strip()
    if not ninja.startswith(json.loads(LOCK.read_text())['ninja_package_version']):
        raise RuntimeError('Use the pinned Ninja version in intel-engine-requirements.txt')
    return {'host':platform.machine(),'macos':platform.mac_ver()[0],'python':platform.python_version(),'sdk':sdk,'ninja':ninja}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=['plan','prepare','build','verify'])
    parser.add_argument('--workspace',type=Path)
    parser.add_argument('--cache',type=Path)
    parser.add_argument('--source-tree',type=Path,help='Read-only verification of an existing completed source build')
    parser.add_argument('--jobs',type=int,default=2)
    parser.add_argument('--report',type=Path)
    args=parser.parse_args()
    lock=json.loads(LOCK.read_text())
    host=platform.machine()
    if args.jobs < 1:
        parser.error('--jobs must be positive')
    if args.action=='plan':
        print(json.dumps({'recipe_sha256':digest(LOCK),'version':lock['engine_version'],'target':'x86_64','host':host,'downloads':[lock['downloads'][k] for k in select_downloads(host)],'steps':['verify/download pinned archives','extract pristine Chromium','prune + fingerprint patches + macOS patches + domain substitutions','install native LLVM/Rust/Node and Intel Rust stdlib','fixed bindgen source with locked Cargo dependencies','bootstrap GN and generate x64 Ninja files','ninja chrome','all-code architecture and fingerprint behavior verification']},indent=2))
        return
    tools=preflight()
    if args.action=='verify':
        if args.source_tree is None:
            parser.error('verify requires --source-tree')
        report=verify_engine(args.source_tree.resolve(),lock,clean_environment())
    else:
        if args.workspace is None:
            parser.error('prepare/build requires a new --workspace')
        recipe=Recipe(args.workspace,args.cache or args.workspace/'downloads',lock,host)
        if args.action=='prepare':
            source=recipe.prepare()
            report={'prepared':True,'full_engine_compiled':False,'source_tree':str(source),**source_contract(source,lock)}
        else:
            report=recipe.build(args.jobs)
    report.update(recipe_sha256=digest(LOCK),tools=tools)
    if args.report:
        args.report.parent.mkdir(parents=True,exist_ok=True)
        write_json(args.report,report)
    print(json.dumps(report,indent=2))


if __name__=='__main__':
    try:
        main()
    except (RuntimeError,OSError,ValueError,subprocess.CalledProcessError) as error:
        print('Intel engine build failed:',error,file=sys.stderr)
        sys.exit(1)
