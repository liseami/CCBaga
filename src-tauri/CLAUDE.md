# src-tauri/
> L2 | 父级: ../CLAUDE.md
宿主保持网络检测、SSH 与系统权限在 Rust 中；剪贴板复制与密钥文件选择只在主线程执行。清理与电脑设置使用后台任务；清理不读取钥匙串正文。
src/ - 清理、浏览器、地区与 VPS 事务的独立边界；见其 CLAUDE.md。
native/ - macOS SSH 文件选择、剪贴板、授权、直接钥匙串删除与电脑环境设置；见其 CLAUDE.md。
capabilities/ - 主窗口能力边界；见其 CLAUDE.md。
icons/ - 应用包图标；见其 CLAUDE.md。
Cargo.toml: Tauri 宿主与原生桥依赖；custom-protocol 映射 tauri/custom-protocol，由正式打包启用以读取内嵌前端，开发模式默认不启用。
Cargo.lock: 锁定 Rust 与平台库的实际版本。
build.rs: 编译 Objective-C 桥并生成 Tauri 构建元数据。
tauri.conf.json: 产品名 CC清理器、窗口、应用包标识、内容安全策略与开发默认临时签名；发布脚本显式覆盖 Developer ID 签名身份。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
