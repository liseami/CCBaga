#!/usr/bin/env python3
# [INPUT]: 本机构建的 Chromium.app、固定引擎版本和临时无账号浏览器资料。
# [OUTPUT]: 所有 Mach-O 的 Intel 架构、两个指纹参数探针与 Canvas/原生画面开关比较；全部通过才写验收记录。
# [POS]: Intel 引擎候选验收；不访问用户资料、不安装应用，不替代签名公证与完整产品验收。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import argparse
import base64
from html.parser import HTMLParser
import json
import math
import os
from pathlib import Path
import platform
import plistlib
import re
import signal
import subprocess
import tempfile
from macos_arch import check_bundle

ROOT = Path(__file__).resolve().parent.parent
PAGE = '''<!doctype html><pre id="ccbaga-probe"></pre><script>
document.getElementById('ccbaga-probe').textContent=JSON.stringify({
cores:navigator.hardwareConcurrency,timezone:Intl.DateTimeFormat().resolvedOptions().timeZone,
language:navigator.language,platform:navigator.platform});</script>'''
GRAPHICS_PAGE = '''<!doctype html><!-- Relative layout exercises noise; the upstream patch exempts fixed absolute positions. -->
<div id="shape" style="position:relative;left:17px;top:23px;width:123.5px;height:47.25px;padding:3px;border:1px solid"></div>
<pre id="ccbaga-probe"></pre><script>
function hash(values) {
  let result=2166136261;
  for(let i=0;i<values.length;i++) result=Math.imul(result^(typeof values==='string'?values.charCodeAt(i):values[i]),16777619);
  return (result>>>0).toString(16).padStart(8,'0');
}
function sample() {
  const canvas=document.createElement('canvas'); canvas.width=64;canvas.height=32;
  const context=canvas.getContext('2d',{willReadFrequently:true});
  if(!context) throw new Error('Canvas 2D unavailable');
  const image=context.createImageData(64,32);
  for(let i=0;i<image.data.length;i+=4) {
    image.data[i]=(i*17)%251+2;image.data[i+1]=(i*31)%251+2;
    image.data[i+2]=(i*47)%251+2;image.data[i+3]=255;
  }
  context.putImageData(image,0,0);context.font='17px Arial';
  const pixels=context.getImageData(0,0,64,32).data;
  const png=canvas.toDataURL('image/png');
  const rect=document.getElementById('shape').getBoundingClientRect();
  const clientRect=document.getElementById('shape').getClientRects()[0];
  return {pixels:hash(pixels),png:hash(png),pngValid:png.startsWith('data:image/png;base64,'),
    textWidth:context.measureText('CCBaga Intel browser').width,rect:[rect.x,rect.y,rect.width,rect.height],
    clientRect:[clientRect.x,clientRect.y,clientRect.width,clientRect.height]};
}
try { document.getElementById('ccbaga-probe').textContent=JSON.stringify({samples:[sample(),sample()]}); }
catch(error) { document.getElementById('ccbaga-probe').textContent=JSON.stringify({error:String(error)}); }
</script>'''


class Probe(HTMLParser):
    def __init__(self):
        super().__init__()
        self.active = False
        self.parts = []

    def handle_starttag(self, tag, attrs):
        if tag == 'pre' and dict(attrs).get('id') == 'ccbaga-probe':
            self.active = True

    def handle_data(self, data):
        if self.active:
            self.parts.append(data)

    def handle_endtag(self, tag):
        if tag == 'pre':
            self.active = False


def read_probe(document):
    parser = Probe()
    parser.feed(document)
    return json.loads(''.join(parser.parts))


def validate_probe(document, expected):
    observed = read_probe(document)
    if observed != expected:
        raise RuntimeError(f'Fingerprint switches did not take effect: expected {expected}, observed {observed}')
    return observed


def validate_graphics(document):
    observed = read_probe(document)
    samples = observed.get('samples') if isinstance(observed, dict) else None
    if not isinstance(samples, list) or len(samples) != 2:
        raise RuntimeError(f'Missing Canvas and geometry observations: {observed}')
    for sample in samples:
        if not isinstance(sample, dict) or sample.get('pngValid') is not True:
            raise RuntimeError('Canvas PNG encoding did not produce an image')
        if any(not isinstance(sample.get(key), str) or not re.fullmatch('[0-9a-f]{8}', sample[key]) for key in ['pixels', 'png']):
            raise RuntimeError('Invalid Canvas pixel or PNG observation')
        for key in ['rect', 'clientRect']:
            rect = sample.get(key)
            numbers = [sample.get('textWidth')] + rect if isinstance(rect, list) else []
            if len(numbers) != 5 or any(type(value) not in [int, float] or not math.isfinite(value) for value in numbers):
                raise RuntimeError('Missing finite text and geometry observations')
            if numbers[0] <= 0 or numbers[3] <= 0 or numbers[4] <= 0:
                raise RuntimeError('Canvas text or rectangle has no dimensions')
    if samples[0] != samples[1]:
        raise RuntimeError('Fingerprint graphics observations are unstable within one document')
    return samples[0]


def compare_graphics(observations):
    if len(observations) != 4:
        raise RuntimeError('Expected two seeded and two native graphics observations')
    first, second, native_first, native_second = observations
    if native_first != native_second:
        raise RuntimeError('Native graphics must ignore seed changes when canvas and clientrects spoofing are disabled')
    for key in ['pixels', 'png', 'textWidth', 'rect', 'clientRect']:
        if first[key] == second[key] or first[key] == native_first[key] or second[key] == native_first[key]:
            raise RuntimeError(f'Canvas/clientrects fingerprint control did not take effect: {key}')
    return observations


def architectures(app):
    binaries = check_bundle(app, 'x86_64')
    if 'Contents/MacOS/Chromium' not in binaries or len(binaries) < 2:
        raise RuntimeError('Expected a complete Chromium app with its native Framework and helpers')
    return binaries


def run_probe(binary, seed, cores, timezone, language, page=PAGE, extra_arguments=()):
    with tempfile.TemporaryDirectory(prefix='ccbaga-intel-probe-') as temporary:
        home = Path(temporary)
        env = {key: value for key, value in os.environ.items()
               if not key.startswith(('APPLE_', 'TAURI_SIGNING_PRIVATE_KEY', 'CC_SIGN_'))}
        env.update(HOME=str(home), XDG_CONFIG_HOME=str(home / 'config'), XDG_CACHE_HOME=str(home / 'cache'))
        data_url = 'data:text/html;base64,' + base64.b64encode(page.encode()).decode()
        command = [str(binary), '--headless=new', '--disable-gpu', '--dump-dom',
                   '--no-first-run', '--no-default-browser-check', '--disable-background-networking',
                   '--disable-component-update', '--use-mock-keychain', '--password-store=basic',
                   '--host-resolver-rules=MAP * ~NOTFOUND', '--proxy-server=http://127.0.0.1:9',
                   f'--user-data-dir={home / "profile"}', f'--fingerprint={seed}',
                   '--fingerprint-platform=macos', f'--fingerprint-hardware-concurrency={cores}',
                   f'--timezone={timezone}', f'--lang={language}', f'--accept-lang={language}', *extra_arguments, data_url]
        process = subprocess.Popen(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
        try:
            stdout, stderr = process.communicate(timeout=60)
            if process.returncode:
                raise RuntimeError(f'Intel browser probe exited {process.returncode}: {stderr[-2000:]}')
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
                process.communicate()
        return stdout


def probe(binary, seed, cores, timezone, language):
    return validate_probe(run_probe(binary, seed, cores, timezone, language),
                          {'cores': cores, 'timezone': timezone, 'language': language, 'platform': 'MacIntel'})


def graphics_probe(binary, seed, native_graphics):
    disabled = 'font,audio,gpu,canvas,clientrects' if native_graphics else 'font,audio,gpu'
    document = run_probe(binary, seed, 7, 'UTC', 'en-US', GRAPHICS_PAGE, [f'--disable-spoofing={disabled}'])
    return validate_graphics(document)


def verify(app):
    if platform.system() != 'Darwin' or platform.machine() != 'x86_64':
        raise RuntimeError('Run acceptance on a native Intel Mac')
    if subprocess.check_output(['sysctl', '-in', 'sysctl.proc_translated'], text=True).strip() == '1':
        raise RuntimeError('Rosetta execution cannot substitute for native Intel acceptance')
    info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
    version = (ROOT / 'browser-runtime/version.txt').read_text().strip()
    if info.get('CFBundleShortVersionString') != version or info.get('CFBundleExecutable') != 'Chromium':
        raise RuntimeError('Unexpected engine version or executable')
    try:
        minimum = [int(part) for part in info['LSMinimumSystemVersion'].split('.')]
        minimum += [0] * (3 - len(minimum))
        if len(minimum) != 3 or tuple(minimum) > (12, 0, 0):
            raise ValueError('newer runtime requirement')
    except (KeyError, AttributeError, ValueError):
        raise RuntimeError('Intel engine must retain macOS 12 runtime support') from None
    binaries = architectures(app)
    binary = app / 'Contents/MacOS/Chromium'
    actual = subprocess.check_output([str(binary), '--version'], text=True, timeout=30).strip()
    if version not in actual:
        raise RuntimeError('Actual executable version does not match the pinned engine')
    observations = [probe(binary, 1001, 7, 'Asia/Tokyo', 'en-US'),
                    probe(binary, 2002, 11, 'America/New_York', 'fr-FR')]
    graphics_cases = [(False, 1001), (False, 2002), (True, 1001), (True, 2002)]
    graphics = compare_graphics([graphics_probe(binary, seed, native) for native, seed in graphics_cases])
    return {'version': version, 'minimum_macos': info.get('LSMinimumSystemVersion'),
            'native_execution': platform.machine(), 'binaries': binaries, 'observations': observations,
            'graphics_observations': [{'seed': seed, 'native_graphics': native, 'observed': observed}
                                      for (native, seed), observed in zip(graphics_cases, graphics)],
            'scope': 'architecture, CLI version, CPU/language/timezone, seeded Canvas pixels/PNG/text/geometry and native graphics controls in software headless mode; full product acceptance pending'}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('app', type=Path)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    report = verify(args.app.resolve())
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print('Intel engine architecture and real fingerprint observations passed')


if __name__ == '__main__':
    main()
