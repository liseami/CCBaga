#!/bin/sh
# [INPUT]: 已打包的 CC清理器.app，以及品牌化后的 browser-runtime/Browser.app。
# [OUTPUT]: /Applications 中的 CC清理器 与 CC浏览器 被整包替换、清除隔离/来源标记并重新注册；~/Applications 下的 CC*.app 与旧版需求法语*.app 退出、注销并删除；应用数据中的旧引擎副本只注销不删除。
# [POS]: 每次迭代的安装入口；只替换这两个应用，不清理资料，不改系统默认浏览器。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
dest=/Applications
legacy=${HOME}/Applications
host_name=CC清理器.app
browser_name=CC浏览器.app
host_src=${root}/src-tauri/target/release/bundle/macos/${host_name}
browser_src=${root}/browser-runtime/Browser.app
engine_copy="${HOME}/Library/Application Support/ai.local.requirement-french/browser/engine/Browser.app"
lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister

[ -w "$dest" ] || { echo "${dest} 不可写，需要管理员权限。" >&2; exit 1; }
[ -d "$host_src" ] || { echo "缺少 ${host_src}。先执行 bun run tauri build --bundles app" >&2; exit 1; }
[ -d "$browser_src" ] || { echo "缺少 ${browser_src}" >&2; exit 1; }
mkdir -p "$dest"

quit_copy() {
  app=$1
  [ -d "$app" ] || return 0
  pids=$(/usr/sbin/lsof -t +D "$app" 2>/dev/null || true)
  [ -n "$pids" ] || return 0
  # shellcheck disable=SC2086
  kill $pids 2>/dev/null || true
  sleep 1
  # shellcheck disable=SC2086
  kill -9 $pids 2>/dev/null || true
}

clean_marks() {
  /usr/bin/xattr -dr com.apple.quarantine "$1" 2>/dev/null || true
  /usr/bin/xattr -dr com.apple.provenance "$1" 2>/dev/null || true
}

install_app() {
  src=$1
  name=$2
  target=${dest}/${name}
  quit_copy "$target"
  "$lsregister" -u "$target" 2>/dev/null || true
  rm -rf "$target"
  /usr/bin/ditto "$src" "$target"
  clean_marks "$target"
  /usr/bin/codesign --verify --deep --strict "$target"
  "$lsregister" -f "$target"
  /usr/bin/touch "$target"
  echo "已安装 ${target}"
}

remove_old() {
  old=$1
  [ -d "$old" ] || return 0
  quit_copy "$old"
  "$lsregister" -u "$old" 2>/dev/null || true
  rm -rf "$old"
  echo "已移除旧版 ${old}"
}

install_app "$host_src" "$host_name"
install_app "$browser_src" "$browser_name"
for old in "${legacy}/${host_name}" "${legacy}/${browser_name}" "${legacy}/需求法语.app" "${legacy}/需求法语浏览器.app" \
  "${dest}/需求法语.app" "${dest}/需求法语浏览器.app"; do
  remove_old "$old"
done
# 同一包标识只保留 ~/Applications/CC浏览器.app 一份注册；旧引擎副本保留在磁盘（宿主回退用），不删除资料。
[ -d "$engine_copy" ] && "$lsregister" -u "$engine_copy" 2>/dev/null || true
# 旧构建产物与改名前工程路径的注册一并注销（路径不存在时忽略）。
for stale in "${root}/src-tauri/target/release/bundle/macos/需求法语.app" \
  "${HOME}/Cromma/需求法语/src-tauri/target/release/bundle/macos/需求法语.app" \
  "${HOME}/Cromma/需求法语/browser-runtime/Browser.app" \
  "$browser_src" "${root}/src-tauri/target/release/bundle/macos/${host_name}"; do
  "$lsregister" -u "$stale" 2>/dev/null || true
done
# 刷新 Finder/Dock 图标缓存。
/usr/bin/killall Dock 2>/dev/null || true
/usr/bin/killall Finder 2>/dev/null || true
