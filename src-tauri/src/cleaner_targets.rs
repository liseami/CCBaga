// [INPUT]: 当前用户目录、Windows 实际 AppData、Claude 清理白名单与可选组织编号。
// [OUTPUT]: 分类清理目标（站点存储不含个人偏好、密码密钥或工作区）、无条目截断的目标发现与读取警告、完整目录元数据统计及受限旧组织文件匹配。
// [POS]: 清理发现边界；不跟随链接，不遍历项目仓库，不读取凭据内容。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
use std::time::{Duration, Instant};

pub(super) struct Target {
    pub path: PathBuf,
    pub title: String,
    pub group: &'static str,
    pub risk: &'static str,
    pub impact: String,
    pub operation: Operation,
}
fn target(
    root: &Path,
    relative: &str,
    title: &str,
    group: &'static str,
    risk: &'static str,
    impact: &str,
) -> Target {
    Target {
        path: root.join(relative),
        title: title.into(),
        group,
        risk,
        impact: impact.into(),
        operation: Operation::Delete,
    }
}
fn desktop_storage(root: &Path, desktop: &Path) -> Vec<Target> {
    [
        "Local Storage",
        "Session Storage",
        "IndexedDB",
        "Network",
        "Cookies",
        "Cookies-journal",
        "Service Worker",
        "WebStorage",
        "Sessions",
        "Storage",
        "Shared Dictionary",
        "SharedStorage",
        "SharedStorage-wal",
        "DIPS",
        "DIPS-journal",
        "DIPS-wal",
        "DIPS-shm",
        "TransportSecurity",
        "Trust Tokens",
        "Trust Tokens-journal",
        "Reporting and Nel",
        "Reporting and Nel-journal",
        "Network Persistent State",
        "BrowsingTopicsState",
        "blob_storage",
    ]
    .into_iter()
    .map(|name| Target {
        path: desktop.join(name),
        ..target(
            root,
            "",
            "Desktop 会话与登录存储",
            "desktop",
            "login",
            "清除本地登录数据库与网页状态；保留桌面配置、MCP 和其他个人文件。",
        )
    })
    .collect()
}
#[cfg(target_os = "windows")]
pub(super) fn win_dir(root: &Path, variable: &str, fallback: &str) -> PathBuf {
    if super::home().ok().as_deref() == Some(root) {
        if let Some(path) = std::env::var_os(variable)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
        {
            return path;
        }
    }
    root.join(fallback)
}
fn entries(path: &Path, warnings: &mut Vec<String>) -> Vec<fs::DirEntry> {
    let directory = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                warnings.push(format!("目录无法完整检查：{}：{error}", path.display()));
            }
            return Vec::new();
        }
    };
    directory
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(error) => {
                warnings.push(format!("目录无法完整检查：{}：{error}", path.display()));
                None
            }
        })
        .collect()
}
pub(super) fn desktop_roots(root: &Path) -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let appdata = win_dir(root, "APPDATA", "AppData/Roaming");
        let localappdata = win_dir(root, "LOCALAPPDATA", "AppData/Local");
        vec![
            appdata.join("Claude"),
            appdata.join("Claude-3p"),
            localappdata.join("Claude"),
            localappdata.join("Claude-3p"),
            localappdata.join("Packages/Claude_pzs8sxrjxfjjc/LocalCache/Roaming/Claude"),
            localappdata.join("Packages/Claude_pzs8sxrjxfjjc/LocalCache/Roaming/Claude-3p"),
            localappdata.join("Packages/Claude_pzs8sxrjxfjjc/LocalCache/Local/Claude"),
            localappdata.join("Packages/Claude_pzs8sxrjxfjjc/LocalCache/Local/Claude-3p"),
        ]
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec![
            root.join("Library/Application Support/Claude"),
            root.join("Library/Application Support/Claude-3p"),
            root.join("Library/Application Support/com.anthropic.claudefordesktop"),
        ]
    }
}
pub(super) fn definitions(root: &Path) -> Vec<Target> {
    definitions_with_warnings(root).0
}
pub(super) fn definitions_with_warnings(root: &Path) -> (Vec<Target>, Vec<String>) {
    let mut result = Vec::new();
    let mut warnings = Vec::new();
    for (name, title) in [
        ("cache", "Code 缓存"),
        ("stats-cache.json", "使用统计缓存"),
        ("telemetry", "本地遥测缓存"),
        ("usage-data", "用量缓存"),
        ("usage.jsonl", "用量记录"),
        ("usage.with-fix.jsonl", "用量修正记录"),
        ("paste-cache", "粘贴缓存"),
        ("statsig", "实验特性与遥测缓存"),
        ("todos", "待办缓存"),
        ("ide", "IDE 连接状态缓存"),
        ("daemon", "后台守护进程状态"),
        ("daemon.lock", "后台守护进程锁"),
        ("daemon.log", "后台守护进程日志"),
        ("daemon.status.json", "后台守护进程状态记录"),
        ("downloads", "Code 下载缓存"),
        (".last-cleanup", "清理时间戳记录"),
        (".last-update-result.json", "更新结果记录"),
        (".oauth_refresh.lock", "Code 缓存"),
        (".oauth_refresh.lock.owner", "Code 缓存"),
    ] {
        result.push(target(
            root,
            &format!(".claude/{name}"),
            title,
            "cache",
            "safe",
            "可重新生成，不移除登录、技能或设置。",
        ));
    }
    for (relative, title) in [
        (".claude.lock", "Code 配置互斥锁"),
        (".claude.json.lock", "Code 配置互斥锁"),
    ] {
        result.push(target(
            root,
            relative,
            title,
            "cache",
            "safe",
            "可重新生成，不移除登录、技能或设置。",
        ));
    }
    for (relative, title) in [
        ("Library/Caches/claude-cli-nodejs", "Code 系统缓存"),
        ("Library/Logs/Claude", "Desktop 日志"),
        (
            "Library/Application Support/Claude/Cache",
            "Desktop 页面缓存",
        ),
        (
            "Library/Application Support/Claude/Code Cache",
            "Desktop 编译缓存",
        ),
        (
            "Library/Application Support/Claude/GPUCache",
            "Desktop 图形缓存",
        ),
    ] {
        result.push(target(
            root,
            relative,
            title,
            "cache",
            "safe",
            "仅移除本地缓存或日志。",
        ));
    }
    for (parent, prefix, title) in [
        (
            "Library/Caches",
            "com.anthropic.claudefordesktop",
            "Desktop 系统缓存",
        ),
        (
            "Library/Logs/DiagnosticReports",
            "Claude",
            "Desktop 崩溃报告",
        ),
        (
            "Library/Application Support/CrashReporter",
            "Claude",
            "Desktop 崩溃缓存",
        ),
    ] {
        for entry in entries(&root.join(parent), &mut warnings) {
            if entry.file_name().to_string_lossy().starts_with(prefix) {
                result.push(target(
                    root,
                    &format!("{parent}/{}", entry.file_name().to_string_lossy()),
                    title,
                    "cache",
                    "safe",
                    "仅移除 Claude 的缓存或崩溃记录。",
                ));
            }
        }
    }
    for (name, title) in [
        ("projects", "Code 项目会话"),
        ("sessions", "Code 会话索引"),
        ("history.jsonl", "Code 输入历史"),
        ("file-history", "Code 文件历史"),
        ("debug", "Code 调试日志"),
        ("plans", "Code 会话计划"),
        ("image-cache", "Code 会话图片"),
        ("uploads", "Code 会话附件"),
        ("session-env", "Code 会话环境"),
        ("tasks", "Code 会话任务"),
        ("shell-snapshots", "Code 终端快照"),
        ("feedback-bundles", "Code 反馈归档"),
        ("feedback/drafts", "Code 反馈草稿"),
        ("dev-mods", "Code 会话扩展"),
        ("jobs", "Code 后台作业记录"),
    ] {
        result.push(target(
            root,
            &format!(".claude/{name}"),
            title,
            "history",
            "history",
            "移除本地会话或历史；不会删除实际项目文件。",
        ));
    }
    result.push(target(
        root,
        ".claude/.credentials.json",
        "Code 文件登录凭据",
        "account",
        "login",
        "退出文件凭据登录。凭据内容不会显示。",
    ));
    for (name, title) in [
        (".user-id", "Code 本地用户标识"),
        (".machine-id", "Code 本地设备标识"),
    ] {
        result.push(target(
            root,
            &format!(".claude/{name}"),
            title,
            "account",
            "login",
            "清除本地登录凭据或设备标识；不会删除服务端记录。",
        ));
    }
    let mut account = target(root, ".claude.json", "Code 账号与本地标识", "account", "login", "清除账号缓存及本地 userID、machineID；保留项目、MCP 和个人设置。再次运行时由 Claude 重新生成本地标识。");
    account.operation = Operation::Edit(config::EditKind::Account);
    result.push(account);
    for (relative, title) in [
        (
            "Library/HTTPStorages/com.anthropic.claudefordesktop",
            "Desktop HTTP 登录存储",
        ),
        (
            "Library/Preferences/com.anthropic.claudefordesktop.plist",
            "Desktop 偏好设置",
        ),
        (
            "Library/Saved Application State/com.anthropic.claudefordesktop.savedState",
            "Desktop 窗口状态",
        ),
        (
            "Library/WebKit/com.anthropic.claudefordesktop",
            "Desktop WebKit 存储",
        ),
    ] {
        result.push(target(
            root,
            relative,
            title,
            "desktop",
            "login",
            "重置桌面端登录和本地状态；服务器上的对话不会被删除。",
        ));
    }
    for relative in [
        "Library/Application Support/Claude",
        "Library/Application Support/Claude-3p",
        "Library/Application Support/com.anthropic.claudefordesktop",
    ] {
        result.extend(desktop_storage(root, &root.join(relative)));
    }
    for entry in entries(&root.join("Library/Preferences/ByHost"), &mut warnings) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("com.anthropic.claudefordesktop") && name.ends_with(".plist") {
            result.push(target(
                root,
                &format!("Library/Preferences/ByHost/{name}"),
                "Desktop 设备偏好",
                "desktop",
                "login",
                "重置桌面端在这台设备上的偏好设置。",
            ));
        }
    }
    // ----- 既有备份也是删除目标，不创建新的备份 -----
    for (relative, title) in [
        ("ClaudeBackups", "本工具与 cleanup skill 的历史备份"),
        (".claude/backups", "Code 自动配置备份"),
    ] {
        result.push(target(
            root,
            relative,
            title,
            "backups",
            "backup",
            "永久删除此目录中的既有备份与旧清理记录，不保留恢复副本。",
        ));
    }
    for entry in entries(root, &mut warnings) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(".claude.json.backup.") || name == ".claude.json.backup" {
            result.push(target(
                root,
                &name,
                "Code 历史配置备份",
                "backups",
                "backup",
                "永久删除旧配置备份，避免其中残留账号与认证信息。",
            ));
        }
    }
    for relative in [
        "Library/Application Support/Claude/ant-did",
        "Library/Application Support/Claude/ant-device-registry.json",
    ] {
        result.push(target(
            root,
            relative,
            "Desktop 本地设备标识",
            "account",
            "login",
            "仅清除本地记录；不能删除服务端账号或设备记录。",
        ));
    }
    for (relative, kind) in [
        (".claude/settings.json", config::EditKind::Settings),
        (".claude/settings.local.json", config::EditKind::Settings),
        (".zshrc", config::EditKind::Shell),
        (".zshenv", config::EditKind::Shell),
        (".zprofile", config::EditKind::Shell),
        (".bashrc", config::EditKind::Shell),
        (".bash_profile", config::EditKind::Shell),
        (".profile", config::EditKind::Shell),
        (".npmrc", config::EditKind::Npm),
        (".gitconfig", config::EditKind::Git),
    ] {
        let mut item = target(
            root,
            relative,
            &format!("配置残留 · {relative}"),
            "config",
            "config",
            "只移除预览中的代理或认证配置，保留其他内容。共享代理设置会影响其他工具，请按需选择。",
        );
        item.operation = Operation::Edit(kind);
        result.push(item);
    }
    #[cfg(target_os = "windows")]
    {
        result.retain(|item| !item.path.starts_with(root.join("Library")));
        let localappdata = win_dir(root, "LOCALAPPDATA", "AppData/Local");
        for (path, title) in [
            (localappdata.join("claude-cli-nodejs"), "Code 系统缓存"),
            (localappdata.join("Claude/logs"), "Desktop 日志"),
            (
                localappdata
                    .join("Packages/Claude_pzs8sxrjxfjjc/LocalCache/Local/claude-cli-nodejs"),
                "Code 系统缓存",
            ),
            (
                localappdata.join("Packages/Claude_pzs8sxrjxfjjc/LocalCache/Local/Claude/logs"),
                "Desktop 日志",
            ),
        ] {
            result.push(Target {
                path,
                ..target(root, "", title, "cache", "safe", "仅移除本地缓存或日志。")
            });
        }
        for desktop in desktop_roots(root) {
            for (name, title) in [
                ("Cache", "Desktop 页面缓存"),
                ("Code Cache", "Desktop 编译缓存"),
                ("GPUCache", "Desktop 图形缓存"),
                ("DawnCache", "Desktop 图形缓存"),
                ("DawnGraphiteCache", "Desktop 图形缓存"),
                ("DawnWebGPUCache", "Desktop 图形缓存"),
                ("GraphiteDawnCache", "Desktop 图形缓存"),
                ("ShaderCache", "Desktop 着色器缓存"),
                ("GrShaderCache", "Desktop 着色器缓存"),
                ("Service Worker/CacheStorage", "Desktop 离线缓存"),
                ("Service Worker/ScriptCache", "Desktop 脚本缓存"),
                ("logs", "Desktop 日志"),
                ("Crashpad", "Desktop 崩溃报告"),
            ] {
                result.push(Target {
                    path: desktop.join(name),
                    ..target(root, "", title, "cache", "safe", "仅移除本地缓存或日志。")
                });
            }
            result.extend(desktop_storage(root, &desktop));
            for name in [
                "ant-did",
                "ant-device-registry.json",
                ".machine-id",
                "machine-id",
            ] {
                result.push(Target {
                    path: desktop.join(name),
                    ..target(
                        root,
                        "",
                        "Desktop 本地设备标识",
                        "account",
                        "login",
                        "仅清除本地记录；不能删除服务端账号或设备记录。",
                    )
                });
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        result.retain(|item| !item.path.starts_with(root.join("Library")));
        let cache = xdg_dir(root, "XDG_CACHE_HOME", ".cache");
        let config = xdg_dir(root, "XDG_CONFIG_HOME", ".config");
        for path in [
            cache.join("claude-cli-nodejs"),
            config.join("Claude/Cache"),
            config.join("Claude/Code Cache"),
            config.join("Claude/GPUCache"),
            config.join("Claude/logs"),
        ] {
            let mut item = target(
                root,
                "",
                "Claude Linux 缓存",
                "cache",
                "safe",
                "仅移除 Claude 缓存或日志。",
            );
            item.path = path;
            result.push(item);
        }
        // Linux Desktop packages are third-party; do not reset the whole config directory.
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    result.dedup_by(|a, b| a.path == b.path);
    (result, warnings)
}

#[cfg(target_os = "linux")]
fn xdg_dir(root: &Path, variable: &str, fallback: &str) -> PathBuf {
    // Fixture scans must not escape into the real user's XDG directories.
    if super::home().ok().as_deref() == Some(root) {
        if let Some(path) = std::env::var_os(variable)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
        {
            return path;
        }
    }
    root.join(fallback)
}

// ----- 完整统计；不读取文件正文，不进入链接目标 -----
pub(super) fn measure(path: &Path) -> Result<(u64, usize, Vec<u8>), String> {
    use sha2::{Digest, Sha256};
    let mut pending = vec![path.to_path_buf()];
    let mut hash = Sha256::new();
    let (mut bytes, mut files) = (0u64, 0usize);
    while let Some(path) = pending.pop() {
        let failure = |error: std::io::Error| {
            format!(
                "目录无法完整读取：{}：{error}。请检查权限、退出 Claude 后重新扫描。",
                path.display()
            )
        };
        let meta = fs::symlink_metadata(&path).map_err(failure)?;
        hash.update(path.as_os_str().as_encoded_bytes());
        hash.update([0]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            for field in [
                meta.dev(),
                meta.ino(),
                meta.mtime() as u64,
                meta.mtime_nsec() as u64,
                meta.ctime() as u64,
                meta.ctime_nsec() as u64,
                meta.len(),
            ] {
                hash.update(field.to_le_bytes());
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            for field in [
                meta.file_attributes() as u64,
                meta.last_write_time(),
                meta.creation_time(),
                meta.file_size(),
            ] {
                hash.update(field.to_le_bytes());
            }
        }
        if meta.is_file() {
            bytes = bytes.saturating_add(meta.len());
            files += 1;
        } else if meta.is_dir() && !linked(&meta) {
            let entries = fs::read_dir(&path).map_err(failure)?;
            let mut children = entries
                .map(|entry| entry.map(|e| e.path()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(failure)?;
            children.sort();
            pending.extend(children);
        }
    }
    Ok((bytes, files, hash.finalize().to_vec()))
}
pub(super) fn org_files(root: &Path, org: &str) -> Result<(Vec<PathBuf>, bool), String> {
    use std::io::{BufRead, BufReader};
    if org.len() < 8
        || org.len() > 100
        || !org
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("组织编号须为 8–100 位字母、数字、横线或下划线。".into());
    }
    let mut pending = Vec::new();
    let mut candidates = vec![root.join(".claude/projects"), root.join(".claude/sessions")];
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    for desktop in desktop_roots(root) {
        for sub in [
            "local_sessions",
            "conversations",
            "claude-code-sessions",
            "local-agent-mode-sessions",
        ] {
            candidates.push(desktop.join(sub));
        }
    }
    for p in candidates {
        if canonicalize(&p).ok().as_deref() == Some(p.as_path()) {
            pending.push((p, 0));
        }
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let (mut files, mut seen, mut complete) = (Vec::new(), 0usize, true);
    while let Some((path, depth)) = pending.pop() {
        seen += 1;
        if seen > 20_000 || Instant::now() > deadline {
            return Ok((files, false));
        }
        let Ok(meta) = fs::symlink_metadata(&path) else {
            complete = false;
            continue;
        };
        #[cfg(unix)]
        let wrong_user = meta.uid() != unsafe { getuid() };
        #[cfg(windows)]
        let wrong_user = !crate::windows::owned(&path);
        if wrong_user || linked(&meta) {
            continue;
        }
        if meta.is_dir() {
            if depth >= 10 {
                complete = false;
                continue;
            }
            let Ok(entries) = fs::read_dir(&path) else {
                complete = false;
                continue;
            };
            for entry in entries.take(20_001) {
                match entry {
                    Ok(entry) => pending.push((entry.path(), depth + 1)),
                    Err(_) => complete = false,
                }
            }
            if pending.len() > 20_000 {
                return Ok((files, false));
            }
        } else if meta.is_file()
            && matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("json" | "jsonl" | "log" | "txt")
            )
        {
            if meta.len() > 64 * 1024 * 1024 {
                complete = false;
                continue;
            }
            let Ok(file) = fs::File::open(&path) else {
                complete = false;
                continue;
            };
            let mut reader = BufReader::new(file);
            let mut line = Vec::new();
            loop {
                if Instant::now() > deadline {
                    return Ok((files, false));
                }
                line.clear();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let boundary = |b: u8| !b.is_ascii_alphanumeric() && b != b'-' && b != b'_';
                        if line.windows(org.len()).enumerate().any(|(offset, value)| {
                            value == org.as_bytes()
                                && (offset == 0 || boundary(line[offset - 1]))
                                && (offset + org.len() == line.len()
                                    || boundary(line[offset + org.len()]))
                        }) {
                            files.push(path);
                            break;
                        }
                    }
                    Err(_) => {
                        complete = false;
                        break;
                    }
                }
            }
        }
    }
    Ok((files, complete))
}
