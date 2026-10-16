# browser-runtime/
> L2 | 父级: ../CLAUDE.md
macOS 架构：Browser.app 与本机 arm64/x86_64 一致；arm64 保留固定摘要的上游 DMG，Intel 使用固定源码构建并经真实指纹验收的 ZIP。品牌化、启动参数、资料隔离与 UI 不分叉；共享 macos_arch.py 校验所有嵌套 Mach-O。
Browser.app/: 固定摘要下载的 Fingerprint Chromium 148 引擎与许可证；由 scripts/brand-browser.sh 品牌化为「CCBaga-B」：CFBundleIdentifier 显式设为 ai.local.requirement-french.browser，与宿主进程/默认浏览器识别一致；CFBundleExecutable 为 CCBrowser 入口，各 lproj 显示名、app.icns（橙底奶白浏览器 B）、LSHandlerRank Default。正式版本用与宿主相同的 Developer ID、Hardened Runtime 与可信时间戳，嵌套程序同团队；本机开发临时签名不启用 runtime，以允许无 Team ID 的 Framework 加载，仅 Renderer/GPU Helper 允许 JIT。安装为 /Applications/CCBaga-B.app 后进入默认浏览器列表，宿主优先使用这份。
launcher/CLAUDE.md: macOS 入口的局部地图。
launcher/CCBrowser.c: 应用包入口；宿主未运行时后台打开 CCBaga-A，再 exec Contents/MacOS/Chromium，未指定资料时注入宿主写入的 launch.args（身份切换时阻断，缺失时拒绝启动），并把宿主写入的 start.url（回环端口可连才用；宿主新启动时最多等 8 秒）作为首个标签；网址参数排在其后，Apple Event 链接照常打开。
icons/: make-icons.sh 生成的 CCBaga-B 图标 icon.png 与 app.icns。
Browser.zip: ditto 生成的引擎归档，保留 Framework 链接；应用首次打开浏览器时安装到独立 engine 目录。
LICENSE.fingerprint-chromium: 指纹引擎上游 BSD-3-Clause 许可证，随工程和应用包保留归属。
entitlements.plist: 只向 Renderer/GPU Helper 开放 MAP_JIT，对齐 Chromium V8/SwiftShader 需求；不启用调试、任意库加载或未签名可执行内存。
version.txt: 从实际 Info.plist 生成的引擎版本。
LinuxBrowser/: 固定上游 tar.xz，经 SHA-256、ELF x86_64 与实际版本核验；被 Git 忽略；系统安装包走 libexec，源码/Arch 由 package-browser.sh 准备构建资源树。ccbaga-browser.desktop: Linux 包的 B 启动项，通过 requirement-french --browser 启动宿主和同一独立资料，不修改默认浏览器。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
