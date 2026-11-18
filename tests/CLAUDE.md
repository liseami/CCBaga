# tests/
> L2 | 父级: ../CLAUDE.md
network.html: 网络页独立 Vite 测试入口，不进入生产构建。
network-fixture.tsx: 系统 DNS 的可延期完整/不完整/空/无效/超时观测样本；真实 VpsPanel 与可显式释放的虚构诊断 IPC；成功/部分失败/异常/空返回，无真实联网或导航。
identities.html: 独立 Vite 测试入口，生产 rollup 只含 index/browser，不打包本页。
identities-fixture.tsx: 真实 IdentityPanel 或 query panel=browser 的 BrowserPanel/Cromma/Motion/TooltipProvider + Tauri 官方 mockIPC；40 个虚构账号与有界进程/本地期限元数据、残留清单及可延期命令，官方 runCallback 发送有序 Channel 事件验证真实进度；浏览器模式从无缓存条目的资料开始，覆盖全清理默认取消/强制勾选/精确参数/任务锁与失败；一次性迁移文件选择/导入导出仅内存模拟；覆盖不完整统计、部分失败、命令审计与切换/删除/恢复/双语，不读取本机账号或文件。
执行入口 ../scripts/test-identities-ui.ts 使用 Node.js 原生管道及临时 HOME/XDG/浏览器上下文、阻止外网、保留真实 DOM 输入；不采集截图或用户目录。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
