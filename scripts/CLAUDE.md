# scripts/
> L2 | 父级: ../CLAUDE.md
make-icon.swift: AppKit 将压缩品牌图绘入 macOS 圆角网格，保持原图并添加透明留白与投影。
make-icons.sh: 从 assets/ 品牌图生成各尺寸 iconset，iconutil 输出 src-tauri/icons 与 browser-runtime/icons。
brand-browser.sh: 校验全部引擎 Mach-O 与本机 arm64/x86_64 匹配，再按相同架构编译 CCBrowser；品牌、图标、Bundle ID 和签名流程一致，不改变启动行为。
macos_arch.py: 共同的 arm64/x86_64 命名、updater 平台键和全包 Mach-O/内部链接校验；供引擎、启动器、开发安装与发布使用。
test-macos-packaging.py: 离线 Intel 导入验证探针前置、摘要/嵌套架构/路径链接拒绝和两个 updater 目标；macOS 另以真实 ditto 验证构造包的 Framework 链接/Finder 元数据往返，不执行真实浏览器或操作用户应用。
sign-browser.py: 默认 Browser.app 或显式 --app 候选的内层 Mach-O → Helper 应用 → Framework → 浏览器外壳顺序签名；Developer ID 使用可信时间戳、Hardened Runtime，仅 Renderer/GPU 使用 JIT entitlement；本机临时签名清除 runtime，防止缺 Team ID 的 Framework 被库验证拒绝。
build-macos-intel-engine.py: 固定 148 指纹源码与同版 macOS builder，SDK 最低 15，使用已有 Xcode；独立目录准备/可恢复 Ninja 编译，可分离下载缓存，命令运行时保护 10 GiB 可用空间；幂等应用 SDK 与 headless 退出兼容补丁，真实架构与指纹验收后输出候选/来源/补丁摘要，不改用户应用或发布。
macos-sdk15-fileport.patch: 仅在 SDK 不提供 sys/fileport.h 时，为 Chromium 两处 Mach 文件描述符传输声明系统已存在的接口；不修改 SDK、传输实现或功能。
macos-sdk15-coregraphics.patch: Skia/Blink 的 macOS 位图上下文使用旧 SDK 已提供的等值 host 字节序常量，保留 BGRA 像素格式；不修改布局或绘制流程。
macos-sdk15-appkit.patch: SDK 低于 26 时仅声明 NSScreen 的 CGDirectDisplayID getter，保留原 macOS 26 availability 判断与旧系统回退；不替换显示器查找实现。
macos-sdk15-pasteboard.patch: SDK 低于 15.4 时集中补充剪贴板访问枚举/getter 声明，三个原生调用点共用；保留原系统版本判断与询问/允许/拒绝策略。
macos-headless-shutdown.patch: headless 命令完成后先解除 macOS 常驻标记，再检查可能已关闭的浏览器窗口；修复页面已输出却不退出，不改变有窗口模式、页面或指纹参数。
verify-macos-intel-engine.py: Intel 原生候选的全部 Mach-O/内部链接、实际版本、两个 CPU/语言/时区观测与四次 Canvas 像素/PNG/文字/几何比较；相同页面重复稳定、换种子生效、原生画面开关禁用后不随种子变化；仅临时无账号软件 headless，不替代真实 GPU/完整 UI/资料隔离/签名公证验收。
test-macos-intel-engine.py: 三平台离线样本回归拒绝 arm64 Helper 混装、外部链接、普通浏览器/部分图形开关冒充、错误/中断源码目录与凭据继承；验证图形观测完整/稳定与原生画面不随种子变化；有 patch 命令时以真实临时双文件补丁核对幂等/摘要、半套与无关源码拒绝且不改文件；显式候选签名不得触碰默认引擎。
test-release-env.py: 伪造凭据环境导入真实发布入口，断言 Apple/更新私钥不进入构建子进程环境，不加载真实密钥，并验证票据网络重试不绕过无效/持续失败。
test-sign-browser.py: 临时伪引擎树验证叶子到容器的签名顺序、最小 JIT 授权与符号链接不重复遍历。
windows-region.ps1: 当前用户 International 首选语言/显示语言偏好与区域格式设置；PowerShell 5.1 泛型列表保留其他语言/输入法、复核持久化，当前会话未生效时明确报告需重新登录。
claude-cli.ps1: 包管理器仅按 PATH 外部程序发现，避免精确命令触发模块扫描；Windows 唯一目录暂存/逐文件哈希回滚与失败保留恢复文件；Windows 官方 CLI 程序管理与真实完成协议；原生重装先下载、严格 UTF-8 解码字节响应并解析官方安装器，再卸载旧程序；下载/内容异常保留旧程序，安装失败使用固定脱敏文案，保留配置和项目，不执行 VPS 配置。
package-browser.ps1: Windows 固定引擎前置核验，不打包用户资料或私钥。
test-skill-install.ts: Agent Skills metadata/完整公开文件与 npm skills 1.7.2 的四项真实安装；临时 HOME 和缓存，关闭统计，验证共享目录、完整文件与无 symlink。
test-network-diagnostics.ts: 共用隔离引擎验证真实网络页的 Claude 出口四态、逐域 403 原因、命令异常/空返回拒绝、重测清旧结果和双语；不访问真实网络。
test-browser-verification.ts: 顶部身份卡顺序、双语与不匹配/空身份/未联动 DOM 回归； 复用隔离 Playwright 引擎，真实 browser.html 的三态背景/波纹、原因、图片实际解码、紧凑布局、重测和双语 DOM 回归；虚构本机 HTTP 回报，无截图/真实账号。
test-identities-ui.ts: 默认勾选、关闭确认/取消与恢复联动； Node.js 标准管道 + 固定引擎 + playwright-core 的隔离 DOM 模拟；40 个虚构账号验证当前槽位去重/无最近使用卡片、竖式工牌、弹簧/视差、Alert 安全默认、双语、保留选择与切换/删除/失败/恢复，临时 HOME/XDG，不调用真实 IPC、不截图或外连。
test-claude-residue.ts: 复用身份页内存 IPC 验证显式扫描、仅精确缓存默认选择、不完整项禁选、JSON 键提示、不可逆确认默认取消、精确 ID 提交、部分失败和停止失败后的重扫读回、旧计划失效与英文确认；不触碰本机文件或钥匙串。
test-identity-operation.ts: 通过延期 IPC 和官方 Channel 回调验证工牌下方共享进度；未知总量不制造百分比，实际 1/4 事件显示且停留 25%，空身份/暂存身份/清理复查走同一进度，失败与回滚保留真实状态；不截图或触碰本机资料。
test-windows.py: 子进程大小写无关地清除继承 PSModulePath，90 秒冷启动预算/逐项日志保留安全断言；Windows 原生/npm/WinGet 程序隔离样本的卸载重装、PATH外全局npm启动器识别；离线原生安装器字节/文本响应、安装/版本失败恢复哈希、junction 阻断回滚保留备份、下载/解析失败保留旧程序、安装失败不报告成功与自定义启动器拒绝；CI 实际语言/区域写入、输入法保留和完整还原。
fetch-browser.py: 保留固定摘要 arm64 DMG；Intel 接受明确 SHA-256 的源码构建 ZIP/HTTPS 输入，拒绝越界链接、混装架构和失败的真实指纹探针；完整复制后才公开 Browser.app，保留既有三平台失败清理边界。
test-fetch-browser.py: 三平台真实临时文件树与显式下载/摘要/平台工具替身，回归复制失败后可重试、完整资源/权限/链接、既有引擎不变与清理失败路径；不下载或运行真实引擎。
release-macos.py: arm64/x86_64 共用三阶段公证、凭据隔离和最终更新签名；校验 A/B 全部 Mach-O，生成各架构 DMG/更新归档/内部清单与摘要，分别验证对应 updater 目标；公证网络重试和正式信任门槛不变。
dmg-settings.py: 固定 800×500 像素背景、两应用到 Applications 的拖拽坐标；通知文件保留在盘内但默认隐藏，不依赖 Finder 自动化。
verify-dmg.py: 只读实际 DMG，核对双应用名称/标识/入口、目标架构、UI 实际引用、签名/Gatekeeper、Applications 链接和像素背景；不接受错误架构。
release-requirements.txt: dmgbuild、ds-store 与 mac-alias 的完整版本及 wheel SHA-256；仅发布阶段安装，不增大应用包。
package-browser.sh: 先品牌化签名，再打包引擎，ditto 保留 Framework 链接与可执行权限；归档进入应用资源；Linux 将源引擎暂存后完整替换 CARGO_TARGET_DIR 的 release 资源树中的 LinuxBrowser，保持源码/Arch 安装契约，系统包另走 libexec。
dev-install.py: macOS arm64/x86_64 共用 debug/custom-protocol 构建和临时签名双应用事务安装，构建前检查引擎架构；不读取公证凭据或发布。
test-install-apps.py: 两个应用在移动前后取消及取消后恢复失败的真实目录样本；临时双应用事务成功、正常回滚与双重失败恢复目录回归；安装模式信任边界与环境隔离的离线回归，不操作真实应用或密钥。
install-apps.sh: bun install:apps 兼容入口，只转交统一安装层。
install-apps.py: 替换前复用 browser_ui 的实际引用预检；根据磁盘状态恢复移动前后取消的双应用，不依赖移动后内存记账；回滚失败保留并报告恢复目录；正式包严格签名/Gatekeeper；--development 仅本机 debug/临时签名，拒绝外部 DMG；验收后的事务替换、同标识旧名迁移与失败回滚；保留隔离和资料，正常退出后安装、只刷新相关 Dock 项并启动宿主。
claude-cli.sh: 唯一私有目录移动暂存原生程序/链接，原本无启动链接也可捕获失败恢复，失败保留备份；不接受环境安装器注入。原生宿主内嵌的只读安装检查与官方 CLI 卸载/重装，使用登录终端 PATH，发现 nvm/全局 npm、Homebrew 双通道与原生版本；只卸载程序，保护自定义启动器与配置资产，重装沿用已识别包管理器并核验版本。
browser_ui.py: HTMLParser 验证浏览器入口实际引用的脚本、预加载模块和样式，限定本地资源边界；Mac/Linux 安装预检、三平台正式载荷与构建 CI 共用函数/只读 CLI，不执行网页。
install-linux.py: 共用实际 UI 引用预检；目录移动前后取消同样回滚，双重失败保留恢复目录；Linux 用户前缀安装并保留资料；替换前拒绝不可执行的浏览器引擎、不完整浏览器页面/脚本、自定义启动器、链接父目录与整个安装目录下仍运行的 A/B，失败回滚，双重失败保留恢复目录。
test-linux.py: glibc 符号上限/非ELF跳过及浏览器入口/共享模块/样式缺失与外部链接拒绝、四种移动取消及取消后恢复失败保留备份；临时构建验证安装隔离、B 运行保护、不可执行引擎/缺失页面或脚本保留旧安装、桌面写入回滚、Exec 引用与载荷缺失/引擎篡改拒绝；双重失败保留恢复目录；不访问用户资料。

assemble-release.py: 验证两种 macOS/Windows/Linux 当前版本必需产物摘要后合并三个 updater 目标，以十一文件白名单交付；Intel/ARM 更新 URL 和签名不可混用，内部签名/来源记录只留 CI。
test-assemble-release.py: 临时完整产物验证十一文件白名单、三目标更新合并；遗漏任一 Intel/其他平台摘要或把 Intel 指向 ARM 归档时，在写公开清单前拒绝，不发布或下载。
sign-windows-update.py: 已验收安装器的独立版本绑定更新签名和官方更新器核验；更新私钥不进入构建或测试。
release-windows.py: Windows x64 MSVC 无凭据构建，check/build/verify 阶段在 CI 分开呈现，原生/40 身份 UI 模拟/CLI/Skill/npm 安装回归与真实 NSIS 安装、启动、升级、卸载验证，交付单安装器与内部引擎哈希清单。
verify-windows-installer.py: 仅在空白 CI runner 对刚创建的样本复用 Skill 私有 ACL 层，按子项到父目录顺序明确当前用户所有者与独占 ACL，避免继承 ACE 迁移中断及管理员默认所有者误触生产保护；UTF-8 控制台诊断保留中文启动错误；复用 browser_ui 核对已安装页面的实际资源引用；使用原生 Known Folder 和一致的父子进程环境定位空白 CI 数据；拒绝预存 Claude/应用目录再创建可丢弃用户资产样本，通过已持有句柄的宿主树/本次唯一安装目录回收 CI 程序，不给 GUI 沙箱套额外 Job，PowerShell 5.1 优先自身模块以保证 CIM 进程查询，查询错误即时报告、统一 Windows 8.3 别名/扩展路径/分隔符，只记录匹配布尔值；严格核对安装后引擎/图标/UI、实际浏览器 JS 回报的平台/CPU/版本与 A/B 回环、升级和卸载资产保留；验收后的 PermissionError 仅在本次固定 Chromium Cache 子树允许交由一次性 runner 销毁，其他路径/错误仍失败。
release-linux.py: 无凭据的 Ubuntu 22.04 x86_64 三格式构建，先跑身份 UI/Skill 回归；Tauri 交付 DEB/AppImage，rpmbuild 从同一文件树生成 RPM，厂商引擎不剥离且隔离在 libexec；真实解包核对宿主、UI、B 入口、引擎字节及全部 ELF 的 glibc 2.35 上限，生成校验清单。
test-rpm.py: 临时 DEB 文件树经真实 rpmbuild 转换，回归 RPM 版本/架构、桌面入口与引擎字节，不执行样本程序。
verify-linux-package.py: 共用页面引用与只读载荷验收，Ubuntu/Arch 均拒绝缺失 UI、入口或引擎改写；独立 verify_glibc 对标准发行包实际 ELF 符号需求执行 2.35 上限，Arch 不套旧系统基线。
build-arch.sh: 固定摘要容器准备 Node.js 等构建/测试依赖，以非 root makepkg、原生回归与解包验收，公开源码与 Cargo 缓存挂载；不接触 macOS 凭据。

package-linux-browser.py: 旧目录移动与新目录提交共处取消/回滚边界；Linux 源码/Arch 的所属引擎目录完整替换；暂存和提交失败保留旧构建，回滚失败保留备份并报告恢复路径，保持符号链接/权限和其他资源，不混入已删除的旧引擎文件。
test-package-linux-browser.py: 四种目录移动取消恢复及取消后恢复失败保留备份；真实打包入口的首次/重建、版本/执行权限前置检查、复制/提交回滚及双重失败备份保留回归；临时样本不安装引擎，非 Linux 仅替换 uname 选择 Linux 分支。
test-browser-cleanup.ts: 复用身份测试入口的 browser 模式，验证无缓存全清理、默认取消/强制勾选、精确删除参数、任务锁/真实 Channel 进度、失败与双语；仅内存 IPC，无原生或真实用户资料。
test-browser-profiles.ts: 固定引擎的真实持久资料隔离回归；虚构 Cookie/LocalStorage 的 A/B 隔离、重开恢复和空资料，仅临时 HOME/本机页面。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
