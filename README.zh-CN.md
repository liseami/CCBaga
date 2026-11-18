<!--
[INPUT]: 英文 README 中的产品功能、支持平台、安装入口与使用边界。
[OUTPUT]: 简明中文项目入口、安装与操作导航，以及返回英文详细文档的链接。
[POS]: README.md 的中文入口；详细技术说明由现有文档维护。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
-->

# CCBaga

[English](README.md) · **简体中文**

CCBaga 是面向 macOS、Linux 和 Windows 的本地工具，用于管理 Claude 身份、伴生浏览器指纹与系统环境，并核验实际浏览器环境。**CCBaga-A** 负责本机操作，**CCBaga-B** 是使用独立资料目录的伴生浏览器。

**用途声明：**本项目仅供旅居海外人员管理本人的 Claude 身份；禁止违反 Anthropic 官方用户协议的用途。本地设置不会更改服务端记录，也不能保证匿名或账号可用。

本页提供简明中文导航；完整功能、开发步骤和平台限制见[英文 README](README.md)。应用界面和侧栏的「使用说明」均支持中文，可在侧栏底部切换语言。

## 安装

从 [GitHub Releases](https://github.com/liseami/CCBaga/releases/latest) 下载由项目所有者 **liseami** 发布的官方安装包。

- **macOS 12+，Apple Silicon：**下载包含 A、B 两个应用的 DMG，将两者拖入 Applications，然后启动 CCBaga-A。详见 [macOS 安装说明（英文）](README.md#macos)。
- **Linux x86_64：**提供基于 Ubuntu 22.04（glibc 2.35）的 DEB、RPM、AppImage，以及面向当前 Arch 的 pacman 包，均包含伴生浏览器。依赖与限制见 [Linux 说明](docs/linux.md)。
- **Windows 10/11 x86_64：**使用包含 A、B 的 NSIS 安装器。安装器未进行 Authenticode 签名，可能出现 SmartScreen 提示或被组织策略阻止。详见 [Windows 说明](docs/windows.md)。

## 常用操作

- **傻瓜设置：**选择国家或地区后执行所需操作；地区设置不会改变公网 IP。
- **Claude 身份管理：**保存备份或切换本地身份，并选择保留哪些工作区资料；保存备份不改变当前工作区或浏览器绑定。可用密码加密包在同系统设备间导出/导入保存身份，不迁移网页登录。切换前保存工作；直接删除不可撤销且不会新增备份。
- **浏览器环境：**仅清缓存保留登录；全清理永久删除当前身份的密码、书签、历史、扩展、设置和站点数据，无备份，需单独确认。换指纹会重启 CCBaga-B；绿色核验仅代表已测项目匹配，不包含独立 DNS/WebRTC 泄漏验证。
- **电脑环境：**分别调整时区、语言与地区；部分操作需要系统授权、重启或注销。
- **网络环境：**复制提示词，让本地 AI 工具安装独立的 Network Skill。应用不收集 SSH 密钥或执行服务器部署，远端变更需单独确认。

## 系统 DNS 出口观测

在「网络环境」中点击「系统 DNS 出口观测」，通过系统解析器查询 12 个随机测试域名，并从权威端观测实际递归解析服务器；仅显示请求公网出口、DNS 地址、服务商与观测完整性，不读取服务端泄漏结论，也不自动判断 DNS 是否泄露。TXT 查询避免 Fake-IP 或禁用 AAAA 时只得到本地合成答案。

固定第三方服务 `dnsleak.dev` 会看到请求出口 IP 和测试域名，不接收账号、浏览记录或本地配置。请求跟随系统路由/TUN，不使用显式 HTTP 代理。请核对服务商是否符合代理设置；地理位置仅供参考，IPv6 解析服务器不代表本机 IPv6 绕行。不包含抓包、浏览器安全 DNS 或 WebRTC 验证，也不保证所有域名与后续网络状态；不修改 DNS/代理设置。

## 详细文档

- [使用速查（英文）](docs/usage.md)与[身份存储和恢复边界](docs/identity-storage.md)
- [更新说明（英文）](README.md#updates)
- [开发与检查（英文）](README.md#development)
- [贡献指南（英文）](CONTRIBUTING.md)
- [MIT 许可证](LICENSE)与[第三方声明](NOTICE.md)
