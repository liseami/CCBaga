// [INPUT]: 隔离临时目录/Windows 子进程 AppData、脱敏配置、清理事务与当前用户官方原生 CLI 版本路径。
// [OUTPUT]: 白名单、文件夹范围、配置保留、组织检索、直接删除、大目录完整统计、读失败拒绝快照、快照保护、真实日志次序与原生版本/可丢弃 Node CLI 进程的识别/终止/隔离验证。
// [POS]: 仅操作测试数据，进程样本由本机编译并握手确认启动，避免重用 Apple 平台二进制，不访问现有 Claude 数据。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
#[cfg(windows)]
fn junction(source: &Path, target: &Path) {
    assert!(std::process::Command::new("cmd.exe").args(["/D", "/C", "mklink", "/J"]).arg(target.to_string_lossy().replace('/', "\\")).arg(source.to_string_lossy().replace('/', "\\")).output().unwrap().status.success());
}

#[test]
fn folder_scope_preserves_project_assets_and_refuses_outside_targets() {
    let f = Fixture::new();
    let outside = Fixture::new();
    f.file("repo/.claude/projects/session.jsonl", "history");
    f.file("repo/.claude/cache/data", "cache");
    f.file("repo/.claude/settings.local.json", "permissions");
    f.file("repo/.claude/.credentials.json", "credential");
    f.file("repo/.claude/skills/SKILL.md", "skill");
    f.file("repo/CLAUDE.md", "instructions");
    f.file("repo/.claude.json.backup.1", "old configuration");
    f.file(
        "node_modules/ignored/.claude/projects/session.jsonl",
        "dependency",
    );
    outside.file(".claude/projects/session.jsonl", "outside");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside.0, f.0.join("linked")).unwrap();
    #[cfg(windows)]
    junction(&outside.0, &f.0.join("linked"));
    let plan = folder::collect(&f.0, &Reporter::channel(None)).unwrap();
    assert_eq!(plan.items.len(), 3);
    assert_eq!(plan.folder, f.0.to_string_lossy());
    assert!(plan.items.iter().all(|i| identity(i, &f.0).is_ok()));
    assert!(plan.items.iter().all(|i| identity(i, &outside.0).is_err()));
    let ids = plan.items.iter().map(|i| i.id.clone()).collect::<Vec<_>>();
    assert!(
        perform_items(
            &f.0,
            &selected(&plan, &ids).unwrap(),
            &Cleaner::default(),
            || Ok(false),
            &Reporter::channel(None)
        )
        .unwrap()
        .completed
    );
    for name in [
        "repo/.claude/settings.local.json",
        "repo/.claude/.credentials.json",
        "repo/.claude/skills/SKILL.md",
        "repo/CLAUDE.md",
        "node_modules/ignored/.claude/projects/session.jsonl",
    ] {
        assert!(f.0.join(name).exists());
    }
    assert!(outside.0.join(".claude/projects/session.jsonl").exists());
    assert!(folder::validate(&f.0.join("linked")).is_err());
    assert!(folder::validate(&f.0.join("repo/CLAUDE.md")).is_err());
}
#[test]
fn dropped_session_subfolder_keeps_sibling_sessions() {
    let f = Fixture::new();
    f.file(".claude/projects/a/session.jsonl", "a");
    f.file(".claude/projects/b/session.jsonl", "b");
    let root = f.0.join(".claude/projects/a");
    let plan = folder::collect(&root, &Reporter::channel(None)).unwrap();
    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].path, root.to_string_lossy());
    perform_items(
        &root,
        &plan.items,
        &Cleaner::default(),
        || Ok(false),
        &Reporter::channel(None),
    )
    .unwrap();
    assert!(f.0.join(".claude/projects/b/session.jsonl").exists());
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pid = std::process::id();
        let p = std::env::temp_dir().join(format!("rf-cleaner-test-{pid}-{id}-{unique}"));
        fs::create_dir(&p).unwrap();
        #[cfg(windows)]
        crate::windows::fixture_owner(&p);
        Self(canonicalize(p).unwrap())
    }
    fn file(&self, name: &str, value: &str) {
        let p = self.0.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, value).unwrap();
        #[cfg(windows)]
        for path in p.ancestors().take_while(|path| path.starts_with(&self.0)) { crate::windows::fixture_owner(path); }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn discovers_supported_data_and_preserves_personal_assets() {
    let f = Fixture::new();
    f.file(".claude/cache/data", "cache");
    f.file(".claude/projects/project/session.jsonl", "history");
    f.file(".claude/skills/skill/SKILL.md", "keep");
    f.file(".claude/plugins/plugin", "keep");
    f.file(
        ".claude/settings.json",
        r#"{"permissions":{"allow":["Read"]}}"#,
    );
    f.file(
        ".claude.json",
        r#"{"userID":"old","projects":{"/my-project":{}}}"#,
    );
    f.file("unrelated/.claude.json", r#"{"userID":"unrelated"}"#);
    let plan = collect(&f.0, None).unwrap();
    assert!(plan.items.iter().any(|i| i.group == "cache"));
    assert!(plan.items.iter().any(|i| i.group == "history"));
    assert!(plan.items.iter().any(|i| i.group == "account"));
    assert!(!plan
        .items
        .iter()
        .any(|i| ["skills", "plugins", "settings.json", "unrelated"]
            .iter()
            .any(|name| i.path.contains(name))));
    assert_eq!(
        fs::read_to_string(f.0.join(".claude/skills/skill/SKILL.md")).unwrap(),
        "keep"
    );
}
#[test]
fn ignores_linked_targets() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("outside")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(f.0.join("outside"), f.0.join(".claude")).unwrap();
    #[cfg(windows)]
    junction(&f.0.join("outside"), &f.0.join(".claude"));
    assert!(collect(&f.0, None).unwrap().items.is_empty());
}
#[test]
fn rejects_stale_or_unknown_targets() {
    let f = Fixture::new();
    f.file(".claude.json", r#"{"userID":"old"}"#);
    let plan = collect(&f.0, None).unwrap();
    let i = &plan.items[0];
    assert!(identity(i, &f.0).is_ok());
    assert!(selected(&plan, &["unknown".into()]).is_err());
    assert_eq!(
        selected(&plan, &[i.id.clone(), i.id.clone()])
            .unwrap()
            .len(),
        1
    );
    fs::rename(f.0.join(".claude.json"), f.0.join("old")).unwrap();
    f.file(".claude.json", r#"{"userID":"old"}"#);
    assert!(identity(i, &f.0).is_err());
}
#[cfg(unix)]
#[test]
fn matches_only_current_user_and_exact_claude_processes() {
    let root = home().unwrap();
    let official = format!(
        "501 46 {}/.local/share/claude/versions/2.1.296",
        root.display()
    );
    assert_eq!(
        process_row(&official, &[], 501).unwrap().name,
        "Claude Code"
    );
    assert!(process_row("501 46 /tmp/.local/share/claude/versions/2.1.296", &[], 501).is_none());
    assert!(process_row(
        &format!(
            "501 46 {}/.local/share/claude/versions/custom",
            root.display()
        ),
        &[],
        501
    )
    .is_none());
    assert!(process_row("501 42 /tmp/claude", &[], 501).is_some());
    assert!(process_row("501 43 /tmp/claude.exe", &[], 501).is_some());
    assert!(process_row("502 42 /tmp/claude", &[], 501).is_none());
    assert!(process_row("501 42 /tmp/claude-code-vm", &[], 501).is_some());
    assert!(process_row("501 42 /tmp/.claude/worktrees/im-launch", &[], 501).is_none());
    assert!(process_row("501 42 /tmp/claude-code-vm-backup", &[], 501).is_none());
    assert!(process_row("501 1 /tmp/claude", &[], 501).is_none());
    let app = process_row(
        "501 42 /Applications/Claude.app/Contents/MacOS/Claude",
        &[42],
        501,
    )
    .unwrap();
    assert_eq!(app.name, "Claude Desktop");
    let helper = process_row("501 44 /Applications/Claude.app/Contents/Frameworks/Claude Helper (Renderer).app/Contents/MacOS/Claude Helper (Renderer)", &[44], 501).unwrap();
    assert_eq!(helper.name, "Claude Helper (Renderer)");
    assert!(process_row("501 45 /usr/bin/node", &[], 501).is_none());
    assert_eq!(
        process_row("501 45 /usr/bin/node", &[45], 501)
            .unwrap()
            .name,
        "Claude Code"
    );
}
#[cfg(unix)]
#[test]
fn ends_only_the_captured_disposable_process() {
    let f = Fixture::new();
    let executable = f.0.join("claude");
    f.file(
        "process.c",
        "#include <stdio.h>\n#include <unistd.h>\nint main(void) { puts(\"READY\"); fflush(stdout); sleep(30); return 0; }\n",
    );
    // ----- 自编译样本并等待启动握手；不复制带有平台身份的系统二进制 -----
    let compilation = std::process::Command::new("cc")
        .arg(f.0.join("process.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "临时进程编译失败：{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let mut child = std::process::Command::new(&executable)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut ready = String::new();
    std::io::BufRead::read_line(
        &mut std::io::BufReader::new(child.stdout.take().unwrap()),
        &mut ready,
    )
    .unwrap();
    assert_eq!(ready, "READY\n", "测试样本必须完成启动后再验证进程边界");
    let target = ClaudeProcess {
        pid: child.id() as i32,
        name: "claude".into(),
        executable: executable.to_string_lossy().into_owned(),
    };
    let mismatched = ClaudeProcess {
        executable: "/wrong/claude".into(),
        ..target.clone()
    };
    signal_same_process(&mismatched, 9).unwrap();
    assert!(child.try_wait().unwrap().is_none());
    signal_same_process(&target, 9).unwrap();
    let status = child.wait().unwrap();
    assert!(!status.success());
}

#[test]
fn nested_file_changes_invalidate_directory_snapshot() {
    let f = Fixture::new();
    f.file(".claude/cache/nested/file", "before");
    let scan = collect(&f.0, None).unwrap();
    let item = scan.items.iter().find(|i| i.group == "cache").unwrap();
    assert!(identity(item, &f.0).is_ok());
    f.file(".claude/cache/nested/file", "changed");
    assert!(identity(item, &f.0).is_err());
}
#[cfg(target_os = "macos")]
#[test]
fn desktop_cleaning_preserves_personal_configuration() {
    let f = Fixture::new();
    f.file("Library/Application Support/Claude/Cache/file", "cache");
    let scan = collect(&f.0, None).unwrap();
    let all = selected(
        &scan,
        &scan.items.iter().map(|i| i.id.clone()).collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].path.ends_with("/Claude/Cache"));
    f.file("Library/Application Support/Claude/claude_desktop_config.json", "personal MCP configuration");
    f.file("Library/Application Support/Claude/plugins/personal", "personal plugin");
    perform_items(&f.0, &all, &Cleaner::default(), || Ok(false), &Reporter::channel(None)).unwrap();
    assert!(f.0.join("Library/Application Support/Claude/claude_desktop_config.json").exists());
    assert!(f.0.join("Library/Application Support/Claude/plugins/personal").exists());
}
#[test]
fn account_reset_preserves_projects_and_mcp_and_removes_only_known_account_keys() {
    let original = r#"{"oauthAccount":{"token":"test-only"},"userID":"old","machineID":"old","projects":{"/source":{}},"mcpServers":{"server":{}},"unknown":9}"#;
    let (updated, keys) = config::transform(original, config::EditKind::Account).unwrap();
    let value: serde_json::Value = serde_json::from_str(&updated).unwrap();
    assert!(value.get("oauthAccount").is_none());
    assert!(value.get("userID").is_none());
    assert!(value["projects"]["/source"].is_object());
    assert!(value["mcpServers"]["server"].is_object());
    assert_eq!(value["unknown"], 9);
    assert!(!keys.join(" ").contains("test-only"));
}
#[test]
fn settings_cleanup_preserves_telemetry_hooks_permissions_and_unrelated_secrets() {
    let original = r#"{"env":{"ANTHROPIC_API_KEY":"test-only","HTTPS_PROXY":"test-proxy","DISABLE_TELEMETRY":"1","CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC":"1","OTHER_SECRET":"keep"},"hooks":{"Stop":[]},"permissions":{"allow":["Read"]},"model":"keep"}"#;
    let (updated, _) = config::transform(original, config::EditKind::Settings).unwrap();
    let value: serde_json::Value = serde_json::from_str(&updated).unwrap();
    assert!(value["env"].get("ANTHROPIC_API_KEY").is_none());
    assert_eq!(value["env"]["DISABLE_TELEMETRY"], "1");
    assert_eq!(value["env"]["OTHER_SECRET"], "keep");
    assert_eq!(value["hooks"]["Stop"], serde_json::json!([]));
    assert_eq!(value["model"], "keep");
    assert!(config::transform("{invalid", config::EditKind::Settings).is_err());
    assert!(config::transform(r#"{"env":[]}"#, config::EditKind::Settings).is_err());
}
#[test]
fn shell_and_git_cleanups_do_not_remove_unrelated_commands() {
    let shell = "# HTTPS_PROXY=comment\nexport HTTPS_PROXY='test-only'\nexport ANTHROPIC_API_KEY='test-only'\nexport HTTPS_PROXY=foo; echo keep\nexport KEEP=1\necho proxy\n";
    let (updated, keys) = config::transform(shell, config::EditKind::Shell).unwrap();
    assert_eq!(keys, vec!["ANTHROPIC_API_KEY", "HTTPS_PROXY"]);
    assert!(updated.contains("HTTPS_PROXY=comment"));
    assert!(updated.contains("foo; echo keep"));
    assert!(updated.contains("export KEEP=1"));
    let git = "[http]\n proxy = test-only\n sslVerify = true\n[remote \"origin\"]\n proxy = keep\n";
    let (updated, _) = config::transform(git, config::EditKind::Git).unwrap();
    assert!(!updated.contains("test-only"));
    assert!(updated.contains("sslVerify = true"));
    assert!(updated.contains("proxy = keep"));
}
#[test]
fn organization_search_stays_in_claude_sessions_and_does_not_follow_links() {
    let f = Fixture::new();
    let org = "org-12345678";
    f.file(
        ".claude/projects/a/match.jsonl",
        r#"{"org_id":"org-12345678"}"#,
    );
    f.file(
        ".claude/projects/a/other.jsonl",
        r#"{"org_id":"org-123456789"}"#,
    );
    f.file("source/private.json", r#"{"org_id":"org-12345678"}"#);
    #[cfg(unix)]
    std::os::unix::fs::symlink(f.0.join("source"), f.0.join(".claude/projects/link")).unwrap();
    #[cfg(windows)]
    junction(&f.0.join("source"), &f.0.join(".claude/projects/link"));
    let (files, complete) = targets::org_files(&f.0, org).unwrap();
    assert!(complete);
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("match.jsonl"));
    assert!(targets::org_files(&f.0, "../source").is_err());
}
#[test]
fn direct_cleanup_removes_selected_data_and_backups_without_copies() {
    let f = Fixture::new();
    f.file(".claude/cache/data", "cache");
    f.file(".claude/projects/a/history.jsonl", "conversation");
    f.file(".claude/skills/keep/SKILL.md", "skill");
    f.file(".claude/plugins/keep", "plugin");
    f.file("ClaudeBackups/requirement-french-old/data", "old backup");
    f.file(".claude/backups/.claude.json.backup.1", "old configuration");
    f.file(".claude.json.backup.2", "old configuration");
    f.file(
        ".claude.json",
        r#"{"userID":"old","mcpServers":{"keep":{}}}"#,
    );
    f.file(".npmrc", "proxy=test-only\nregistry=keep\n");
    f.file("outside-secret", "outside");
    #[cfg(unix)]
    std::os::unix::fs::symlink(f.0.join("outside-secret"), f.0.join(".claude/cache/link")).unwrap();
    let scan = collect(&f.0, None).unwrap();
    assert_eq!(
        scan.items.iter().filter(|i| i.group == "backups").count(),
        3
    );
    let items = selected(
        &scan,
        &scan.items.iter().map(|i| i.id.clone()).collect::<Vec<_>>(),
    )
    .unwrap();
    let reporter = Reporter::channel(None);
    let result = perform_items(&f.0, &items, &Cleaner::default(), || Ok(false), &reporter).unwrap();
    assert!(result.completed);
    assert_eq!(result.cleaned, items.len());
    for path in [
        ".claude/cache",
        ".claude/projects",
        "ClaudeBackups",
        ".claude/backups",
        ".claude.json.backup.2",
    ] {
        assert!(!f.0.join(path).exists());
    }
    assert_eq!(
        fs::read_to_string(f.0.join("outside-secret")).unwrap(),
        "outside"
    );
    assert_eq!(
        fs::read_to_string(f.0.join(".claude/skills/keep/SKILL.md")).unwrap(),
        "skill"
    );
    assert_eq!(
        fs::read_to_string(f.0.join(".claude/plugins/keep")).unwrap(),
        "plugin"
    );
    assert_eq!(
        fs::read_to_string(f.0.join(".npmrc")).unwrap(),
        "registry=keep\n"
    );
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(f.0.join(".claude.json")).unwrap()).unwrap();
    assert!(value.get("userID").is_none());
    assert!(value["mcpServers"]["keep"].is_object());
    let events = reporter.events();
    assert!(!events
        .iter()
        .any(|e| e.stage() == "backup" || e.stage() == "restore"));
    let success_counts: Vec<_> = events
        .iter()
        .filter(|e| e.message().starts_with("已清理："))
        .map(|e| e.current())
        .collect();
    assert_eq!(success_counts, (1..=items.len()).collect::<Vec<_>>());
    assert!(!events
        .iter()
        .any(|e| e.message().contains("test-only") || e.message().contains("conversation")));
    assert!(collect(&f.0, None).unwrap().items.is_empty());
}
#[test]
fn interrupted_cleanup_reports_partial_and_keeps_remaining_data() {
    let f = Fixture::new();
    f.file(".claude/cache/data", "cache");
    f.file(".claude/projects/a/history.jsonl", "history");
    let scan = collect(&f.0, None).unwrap();
    let calls = std::cell::Cell::new(0);
    let reporter = Reporter::channel(None);
    let result = perform_items(
        &f.0,
        &scan.items,
        &Cleaner::default(),
        || {
            let count = calls.get();
            calls.set(count + 1);
            Ok(count > 0)
        },
        &reporter,
    )
    .unwrap();
    let events = reporter.events();
    let failed = events
        .iter()
        .position(|e| e.stage() == "clean" && e.status() == "error")
        .unwrap();
    assert_eq!(events[failed].current(), 1);
    assert!(!events[failed + 1..]
        .iter()
        .any(|e| e.status() == "running" && e.stage() == "clean"));
    assert!(!result.completed);
    assert_eq!(result.cleaned, 1);
    assert_eq!(result.errors.len(), 1);
    assert!(f.0.join(".claude/projects/a/history.jsonl").exists());
    assert!(!f.0.join("ClaudeBackups").exists());
}
#[test]
fn stale_snapshot_stops_before_any_delete() {
    let f = Fixture::new();
    f.file(".claude/cache/data", "keep");
    let scan = collect(&f.0, None).unwrap();
    f.file(".claude/cache/data", "new");
    let reporter = Reporter::channel(None);
    let error = perform_items(
        &f.0,
        &scan.items,
        &Cleaner::default(),
        || Ok(false),
        &reporter,
    )
    .err()
    .unwrap();
    reporter.fail(&error);
    assert_eq!(reporter.events().last().unwrap().stage(), "validate");
    assert!(!reporter.events().iter().any(|e| e.stage() == "clean"));
    assert_eq!(
        fs::read_to_string(f.0.join(".claude/cache/data")).unwrap(),
        "new"
    );
}

#[test]
fn node_claude_entry_is_identified_and_unrelated_node_is_preserved() {
    struct Children(Vec<std::process::Child>);
    impl Drop for Children {
        fn drop(&mut self) {
            for child in &mut self.0 {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    let f = Fixture::new();
    f.file(
        "node_modules/@anthropic-ai/claude-code/cli.js",
        "setInterval(()=>{},1000)",
    );
    f.file("other.js", "setInterval(()=>{},1000)");
    let mut children = Children(vec![
        std::process::Command::new("node")
            .arg(f.0.join("node_modules/@anthropic-ai/claude-code/cli.js"))
            .spawn()
            .unwrap(),
        std::process::Command::new("node")
            .arg(f.0.join("other.js"))
            .spawn()
            .unwrap(),
    ]);
    let pid = children.0[0].id() as i32;
    let other = children.0[1].id() as i32;
    let mut target = None;
    for _ in 0..20 {
        let current = processes().unwrap();
        assert!(!current.iter().any(|p| p.pid == other));
        target = current.into_iter().find(|p| p.pid == pid);
        if target.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let target = target.expect("必须识别 Node 启动的官方 CLI 入口");
    assert_eq!(target.name, "Claude Code");
    #[cfg(windows)] {
        let mut stale = target.clone(); stale.created ^= 1;
        signal_same_process(&stale, 9).unwrap();
        assert!(children.0[0].try_wait().unwrap().is_none());
        let serialized = serde_json::to_string(&target).unwrap();
        assert!(!serialized.contains("created") && !serialized.contains("executable"));
    }
    signal_same_process(&target, 9).unwrap();
    assert!(!children.0[0].wait().unwrap().success());
    assert!(children.0[1].try_wait().unwrap().is_none());
}

#[cfg(target_os = "linux")]
#[test]
fn linux_cleanup_uses_xdg_cache_paths_and_preserves_desktop_configuration() {
    let f = Fixture::new();
    f.file(".cache/claude-cli-nodejs/data", "cache");
    f.file(".config/Claude/Cache/data", "cache");
    f.file(".config/Claude/claude_desktop_config.json", "settings");
    f.file(
        "Library/Caches/claude-cli-nodejs/data",
        "not a Linux target",
    );
    let plan = collect(&f.0, None).unwrap();
    assert_eq!(plan.items.len(), 2);
    let state = Cleaner::default();
    let result = perform_items(
        &f.0,
        &plan.items,
        &state,
        || Ok(false),
        &Reporter::channel(None),
    )
    .unwrap();
    assert!(result.completed);
    assert!(f
        .0
        .join(".config/Claude/claude_desktop_config.json")
        .exists());
    assert!(f.0.join("Library/Caches/claude-cli-nodejs/data").exists());
}

#[cfg(windows)]
#[test]
fn windows_desktop_storage_preserves_mcp_plugins_and_programs() {
    let f = Fixture::new();
    for desktop in ["AppData/Roaming/Claude", "AppData/Local/Packages/Claude_pzs8sxrjxfjjc/LocalCache/Roaming/Claude"] {
        f.file(&format!("{desktop}/Cache/data"), "cache");
        f.file(&format!("{desktop}/Local Storage/token"), "local login state");
        f.file(&format!("{desktop}/claude_desktop_config.json"), "personal MCP");
        f.file(&format!("{desktop}/plugins/personal"), "personal plugin");
    }
    f.file("AppData/Local/AnthropicClaude/Claude.exe", "installed program");
    let plan = collect(&f.0, None).unwrap();
    perform_items(&f.0, &plan.items, &Cleaner::default(), || Ok(false), &Reporter::channel(None)).unwrap();
    for desktop in ["AppData/Roaming/Claude", "AppData/Local/Packages/Claude_pzs8sxrjxfjjc/LocalCache/Roaming/Claude"] {
        assert!(!f.0.join(desktop).join("Cache").exists());
        assert!(!f.0.join(desktop).join("Local Storage").exists());
        assert!(f.0.join(desktop).join("claude_desktop_config.json").exists());
        assert!(f.0.join(desktop).join("plugins/personal").exists());
    }
    assert!(f.0.join("AppData/Local/AnthropicClaude/Claude.exe").exists());
}

#[test]
fn large_directory_is_fully_measured_and_cleaned_with_snapshot_protection() {
    let f = Fixture::new();
    f.file(".claude/cache/nested/keep-counted", "data");
    f.file(".claude/skills/personal/SKILL.md", "keep");
    let cache = f.0.join(".claude/cache");
    for index in 0..30_005 {
        fs::write(cache.join(format!("entry-{index:05}")), b"x").unwrap();
    }
    let scan = collect(&f.0, None).unwrap();
    let item = scan.items.iter().find(|item| item.path == cache.to_string_lossy()).unwrap();
    assert!(item.exact);
    assert_eq!((item.files, item.bytes), (30_006, 30_009));
    assert!(identity(item, &f.0).is_ok());
    fs::write(cache.join("entry-30004"), b"changed").unwrap();
    assert!(identity(item, &f.0).is_err());
    let scan = collect(&f.0, None).unwrap();
    let result = perform_items(&f.0, &scan.items, &Cleaner::default(), || Ok(false), &Reporter::channel(None)).unwrap();
    assert!(result.completed && !cache.exists());
    assert!(f.0.join(".claude/skills/personal/SKILL.md").exists());
}

#[cfg(unix)]
#[test]
fn unreadable_directory_is_reported_and_never_offered_for_deletion() {
    use std::os::unix::fs::PermissionsExt;
    if unsafe { getuid() } == 0 { return; }
    let f = Fixture::new();
    f.file(".claude/cache/blocked/data", "keep");
    let blocked = f.0.join(".claude/cache/blocked");
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();
    let scan = collect(&f.0, None).unwrap();
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(!scan.items.iter().any(|item| item.path == f.0.join(".claude/cache").to_string_lossy()));
    assert!(scan.warnings.iter().any(|warning| warning.contains("目录无法完整读取") && warning.contains("blocked")));
    assert!(blocked.join("data").exists());
}

#[cfg(windows)]
#[test]
fn windows_redirected_appdata_remains_whitelisted() {
    // ----- 子进程隔离环境变量，避免并行回归改变真实用户目录 -----
    if std::env::var_os("CC_REDIRECTED_APPDATA_FIXTURE").is_none() {
        let home = Fixture::new();
        let outside = Fixture::new();
        outside.file("roaming/Claude/Cookies", "disposable cookie fixture");
        outside.file("roaming/Claude/local_sessions/record.json", r#"{"org":"org-fixture-123"}"#);
        outside.file("local/claude-cli-nodejs/cache", "disposable cache fixture");
        outside.file("unrelated/data", "keep");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "cleaner::tests::windows_redirected_appdata_remains_whitelisted", "--nocapture"])
            .env("CC_REDIRECTED_APPDATA_FIXTURE", &outside.0)
            .env("USERPROFILE", &home.0).env("HOME", &home.0)
            .env("APPDATA", outside.0.join("roaming"))
            .env("LOCALAPPDATA", outside.0.join("local"))
            .status().unwrap();
        assert!(status.success());
        assert_eq!(fs::read_to_string(outside.0.join("unrelated/data")).unwrap(), "keep");
        return;
    }
    let root = home().unwrap();
    let outside = PathBuf::from(std::env::var_os("CC_REDIRECTED_APPDATA_FIXTURE").unwrap());
    let scan = current_scan(&root, None).unwrap();
    let cookie = scan.items.iter().find(|item| Path::new(&item.path) == outside.join("roaming/Claude/Cookies")).unwrap();
    assert!(identity(cookie, &root).is_ok());
    let mut unrelated = cookie.clone();
    unrelated.path = outside.join("unrelated/data").to_string_lossy().into();
    assert!(identity(&unrelated, &root).is_err());
    let (files, complete) = targets::org_files(&root, "org-fixture-123").unwrap();
    assert!(complete);
    assert_eq!(files, vec![outside.join("roaming/Claude/local_sessions/record.json")]);
    let sources = super::identity::sources(&root, &crate::identities::Policy::default()).unwrap();
    assert!(sources.iter().any(|source| source.path == Path::new(&cookie.path)));
    assert!(sources.iter().all(|source| super::identity::allowed(&root, source)));
}
