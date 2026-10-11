#!/usr/bin/env bash
# [INPUT]: SSH 标准输入注入的家宽 SOCKS5 与 VPS 参数；Debian 12/13 root 环境。
# [OUTPUT]: Xray/Reality、家宽出口、防火墙、SSH 加固与通过自检的 Clash 订阅。
# [POS]: 随 CC清理器编译的部署脚本，源自用户 Downloads/setup-vps.sh；身份保存在远端 root 状态文件。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
# =============================================================================
# setup-vps.sh — Reproduce the "DMIT LAX" proxy node build on a fresh Debian 12/13
#
#   Client ──VLESS+Reality+Vision (tcp/443)──► this VPS ──SOCKS5──► residential proxy ──► Internet
#
# Design rules mirrored from production (2026-10-09):
#   * Everything (incl. Claude / Anthropic / OpenAI) leaves via the residential SOCKS
#     proxy ("home"). If home is down, traffic FAILS CLOSED. There is NO direct fallback.
#   * Only the domains in DIRECT_DOMAINS (default: Stripe) leave directly from the VPS (IPv4).
#   * All UDP is blocked (the residential SOCKS has no UDP ASSOCIATE), BitTorrent blocked,
#     private IP ranges blocked.
#
# Idempotent: safe to re-run. Generated secrets are persisted in $STATE_FILE and reused.
# Precedence for every secret: environment variable  >  $STATE_FILE  >  freshly generated.
#
# Usage (as root):
#   HOME_HOST=1.2.3.4 HOME_PORT=1080 HOME_USER=u HOME_PASS=p bash setup-vps.sh
#
# Re-using the current production identity (so existing clients keep working):
#   XRAY_UUID=... XRAY_PRIVATE_KEY=... XRAY_SHORT_ID=... SUB_TOKEN=... \
#   HOME_HOST=... HOME_PORT=... HOME_USER=... HOME_PASS=... bash setup-vps.sh
# (The private key is in /usr/local/etc/xray/config.json on the old server; the public key
#  is derived from it automatically.)
# =============================================================================
set -euo pipefail

# ----------------------------------------------------------------------------- variables
# REQUIRED — residential SOCKS5 proxy (placeholders on purpose; never hardcode real creds)
HOME_HOST="${HOME_HOST:-}"            # e.g. 203.0.113.10
HOME_PORT="${HOME_PORT:-}"            # e.g. 1080
HOME_USER="${HOME_USER:-}"            # SOCKS username
HOME_PASS="${HOME_PASS:-}"            # SOCKS password
# Expected public exit IP of the residential proxy (self-test). Defaults to HOME_HOST.
EXPECTED_EXIT_IP="${EXPECTED_EXIT_IP:-}"

# Optional — identity (generated if empty, then persisted in STATE_FILE)
XRAY_UUID="${XRAY_UUID:-}"
XRAY_PRIVATE_KEY="${XRAY_PRIVATE_KEY:-}"
XRAY_SHORT_ID="${XRAY_SHORT_ID:-}"

# Optional — node settings
SERVER_ADDR="${SERVER_ADDR:-}"                         # public IPv4/hostname for links (auto-detected)
XRAY_PORT="${XRAY_PORT:-443}"
REALITY_DEST="${REALITY_DEST:-addons.mozilla.org}"      # Reality dest + SNI
NODE_NAME="${NODE_NAME:-DMIT-LAX-Home}"
DIRECT_DOMAINS="${DIRECT_DOMAINS:-stripe.com stripe.network stripecdn.com stripe.dev}"  # space separated; "" = none
XRAY_VERSION="${XRAY_VERSION:-}"                       # e.g. v26.3.27 ; empty = latest stable
SSH_PORT="${SSH_PORT:-22}"                             # only opened in ufw; sshd port is NOT changed

# Optional — Clash subscription
ENABLE_SUB="${ENABLE_SUB:-1}"                          # 1 = serve clash.yaml over HTTP, 0 = disable
SUB_PORT="${SUB_PORT:-8443}"
SUB_TOKEN="${SUB_TOKEN:-}"                             # random 32-hex path if empty
SUB_DIR="${SUB_DIR:-/srv/sub}"

# Optional — behaviour
SKIP_UPGRADE="${SKIP_UPGRADE:-0}"                      # 1 = skip apt full-upgrade
STATE_FILE="${STATE_FILE:-/etc/xray-setup.env}"

XRAY_CONF=/usr/local/etc/xray/config.json
TEST_SOCKS_PORT=10808

# ----------------------------------------------------------------------------- helpers
log()  { printf '\033[1;32m[+]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[!]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[x]\033[0m %s\n' "$*" >&2; exit 1; }

# Back up a file once per run before overwriting it (keeps .bak-<timestamp>)
RUN_TS="$(date +%Y%m%d-%H%M%S)"
backup() { [[ -f "$1" ]] && cp -a "$1" "$1.bak-$RUN_TS" || true; }

# Write $2 (content) to $1 only if different; returns 0 if changed, 1 if unchanged
write_file() {
  local path="$1" content="$2" mode="${3:-644}"
  if [[ -f "$path" ]] && [[ "$(cat "$path")" == "$content" ]]; then return 1; fi
  backup "$path"
  mkdir -p "$(dirname "$path")"
  printf '%s\n' "$content" > "$path"
  chmod "$mode" "$path"
  return 0
}

# Wait for other apt/dpkg processes (cloud-init, unattended-upgrades) to release the lock
wait_dpkg() {
  local i=0
  while fuser /var/lib/dpkg/lock-frontend /var/lib/dpkg/lock /var/lib/apt/lists/lock >/dev/null 2>&1; do
    (( i++ == 0 )) && log "Waiting for dpkg/apt lock..."
    (( i > 120 )) && die "dpkg lock still held after 10 minutes"
    sleep 5
  done
}
apt_get() {
  wait_dpkg
  DEBIAN_FRONTEND=noninteractive apt-get -o DPkg::Lock::Timeout=600 \
    -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold -y -qq "$@"
}

# Retry a command N times with backoff (for GitHub / network downloads)
retry() {
  local n=0 max=5
  until "$@"; do
    n=$((n + 1)); (( n >= max )) && return 1
    warn "Command failed (attempt $n/$max), retrying in $((n * 5))s: $*"
    sleep $((n * 5))
  done
}

# ----------------------------------------------------------------------------- preflight
[[ $EUID -eq 0 ]] || die "Run as root."
# shellcheck disable=SC1091
[[ -r /etc/os-release ]] && . /etc/os-release
[[ "${ID:-}" == "debian" && ( "${VERSION_ID:-}" == "12" || "${VERSION_ID:-}" == "13" ) ]] \
  || warn "Tested on Debian 12/13 only (detected: ${PRETTY_NAME:-unknown}). Continuing."

for v in HOME_HOST HOME_PORT HOME_USER HOME_PASS; do
  [[ -n "${!v}" ]] || die "Required variable $v is not set. See header of this script."
done
[[ "$HOME_PORT" =~ ^[0-9]+$ ]] || die "HOME_PORT must be numeric"
EXPECTED_EXIT_IP="${EXPECTED_EXIT_IP:-$HOME_HOST}"

# Load persisted state (only fills variables that were not supplied via env)
if [[ -f "$STATE_FILE" ]]; then
  log "Loading saved identity from $STATE_FILE"
  while IFS='=' read -r k val; do
    [[ "$k" =~ ^(XRAY_UUID|XRAY_PRIVATE_KEY|XRAY_SHORT_ID|SUB_TOKEN)$ ]] || continue
    if [[ -z "${!k}" ]]; then printf -v "$k" '%s' "$val"; fi
  done < "$STATE_FILE"
fi

# ----------------------------------------------------------------------------- 1. system update + base packages
log "Updating system and installing base packages"
apt_get update
[[ "$SKIP_UPGRADE" == "1" ]] || apt_get full-upgrade
apt_get install ca-certificates curl wget unzip openssl python3 psmisc \
  ufw fail2ban python3-systemd nftables unattended-upgrades apt-listchanges \
  htop mtr-tiny vnstat iperf3

# iperf3 ships an optional daemon; keep it off (no extra open port)
systemctl disable --now iperf3 >/dev/null 2>&1 || true
systemctl enable --now vnstat >/dev/null 2>&1 || true
# Time sync (systemd-timesyncd is the Debian default)
if ! systemctl is-active --quiet systemd-timesyncd && ! systemctl is-active --quiet chrony; then
  apt_get install systemd-timesyncd || true
  systemctl enable --now systemd-timesyncd >/dev/null 2>&1 || true
fi

# ----------------------------------------------------------------------------- 2. sysctl / kernel tuning
log "Applying kernel network tuning (BBR + fq, buffers, conntrack)"
write_file /etc/sysctl.d/98-proxy-tuning.conf "$(cat <<'EOF'
# Proxy-node network tuning (setup-vps.sh). Rollback: rm this file && sysctl --system
# BBR + fq
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr
# Buffers sized for ~1-2 Gbps x ~200 ms trans-Pacific RTT (BDP ~25-50 MB)
net.core.rmem_max = 67108864
net.core.wmem_max = 67108864
net.core.rmem_default = 262144
net.core.wmem_default = 262144
net.ipv4.tcp_rmem = 4096 131072 67108864
net.ipv4.tcp_wmem = 4096 65536 67108864
net.ipv4.udp_rmem_min = 16384
net.ipv4.udp_wmem_min = 16384
# Keep unsent data in socket small -> lower latency with BBR
net.ipv4.tcp_notsent_lowat = 131072
net.ipv4.tcp_slow_start_after_idle = 0
net.ipv4.tcp_mtu_probing = 1
net.ipv4.tcp_fastopen = 3
# Queues / backlog
net.core.somaxconn = 8192
net.ipv4.tcp_max_syn_backlog = 8192
net.core.netdev_max_backlog = 16384
net.ipv4.tcp_syncookies = 1
# Ports, TIME_WAIT, keepalive
net.ipv4.ip_local_port_range = 10240 65000
net.ipv4.tcp_fin_timeout = 15
net.ipv4.tcp_max_tw_buckets = 262144
net.ipv4.tcp_keepalive_time = 300
net.ipv4.tcp_keepalive_intvl = 30
net.ipv4.tcp_keepalive_probes = 5
# Conntrack (ufw loads nf_conntrack)
net.netfilter.nf_conntrack_max = 262144
net.netfilter.nf_conntrack_tcp_timeout_established = 7200
net.netfilter.nf_conntrack_tcp_timeout_time_wait = 30
# Misc hardening
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv6.conf.all.accept_redirects = 0
net.ipv6.conf.default.accept_redirects = 0
net.ipv4.conf.all.send_redirects = 0
net.ipv4.icmp_ignore_bogus_error_responses = 1
EOF
)" || true
write_file /etc/modules-load.d/nf_conntrack.conf "nf_conntrack" || true
write_file /etc/modprobe.d/nf_conntrack.conf "options nf_conntrack hashsize=65536" || true
modprobe tcp_bbr 2>/dev/null || true
modprobe nf_conntrack 2>/dev/null || true
[[ -w /sys/module/nf_conntrack/parameters/hashsize ]] && echo 65536 > /sys/module/nf_conntrack/parameters/hashsize || true
sysctl --system >/dev/null 2>&1 || warn "sysctl --system reported errors (check: sysctl --system)"
[[ "$(sysctl -n net.ipv4.tcp_congestion_control)" == "bbr" ]] || warn "BBR is not active"

# ----------------------------------------------------------------------------- 3. SSH hardening (lock-out safe)
log "Hardening SSH (key-only)"
if [[ -s /root/.ssh/authorized_keys ]] || compgen -G "/home/*/.ssh/authorized_keys" >/dev/null; then
  if write_file /etc/ssh/sshd_config.d/01-hardening.conf "$(cat <<'EOF'
# setup-vps.sh. Rollback: rm this file; systemctl reload ssh
PermitRootLogin prohibit-password
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitEmptyPasswords no
AuthenticationMethods publickey
X11Forwarding no
MaxAuthTries 3
LoginGraceTime 30
MaxStartups 10:30:60
ClientAliveInterval 60
ClientAliveCountMax 3
EOF
)"; then
    if sshd -t; then
      systemctl reload ssh 2>/dev/null || systemctl reload sshd
      warn "SSH reloaded: TEST A NEW SSH SESSION BEFORE CLOSING THIS ONE."
    else
      warn "sshd -t failed; reverting SSH drop-in"
      rm -f /etc/ssh/sshd_config.d/01-hardening.conf
    fi
  fi
else
  warn "No authorized_keys found — SKIPPING key-only SSH hardening to avoid lock-out."
fi

# ----------------------------------------------------------------------------- 4. fail2ban
log "Configuring fail2ban (sshd jail)"
write_file /etc/fail2ban/jail.d/zz-local.conf "$(cat <<'EOF'
# setup-vps.sh. Rollback: rm this file; systemctl restart fail2ban
[DEFAULT]
ignoreip = 127.0.0.1/8 ::1
banaction = nftables
banaction_allports = nftables[type=allports]
bantime = 1h
bantime.increment = true
bantime.maxtime = 1w
findtime = 10m
maxretry = 5

[sshd]
enabled = true
mode = aggressive
backend = systemd
journalmatch = _SYSTEMD_UNIT=ssh.service + _COMM=sshd
EOF
)" || true
systemctl enable fail2ban >/dev/null 2>&1
systemctl restart fail2ban

# ----------------------------------------------------------------------------- 5. unattended-upgrades
log "Enabling unattended security upgrades (no automatic reboot)"
write_file /etc/apt/apt.conf.d/20auto-upgrades "$(cat <<'EOF'
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
APT::Periodic::AutocleanInterval "7";
EOF
)" || true
write_file /etc/apt/apt.conf.d/52unattended-upgrades-local "$(cat <<'EOF'
// setup-vps.sh: never reboot automatically
Unattended-Upgrade::Automatic-Reboot "false";
EOF
)" || true

# ----------------------------------------------------------------------------- 6. Xray install
# shellcheck disable=SC2329  # invoked indirectly via retry
install_xray() {
  local script rc=0
  script="$(mktemp)"
  curl -fsSL --retry 3 -o "$script" \
    https://github.com/XTLS/Xray-install/raw/main/install-release.sh || { rm -f "$script"; return 1; }
  if [[ -n "$XRAY_VERSION" ]]; then
    bash "$script" install --version "$XRAY_VERSION" || rc=$?
  else
    bash "$script" install || rc=$?
  fi
  rm -f "$script"
  return "$rc"
}
if ! command -v xray >/dev/null 2>&1 || [[ -n "$XRAY_VERSION" && "$(xray version | head -1)" != *"${XRAY_VERSION#v} "* ]]; then
  log "Installing Xray ${XRAY_VERSION:-latest stable}"
  retry install_xray || die "Xray install failed"
else
  log "Xray already installed: $(xray version | head -1)"
fi
# geoip.dat is needed for geoip:private
[[ -f /usr/local/share/xray/geoip.dat ]] || retry bash -c \
  "curl -fsSL https://github.com/XTLS/Xray-install/raw/main/install-release.sh | bash -s -- install-geodata" \
  || die "geodata download failed"

# nofile limit for xray (the official unit sets it too; this drop-in makes it explicit)
write_file /etc/systemd/system/xray.service.d/20-limits.conf "$(cat <<'EOF'
[Service]
LimitNOFILE=1000000
LimitNPROC=10000
EOF
)" && systemctl daemon-reload || true

# ----------------------------------------------------------------------------- 7. identity
[[ -n "$XRAY_UUID" ]] || XRAY_UUID="$(xray uuid)"
[[ -n "$XRAY_SHORT_ID" ]] || XRAY_SHORT_ID="$(openssl rand -hex 8)"
[[ "$XRAY_SHORT_ID" =~ ^[0-9a-f]{0,16}$ ]] || die "XRAY_SHORT_ID must be 0-16 lowercase hex chars"
# x25519: output format differs between versions ("Private key:"/"Public key:" vs "PrivateKey:"/"Password:")
x25519_field() { awk -F': *' -v k="$1" 'tolower($1) ~ k {print $2; exit}'; }
if [[ -z "$XRAY_PRIVATE_KEY" ]]; then
  KEYS="$(xray x25519)"
  XRAY_PRIVATE_KEY="$(x25519_field 'private' <<<"$KEYS")"
fi
KEYS="$(xray x25519 -i "$XRAY_PRIVATE_KEY")"
XRAY_PUBLIC_KEY="$(x25519_field 'public|password' <<<"$KEYS")"
[[ -n "$XRAY_PRIVATE_KEY" && -n "$XRAY_PUBLIC_KEY" ]] || die "Could not obtain Reality keypair"
[[ -n "$SUB_TOKEN" ]] || SUB_TOKEN="$(openssl rand -hex 16)"
[[ "$SUB_TOKEN" =~ ^[A-Za-z0-9_-]+$ ]] || die "SUB_TOKEN may only contain [A-Za-z0-9_-]"

# Persist (root-only) so re-runs keep the same identity
umask 077
install -m 600 /dev/null "$STATE_FILE"
printf 'XRAY_UUID=%s\nXRAY_PRIVATE_KEY=%s\nXRAY_SHORT_ID=%s\nSUB_TOKEN=%s\n' \
  "$XRAY_UUID" "$XRAY_PRIVATE_KEY" "$XRAY_SHORT_ID" "$SUB_TOKEN" > "$STATE_FILE"
umask 022

if [[ -z "$SERVER_ADDR" ]]; then
  SERVER_ADDR="$(retry curl -4 -fsS --max-time 10 https://api.ipify.org || true)"
  [[ -n "$SERVER_ADDR" ]] || SERVER_ADDR="$(ip -4 route get 1.1.1.1 | awk '{for(i=1;i<=NF;i++) if($i=="src") print $(i+1)}')"
fi

LINK_ADDR="$SERVER_ADDR"
[[ "$SERVER_ADDR" == *:* ]] && LINK_ADDR="[$SERVER_ADDR]"

# ----------------------------------------------------------------------------- 8. Xray config
log "Writing Xray config"
# Built with python3/json so credentials with special characters are escaped correctly.
NEW_CONF="$(mktemp --suffix=.json)"
XRAY_UUID="$XRAY_UUID" XRAY_PRIVATE_KEY="$XRAY_PRIVATE_KEY" XRAY_SHORT_ID="$XRAY_SHORT_ID" \
XRAY_PORT="$XRAY_PORT" REALITY_DEST="$REALITY_DEST" DIRECT_DOMAINS="$DIRECT_DOMAINS" \
HOME_HOST="$HOME_HOST" HOME_PORT="$HOME_PORT" HOME_USER="$HOME_USER" HOME_PASS="$HOME_PASS" \
python3 - "$NEW_CONF" <<'PY'
import json, os, sys
e = os.environ
keepalive = {"tcpKeepAliveIdle": 300, "tcpKeepAliveInterval": 30, "tcpcongestion": "bbr"}
rules = [
    # 1. never route to private / LAN ranges
    {"type": "field", "ip": ["geoip:private"], "outboundTag": "block"},
    # 2. protect the residential IP
    {"type": "field", "protocol": ["bittorrent"], "outboundTag": "block"},
]
direct = e["DIRECT_DOMAINS"].split()
if direct:
    # 3. the ONLY traffic allowed to leave from the VPS IP (e.g. Stripe)
    rules.append({"type": "field", "domain": ["domain:" + d for d in direct], "outboundTag": "direct"})
rules += [
    # 4. residential SOCKS has no UDP -> block all UDP so it fails fast and closed
    {"type": "field", "network": "udp", "outboundTag": "block"},
    # 5. everything else (incl. Claude/Anthropic/OpenAI) -> home. No fallback: fails closed.
    {"type": "field", "network": "tcp", "outboundTag": "home"},
]
conf = {
    "log": {"loglevel": "warning"},
    "dns": {"servers": ["https+local://1.1.1.1/dns-query"], "queryStrategy": "UseIPv4"},
    "inbounds": [{
        "port": int(e["XRAY_PORT"]),
        "protocol": "vless",
        "settings": {"clients": [{"id": e["XRAY_UUID"], "flow": "xtls-rprx-vision"}], "decryption": "none"},
        "streamSettings": {
            "network": "tcp",
            "security": "reality",
            "realitySettings": {
                "dest": e["REALITY_DEST"] + ":443",
                "serverNames": [e["REALITY_DEST"]],
                "privateKey": e["XRAY_PRIVATE_KEY"],
                "shortIds": [e["XRAY_SHORT_ID"]],
            },
            "sockopt": dict(tcpFastOpen=True, **keepalive),
        },
        "sniffing": {"enabled": True, "destOverride": ["http", "tls", "quic"]},
    }],
    "outbounds": [
        {"tag": "home", "protocol": "socks",
         "settings": {"servers": [{"address": e["HOME_HOST"], "port": int(e["HOME_PORT"]),
                                   "users": [{"user": e["HOME_USER"], "pass": e["HOME_PASS"]}]}]},
         "streamSettings": {"sockopt": dict(tcpFastOpen=False, **keepalive)}},
        {"tag": "block", "protocol": "blackhole"},
        {"tag": "direct", "protocol": "freedom", "settings": {"domainStrategy": "UseIPv4"}},
    ],
    "routing": {"domainStrategy": "AsIs", "rules": rules},
    "policy": {
        "levels": {"0": {"handshake": 4, "connIdle": 300, "uplinkOnly": 1, "downlinkOnly": 1, "bufferSize": 512}},
        "system": {"statsInboundUplink": False, "statsInboundDownlink": False},
    },
}
with open(sys.argv[1], "w") as f:
    json.dump(conf, f, indent=1)
PY
xray run -test -config "$NEW_CONF" >/dev/null || { rm -f "$NEW_CONF"; die "Generated Xray config failed validation"; }
if [[ -f "$XRAY_CONF" ]] && cmp -s "$NEW_CONF" "$XRAY_CONF"; then
  log "Xray config unchanged"
  rm -f "$NEW_CONF"
else
  backup "$XRAY_CONF"
  install -m 640 "$NEW_CONF" "$XRAY_CONF"; rm -f "$NEW_CONF"
fi
# 服务仍需读取配置；仅 root 与实际服务组可读，避免家宽凭据全局可读。
XRAY_SERVICE_USER="$(systemctl show xray.service -p User --value)"
XRAY_SERVICE_GROUP="$(systemctl show xray.service -p Group --value)"
[[ -n "$XRAY_SERVICE_GROUP" ]] || XRAY_SERVICE_GROUP="$(id -gn "${XRAY_SERVICE_USER:-root}")"
chown "root:${XRAY_SERVICE_GROUP}" "$XRAY_CONF"
chmod 640 "$XRAY_CONF"
systemctl enable xray >/dev/null 2>&1
systemctl restart xray

# ----------------------------------------------------------------------------- 9. firewall
log "Configuring ufw"
ufw default deny incoming >/dev/null
ufw default allow outgoing >/dev/null
ufw allow "${SSH_PORT}/tcp" >/dev/null     # always first, so we never lock ourselves out
ufw allow "${XRAY_PORT}/tcp" >/dev/null
if [[ "$ENABLE_SUB" == "1" ]]; then
  ufw allow "${SUB_PORT}/tcp" >/dev/null
else
  ufw delete allow "${SUB_PORT}/tcp" >/dev/null 2>&1 || true
fi
ufw --force enable >/dev/null

# ----------------------------------------------------------------------------- 10. Clash subscription (optional)
VLESS_LINK="vless://${XRAY_UUID}@${LINK_ADDR}:${XRAY_PORT}?encryption=none&flow=xtls-rprx-vision&security=reality&sni=${REALITY_DEST}&fp=chrome&pbk=${XRAY_PUBLIC_KEY}&sid=${XRAY_SHORT_ID}&type=tcp#${NODE_NAME}"
SUB_URL=""
if [[ "$ENABLE_SUB" == "1" ]]; then
  log "Generating Clash subscription"
  mkdir -p "$SUB_DIR/$SUB_TOKEN"
  # Domestic (China) sites that go DIRECT on the client side
  CN_DOMAINS="bilibili.com bilivideo.com hdslb.com biliapi.net douyin.com douyinvod.com douyinpic.com snssdk.com amemv.com bytedance.com iqiyi.com youku.com qq.com weixin.qq.com gtimg.com taobao.com tmall.com alicdn.com jd.com weibo.com sinaimg.cn xiaohongshu.com xhscdn.com zhihu.com zhimg.com baidu.com bdstatic.com 163.com 126.net music.163.com kuaishou.com meituan.com alipay.com"
  CN_DNS_POLICY="+.bilibili.com,+.bilivideo.com,+.hdslb.com,+.biliapi.net,+.douyin.com,+.douyinvod.com,+.douyinpic.com,+.snssdk.com,+.amemv.com,+.bytedance.com,+.iqiyi.com,+.youku.com,+.qq.com,+.gtimg.com,+.taobao.com,+.tmall.com,+.alicdn.com,+.jd.com,+.weibo.com,+.sinaimg.cn,+.xiaohongshu.com,+.xhscdn.com,+.zhihu.com,+.zhimg.com,+.baidu.com,+.bdstatic.com,+.163.com,+.126.net,+.kuaishou.com,+.meituan.com,+.alipay.com"
  AI_DOMAINS="anthropic.com claude.ai claude.com openai.com chatgpt.com oaistatic.com oaiusercontent.com"
  {
    cat <<EOF
mixed-port: 7890
mode: rule
ipv6: false
tun: {enable: true, stack: mixed, auto-route: true, auto-detect-interface: true, dns-hijack: [any:53]}
dns: {enable: true, ipv6: false, enhanced-mode: fake-ip, fake-ip-range: 198.18.0.1/16, respect-rules: true, default-nameserver: [223.5.5.5, 223.6.6.6], proxy-server-nameserver: [223.5.5.5, 223.6.6.6], nameserver: ["https://1.1.1.1/dns-query#PROXY", "https://8.8.8.8/dns-query#PROXY"], nameserver-policy: {"+.anthropic.com,+.claude.ai,+.claude.com,+.openai.com,+.chatgpt.com": ["https://1.1.1.1/dns-query#AI"], "${CN_DNS_POLICY}": ["https://223.5.5.5/dns-query", 223.6.6.6]}}
proxies:
  - {name: ${NODE_NAME}, type: vless, server: "${SERVER_ADDR}", port: ${XRAY_PORT}, uuid: ${XRAY_UUID}, network: tcp, udp: true, tls: true, flow: xtls-rprx-vision, servername: ${REALITY_DEST}, client-fingerprint: chrome, reality-opts: {public-key: ${XRAY_PUBLIC_KEY}, short-id: ${XRAY_SHORT_ID}}}
proxy-groups:
  # AI traffic: node or REJECT only - never DIRECT
  - {name: AI, type: select, proxies: [${NODE_NAME}, REJECT]}
  # Everything else: node or REJECT - no DIRECT option
  - {name: PROXY, type: select, proxies: [${NODE_NAME}, REJECT]}
rules:
  - IP-CIDR,223.5.5.5/32,DIRECT,no-resolve
  - IP-CIDR,223.6.6.6/32,DIRECT,no-resolve
EOF
    for d in $AI_DOMAINS; do echo "  - DOMAIN-SUFFIX,$d,AI"; done
    for d in $CN_DOMAINS; do echo "  - DOMAIN-SUFFIX,$d,DIRECT"; done
    echo "  - MATCH,PROXY"
  } > "$SUB_DIR/$SUB_TOKEN/clash.yaml.new"
  mv "$SUB_DIR/$SUB_TOKEN/clash.yaml.new" "$SUB_DIR/$SUB_TOKEN/clash.yaml"
  # Empty index.html files stop http.server from listing directories (hides the token path)
  : > "$SUB_DIR/index.html"; : > "$SUB_DIR/$SUB_TOKEN/index.html"
  chmod 755 "$SUB_DIR" "$SUB_DIR/$SUB_TOKEN"
  chmod 644 "$SUB_DIR/index.html" "$SUB_DIR/$SUB_TOKEN/index.html" "$SUB_DIR/$SUB_TOKEN/clash.yaml"

  write_file /etc/systemd/system/sub.service "$(cat <<EOF
[Unit]
Description=Clash subscription (static file server)
After=network.target
[Service]
WorkingDirectory=${SUB_DIR}
ExecStart=/usr/bin/python3 -m http.server ${SUB_PORT}
Restart=always
[Install]
WantedBy=multi-user.target
EOF
)" || true
  write_file /etc/systemd/system/sub.service.d/hardening.conf "$(cat <<'EOF'
# Run the subscription server unprivileged & sandboxed. Rollback: rm this file; systemctl daemon-reload; systemctl restart sub
[Service]
DynamicUser=yes
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_INET AF_INET6
CapabilityBoundingSet=
MemoryMax=128M
EOF
)" || true
  systemctl daemon-reload
  systemctl enable sub >/dev/null 2>&1
  systemctl restart sub
  SUB_URL="http://${LINK_ADDR}:${SUB_PORT}/${SUB_TOKEN}/clash.yaml"
else
  if systemctl list-unit-files sub.service >/dev/null 2>&1; then
    systemctl disable --now sub >/dev/null 2>&1 || true
  fi
fi

# ----------------------------------------------------------------------------- 11. self-test
log "Running self-test"
FAIL=0
check() { if "$@" >/dev/null 2>&1; then printf '  \033[32mPASS\033[0m %s\n' "$DESC"; else printf '  \033[31mFAIL\033[0m %s\n' "$DESC"; FAIL=1; fi; }

DESC="xray -test";               check xray run -test -config "$XRAY_CONF"
sleep 1
DESC="xray service active";      check systemctl is-active --quiet xray
DESC="fail2ban active";          check systemctl is-active --quiet fail2ban
DESC="BBR active";               check test "$(sysctl -n net.ipv4.tcp_congestion_control)" = bbr
if [[ "$ENABLE_SUB" == "1" ]]; then
  DESC="sub service active";     check systemctl is-active --quiet sub
  DESC="subscription served";    check curl -fsS -o /dev/null --max-time 5 "http://127.0.0.1:${SUB_PORT}/${SUB_TOKEN}/clash.yaml"
  DESC="directory listing hidden"; check test -z "$(curl -fsS --max-time 5 "http://127.0.0.1:${SUB_PORT}/")"
fi

# End-to-end: local VLESS client -> this server -> home proxy
CLIENT_CONF="$(mktemp --suffix=.json)"
cat > "$CLIENT_CONF" <<EOF
{"log":{"loglevel":"warning"},
 "inbounds":[{"port":${TEST_SOCKS_PORT},"listen":"127.0.0.1","protocol":"socks"}],
 "outbounds":[{"protocol":"vless","settings":{"vnext":[{"address":"127.0.0.1","port":${XRAY_PORT},"users":[{"id":"${XRAY_UUID}","flow":"xtls-rprx-vision","encryption":"none"}]}]},
  "streamSettings":{"network":"tcp","security":"reality","realitySettings":{"serverName":"${REALITY_DEST}","fingerprint":"chrome","publicKey":"${XRAY_PUBLIC_KEY}","shortId":"${XRAY_SHORT_ID}"}}}]}
EOF
xray run -config "$CLIENT_CONF" >/tmp/setup-vps-client.log 2>&1 &
CLIENT_PID=$!
cleanup() { kill "$CLIENT_PID" 2>/dev/null || true; wait "$CLIENT_PID" 2>/dev/null || true; rm -f "$CLIENT_CONF"; }
trap cleanup EXIT
sleep 2
EXIT_IP="$(curl -fsS --max-time 15 -x "socks5h://127.0.0.1:${TEST_SOCKS_PORT}" https://api.ipify.org || echo none)"
DESC="exit IP via node = ${EXPECTED_EXIT_IP} (got ${EXIT_IP})"; check test "$EXIT_IP" = "$EXPECTED_EXIT_IP"
if [[ " $DIRECT_DOMAINS " == *" stripe.com "* ]]; then
  CODE="$(curl -s -o /dev/null --max-time 15 -w '%{http_code}' -x "socks5h://127.0.0.1:${TEST_SOCKS_PORT}" https://dashboard.stripe.com || true)"
  DESC="Stripe via node (direct) HTTP ${CODE}"; check test "$CODE" = 303
fi
cleanup; trap - EXIT

# ----------------------------------------------------------------------------- 12. summary
cat <<EOF

================================================================================
 Setup complete $( [[ $FAIL -eq 0 ]] && echo "- all self-tests passed" || echo "- SOME SELF-TESTS FAILED (see above)")
--------------------------------------------------------------------------------
 Server            : ${SERVER_ADDR}:${XRAY_PORT}  (VLESS + Reality + Vision, SNI ${REALITY_DEST})
 Xray              : $(xray version | head -1 | awk '{print $1, $2}')
 UUID              : ${XRAY_UUID}
 Reality public key: ${XRAY_PUBLIC_KEY}
 Short ID          : ${XRAY_SHORT_ID}
 Egress            : all TCP -> SOCKS ${HOME_HOST}:${HOME_PORT}   (fails closed, no direct fallback)
 Direct domains    : ${DIRECT_DOMAINS:-none}
 Blocked           : private IPs, all UDP, BitTorrent
 Firewall (ufw)    : ${SSH_PORT}/tcp ${XRAY_PORT}/tcp$( [[ "$ENABLE_SUB" == "1" ]] && echo " ${SUB_PORT}/tcp")
 Secrets saved in  : ${STATE_FILE}  (root only; re-runs reuse them)

 VLESS link:
 ${VLESS_LINK}
EOF
if [[ -n "$SUB_URL" ]]; then
  printf '\n Clash subscription URL (plain HTTP - treat as a secret):\n %s\n' "$SUB_URL"
fi
echo "================================================================================"
if [[ "$FAIL" -eq 0 && -n "$SUB_URL" ]]; then printf "CC_SUB_URL=%s\n" "$SUB_URL"; fi
exit "$FAIL"
