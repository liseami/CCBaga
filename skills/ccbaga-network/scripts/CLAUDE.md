# scripts/
> L2 | 父级: ../CLAUDE.md
windows.py: 当前用户专属 NTFS ACL、所有权/DACL 核验与本 Skill 子进程 Job Object；无账户凭据读取或系统策略修改。
network.py: AI 标准输入配置、阶段式密钥/主机/出口验证与显式部署；expected_ip必须为字符串，主机名末段不得为纯数字（拒绝写错的 IPv4），密钥文件拒绝链接；临时私钥目录在写秘密前收紧 Windows DACL。私密配置位于 Git/Skill 外，严格后续 SSH、有界脱敏输出与本次进程树回收；订阅经 SSH 取本机私密文件，curl TUN 实测不使用代理环境。
setup-vps.sh: 仅 Debian 12/13 root，系统/SSH/防火墙与 Xray/Reality 套 SOCKS5，UDP 阻断；自检配置与日志用唯一临时文件并在退出清理，全部通过才生成订阅回报，状态留在远端 root。
test_network.py: 可丢弃密钥/文件和本机子进程，验证三文件配对、UTF-8 部署载荷、私密权限、地址/注入拒绝、非交互配置拒绝越界/覆盖、跨安装位置 Git Bash 输入传输和超时；不连接真实 VPS，不使用用户凭据。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
