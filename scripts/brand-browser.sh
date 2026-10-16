#!/bin/sh
# [INPUT]: 与本机 arm64/x86_64 匹配的 Browser.app、launcher/CCBrowser.c、icons/app.icns 与签名身份。
# [OUTPUT]: 品牌为「CCBaga-B」的引擎包：启动入口 CCBrowser、橙底奶白浏览器 B 图标、各语言显示名与固定宿主约定的 Bundle ID、清除构建资源扩展属性，嵌套程序启用 Hardened Runtime 与可信时间戳后签名。
# [POS]: package-browser.sh 的前置步骤；可重复执行，只改应用包元数据与入口，不触碰浏览器资料。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
app=${root}/browser-runtime/Browser.app
plist=${app}/Contents/Info.plist
identity=${CC_SIGN_IDENTITY:--}
name=CCBaga-B
arch=$(python3 "${root}/scripts/macos_arch.py")
python3 "${root}/scripts/macos_arch.py" --check "$app"

/usr/bin/clang -O2 -Wall -arch "$arch" -mmacosx-version-min=12.0 \
  -o "${app}/Contents/MacOS/CCBrowser" "${root}/browser-runtime/launcher/CCBrowser.c"
cp "${root}/browser-runtime/icons/app.icns" "${app}/Contents/Resources/app.icns"

/usr/bin/plutil -replace CFBundleIdentifier -string ai.local.requirement-french.browser "$plist"
/usr/bin/plutil -replace CFBundleExecutable -string CCBrowser "$plist"
/usr/bin/plutil -replace CFBundleName -string "$name" "$plist"
/usr/bin/plutil -replace CFBundleDisplayName -string "$name" "$plist"
/usr/bin/plutil -replace CFBundleIconFile -string app.icns "$plist"
/usr/bin/plutil -remove CFBundleIconName "$plist" 2>/dev/null || true
/usr/bin/plutil -replace LSHandlerRank -string Default "$plist"
/usr/bin/plutil -lint "$plist" >/dev/null

for strings in "${app}"/Contents/Resources/*.lproj/InfoPlist.strings; do
  tmp=${strings}.tmp
  /usr/bin/grep -v -E '^"?(CFBundleName|CFBundleDisplayName)"? = ' "$strings" > "$tmp" || true
  printf 'CFBundleName = "%s";\nCFBundleDisplayName = "%s";\n' "$name" "$name" >> "$tmp"
  mv "$tmp" "$strings"
done

/usr/bin/xattr -cr "$app" 2>/dev/null || true
# ----- 所有嵌套程序由单一签名入口按依赖顺序处理 -----
CC_SIGN_IDENTITY="$identity" python3 "${root}/scripts/sign-browser.py"
echo "已品牌化并签名 ${app}"
