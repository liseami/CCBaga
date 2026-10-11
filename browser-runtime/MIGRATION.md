# CCBroswer 迁移记录
> L2 附件 | 父级: ./CLAUDE.md

2026-10-09：原 `~/Library/Application Support/US Browser` 的 Python 工程、虚假位置扩展和运行环境已移除；`/Applications/CCBroswer.app`、`~/Applications/美国浏览器.app` 及其桌面链接已移除。

现有资料只迁移一次，不复制恢复副本：Chromium 的 667 个文件迁入 `~/Library/Application Support/ai.local.requirement-french/browser/profile`，Camoufox 的 804 个文件迁入同级 `legacy-firefox-profile`。这些数量是迁移时核对值，浏览器继续使用后会变化。旧 Camoufox 资料不在 Chromium 清理范围。

Chromium 148.0.7778.215 引擎由旧应用迁入工程；主入口改为 Chromium 并重新签名。Framework 链接通过 ditto 归档保持，源码引擎与运行时引擎均通过 codesign 深层严格验证。宿主应用从内置归档安装引擎，不再引用旧工程。

浏览器已从需求法语启动并回传实际出口、时区、语言、系统/CPU 和版本。机器出口与浏览器出口需单独核验；DNS/WebRTC、图形读数没有独立检测时保留未核验。Default 的近似/精确定位权限设为拒绝，启动明确选择 Default，其他资料不自动改写。没有通过测试删除真实账户凭据，也没有自动修改系统代理或切换节点。

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
