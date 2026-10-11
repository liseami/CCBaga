// [INPUT]: 当前用户目录、Claude 清理白名单与可选组织编号。
// [OUTPUT]: 分类清理目标、受限目录统计及旧组织文件匹配。
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
pub(super) fn definitions(root: &Path) -> Vec<Target> {
    let mut result = Vec::new();
    for (name, title) in [
        ("cache", "Code 缓存"),
        ("stats-cache.json", "使用统计缓存"),
        ("telemetry", "本地遥测缓存"),
        ("usage-data", "用量缓存"),
        ("usage.jsonl", "用量记录"),
        ("usage.with-fix.jsonl", "用量修正记录"),
        ("paste-cache", "粘贴缓存"),
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
        if let Ok(entries) = fs::read_dir(root.join(parent)) {
            for entry in entries.flatten().take(20_000) {
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
    }
    for (name, title) in [
        ("projects", "Code 项目会话"),
        ("sessions", "Code 会话索引"),
        ("history.jsonl", "Code 输入历史"),
        ("file-history", "Code 文件历史"),
        ("debug", "Code 调试日志"),
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
    let mut account = target(root, ".claude.json", "Code 账号与本地标识", "account", "login", "清除账号缓存及本地 userID、machineID；保留项目、MCP 和个人设置。再次运行时由 Claude 重新生成本地标识。");
    account.operation = Operation::Edit(config::EditKind::Account);
    result.push(account);
    for (relative, title) in [
        ("Library/Application Support/Claude", "Desktop 完整本地数据"),
        (
            "Library/Application Support/Claude-3p",
            "Desktop 第三方配置数据",
        ),
        (
            "Library/Application Support/com.anthropic.claudefordesktop",
            "Desktop 应用支持数据",
        ),
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
    if let Ok(entries) = fs::read_dir(root.join("Library/Preferences/ByHost")) {
        for entry in entries.flatten().take(20_000) {
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
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten().take(20_000) {
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
    result.sort_by(|a, b| a.path.cmp(&b.path));
    result.dedup_by(|a, b| a.path == b.path);
    result
}

// ----- 受限统计；链接只计入条目，不进入链接目标 -----
pub(super) fn measure(path: &Path) -> (u64, usize, bool, Vec<u8>) {
    use sha2::{Digest, Sha256};
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut pending = vec![path.to_path_buf()];
    let mut hash = Sha256::new();
    let (mut bytes, mut files, mut seen) = (0u64, 0usize, 0usize);
    while let Some(path) = pending.pop() {
        seen += 1;
        if seen > 30_000 || Instant::now() > deadline {
            return (bytes, files, false, hash.finalize().to_vec());
        }
        let Ok(meta) = fs::symlink_metadata(&path) else { return (bytes, files, false, hash.finalize().to_vec()); };
        hash.update(path.as_os_str().as_encoded_bytes());
        hash.update([0]);
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
        if meta.is_file() {
            bytes = bytes.saturating_add(meta.len());
            files += 1;
        } else if meta.is_dir() && !meta.file_type().is_symlink() {
            let Ok(entries) = fs::read_dir(&path) else { return (bytes, files, false, hash.finalize().to_vec()); };
            let mut children = Vec::new();
            for entry in entries.take(30_001) {
                let Ok(entry) = entry else { return (bytes, files, false, hash.finalize().to_vec()); };
                children.push(entry.path());
            }
            children.sort();
            pending.extend(children);
            if pending.len() > 30_000 {
                return (bytes, files, false, hash.finalize().to_vec());
            }
        }
    }
    (bytes, files, true, hash.finalize().to_vec())
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
    for relative in [
        ".claude/projects",
        ".claude/sessions",
        "Library/Application Support/Claude/local_sessions",
        "Library/Application Support/Claude/conversations",
    ] {
        let p = root.join(relative);
        if fs::canonicalize(&p).ok().as_deref() == Some(p.as_path()) {
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
        let Ok(meta)=fs::symlink_metadata(&path) else { complete=false;continue; };
        if meta.uid() != unsafe { getuid() } || meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            if depth >= 10 {
                complete = false;
                continue;
            }
            let Ok(entries)=fs::read_dir(&path) else { complete=false;continue; };
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
            let Ok(file)=fs::File::open(&path) else { complete=false;continue; };
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
