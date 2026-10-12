# native/
> L2 | 父级: ../CLAUDE.md
清理、剪贴板与电脑设置分别使用独立系统桥；无选区捕获或模拟键盘事务。
IdentityTests.m: 临时独立钥匙串与虚构字段，编译期替换默认钥匙串获取；实际 Security API 验证服务范围、MCP 保留、陈旧拒绝与单项恢复，不改变用户默认钥匙串。
Identity.m: 经用户确认读取默认 Code/Desktop 凭据到原生保险库；系统授权、写前复核正文、单条删除与恢复，不经命令行/UI/网络。
Keychain.m: 用户触发登录钥匙串解锁，不读取翻译 Key，不禁用清理的系统授权弹窗。
CleanupTests.m: 独立临时钥匙串复现外部应用所有者 -25244，并验证原生删除、同服务账号隔离与非 Claude 凭据保留，不触碰登录钥匙串。
Cleanup.m: libproc 按外层 Claude 应用标识识别全部辅助进程；sysctl 仅识别 Node/Bun 首个官方 Claude CLI 脚本参数，不输出完整命令；已知凭据服务；只读属性核验后将单条持久引用解析为系统项目引用；文件钥匙串原生单项删除兼容共享密码，其他实现使用单项查询删除，保留系统错误代码和原因，不读取密码或保留撤回副本。
System.m: Foundation 首选语言、地区格式和时区、CoreLocation 状态、系统授权及稳定标识/语义标签定位开关；定位关闭结果必须经系统读取确认。
Browser.m: 真实机器状态与代理；LaunchServices 注册独立浏览器、按包标识查询运行、读取 http/https 默认处理程序并以 LSSetDefaultHandlerForURLScheme/LSSetDefaultRoleHandlerForContentType 请求设为默认（系统弹窗确认）；进程身份核验（Chromium 或 CCBrowser 可执行名）。
Clipboard.m: 主线程文本复制、辅助功能设置入口与共享响应释放；不捕获选区、不模拟键盘。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
