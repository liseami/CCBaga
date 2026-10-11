# CCTool - CC清理器（宿主，橙底清理员 A）与 CC浏览器（指纹 Chromium，橙底浏览器 B）
工程目录 ~/Cromma/CCTool（原 ~/Cromma/需求法语）；安装为 /Applications/CC清理器.app（ai.local.requirement-french）与 /Applications/CC浏览器.app（ai.local.requirement-french.browser）。包标识沿用旧值，钥匙串、设置与浏览器资料不迁移。
Tauri 2.12.1 + Rust + macOS AppKit/UniformTypeIdentifiers/Accessibility/Security/CoreLocation + SHA-256 + React 19.3.0 + TypeScript 5.9.3 + Cromma UI 本地副本 + Radix UI 1.6.7 + Tailwind CSS 4.3.3 + Motion 14.0.0 + Vite 7.3.6 + YAML 2.9.1（仅构建）

<directory>
src/ - Cromma UI 极简侧栏导航与右侧傻瓜设置（国家地区选择和四个快捷按钮）、Claude 清理与 CLI 卸载/重装、浏览器核验、电脑设置与网络环境（VPS 三文件密钥 SSH、固定供应商获取入口与按需选购帮助）及 YAML 中英文切换；见其 CLAUDE.md
src-tauri/ - 白名单深度清理、Claude Code CLI 安装管理、独立指纹浏览器、国家地区目录、电脑环境设置与 OpenSSH 远程执行；见其 CLAUDE.md
browser-runtime/ - 已迁入的独立 Fingerprint Chromium 引擎与保留 Framework 链接的打包归档；见其 CLAUDE.md
scripts/ - 图标、签名、引擎打包、应用安装与宿主内嵌的 VPS 设置、Claude Code CLI 管理脚本；见其 CLAUDE.md
assets/ - 两张压缩品牌图，图标生成与 README 共用；见其 CLAUDE.md
docs/ - 英文仓库封面与可执行发布说明；见其 CLAUDE.md
vendor/ - cromma-web 同款组件库副本，主题、原语与 Pika 图标自包含；见其 CLAUDE.md
</directory>
<config>
package.json - 前端构建与 Tauri 应用打包入口
bun.lock - 锁定前端依赖的实际版本
index.html - 单窗口 HTML 入口
browser.html - 独立浏览器实测页入口
vite.config.ts - 双入口静态构建与 YAML 编译，文案单独共享分块
tsconfig.json - 严格类型检查
README.md - 英文项目入口，功能、安装、架构、开发与验证边界
CONTRIBUTING.md - 英文贡献流程与模块契约
LICENSE - 项目原创代码 MIT 授权
NOTICE.md - 第三方引擎、组件与图标的授权边界
.gitignore - 排除本机凭据、依赖、引擎与发布产物
</config>

法则：侧栏第一项为傻瓜设置，分隔线后依次网络环境、Claude 清理、浏览器环境、电脑环境；底部只放界面语言切换。界面语言独立保存在应用目录 ui-language，通过回环上下文同步浏览器实测页，不修改系统或指纹语言。页面只呈现标题、操作、结果，影响说明放确认窗或展开项。发布包必须启用 custom-protocol 加载内嵌前端，不依赖本机开发服务；缺失时编译失败。一个窗口；文本翻译功能、DeepSeek、设置与全局快捷键已删除，不读取旧翻译设置或密钥。共用地区目录只设定语言、代表时区和区域格式，不宣称改变实际 IP。快捷清缓存/换指纹先正常退出 CC浏览器；换指纹更新随机种子、启用 Canvas/布局种子并将代理留空以跟随 TUN。Claude 快捷清理先展示完整白名单快照，确认后永久删除。任务期间锁定窗口导航和其他快捷操作，避免扫描覆盖与后台事务失去 UI。VPS 使用本机 OpenSSH（首次 accept-new，变更拒绝），仅支持 root:22 三文件密钥包（pem/ppk/pub），禁用密码和 Agent；PPK/PUB 与 PEM/PUB 均核验配对，私钥只在原生层以 0600 临时副本使用并随事务删除；家宽凭据仅在本次内存和加密标准输入中，不写本机配置或日志；设置与自检全部成功才返回订阅，提示只开 TUN、关闭系统代理；订阅为 HTTP 私密地址。进程检查覆盖当前用户 Claude Desktop 应用标识下的所有辅助进程、Claude Code 精确进程名与 Node/Bun 官方 CLI 脚本入口；结束按钮逐轮终止重新核验的匹配进程并重扫，确认窗不因旧扫描的进程状态永久锁定，重扫后重新确认删除。清理只接收有效快照编号；核验后直接删除，失败即停止后续操作，不创建备份、不经过废纸篓、不提供撤回。既有 ClaudeBackups、Code 自动备份与根目录配置备份纳入独立分类和完整清理。阶段、完成数量和日志由实际事务推送，未知总量使用活动动画，不显示虚构百分比；日志仅在内存保留。配置只移除明确代理或认证键；钥匙串只读属性并将单条持久引用解析为系统项目，文件钥匙串用原生单项删除兼容共享凭据，不读取密码；失败报告系统原因，0 项完成时不宣称数据已删除。文件夹拖入只扫描目录内的 Claude 白名单，不访问目录外凭据或会话；清理沿用范围与快照核验。Skills、plugins、MCP、权限、遥测和项目源码保留；历史会话独立选择。电脑设置与清理独立：时区经系统管理员授权设置并核验；首选语言保留其他语言；定位走系统设置开关，CoreLocation 核验关闭，不篡改定位数据库。页面加载不修改系统设置。任一应用启动会带起另一应用；宿主重启时给已运行浏览器打开新会话实测页，避免旧端口失联；双方显示实际回报/接收时间。CC浏览器 可在浏览器页一键请求设为默认浏览器（系统确认，不代用户切换）。浏览器资料独立存放在应用数据 browser/profile，明确启动 Default 以对齐偏好与清理范围；指纹系统版本与 CPU 读取本机，图形默认真实。出口必须实际检测，不固定 IP 或坐标；ipwho.is 限流/故障时经相同路由切换 ipapi.co，不切换代理或直连，全部失败保留 HTTP 原因。站点数据与缓存分离直接删除；清理、保存与对齐要求先退出资料对应浏览器。DNS/WebRTC 未独立核验时保持未核验。
CLI 管理：Claude 清理页提供一键卸载/重装，确认后结束全部已识别 Claude 进程；原生、Homebrew 与登录终端/nvm 全局 npm 按官方流程卸载程序，配置、会话、技能、项目与钥匙串保留。拒绝删除自定义启动器或外部链接；重装沿用已识别包管理器，无安装时用官方原生安装器；版本核验成功且收到完整回报才报告完成。超时结束本次子进程组，错误保留部分修改的事实。
发布契约：开发构建默认临时签名；release-macos.py 使用外部环境变量指定 Developer ID 和 Apple API key，先公证装订浏览器，再将其归档嵌入宿主，最后公证宿主与双应用 DMG。必须取得 Accepted、装订票据并通过 Gatekeeper；不以移除隔离属性替代验收。密钥和发布产物不入 Git。
交付流程：每轮修改和测试完成后，必须重新打包正式应用、校验签名并安装到 /Applications，然后启动新版本。视觉与功能验收由用户负责，助手持续根据反馈修改、验证及重装。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
