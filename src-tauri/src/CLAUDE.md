# src/
> L2 | 父级: ../CLAUDE.md
清理在本机执行；网络页只提供固定资源入口，SSH 由应用外 Skill 执行。清理直接删除所选数据和既有备份，不保留恢复副本；系统设置与清理独立。
updater_tests.rs: 官方更新器本机 HTTP 公开向量验证篡改与版本冒用；发布显式传入实际清单/归档，下载核验而不安装。
main.rs: 官方更新/重启插件与 Tauri 生命周期、双语托盘和清理/CLI/浏览器/固定网络入口命令注册；启动时带起 CCBaga-B，退出删除 start.url；发布缺少 custom-protocol 时编译失败，防止安装包依赖开发服务；Windows 正式宿主使用 GUI 子系统，CI 仅记录固定启动阶段定位实际运行阻塞，不记录参数或用户资料。
ui_language.rs: 受 zh/en 白名单限制的界面偏好，原子持久化后发布内存状态；共用托盘菜单，独立于电脑与浏览器语言。
browser/: 指纹浏览器、资料与缓存边界、真实浏览器回报及目标地区环境核验；见其 CLAUDE.md。
windows.rs: Win32 剪贴板、当前用户 SID/文件所有权、Win32 直接读取时区/语言、缓存系统版本与用户触发设置；控制台工具隐藏窗口，外部链接；拒绝凭据管理器自动读取和 SSH 操作。
windows_timezones.rs: CLDR 48 固定全量 Windows/IANA 映射与目录别名，无运行时下载或猜测；原始许可证作为 Windows 资源交付。
windows_process.rs: 当前用户 Toolhelp/MSIX 固定 Claude 包族与精确 CLI 参数识别、创建时间和句柄复核、独立资料的正常窗口关闭；原始参数不序列化，超时子进程使用 Job Object；Windows CI 回归实际映射时区写入后还原。
system.rs: 电脑环境查询、独立修改命令与目录限定的地区时区/语言/区域应用；后台执行原生操作，错误后重新读取系统状态。
native.rs: 剪贴板、辅助功能设置、进程编号与凭据解锁桥；清理只读凭据属性并直接删除。
cleaner.rs: 全机/文件夹扫描与快照保护、只读进程状态；外置 Windows AppData 仍仅接受定义白名单并在执行时复核所有权/规范路径；原生 CLI 识别当前用户官方 versions 下的精确数字版本程序；结束全部已识别的 Claude 进程时逐轮核验与收敛，重新扫描保留范围；CLI 程序修改共用事务锁与进程核验，各调用独立进度通道。
claude_cli.rs: 内嵌脚本的只读状态与 CLI 程序卸载/重装，登录终端加载 nvm 环境，固定动作与原生互斥锁限制修改；真实阶段沿用进度通道，超时结束本次子进程组或 Job Object；Windows 完成时先回收本次子进程树再读取完整协议，成功要求完整回报；临时目录测试验证版本、卸载、资产保留和失败不报成功。
cleaner_folder.rs: 用户所选目录内的 Claude 白名单发现；不跟随链接、不扫描目录外、不处理项目权限或凭据。
progress.rs: 有序脱敏阶段事件与有界内存日志；供清理与浏览器共享，只读取事务传入的计数和状态。
cleaner_transaction.rs: 范围与快照核验后的直接删除和窄范围配置写入；逐项记录，失败停止，不创建备份或撤回记录。
cleaner_targets.rs: Claude 白名单、桌面登录存储子目录（不删整个应用资料目录）、完整目录规模和元数据指纹（无时间/条目截断，读取失败不生成可删除快照）、按实际 Windows APPDATA 解析旧组织会话检索；不进入链接目标或项目源码。
cleaner_config.rs: 账号缓存和代理认证配置的窄范围变换；保留其他配置并只返回键名。
cleaner_io.rs: 清理后配置原子写入；Windows 写入前保留原所有者/DACL，临时文件仅保存新内容，失败移除，不保留原配置。
cleaner_tests.rs: 临时白名单/快照/日志验证，覆盖超过 30,000 文件的完整清理与权限失败保护；官方原生版本路径、自编译并经启动握手的可丢弃进程与 Node 样本验证身份、终止与其他 Node 保留；不复制系统平台二进制，不修改用户数据。
regions.rs: 地区编号到语言、区域格式和代表时区的唯一白名单与目标语言/时区地区匹配；供前端选择与系统/浏览器快捷操作共用，包含目录与本机时区验证。
linux.rs: Linux procfs 当前用户进程、zoneinfo/timedatectl、GNOME 定位偏好、GNOME/KDE 设置入口与 Wayland/X11 剪贴板；LC_MESSAGES/LC_TIME 分别读取会话语言/区域格式，不匹配时先报手动操作再改时区；未知定位为 null，Linux 钥匙串明确不可用；不将系统代理凭据序列化给 WebView。
network_links.rs: 固定供应商与 Skill 导航白名单，编译消费 Skill 的 providers.json 组装原始 URL，WebView 不能指定 URL；无密钥访问或 SSH 执行。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
