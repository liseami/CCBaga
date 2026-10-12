# CCBaga - CCBaga-A（宿主，橙底清理员 A）与 CCBaga-B（指纹 Chromium，橙底浏览器 B）
安装为 /Applications/CCBaga-A.app（ai.local.requirement-french）与 /Applications/CCBaga-B.app（ai.local.requirement-french.browser）；稳定包标识与独立资料目录保护钥匙串、设置和浏览器资料。
Tauri 2.12.1 + Rust + macOS AppKit/UniformTypeIdentifiers/Accessibility/Security/CoreLocation + SHA-256 + React 19.3.0 + TypeScript 5.9.3 + Cromma UI 本地副本 + Radix UI 1.6.7 + Tailwind CSS 4.3.3 + Motion 14.0.0 + Vite 7.3.6 + YAML 2.9.1（仅构建）

<directory>
.github/ - PR 的 Ubuntu/macOS/Windows 无凭据回归、release 分支自动测试、公证与发布工作流，以及依赖更新；见其 CLAUDE.md
src/ - 黑色主题的 Laper 最新 Landing 斜纹/点阵/细线内容壳、Cromma UI 极简侧栏导航与右侧傻瓜设置（国家地区选择和四个快捷按钮）、Claude 身份竖式立体工牌（白底空身份始终最左、当前账号其次、顶部挂孔绿光表示当前身份、弹簧横滚/视差与卡片直接切换）与 CLI 卸载/重装、浏览器核验、电脑设置与网络环境（三步固定资源入口、五项 AI 图标分段、问号服务规格与双语安装/运行提示词）及 YAML 中英文切换；见其 CLAUDE.md
src-tauri/ - 白名单身份暂存/切换、Claude Code CLI 安装管理、独立指纹浏览器、国家地区目录、电脑环境设置与固定网络资源导航；见其 CLAUDE.md
browser-runtime/ - 固定摘要的独立 Fingerprint Chromium 引擎与保留 Framework 链接的打包归档；见其 CLAUDE.md
tests/ - 独立身份 GUI 虚构 IPC 入口，供发布前真实浏览器交互模拟；不进入生产构建；见其 CLAUDE.md
scripts/ - 图标、签名、引擎打包、应用安装与Claude Code CLI 管理脚本；见其 CLAUDE.md
skills/ - Agent Skills 标准目录与 npm/npx 通用安装，MIT/兼容声明和全部脚本独立交付；AI 收集用户资料、运行本机预检与经确认的远端部署，应用只提供安装提示词；见其 CLAUDE.md
packaging/ - 社区 Linux 构建配方；见其 CLAUDE.md
assets/ - 两张压缩品牌图、五项 LobeHub AI SVG 与极客像素安装盘背景，分别供图标/README 和 DMG 布局使用；见其 CLAUDE.md
docs/ - 英文仓库封面与 Linux/Windows 开发说明；见其 CLAUDE.md
vendor/ - cromma-web 同款组件库副本，主题、原语与 Pika 图标自包含；见其 CLAUDE.md
</directory>
<config>
package.json - 前端构建、身份 UI 模拟、Tauri 应用打包与 dev:install 本机快速重装入口
bun.lock - 锁定前端依赖的实际版本
index.html - 单窗口 HTML 入口
browser.html - 独立浏览器实测页入口
vite.config.ts - 双入口静态构建与 YAML 编译，文案单独共享分块
tsconfig.json - 严格类型检查与 Vite 静态资源导入类型，品牌图由构建器打包
README.md - 英文项目入口，功能、安装、架构、开发与验证边界
CONTRIBUTING.md - 英文贡献流程与模块契约
LICENSE - 项目原创代码 MIT 授权
NOTICE.md - 第三方引擎、组件与图标的授权边界
.gitignore - 排除本机凭据、依赖、引擎与发布产物
</config>

法则：侧栏傻瓜设置置顶，分隔线后网络环境、Claude 身份管理、浏览器环境、电脑环境；BAGA! 品牌和黑白 Laper 斜纹/点阵壳，完整侧栏边框，底部更新与界面语言。YAML 中英文独立于系统/指纹语言，运行包嵌入前端，不依赖开发服务。
身份契约：默认范围同时覆盖 Claude Code 与 Claude Desktop；用户逐类选择保留在工作区或随身份暂存。登录/本地标识始终随身份；Skills、插件、MCP、个人设置、项目记录、会话历史和缓存分别选择，实际项目源码不扫描。空身份是把已选择的当前状态从 Claude 默认工作目录移开，保存可恢复身份后留出空工作区，不宣称抹除服务端记录或对同用户程序隐身。
身份显示：只读默认 .claude.json 中 oauthAccount 的 displayName/emailAddress/organizationName 白名单；UI 不接收令牌。Desktop 仅元数据候选检测，未知或不可读账号明确标注，不宣称当前账号已通过服务端验证。
保险库：app_data_dir/identity-vault 与 Claude 工作目录分离；bundled SQLite 保存名称、策略、归档编号/大小/时间与恢复日志，不保存凭据正文；FULL 同步、数据库事务。自定义 BAGAID01 容器用 tar/gzip 压缩、ring AES-256-GCM 认证帧和独立归档密钥，保险库主密钥只在受保护本机 key.bin 中；POSIX 0700/0600、Windows 用户独占 DACL。普通解压工具不能直接读取，但本机同用户/管理员不是绝对隔离边界。
切换：用户确认结束 Claude 进程；完整读取与归档验收成功后才改工作区。SQLite 与文件系统通过持久恢复状态协调；失败回滚，崩溃可恢复，恢复前也存下现状。复用所有权、链接、元数据快照和白名单检查；只合并所选 JSON 键，保留其他项目/MCP/设置。macOS 用户确认后在原生层保存默认 Code/Desktop 钥匙串正文，不经 UI、argv 或网络；不自动修改其他配置目录的账号。Linux/Windows 使用文件凭据；真实登录可能因服务端失效而需重新授权。
直接删除：AlertDialog 警告默认聚焦取消，提示不可撤销/不新增备份，用户勾选并独立确认后按保留策略移除当前本地身份，不创建新归档；已有暂存身份不受影响。删除暂存卡片仅移除选定加密文件与 SQLite 记录，双资源失败留下可回收标记；不把普通文件删除宣称为物理安全擦除。
浏览器与系统：缓存操作保留登录/书签/密码；换指纹先退出 B，再随机种子和目标地区语言/时区，显式代理留空跟随 TUN。真实设备版本/CPU、出口服务备援、回环令牌/回报时间与绿色核验仍沿用原边界，不伪造 IP/坐标或未测结果。系统修改显式授权并读回；未知定位保持未知。
网络：GUI 仅固定 VPS/家宽/Skill 入口与五项 AI 的双语 npm 安装/运行提示词。SSH/三文件/家宽资料和确认部署归独立 Skill；私密资料留在 Git/Skill 外，后续严格主机指纹验证，订阅私密交付后引导 TUN 开/系统代理关。供应商设置唯一来源 skills/ccbaga-network/providers.json，完整原始地址由回归锁定。
CLI 管理：Claude 身份页提供一键卸载/重装，确认后结束全部已识别 Claude 进程；原生、Homebrew 与登录终端/nvm 全局 npm 按官方流程卸载程序，配置、会话、技能、项目与钥匙串保留。拒绝删除自定义启动器或外部链接；重装沿用已识别包管理器，无安装时用官方原生安装器；版本核验成功且收到完整回报才报告完成。超时结束本次子进程组，错误保留部分修改的事实。
发布权限：官方版本仅由仓库所有者 liseami 发布，CI 同时核对仓库、触发者与重跑者；PR 不接触签名凭据。README 不承载发布、公证操作文章。
发布契约：开发构建默认临时签名；release-macos.py 使用外部环境变量指定 Developer ID 和 Apple API key，先公证装订浏览器，再将其归档嵌入宿主，最后公证宿主与双应用 DMG。单个 DMG 同时交付两个应用；每次品牌化都明确设置浏览器 Bundle ID（不能依赖本机旧引擎）；安装盘验收核对双应用名称/标识/入口。哈希锁定的 dmgbuild 只用于发布，写入像素背景与固定拖拽布局，不依赖 Finder GUI。必须取得 Accepted、装订票据并通过 Gatekeeper；不以移除隔离属性替代验收。装订后的 A 由官方 CLI 生成带版本签名的 .app.tar.gz 与公开 latest.json；仅公钥进入应用，原生 updater 强制验证版本，安装后重启，B 继续使用完整 DMG。CI 只缓存 Rust 依赖，签名私钥不传给构建子进程。密钥和发布产物不入 Git。
交付流程：视觉验收由项目所有者完成；获得发布授权后，先推送已验收提交到 release 触发递增版本和 tag；公开产物验收成功后将同一提交推送 main，再回复并关闭相关 issue，避免提交中的关闭关键字提前结束问题。版本说明描述当前产品与安装方式。所有平台构建、载荷、签名更新清单和公开下载验收成功后才清理被替代的 Release/tag；失败时保留可用版本。本机开发使用 bun run dev:install：debug/custom-protocol 内嵌 UI、临时签名、双应用事务重装到 /Applications 并启动/激活；开发模式拒绝外部安装盘，只允许临时签名包。正式交付使用 Developer ID、公证/Gatekeeper 和双应用 DMG，公开安装包保持强验收。
安全契约：src-tauri/SECURITY.md 描述数据流与外部连接；进程参数只在原生层识别，Machine.system_proxy 禁止序列化；宿主无用户信息上传服务；验证未覆盖的第三方、桌面或远程行为不得宣称通过。
Linux x86_64 适配：见 docs/linux.md；平台配置拆分为 tauri.macos.conf.json / tauri.linux.conf.json。Linux 原生边界位于 src-tauri/src/linux.rs，浏览器使用带哈希固定的 LinuxBrowser 与 XDG 启动项；版本任务后并行构建 macOS/Ubuntu/Arch；DEB/RPM/AppImage 引擎隔离在 libexec，避免宿主打包工具改写；Linux 实际安装包均解包核验，四格式与 macOS 同批验收后由所有者统一发布，Linux 不接触 Apple/更新私钥。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md

首页国家地区目录固定为截图的 13 项，顺序不扩展。Windows 完整时区映射只服务实际系统读回与核验。
Windows x64：平台桥使用当前用户 SID、文件所有权和 reparse-point 检查；进程核验精确入口、创建时间和 live handle。凭据管理器不读取或自动删除。当前用户 International cmdlets 保留其他语言并核验实际生效；时区 tzutil 写入后读回，不用默认值伪装状态。默认浏览器只注册自身 HKCU 能力并引导系统确认，独立读取 HTTP/HTTPS。CLI 由内嵌 PowerShell 按原生/npm/bun/WinGet 来源管理，Job Object 限定子进程树；个人资产保留。单 NSIS 同时安装 A/B；Windows 自动更新使用同一安装器且先关闭 B。网络仍只用独立 Skill；Windows 私密文件采用用户 SID 独占 NTFS ACL，原生 OpenSSH/curl，无 WSL 或 VPS UI。无凭据 Windows 预检分支不得发布；正式发布需全部平台验收，Windows 更新私钥仅在测试后的独立签名步骤注入。
