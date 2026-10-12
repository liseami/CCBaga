# CCBaga - CCBaga-A（宿主，橙底清理员 A）与 CCBaga-B（指纹 Chromium，橙底浏览器 B）
安装为 /Applications/CCBaga-A.app（ai.local.requirement-french）与 /Applications/CCBaga-B.app（ai.local.requirement-french.browser）；稳定包标识与独立资料目录保护钥匙串、设置和浏览器资料。
Tauri 2.12.1 + Rust + macOS AppKit/UniformTypeIdentifiers/Accessibility/Security/CoreLocation + SHA-256 + React 19.3.0 + TypeScript 5.9.3 + Cromma UI 本地副本 + Radix UI 1.6.7 + Tailwind CSS 4.3.3 + Motion 14.0.0 + Vite 7.3.6 + YAML 2.9.1（仅构建）

<directory>
.github/ - PR 的 Ubuntu/macOS/Windows 无凭据回归、release 分支自动测试、公证与发布工作流，以及依赖更新；见其 CLAUDE.md
src/ - 黑色主题的 Laper 最新 Landing 斜纹/点阵/细线内容壳、Cromma UI 极简侧栏导航与右侧傻瓜设置（国家地区选择和四个快捷按钮）、Claude 清理与 CLI 卸载/重装、浏览器核验、电脑设置与网络环境（三步固定资源入口、五项 AI 图标分段、问号服务规格与双语安装/运行提示词）及 YAML 中英文切换；见其 CLAUDE.md
src-tauri/ - 白名单深度清理、Claude Code CLI 安装管理、独立指纹浏览器、国家地区目录、电脑环境设置与固定网络资源导航；见其 CLAUDE.md
browser-runtime/ - 固定摘要的独立 Fingerprint Chromium 引擎与保留 Framework 链接的打包归档；见其 CLAUDE.md
scripts/ - 图标、签名、引擎打包、应用安装与Claude Code CLI 管理脚本；见其 CLAUDE.md
skills/ - Agent Skills 标准目录与 npm/npx 通用安装，MIT/兼容声明和全部脚本独立交付；AI 收集用户资料、运行本机预检与经确认的远端部署，应用只提供安装提示词；见其 CLAUDE.md
packaging/ - 社区 Linux 构建配方；见其 CLAUDE.md
assets/ - 两张压缩品牌图、五项 LobeHub AI SVG 与极客像素安装盘背景，分别供图标/README 和 DMG 布局使用；见其 CLAUDE.md
docs/ - 英文仓库封面与 Linux/Windows 开发说明；见其 CLAUDE.md
vendor/ - cromma-web 同款组件库副本，主题、原语与 Pika 图标自包含；见其 CLAUDE.md
</directory>
<config>
package.json - 前端构建、Tauri 应用打包与 dev:install 本机快速重装入口
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

法则：侧栏第一项为傻瓜设置，分隔线后依次网络环境、Claude 清理、浏览器环境、电脑环境；底部放更新检查和界面语言切换。侧栏品牌为 BAGA!、首页标题为傻瓜設定、安装区标题为 Skill；应用包名称与标识不变。界面语言独立保存在应用目录 ui-language，通过回环上下文同步浏览器实测页，不修改系统或指纹语言。页面只呈现标题、操作、结果，影响说明放确认窗或展开项。发布包必须启用 custom-protocol 加载内嵌前端，不依赖本机开发服务；缺失时编译失败。单窗口承载维护、环境设置与验证。共用地区目录只设定语言、代表时区和区域格式，不宣称改变实际 IP。快捷清缓存/换指纹先正常退出 CCBaga-B；换指纹更新随机种子、启用 Canvas/布局种子并将代理留空以跟随 TUN。Claude 快捷清理先展示完整白名单快照，确认后永久删除。目录元数据完整统计，不以时间/条目上限截断；读取失败列出原因且不生成可删除快照，清理前复核整棵树。任务期间锁定窗口导航和其他快捷操作，避免扫描覆盖与后台事务失去 UI。网络 UI 只打开固定 VPS/家宽/Skill 地址并复制所选 AI 的双语安装提示词，不选择密钥、不收集家宽凭据、不注册 SSH/部署命令。独立 Skill 让用户一次提供地址、三个附件与供应商连接资料，AI 整理路径与 JSON，用户不运行终端。标准输入配置验证后写入 Git/Skill 外的 0600 文件；完整配对密钥用临时 0600 PEM，root:22 无密码/Agent，初次主机指纹需核对，后续严格拒绝变更。本机与 VPS 出口验证后展示真实远端变更，经确认才部署；成功后经新 SSH 会话取私密订阅文件，引导 TUN 开、系统代理关并实测；HTTP 订阅地址不宣称加密。供应商设置唯一来源 skills/ccbaga-network/providers.json，Rust 与 Skill 同源组装并测试原始完整 URL。进程检查覆盖当前用户 Claude Desktop 应用标识下的所有辅助进程、Claude Code 精确进程名与 Node/Bun 官方 CLI 脚本入口；结束按钮逐轮终止重新核验的匹配进程并重扫，确认窗不因旧扫描的进程状态永久锁定，重扫后重新确认删除。清理只接收有效快照编号；核验后直接删除，失败即停止后续操作，不创建备份、不经过废纸篓、不提供撤回。既有 ClaudeBackups、Code 自动备份与根目录配置备份纳入独立分类和完整清理。阶段、完成数量和日志由实际事务推送，未知总量使用活动动画，不显示虚构百分比；日志仅在内存保留。配置只移除明确代理或认证键；钥匙串只读属性并将单条持久引用解析为系统项目，文件钥匙串用原生单项删除兼容共享凭据，不读取密码；失败报告系统原因，0 项完成时不宣称数据已删除。文件夹拖入只扫描目录内的 Claude 白名单，不访问目录外凭据或会话；清理沿用范围与快照核验。Skills、plugins、MCP、权限、遥测和项目源码保留；桌面端也保留 claude_desktop_config.json 和未知个人文件，登录重置使用明确存储子目录；历史会话独立选择。电脑设置与清理独立：时区经系统管理员授权设置并核验；首选语言保留其他语言；定位走系统设置开关，CoreLocation 核验关闭，不篡改定位数据库。页面加载不修改系统设置。任一应用启动会带起另一应用；宿主重启时给已运行浏览器打开新会话实测页，避免旧端口失联；双方显示实际回报/接收时间。CCBaga-B 可在浏览器页一键请求设为默认浏览器（系统确认，不代用户切换）。浏览器资料独立存放在应用数据 browser/profile，明确启动 Default 以对齐偏好与清理范围；指纹系统版本与 CPU 读取本机，图形默认真实。出口必须实际检测，不固定 IP 或坐标；ipwho.is 限流/故障时经相同路由切换 ipapi.co，不切换代理或直连，全部失败保留 HTTP 原因。站点数据与缓存分离直接删除；清理、保存与对齐要求先退出资料对应浏览器。DNS/WebRTC 未独立核验时保持未核验。
CLI 管理：Claude 清理页提供一键卸载/重装，确认后结束全部已识别 Claude 进程；原生、Homebrew 与登录终端/nvm 全局 npm 按官方流程卸载程序，配置、会话、技能、项目与钥匙串保留。拒绝删除自定义启动器或外部链接；重装沿用已识别包管理器，无安装时用官方原生安装器；版本核验成功且收到完整回报才报告完成。超时结束本次子进程组，错误保留部分修改的事实。
发布权限：官方版本仅由仓库所有者 liseami 发布，CI 同时核对仓库、触发者与重跑者；PR 不接触签名凭据。README 不承载发布、公证操作文章。
发布契约：开发构建默认临时签名；release-macos.py 使用外部环境变量指定 Developer ID 和 Apple API key，先公证装订浏览器，再将其归档嵌入宿主，最后公证宿主与双应用 DMG。单个 DMG 同时交付两个应用；每次品牌化都明确设置浏览器 Bundle ID（不能依赖本机旧引擎）；安装盘验收核对双应用名称/标识/入口。哈希锁定的 dmgbuild 只用于发布，写入像素背景与固定拖拽布局，不依赖 Finder GUI。必须取得 Accepted、装订票据并通过 Gatekeeper；不以移除隔离属性替代验收。装订后的 A 由官方 CLI 生成带版本签名的 .app.tar.gz 与公开 latest.json；仅公钥进入应用，原生 updater 强制验证版本，安装后重启，B 继续使用完整 DMG。CI 只缓存 Rust 依赖，签名私钥不传给构建子进程。密钥和发布产物不入 Git。
交付流程：视觉验收由项目所有者完成；获得发布授权后，先推送已验收提交到 release 触发递增版本和 tag；公开产物验收成功后将同一提交推送 main，再回复并关闭相关 issue，避免提交中的关闭关键字提前结束问题。版本说明描述当前产品与安装方式。所有平台构建、载荷、签名更新清单和公开下载验收成功后才清理被替代的 Release/tag；失败时保留可用版本。本机开发使用 bun run dev:install：debug/custom-protocol 内嵌 UI、临时签名、双应用事务重装到 /Applications 并启动/激活；开发模式拒绝外部安装盘，只允许临时签名包。正式交付使用 Developer ID、公证/Gatekeeper 和双应用 DMG，公开安装包保持强验收。
安全契约：src-tauri/SECURITY.md 描述数据流与外部连接；进程参数只在原生层识别，Machine.system_proxy 禁止序列化；宿主无用户信息上传服务；验证未覆盖的第三方、桌面或远程行为不得宣称通过。
Linux x86_64 适配：见 docs/linux.md；平台配置拆分为 tauri.macos.conf.json / tauri.linux.conf.json。Linux 原生边界位于 src-tauri/src/linux.rs，浏览器使用带哈希固定的 LinuxBrowser 与 XDG 启动项；版本任务后并行构建 macOS/Ubuntu/Arch；DEB/RPM/AppImage 引擎隔离在 libexec，避免宿主打包工具改写；Linux 实际安装包均解包核验，四格式与 macOS 同批验收后由所有者统一发布，Linux 不接触 Apple/更新私钥。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md

首页国家地区目录固定为截图的 13 项，顺序不扩展。Windows 完整时区映射只服务实际系统读回与核验。
Windows x64：平台桥使用当前用户 SID、文件所有权和 reparse-point 检查；进程核验精确入口、创建时间和 live handle。凭据管理器不读取或自动删除。当前用户 International cmdlets 保留其他语言并核验实际生效；时区 tzutil 写入后读回，不用默认值伪装状态。默认浏览器只注册自身 HKCU 能力并引导系统确认，独立读取 HTTP/HTTPS。CLI 由内嵌 PowerShell 按原生/npm/bun/WinGet 来源管理，Job Object 限定子进程树；个人资产保留。单 NSIS 同时安装 A/B；Windows 自动更新使用同一安装器且先关闭 B。网络仍只用独立 Skill；Windows 私密文件采用用户 SID 独占 NTFS ACL，原生 OpenSSH/curl，无 WSL 或 VPS UI。无凭据 Windows 预检分支不得发布；正式发布需全部平台验收，Windows 更新私钥仅在测试后的独立签名步骤注入。
