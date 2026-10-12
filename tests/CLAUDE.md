# tests/
> L2 | 父级: ../CLAUDE.md
identities.html: 独立 Vite 测试入口，生产 rollup 只含 index/browser，不打包本页。
identities-fixture.tsx: 真实 IdentityPanel/Cromma/Motion/TooltipProvider + Tauri 官方 mockIPC；40 个虚构账号，仅内存模拟切换/重命名/删除/失败/恢复与中英文，不读取任何本机账号。
执行入口 ../scripts/test-identities-ui.ts 使用临时浏览器上下文、阻止外网、保留真实 DOM 输入；不采集截图或用户目录。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
