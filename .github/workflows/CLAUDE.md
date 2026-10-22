# .github/workflows/
> L2 | 父级: ../CLAUDE.md
release.yml: 仅 liseami 在 liseami/CCBaga 的 release push/手动触发及重跑可发布；main 仅在所有者 workflow_dispatch 提供 installer_run 时执行无凭据 Windows 安装样本验证；不允许通过 main push 触发，所有者可手动重放本仓库既有安装样本，版本任务拒绝 release 重用，正式发布始终重新构建，release 统一版本后并行 macOS、Ubuntu 22.04 x86_64、固定摘要 Arch 与 Windows MSVC；Windows 在 2025 runner 分开显示回归、NSIS 构建、实际浏览器回报与安装验收，实际启动失败时也保存无私钥安装样本供诊断，并在 2022 runner 独立验收相同产物（发布仍要求两个 runner 全部成功），只有 release 完成测试后独立注入更新私钥；Linux 使用独立 Ubuntu 22.04 缓存、无签名凭据，DEB/RPM/AppImage 解包逐个检查 glibc 2.35 上限，Arch 单独核验当前发行版；全部产物验收后才允许发布；arm64 runner 哈希安装 DMG 工具、仅成功构建保存完整 Rust 依赖缓存（失败不固化不完整依赖集；版本号仅构建步骤注入，不进入依赖缓存键）、测试前端/原生/Node.js 标准管道的 40 身份 GUI 模拟/临时身份钥匙串/Skill/npm 安装/开发安装边界 并签名公证，临时钥匙串持有 Secrets，固定 SHA Actions、Node 22.23.2 和无凭据 npm 安装实测；独立最小权限发布任务读取不可变产物，校验实际签名更新包并交付 latest.json，成功后显式固定并核对递增版本 tag，用当前产品与安装说明发布 Release（不生成历史比较列表），避免分支推进后隐式建 tag 的权限失败，公开仓库附带来源证明；公开白名单为六个安装器、macOS 更新归档、合并 latest.json 与 SHA256SUMS，独立签名、公证记录和引擎清单只保留在 CI。
checks.yml: 三平台初始引擎准备文件树与真实 Vite 产物资源引用回归；pull_request 合并提交、main push 或手动只读检查的 Ubuntu 22.04/macOS 15/Windows 2025 原生矩阵；只读 token、不持久化凭据、固定 SHA/工具链、含runner镜像版本的独立检查缓存键（禁止跨Ubuntu ABI基线恢复；PR仅恢复主干缓存，只有成功的main检查保存完整依赖，避免PR分支挤占与失败半成品固化）；仅 PR 取消旧检查，main 保留在途任务以保存缓存，合并待运行任务；fail-fast=false；三平台复用前端、custom-protocol 原生、Skill 离线/npm 四客户端安装和脚本语法检查；Linux 验证引擎完整替换/安装隔离/RPM，macOS 用临时钥匙串验证身份桥，并以伪凭据验证签名/安装边界，Windows 验证 CLI/区域设置并还原；测试资源白名单排除未下载引擎，真实引擎与安装器仍由 release 验收，无 Secrets/签名/上传/发布。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md

PR 三平台同时运行 test-assemble-release.py，使用临时假安装器核对完整九文件交付与平台摘要遗漏拒绝，不使用签名材料或发布权限。

checks.yml 的 macOS 检查另运行 test-intel-engine.py：仅离线归档/哈希/工作区契约样本，不下载或编译 Chromium，不接触发布凭据。
