# scripts/
> L2 | 父级: ../CLAUDE.md
windows.py: 当前用户专属 NTFS ACL、所有权/DACL 核验与本 Skill 子进程 Job Object；无账户凭据读取或系统策略修改。
network.py: AI 标准输入配置、阶段式密钥/主机/出口验证与显式部署，无用户终端交互；部署载荷按 UTF-8 读取；验证后写入 Git/Skill 外的 0600 本机配置及临时私钥、严格后续 SSH、有界脱敏输出、退出前关闭本次 Windows 子进程树再回收管道；订阅经 SSH 取本机私密文件，curl TUN 实测不使用代理环境。
setup-vps.sh: 原部署脚本迁出宿主；仅 Debian 12/13 root，系统/SSH/防火墙与 Xray/Reality 套 SOCKS5，UDP 阻断；自检全部通过才生成订阅回报，身份保存在 root 状态文件。
test_network.py: 可丢弃密钥/文件和本机子进程，验证三文件配对、UTF-8 部署载荷、私密权限、地址/注入拒绝、非交互配置拒绝越界/覆盖、跨安装位置 Git Bash 输入传输和超时；不连接真实 VPS，不使用用户凭据。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
