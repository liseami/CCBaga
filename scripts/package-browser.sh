#!/bin/sh
# [INPUT]: 已迁入的 Chromium 引擎、brand-browser.sh 与 macOS ditto。
# [OUTPUT]: 品牌化为 CC浏览器 并签名后，保留链接和权限的 Browser.zip 及实际版本元数据。
# [POS]: 构建资源入口；避免 Tauri 逐文件复制丢失 Framework 目录链接。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
set -eu
cd "$(dirname "$0")/.."
sh scripts/brand-browser.sh
/usr/bin/codesign --verify --deep --strict browser-runtime/Browser.app
/usr/bin/plutil -extract CFBundleShortVersionString raw -o browser-runtime/version.txt browser-runtime/Browser.app/Contents/Info.plist
rm -f browser-runtime/Browser.zip
/usr/bin/ditto -c -k --keepParent browser-runtime/Browser.app browser-runtime/Browser.zip
