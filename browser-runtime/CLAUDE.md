# browser-runtime/
> L2 | 父级: ../CLAUDE.md
Browser.app/: 从已安装 CCBroswer 迁入的 Fingerprint Chromium 148 引擎与许可证；由 scripts/brand-browser.sh 品牌化为「CC浏览器」：CFBundleExecutable 为 CCBrowser 入口，各 lproj 显示名、app.icns（橙底奶白浏览器 B）、LSHandlerRank Default。本机用与宿主相同的 Developer ID 签名，启用 Hardened Runtime、可信时间戳，嵌套程序沿用同一团队，仅 Renderer/GPU Helper 允许 JIT。安装为 /Applications/CC浏览器.app 后进入默认浏览器列表，宿主优先使用这份。
launcher/CCBrowser.c: 应用包入口；宿主未运行时后台打开 CC清理器，再 exec Contents/MacOS/Chromium，未指定资料时注入宿主写入的 launch.args（缺失时仅指定同一 profile），并把宿主写入的 start.url（回环端口可连才用；宿主新启动时最多等 8 秒）作为首个标签；网址参数排在其后，Apple Event 链接照常打开。
icons/: make-icons.sh 生成的 CC浏览器 图标 icon.png 与 app.icns。
Browser.zip: ditto 生成的引擎归档，保留 Framework 链接；应用首次打开浏览器时安装到独立 engine 目录。
LICENSE.fingerprint-chromium: 指纹引擎上游 BSD-3-Clause 许可证，随工程和应用包保留归属。
entitlements.plist: 只向 Renderer/GPU Helper 开放 MAP_JIT，对齐 Chromium V8/SwiftShader 需求；不启用调试、任意库加载或未签名可执行内存。
version.txt: 从实际 Info.plist 生成的引擎版本。
MIGRATION.md: 旧工程与入口移除、唯一资料迁移位置和实际核验边界。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
