# src-tauri/
> L2 | 父级: ../CLAUDE.md
SECURITY.md: 原生数据流与安全契约，列出本地读取、外部连接、确认后修改、代理不序列化及 Linux A/B 安装运行保护；验证范围按实际环境陈述。
宿主保持网络检测与系统权限在 Rust 中，网络资源仅固定导航；剪贴板复制只在主线程执行。清理与电脑设置使用后台任务；旧清理扫描不读取钥匙串正文；身份暂存经用户确认在原生层保存默认 Claude 凭据，正文不送 UI。
src/ - 身份管理、清理白名单、浏览器、地区与固定资源导航的独立边界；见其 CLAUDE.md。
native/ - macOS 剪贴板、授权、直接钥匙串删除与电脑环境设置；见其 CLAUDE.md。
capabilities/ - 主窗口能力边界；见其 CLAUDE.md。
windows/ - NSIS 当前用户安装钩子与 B 入口；见其 CLAUDE.md。
icons/ - 应用包图标；见其 CLAUDE.md。
Cargo.toml: Win32 文件创建时原子指定用户 SID/私有 ACL；SQLite bundled 身份索引、ring 认证加密与 tar/gzip 归档；Tauri 宿主、官方 updater/process 插件与原生桥依赖；custom-protocol 映射 tauri/custom-protocol，由正式打包启用以读取内嵌前端，开发模式默认不启用。
Cargo.lock: 锁定 Rust 与平台库的实际版本。
build.rs: 编译 Objective-C 桥并生成 Tauri 构建元数据。
tauri.conf.json: 产品名 CCBaga-A、窗口、应用包标识、内容安全策略、静态图标 LobeHub 图标许可资源、公开 HTTPS 更新清单/公钥/强制签名版本与开发默认临时签名；发布脚本显式覆盖 Developer ID 签名身份。
tauri.macos.conf.json / tauri.linux.conf.json / tauri.windows.conf.json: 分平台资源与打包配置；macOS 保持双应用归档，Linux 使用 PNG 和 /usr/libexec/CCBaga-B 隔离引擎，防止 linuxdeploy 递归改写厂商 ELF；默认 Tauri 构建 deb/AppImage，RPM 配置的依赖由原生 rpmbuild 消费；Windows 使用当前用户 NSIS、内嵌浏览器引擎、B 启动项和 passive 更新；build.rs 只在 macOS 编译 Objective-C。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
