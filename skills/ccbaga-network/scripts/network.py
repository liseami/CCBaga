#!/usr/bin/env python3
# [INPUT]: 本机 0600 JSON、完整三文件密钥、OpenSSH/curl 和本 Skill UTF-8 部署脚本及相邻 Windows 原生边界。
# [OUTPUT]: 分阶段密钥/连接/出口核验、显式部署、私密订阅文件与 TUN 实测。
# [POS]: Skill 的标准库执行层；AI 收集资料后通过标准输入建私密配置；秘密不进入 argv/公开日志，失败不报告部署成功。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import argparse
import base64
import ipaddress
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import threading
from urllib.parse import urlencode, urlsplit

SKILL = Path(__file__).resolve().parent.parent
if os.name == 'nt':
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import windows as win
DEFAULT = Path.home() / '.config/ccbaga-network/connection.json'


def program(name):
    if os.name == 'nt':
        base = Path(os.environ.get('SystemRoot', r'C:\Windows'))
        path = base/'System32/OpenSSH'/f'{name}.exe' if name in ['ssh','ssh-keygen'] else base/'System32'/f'{name}.exe'
        if path.is_file(): return str(path)
        if name in ['ssh','ssh-keygen']: raise ValueError('Enable Windows OpenSSH Client first; the AI can handle prerequisites after approval')
    found = shutil.which(name)
    if not found: raise ValueError(f'Install local {name} first')
    return found

def provider_url(name):
    settings = json.loads((SKILL / 'providers.json').read_text())[name]
    return settings['base'] + '?' + urlencode({settings['parameter']: settings['value']})


def host_valid(value):
    if not isinstance(value, str) or not value or len(value) > 253:
        return False
    try:
        ipaddress.ip_address(value)
        return True
    except ValueError:
        return all(re.fullmatch(r'[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?', p) for p in value.split('.'))


def private_write(path, content):
    path = Path(path).expanduser().absolute()
    for parent in [path.parent, *path.parents]:
        if parent.is_symlink() or (os.name == 'nt' and parent.is_junction()):
            raise ValueError('Refusing a linked output directory')
    new_parent = not path.parent.exists()
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    if os.name == 'nt' and new_parent: win.protect(path.parent)
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        if os.name == 'nt': win.protect(path)
        with os.fdopen(fd, 'w', encoding='utf-8', newline='') as stream:
            fd = None
            stream.write(content)
    except BaseException:
        if fd is not None: os.close(fd)
        path.unlink(missing_ok=True)
        raise


def read_config(path):
    path = Path(path).expanduser()
    if any(p.is_symlink() or (os.name == 'nt' and p.is_junction()) for p in [path, *path.parents]):
        raise ValueError('Config must be a regular private file outside linked directories')
    if os.name == 'nt': win.assert_private(path)
    with path.open(encoding='utf-8') as stream:
        st = os.fstat(stream.fileno())
        if st.st_size > 65536 or (os.name != 'nt' and (st.st_uid != os.getuid() or st.st_mode & 0o077)):
            raise ValueError('Config must be owned by you, mode 0600, and under 64 KiB')
        conf = json.load(stream)
    return validate_config(conf)


def validate_config(conf):
    if not isinstance(conf, dict) or set(conf) != {'vps', 'pem', 'ppk', 'pub', 'home_host', 'home_port', 'home_user', 'home_pass', 'expected_ip'}:
        raise ValueError('Provide the VPS, three key file paths and complete residential connection details')
    for key in ['pem', 'ppk', 'pub']:
        if not isinstance(conf[key], str) or not conf[key] or any(ord(c) < 32 for c in conf[key]):
            raise ValueError('Provide three local key file paths')
    if not host_valid(conf['vps']) or not host_valid(conf['home_host']):
        raise ValueError('Use a VPS/proxy IP or hostname without URL or command syntax')
    if type(conf['home_port']) is not int or not 1 <= conf['home_port'] <= 65535:
        raise ValueError('Invalid SOCKS5 port')
    for key in ['home_user', 'home_pass']:
        value = conf[key]
        if not isinstance(value, str) or not value or len(value) > 4096 or any(ord(c) < 32 for c in value):
            raise ValueError('Invalid proxy credentials; keep them in the local config')
    conf['expected_ip'] = str(ipaddress.ip_address(conf['expected_ip']))
    return conf


def key_data(pem, ppk, public):
    values = []
    for name, suffix in [(pem, '.pem'), (ppk, '.ppk'), (public, '.pub')]:
        if not isinstance(name, str) or not name:
            raise ValueError('Provide PEM, PPK and PUB file paths')
        path = Path(name).expanduser()
        if path.suffix != suffix or not path.is_file() or not 0 < path.stat().st_size <= 65536:
            raise ValueError('Provide one nonempty PEM/PPK/PUB file, each under 64 KiB')
        values.append(path.read_text(encoding='utf-8'))
    private, putty, pub = values
    expected = pub.split()[:2]
    lines = putty.splitlines()
    if len(expected) != 2 or not lines or not re.fullmatch(r'PuTTY-User-Key-File-[23]: .+', lines[0]):
        raise ValueError('Invalid key bundle')
    public_line = next((i for i, line in enumerate(lines) if line.startswith('Public-Lines: ')), None)
    if public_line is None:
        raise ValueError('PPK public section missing')
    count = int(lines[public_line].split(': ', 1)[1])
    if not 0 < count <= 1000:
        raise ValueError('Invalid PPK public section')
    parts = [lines[0].split(': ', 1)[1], ''.join(lines[public_line + 1:public_line + 1 + count])]
    base64.b64decode(parts[1], validate=True)
    if parts != expected:
        raise ValueError('PPK and PUB do not match')
    return private, expected


class Key:
    def __init__(self, conf):
        private, expected = key_data(conf['pem'], conf['ppk'], conf['pub'])
        self.temp = tempfile.TemporaryDirectory(prefix='ccbaga-key-')
        self.path = Path(self.temp.name).resolve() / 'identity.pem'
        private_write(self.path, private)
        result = subprocess.run([program('ssh-keygen'), '-y', '-P', '', '-f', str(self.path)], capture_output=True, text=True)
        if result.returncode or result.stdout.split()[:2] != expected:
            self.temp.cleanup()
            raise ValueError('PEM is encrypted, invalid, or does not match PPK/PUB')

    def __enter__(self):
        return self.path

    def __exit__(self, *args):
        self.temp.cleanup()


def ssh_args(conf, key, config_path, accept_new=False):
    hosts = Path(config_path).expanduser().absolute().parent / 'known_hosts'
    if hosts.is_symlink():
        raise ValueError('Refusing linked known_hosts')
    return [program('ssh'), '-F', os.devnull, '-T', '-p', '22', '-l', 'root',
            '-o', 'BatchMode=yes', '-o', 'IdentitiesOnly=yes', '-o', 'IdentityAgent=none',
            '-o', 'PreferredAuthentications=publickey', '-o', 'PasswordAuthentication=no',
            '-o', 'KbdInteractiveAuthentication=no', '-o', 'StrictHostKeyChecking=' + ('accept-new' if accept_new else 'yes'),
            '-o', 'UserKnownHostsFile="' + hosts.as_posix() + '"', '-o', 'ConnectTimeout=15',
            '-o', 'ServerAliveInterval=15', '-o', 'ServerAliveCountMax=3', '-i', str(key), conf['vps']]


def execute(command, payload='', timeout=30):
    child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                             stderr=subprocess.STDOUT, start_new_session=os.name != 'nt')
    job = win.Job(child) if os.name == 'nt' else None
    tail = bytearray()
    def read():
        while chunk := child.stdout.read(4096):
            tail.extend(chunk)
            del tail[:-65536]
    def write():
        try:
            child.stdin.write(payload.encode())
            child.stdin.close()
        except (BrokenPipeError, OSError):
            pass
    reader = threading.Thread(target=read, daemon=True)
    writer = threading.Thread(target=write, daemon=True)
    reader.start(); writer.start()
    try:
        code = child.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        if job: job.stop()
        else: os.killpg(child.pid, signal.SIGTERM)
        try:
            child.wait(timeout=2)
        except subprocess.TimeoutExpired:
            if job: job.stop()
            else: os.killpg(child.pid, signal.SIGKILL)
            child.wait()
        raise RuntimeError('Task timed out; remote changes may be partial. Do not blindly rerun.')
    finally:
        if job: job.close()
        reader.join(timeout=2); writer.join(timeout=2)
        child.stdout.close()
    if code:
        raise RuntimeError('SSH/task failed; raw output suppressed to protect credentials. Check connectivity and server state before retrying.')
    return tail.decode(errors='replace')


def exports(conf):
    pairs = {'HOME_HOST': conf['home_host'], 'HOME_PORT': str(conf['home_port']),
             'HOME_USER': conf['home_user'], 'HOME_PASS': conf['home_pass'],
             'EXPECTED_EXIT_IP': conf['expected_ip'], 'SERVER_ADDR': conf['vps'], 'SSH_PORT': '22',
             'DIRECT_DOMAINS': ' ', 'NODE_NAME': 'CCBaga-Home'}
    return 'set -euo pipefail\n' + ''.join(f'export {key}={shlex.quote(value)}\n' for key, value in pairs.items())


def curl_config(conf):
    host = conf['home_host']
    if ':' in host:
        host = '[' + host + ']'
    return '\n'.join(f'{key} = {json.dumps(value, ensure_ascii=False)}' for key, value in [
        ('url', 'https://api.ipify.org'), ('proxy', f'socks5h://{host}:{conf["home_port"]}'),
        ('proxy-user', conf['home_user'] + ':' + conf['home_pass'])]) + '\n'


PREFLIGHT = '''test "$(id -u)" = 0
. /etc/os-release
test "$ID" = debian
case "$VERSION_ID" in 12|13) ;; *) exit 1;; esac
test -s /root/.ssh/authorized_keys
command -v curl >/dev/null
command -v python3 >/dev/null
printf 'CC_PLATFORM=debian-%s\\n' "$VERSION_ID"
python3 - <<'REMOTE'
import json,os
host=os.environ['HOME_HOST']
if ':' in host:host='['+host+']'
for key,value in [('url','https://api.ipify.org'),('proxy','socks5h://'+host+':'+os.environ['HOME_PORT']),('proxy-user',os.environ['HOME_USER']+':'+os.environ['HOME_PASS'])]:
 print(key+' = '+json.dumps(value, ensure_ascii=False))
REMOTE
'''


def check(conf, key, config_path):
    for name in ['ssh', 'ssh-keygen', 'curl']: program(name)
    local = subprocess.run([program('curl'), '--silent', '--show-error', '--fail', '--max-time', '15', '--noproxy', '', '--config', '-'],
                           input=curl_config(conf), text=True, encoding='utf-8', capture_output=True)
    print('LOCAL_SOCKS=' + ('verified' if local.returncode == 0 and local.stdout.strip() == conf['expected_ip'] else 'unverified; checking from VPS'))
    # ----- 导出秘密通过 SSH 标准输入；curl 的配置同样走标准输入 -----
    probe = PREFLIGHT.replace("python3 - <<'REMOTE'", "python3 - <<'REMOTE' | curl --silent --show-error --fail --max-time 20 --noproxy '' --config -")
    output = execute(ssh_args(conf, key, config_path) + ['bash -s'], exports(conf) + probe, timeout=60)
    if output.strip().splitlines()[-1] != conf['expected_ip']:
        raise RuntimeError('VPS SOCKS5 exit does not match the expected IP; stop before deployment')
    print('SSH_ROOT_DEBIAN=verified; VPS_SOCKS_EXIT=verified')


def subscription(output, host):
    matches = re.findall(r'^CC_SUB_URL=(\S+)$', output, re.MULTILINE)
    if not matches:
        raise ValueError('No subscription returned after successful self-checks')
    link = matches[-1]; parsed = urlsplit(link)
    hostname = parsed.hostname
    try:
        same = ipaddress.ip_address(hostname) == ipaddress.ip_address(host)
    except ValueError:
        same = hostname == host.lower()
    if not same or parsed.scheme != 'http' or parsed.port != 8443 or parsed.username or parsed.password or parsed.query or parsed.fragment or not re.fullmatch(r'/[a-f0-9]{32}/clash\.yaml', parsed.path):
        raise ValueError('Unexpected subscription destination')
    return link


def fetch(conf, key, config_path):
    # ----- 新 SSH 会话复查服务，再经加密通道取配置；不请求公开 HTTP 订阅 -----
    task = """set -e
systemctl is-active --quiet xray
systemctl is-active --quiet sub
python3 - <<'REMOTE'
from pathlib import Path
import re,sys
state=Path('/etc/xray-setup.env').read_text()
token=re.search(r'^SUB_TOKEN=([a-f0-9]{32})$',state,re.M).group(1)
p=Path('/srv/sub')/token/'clash.yaml'
assert p.stat().st_size < 65536
sys.stdout.write(p.read_text())
REMOTE
"""
    text = execute(ssh_args(conf,key,config_path)+['bash -s'],task,timeout=30)
    if 'proxies:' not in text or 'reality-opts:' not in text or 'tun:' not in text:
        raise RuntimeError('Subscription is incomplete; no local import file written')
    target = Path(config_path).expanduser().parent / ('clash-' + time.strftime('%Y%m%d-%H%M%S') + '.yaml')
    private_write(target,text)
    print('New SSH session and services verified; private local import file:',target)


def configure(path, stream):
    # ----- AI 提交 JSON 标准输入；用户无需开终端或编辑配置 -----
    payload = stream.read(65537)
    if len(payload) > 65536:
        raise ValueError('Connection details exceed 64 KiB')
    conf = validate_config(json.loads(payload))
    target = Path(path).expanduser().absolute()
    resolved = target.resolve()
    if resolved.is_relative_to(SKILL) or any((parent / '.git').exists() for parent in resolved.parents):
        raise ValueError('Save private connection details outside skill and Git directories')
    private_write(target, json.dumps(conf, indent=2) + '\n')
    print('Private connection details saved; no credentials printed')


def main():
    parser = argparse.ArgumentParser(description='CCBaga network skill — staged, private, explicit')
    parser.add_argument('action', choices=['links', 'configure', 'keys', 'identity', 'check', 'deploy', 'fetch', 'tun-check'])
    parser.add_argument('--config', default=str(DEFAULT))
    parser.add_argument('--pem'); parser.add_argument('--ppk'); parser.add_argument('--pub')
    parser.add_argument('--confirm', action='store_true', help='Use only after reviewing and approving remote changes')
    args = parser.parse_args()
    if args.action == 'links':
        for name in ['dmit', 'webshare']: print(name + ': ' + provider_url(name))
        return
    if args.action == 'configure':
        configure(args.config, sys.stdin); return
    if args.action == 'keys':
        with Key(vars(args)): print('PEM/PPK/PUB pairing verified; temporary private key removed on exit')
        return
    conf = read_config(args.config)
    if args.action == 'tun-check':
        result = execute([program('curl'), '--silent', '--fail', '--max-time', '20', '--proxy', '', '--noproxy', '*', 'https://api.ipify.org'])
        if result.strip() != conf['expected_ip']:
            raise RuntimeError('System/TUN exit differs from the expected residential IP')
        print('TUN_EXIT=verified (CLI path only; also verify browser/DNS/WebRTC)'); return
    if args.action == 'deploy' and not args.confirm:
        raise ValueError('Review the actual VPS and approve changes before passing --confirm')
    with Key(conf) as key:
        if args.action == 'identity':
            output = execute(ssh_args(conf,key,args.config,accept_new=True) + ['bash -s'],
                             'set -e; test "$(id -u)" = 0; . /etc/os-release; test "$ID" = debian; printf "ROOT=verified; OS=%s\\n" "$VERSION_ID"')
            print(output.strip())
            result = subprocess.run([program('ssh-keygen'),'-l','-f',str(Path(args.config).expanduser().parent/'known_hosts')],capture_output=True,text=True,check=True)
            print(result.stdout.strip()); return
        if args.action == 'fetch':
            fetch(conf,key,args.config); return
        check(conf, key, args.config)
        if args.action == 'deploy':
            wrapper = "umask 077; task=$(mktemp); trap 'rm -f \"$task\"' EXIT; cat > \"$task\"; bash \"$task\""
            output = execute(ssh_args(conf,key,args.config) + ['bash -c ' + shlex.quote(wrapper)],
                             exports(conf) + (SKILL/'scripts/setup-vps.sh').read_text(encoding='utf-8'), timeout=1800)
            link = subscription(output,conf['vps'])
            receipt = Path(args.config).expanduser().parent / ('subscription-' + time.strftime('%Y%m%d-%H%M%S') + '.json')
            private_write(receipt,json.dumps({'subscription':link,'expected_exit_ip':conf['expected_ip']})+'\n')
            fetch(conf,key,args.config)
            print('Deployment self-checks passed. Private subscription saved locally:', receipt)


if __name__ == '__main__':
    try:
        main()
    except (ValueError,KeyError,RuntimeError,OSError) as error:
        raise SystemExit(str(error))
