# scripts/
> L2 | 父级: ../CLAUDE.md
make-icon.swift: AppKit 将压缩品牌图绘入 macOS 圆角网格，保持原图并添加透明留白与投影。
make-icons.sh: 从 assets/ 品牌图生成各尺寸 iconset，iconutil 输出 src-tauri/icons 与 browser-runtime/icons。
brand-browser.sh: 编译 CCBrowser 入口、写入 CC浏览器 名称/图标/各语言显示名、清除构建资源扩展属性并调用 sign-browser.py 完整签名（标识不变）。
sign-browser.py: 内层 Mach-O → Helper 应用 → Framework → 浏览器外壳顺序签名；Developer ID 使用可信时间戳、Hardened Runtime，仅 Renderer/GPU 使用 JIT entitlement；支持显式本机临时签名 -。
test-sign-browser.py: 临时伪引擎树验证叶子到容器的签名顺序、最小 JIT 授权与符号链接不重复遍历。
fetch-browser.py: 固定上游引擎 DMG 与 SHA-256，核验版本和 arm64 后恢复引擎；干净 CI 检出不依赖维护者本机 Browser.app。
release-macos.py: 外部凭据与可选 CC_RELEASE_VERSION 驱动浏览器、宿主、DMG 三阶段公证装订与 Gatekeeper 校验；发布产物和哈希记录写入被忽略的 releases/。
package-browser.sh: 先品牌化签名，再打包引擎，ditto 保留 Framework 链接与可执行权限；归档进入应用资源。
install-apps.sh: 将 CC清理器 与 CC浏览器 整包替换到 /Applications（启动台可见），清除隔离/来源标记并 lsregister -f；退出、注销并删除 ~/Applications 下的 CC*.app 与旧需求法语*.app；注销应用数据旧引擎副本与工程内包的注册；重启 Dock/Finder 刷新图标。
setup-vps.sh: 从用户 Downloads 脚本纳入应用的 Debian 12/13 root 设置；Xray Reality 经家宽 SOCKS5 出口、Stripe 直连例外、阻断 UDP、SSH/防火墙加固与自检；仅全部通过输出 CC_SUB_URL，支持 IPv6 订阅地址，失败不打印完整配置，Xray 凭据配置限 root/实际服务组可读。
claude-cli.sh: 原生宿主内嵌的只读安装检查与官方 CLI 卸载/重装，使用登录终端 PATH，发现 nvm/全局 npm、Homebrew 双通道与原生版本；只卸载程序，保护自定义启动器与配置资产，重装沿用已识别包管理器并核验版本。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
