#!/bin/zsh
# [INPUT]: 宿主限定的 read/uninstall/reinstall 动作、当前登录终端 PATH 与真实用户目录 CC_CLI_USER_DIR。
# [OUTPUT]: Claude Code 安装状态标记与官方卸载/重装；仅删除 CLI 程序，保留配置、技能、会话和项目。
# [POS]: 宿主内嵌的本机包管理边界；兼容原生、Homebrew 和终端/nvm 全局 npm，多重安装先去重。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
set -eu
action=${1:-read}
case "$action" in read|uninstall|reinstall) ;; *) exit 2 ;; esac
root=${CC_CLI_USER_DIR:?}
native_dir="$root/.local/share/claude"
launcher="$root/.local/bin/claude"
package=@anthropic-ai/claude-code
fail() { print -u2 -r -- "$1"; exit 1; }
step() { print -r -- "CC_CLI_STEP=$1"; }
typeset -a npm_commands npm_roots casks methods
typeset -A seen_roots
npm_commands=() npm_roots=() casks=() methods=()
brew=$(whence -p brew || true)
native=0
if [[ -e "$native_dir" || -L "$native_dir" || -e "$launcher" || -L "$launcher" ]]; then
  # 原生删除只接受真实目录和指向其版本目录的启动链接；自定义启动器留给用户处理。
  [[ ! -L "$native_dir" && "${native_dir:A}" == "$native_dir" ]] || fail 'Claude Code 原生目录是外部链接，已停止操作。'
  if [[ -e "$launcher" || -L "$launcher" ]]; then
    [[ -L "$launcher" && "${launcher:A}" == "$native_dir"/versions/* ]] || fail 'Claude Code 启动器是自定义文件，已停止操作。'
    [[ "${launcher:h:A}" == "${launcher:h}" ]] || fail 'Claude Code 启动目录是外部链接，已停止操作。'
  fi
  native=1
  methods+=(native)
fi
if [[ -n "$brew" ]]; then
  list=$("$brew" list --cask 2>/dev/null) || fail '无法读取 Homebrew 安装状态。'
  for name in ${(f)list}; do
    if [[ "$name" == claude-code || "$name" == claude-code@latest ]]; then
      casks+=("$name")
      methods+=(Homebrew)
    fi
  done
fi
typeset -a bins
bins=("$path[@]" "$root"/.nvm/versions/node/*/bin(N))
for bin in "$bins[@]"; do
  [[ -x "$bin/npm" ]] || continue
  # 同一 Node 前缀的链接只检查一次；仅查询 npm 的全局根目录，不读取 npmrc 正文。
  npm_root=$(PATH="$bin:$PATH" "$bin/npm" root -g 2>/dev/null) || fail '无法读取 npm 全局安装目录。'
  [[ "$npm_root" == /* ]] || fail 'npm 返回了无效的全局安装目录。'
  [[ -z "${seen_roots[$npm_root]-}" ]] || continue
  seen_roots[$npm_root]=1
  [[ -f "$npm_root/$package/package.json" ]] || continue
  npm_commands+=("$bin/npm")
  npm_roots+=("$npm_root")
  methods+=(npm)
done
methods=("${(@u)methods}")
cli=$(whence -p claude || true)
[[ -n "$cli" ]] || { [[ -x "$launcher" ]] && cli="$launcher" || true; }
if [[ -n "$cli" && ${#methods} == 0 ]]; then
  fail '发现未识别的 Claude Code 安装，请先确认其安装方式。'
fi
version=''
if [[ "$action" == read && -n "$cli" ]]; then
  version=$(DISABLE_AUTOUPDATER=1 "$cli" --version 2>/dev/null) || version=''
fi
if [[ "$action" == read ]]; then
  print -r -- "CC_CLI_METHOD=${(j: / :)methods}"
  print -r -- "CC_CLI_PATH=$cli"
  print -r -- "CC_CLI_VERSION=$version"
  print -r -- 'CC_CLI_DONE=1'
  exit 0
fi
# 重装沿用当前已识别的包管理器；没有旧安装时使用官方原生安装器。
installer=''
if [[ "$action" == reinstall && ${#npm_commands} == 0 && ${#casks} == 0 ]]; then
  step '正在下载 Claude Code 官方安装器。'
  installer=$(/usr/bin/curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --max-time 60 https://claude.ai/install.sh 2>/dev/null) || fail 'Claude Code 官方安装器下载失败，请检查网络后重试。'
  [[ "$installer" == '#!'* && ${#installer} -lt 65536 ]] || fail 'Claude Code 官方安装器内容无效，已停止操作。'
fi
step '正在卸载 Claude Code CLI。'
for npm in "$npm_commands[@]"; do
  PATH="${npm:h}:$PATH" "$npm" uninstall -g "$package" >/dev/null 2>&1 || fail 'npm 卸载失败，请检查安装目录权限。'
done
for cask in "$casks[@]"; do
  HOMEBREW_NO_AUTO_UPDATE=1 "$brew" uninstall --cask "$cask" >/dev/null 2>&1 || fail 'Homebrew 卸载失败，请检查安装目录权限。'
done
if (( native )); then
  [[ ! -L "$native_dir" && "${native_dir:A}" == "$native_dir" ]] || fail 'Claude Code 原生目录已变化，已停止操作。'
  if [[ -e "$launcher" || -L "$launcher" ]]; then
    [[ -L "$launcher" && "${launcher:A}" == "$native_dir"/versions/* && "${launcher:h:A}" == "${launcher:h}" ]] || fail 'Claude Code 启动器是自定义文件，已停止操作。'
  fi
  [[ ! -e "$launcher" && ! -L "$launcher" ]] || /bin/rm -f "$launcher"
  [[ ! -d "$native_dir" ]] || /bin/rm -rf "$native_dir"
fi
for npm_root in "$npm_roots[@]"; do
  [[ ! -e "$npm_root/$package" ]] || fail 'npm 卸载后仍有 Claude Code 程序，未完成卸载。'
done
rehash
if [[ "$action" == uninstall ]]; then
  [[ -z "$(whence -p claude || true)" ]] || fail '仍有其他 Claude Code 安装，未完成卸载。'
  print -r -- 'CC_CLI_METHOD='
  print -r -- 'CC_CLI_PATH='
  print -r -- 'CC_CLI_VERSION='
  print -r -- 'CC_CLI_DONE=1'
  exit 0
fi
step '正在重装 Claude Code CLI。'
if (( ${#npm_commands} )); then
  npm=$npm_commands[1]
  PATH="${npm:h}:$PATH" "$npm" install -g "$package@latest" >/dev/null 2>&1 || fail 'npm 重装失败，请检查网络和安装目录权限。'
  cli="${npm:h}/claude"
  [[ -x "$cli" ]] || cli=$(whence -p claude || true)
  method=npm
elif (( ${#casks} )); then
  HOMEBREW_NO_AUTO_UPDATE=1 "$brew" install --cask "$casks[1]" >/dev/null 2>&1 || fail 'Homebrew 重装失败，请检查网络和安装目录权限。'
  rehash
  cli=$(whence -p claude || true)
  method=Homebrew
else
  print -r -- "$installer" | /bin/bash -s -- latest >/dev/null 2>&1 || fail 'Claude Code 原生重装失败，请检查网络和安装目录权限。'
  cli="$launcher"
  method=native
fi
step '正在核验 Claude Code CLI 版本。'
[[ -n "$cli" && -x "$cli" ]] || fail 'Claude Code 重装后未找到可执行程序。'
version=$(DISABLE_AUTOUPDATER=1 "$cli" --version 2>/dev/null) || fail 'Claude Code 重装后的版本核验失败。'
[[ "$version" == *'Claude Code'* ]] || fail 'Claude Code 重装后的版本信息无效。'
print -r -- "CC_CLI_METHOD=$method"
print -r -- "CC_CLI_PATH=$cli"
print -r -- "CC_CLI_VERSION=$version"
print -r -- 'CC_CLI_DONE=1'
