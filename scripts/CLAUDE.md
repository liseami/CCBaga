# scripts/
> L2 | 父级: ../CLAUDE.md
make-icon.swift: AppKit 将压缩品牌图绘入 macOS 圆角网格，保持原图并添加透明留白与投影。
make-icons.sh: 从 assets/ 品牌图生成各尺寸 iconset，iconutil 输出 src-tauri/icons 与 browser-runtime/icons。
brand-browser.sh: 编译 CCBrowser 入口、写入 CCBaga-B 名称/图标/各语言显示名、清除构建资源扩展属性并调用 sign-browser.py 完整签名（显式固定 ai.local.requirement-french.browser，干净上游引擎也必须与宿主一致）。
sign-browser.py: 内层 Mach-O → Helper 应用 → Framework → 浏览器外壳顺序签名；Developer ID 使用可信时间戳、Hardened Runtime，仅 Renderer/GPU 使用 JIT entitlement；本机临时签名清除 runtime，防止缺 Team ID 的 Framework 被库验证拒绝。
test-release-env.py: 伪造凭据环境导入真实发布入口，断言 Apple/更新私钥不进入构建子进程环境，不加载真实密钥。
test-sign-browser.py: 临时伪引擎树验证叶子到容器的签名顺序、最小 JIT 授权与符号链接不重复遍历。
windows-region.ps1: 当前用户 International 首选语言/显示语言偏好与区域格式设置；PowerShell 5.1 泛型列表保留其他语言/输入法、复核持久化，当前会话未生效时明确报告需重新登录。
claude-cli.ps1: Windows 官方 CLI 程序管理与真实完成协议；原生重装先下载、严格 UTF-8 解码字节响应并解析官方安装器，再卸载旧程序；下载/内容异常保留旧程序，安装失败使用固定脱敏文案，保留配置和项目，不执行 VPS 配置。
package-browser.ps1: Windows 固定引擎前置核验，不打包用户资料或私钥。
test-skill-install.ts: Agent Skills metadata/完整公开文件与 npm skills 1.7.2 的四项真实安装；临时 HOME 和缓存，关闭统计，验证共享目录、完整文件与无 symlink。
test-identities-ui.ts: 固定引擎 + playwright-core 的隔离 DOM 模拟；40 个虚构账号验证竖式工牌、弹簧/视差、Alert 安全默认、双语、保留选择与切换/删除/失败/恢复，不调用真实 IPC、不截图或外连。
test-windows.py: Windows 原生/npm/WinGet 程序隔离样本的卸载重装；离线原生安装器字节/文本响应、下载/解析失败保留旧程序、安装失败不报告成功与自定义启动器拒绝；CI 实际语言/区域写入、输入法保留和完整还原。
fetch-browser.py: 固定上游引擎 DMG 与 SHA-256，核验版本和 arm64 后恢复引擎；干净 CI 检出不依赖维护者本机 Browser.app。
release-macos.py: 外部凭据与可选 CC_RELEASE_VERSION 驱动三阶段公证装订；明确将原生构建目录 Browser.app 暂存为交付名 CCBaga-B.app，再由 dmgbuild 无头写入 Finder 布局；装订后的宿主单独归档并签名绑定版本，更新私钥仅传给签名子进程；Apple 凭据不进入构建、DMG 或更新签名子进程，公证工具只通过明确参数接收其认证输入；官方更新器回归核验后生成公开 latest.json；产物和哈希记录写入被忽略的 releases/。
dmg-settings.py: 固定 800×500 像素背景、两应用到 Applications 的拖拽坐标；通知文件保留在盘内但默认隐藏，不依赖 Finder 自动化。
verify-dmg.py: 只读挂载实际安装盘，校验双应用名称、包标识与启动入口，再校验严格签名与 Gatekeeper、Applications 链接和像素背景；Finder 元数据不得破坏应用签名。
release-requirements.txt: dmgbuild、ds-store 与 mac-alias 的完整版本及 wheel SHA-256；仅发布阶段安装，不增大应用包。
package-browser.sh: 先品牌化签名，再打包引擎，ditto 保留 Framework 链接与可执行权限；归档进入应用资源；Linux 将源引擎复制到 CARGO_TARGET_DIR 的 release 资源树，保持源码/Arch 安装契约，系统包另走 libexec。
dev-install.py: 本机 Apple Silicon 快速 debug/custom-protocol 构建、临时签名与双应用重装，安装前验证 Chromium 实际 CLI 可加载；过滤 Apple/更新私钥环境；不公证、不生成 DMG、不发布。
test-install-apps.py: 安装模式信任边界与环境隔离的离线回归，不操作真实应用或密钥。
install-apps.sh: bun install:apps 兼容入口，只转交统一安装层。
install-apps.py: 正式包严格签名/Gatekeeper；--development 仅本机 debug/临时签名，拒绝外部 DMG；验收后的事务替换、同标识旧名迁移与失败回滚；保留隔离和资料，正常退出后安装、只刷新相关 Dock 项并启动宿主。
claude-cli.sh: 原生宿主内嵌的只读安装检查与官方 CLI 卸载/重装，使用登录终端 PATH，发现 nvm/全局 npm、Homebrew 双通道与原生版本；只卸载程序，保护自定义启动器与配置资产，重装沿用已识别包管理器并核验版本。
install-linux.py: Linux 用户前缀安装并保留资料；拒绝自定义启动器、链接父目录与整个安装目录下仍运行的 A/B，失败回滚。
test-linux.py: 临时构建验证安装隔离、B 运行保护、桌面写入回滚、Exec 引用与载荷缺失/引擎篡改拒绝；不访问用户资料。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md

assemble-release.py: 逐平台校验后合并更新平台清单，以明确白名单交付九个公开文件；内部签名、公证和引擎地图只留 CI，下载表独立展示安装器。
sign-windows-update.py: 已验收安装器的独立版本绑定更新签名和官方更新器核验；更新私钥不进入构建或测试。
release-windows.py: Windows x64 MSVC 无凭据构建，check/build/verify 阶段在 CI 分开呈现，原生/40 身份 UI 模拟/CLI/Skill/npm 安装回归与真实 NSIS 安装、启动、升级、卸载验证，交付单安装器与内部引擎哈希清单。
verify-windows-installer.py: 使用原生 Known Folder 和一致的父子进程环境定位空白 CI 数据；拒绝预存 Claude/应用目录再创建可丢弃用户资产样本，通过已持有句柄的宿主树/本次唯一安装目录回收 CI 程序，不给 GUI 沙箱套额外 Job，PowerShell 5.1 优先自身模块以保证 CIM 进程查询，查询错误即时报告、统一 Windows 8.3 别名/扩展路径/分隔符，只记录匹配布尔值；严格核对安装后引擎/图标/UI、实际浏览器 JS 回报的平台/CPU/版本与 A/B 回环、升级和卸载资产保留。
release-linux.py: 无 Apple/更新凭据的 x86_64 三格式构建，先运行身份 UI 模拟、Skill 离线/npm 安装回归与脚本语法检查；Tauri 以完整工具日志交付 DEB/AppImage，标准 rpmbuild 从相同 /usr 文件树生成 RPM，避免上游 RPM 打包停滞且禁止剥离厂商引擎；引擎位于 libexec，避免 Qt 可选适配库被宿主 linuxdeploy 错当必备依赖；真实解包核对宿主、UI、B 入口、图标与引擎字节，生成独立校验清单。
test-rpm.py: 临时 DEB 文件树经真实 rpmbuild 转换，回归 RPM 版本/架构、桌面入口与引擎字节，不执行样本程序。
verify-linux-package.py: Linux 安装包只读载荷契约，供 Ubuntu/Arch 共用，拒绝缺失 UI、入口或被改写的引擎文件/链接。
build-arch.sh: 固定摘要容器准备 Node.js 等构建/测试依赖，以非 root makepkg、原生回归与解包验收，公开源码与 Cargo 缓存挂载；不接触 macOS 凭据。
