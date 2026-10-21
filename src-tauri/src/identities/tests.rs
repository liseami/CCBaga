// [INPUT]: 身份保险库与临时文件系统样本。
// [OUTPUT]: SQLite 策略持久化、完整认证归档、生成型调试链接跳过、真实阶段/局部完成计数/回滚事件与 Windows 超长暂存路径回归，不读取真实 Claude 凭据。
// [POS]: 身份管理的离线验证入口；平台真实登入需用户验收。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
#[cfg(windows)]
use dunce::canonicalize;
#[cfg(not(windows))]
use std::fs::canonicalize;
fn fixture_write(path: impl AsRef<std::path::Path>, bytes: impl AsRef<[u8]>) -> Result<(), String> {
    private::write(path.as_ref(), bytes.as_ref())?;
    assert_eq!(
        canonicalize(path.as_ref()).unwrap(),
        path.as_ref(),
        "fixture must use canonical paths"
    );
    Ok(())
}
#[test]
fn operation_progress_tracks_completed_sources_and_explicit_rollback() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-progress-{}", new_id().unwrap()));
    let home = folder.join("home");
    private::directory(&home.join(".claude/cache")).unwrap();
    fixture_write(home.join(".claude/cache/entry"), "fixture").unwrap();
    fixture_write(
        home.join(".claude.json"),
        r#"{"oauthAccount":{"accountUuid":"fixture-account"}}"#,
    )
    .unwrap();
    let mut store = store::Store::open(&folder.join("vault"), &home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .switch("empty", "Fixture", &Policy::default(), &reporter)
        .unwrap();
    let events = reporter
        .events()
        .into_iter()
        .map(|event| serde_json::to_value(event).unwrap())
        .collect::<Vec<_>>();
    let position = |stage: &str, status: &str| {
        events
            .iter()
            .position(|event| event["stage"] == stage && event["status"] == status)
            .unwrap()
    };
    assert!(position("compress", "running") < position("compress", "ok"));
    assert!(position("compress", "ok") < position("verify_archive", "running"));
    assert!(position("verify_archive", "running") < position("verify_archive", "ok"));
    assert!(position("verify_archive", "ok") < position("clear", "running"));
    assert!(position("clear", "ok") < position("commit", "ok"));
    assert!(position("commit", "ok") < position("verify", "ok"));
    let completed = events
        .iter()
        .find(|event| event["stage"] == "archive" && event["status"] == "ok")
        .unwrap();
    let total = completed["total"].as_u64().unwrap();
    assert!(total >= 2);
    assert_eq!(completed["current"].as_u64(), Some(total));
    let archived = events
        .iter()
        .filter(|event| {
            event["stage"] == "archive"
                && event["status"] == "running"
                && event["current"].as_u64().unwrap_or(0) > 0
        })
        .map(|event| event["current"].as_u64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(archived, (1..=total).collect::<Vec<_>>());
    let target = store
        .records()
        .into_iter()
        .find(|record| !record.empty)
        .unwrap()
        .id;
    store.fail_after_clear = true;
    let failed = Reporter::channel(None);
    assert!(store
        .switch(&target, "Empty", &Policy::default(), &failed)
        .is_err());
    let events = failed.events();
    assert!(events
        .iter()
        .any(|event| event.stage() == "rollback" && event.status() == "running"));
    assert!(events
        .iter()
        .any(|event| event.stage() == "rollback" && event.status() == "ok"));
    assert!(!events
        .iter()
        .any(|event| event.stage() == "verify" && event.status() == "ok"));
    drop(store);
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn database_metadata_and_recovery_are_transactional() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-identity-test-{}", new_id().unwrap()));
    let mut db = database::Database::open(&folder).unwrap();
    let record = database::Record {
        id: new_id().unwrap(),
        name: "工作".into(),
        archive: "fixture.baga".into(),
        saved_at: 1,
        bytes: 22,
        policy: Policy::default(),
        empty: false,
        account_key: None,
    };
    db.pending(&record, "empty").unwrap();
    assert_eq!(db.get("recovery").unwrap(), Some(record.id.clone()));
    let list = db.records().unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].policy.skills);
    assert!(!list[0].policy.history);
    db.complete("empty").unwrap();
    assert_eq!(db.get("active").unwrap().as_deref(), Some("empty"));
    assert!(db.get("recovery").unwrap().is_none());
    drop(db);
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn empty_identity_and_saved_identity_preserve_user_choices() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-identity-test-{}", new_id().unwrap()));
    let home = folder.join("home");
    let vault = folder.join("vault");
    private::directory(&home).unwrap();
    private::directory(&home.join(".claude/skills/personal")).unwrap();
    fixture_write(home.join(".claude/skills/personal/SKILL.md"), "keep").unwrap();
    fixture_write(home.join(".claude.json"),r#"{"oauthAccount":{"emailAddress":"fixture@example.invalid"},"mcpServers":{"private":"keep"},"projects":{"mine":true}}"#).unwrap();
    fixture_write(
        home.join(".claude/.credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"disposable fixture credential"}}"#,
    )
    .unwrap();
    let mut store = store::Store::open(&vault, &home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .switch("empty", "个人", &Policy::default(), &reporter)
        .unwrap();
    assert!(serde_json::from_slice::<serde_json::Value>(
        &std::fs::read(home.join(".claude/.credentials.json")).unwrap()
    )
    .unwrap()
    .get("claudeAiOauth")
    .is_none());
    assert!(home.join(".claude/skills/personal/SKILL.md").exists());
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(home.join(".claude.json")).unwrap()).unwrap();
    assert!(value.get("oauthAccount").is_none());
    assert!(value.get("mcpServers").is_some());
    let records = store.records();
    let original = records.iter().find(|r| !r.empty).unwrap();
    let id = original.id.clone();
    let bytes = std::fs::read(vault.join(&original.archive)).unwrap();
    assert!(!bytes.windows(10).any(|s| s == b"disposable"));
    store
        .switch(&id, "空身份", &Policy::default(), &reporter)
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(home.join(".claude/.credentials.json")).unwrap()
        )
        .unwrap()["claudeAiOauth"]["accessToken"],
        "disposable fixture credential"
    );
    let mut choice = Policy::default();
    choice.skills = false;
    store
        .switch("empty", "包含技能", &choice, &reporter)
        .unwrap();
    assert!(!home.join(".claude/skills").exists());
    let with_skills = store.records().into_iter().find(|r| r.id == id).unwrap();
    store
        .switch(&with_skills.id, "空身份", &choice, &reporter)
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(home.join(".claude/skills/personal/SKILL.md")).unwrap(),
        "keep"
    );
    store.fail_after_clear = true;
    assert!(store
        .switch("empty", "故障样本", &choice, &reporter)
        .is_err());
    assert!(home.join(".claude/.credentials.json").exists());
    assert!(home.join(".claude/skills/personal/SKILL.md").exists());
    drop(store);
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn interrupted_switch_can_recover_and_preserves_pre_recovery_state() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-identity-test-{}", new_id().unwrap()));
    let home = folder.join("home");
    private::directory(&home.join(".claude")).unwrap();
    fixture_write(
        home.join(".claude/.credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"original fixture"}}"#,
    )
    .unwrap();
    let mut store = store::Store::open(&folder.join("vault"), &home).unwrap();
    let reporter = Reporter::channel(None);
    let (saved, stage) = store.save("原身份", &Policy::default(), &reporter).unwrap();
    store.db.pending(&saved, "empty").unwrap();
    store
        .clear(
            &store::Store::selected(&stage, &Policy::default()),
            &reporter,
            "clear",
        )
        .unwrap();
    fixture_write(
        home.join(".claude/history.jsonl"),
        "new work after interruption",
    )
    .unwrap();
    store.recover("恢复前状态", &reporter).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(home.join(".claude/.credentials.json")).unwrap()
        )
        .unwrap()["claudeAiOauth"]["accessToken"],
        "original fixture"
    );
    assert!(store.records().iter().any(|r| r.name == "恢复前状态"));
    assert!(store.db.get("recovery").unwrap().is_none());
    drop(stage);
    drop(store);
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn damaged_saved_archive_never_changes_live_identity() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-identity-test-{}", new_id().unwrap()));
    let home = folder.join("home");
    let vault = folder.join("vault");
    private::directory(&home.join(".claude")).unwrap();
    fixture_write(
        home.join(".claude/.credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"first"}}"#,
    )
    .unwrap();
    let mut store = store::Store::open(&vault, &home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .switch("empty", "first", &Policy::default(), &reporter)
        .unwrap();
    let original = store.records().into_iter().find(|r| !r.empty).unwrap();
    fixture_write(
        home.join(".claude/.credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"second"}}"#,
    )
    .unwrap();
    let path = vault.join(&original.archive);
    let mut bytes = std::fs::read(&path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    fixture_write(path, bytes).unwrap();
    assert!(store
        .switch(&original.id, "second", &Policy::default(), &reporter)
        .is_err());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(home.join(".claude/.credentials.json")).unwrap()
        )
        .unwrap()["claudeAiOauth"]["accessToken"],
        "second"
    );
    drop(store);
    std::fs::remove_dir_all(folder).unwrap();
}
#[cfg(unix)]
#[test]
fn selected_symlink_is_refused_without_changing_source() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-identity-test-{}", new_id().unwrap()));
    let home = folder.join("home");
    private::directory(&home.join(".claude")).unwrap();
    fixture_write(folder.join("outside"), "keep").unwrap();
    std::os::unix::fs::symlink(
        folder.join("outside"),
        home.join(".claude/.credentials.json"),
    )
    .unwrap();
    let mut store = store::Store::open(&folder.join("vault"), &home).unwrap();
    assert!(store
        .switch(
            "empty",
            "fixture",
            &Policy::default(),
            &Reporter::channel(None)
        )
        .is_err());
    assert_eq!(
        std::fs::read_to_string(folder.join("outside")).unwrap(),
        "keep"
    );
    drop(store);
    std::fs::remove_dir_all(folder).unwrap();
}

#[cfg(unix)]
#[test]
fn generated_debug_latest_symlink_does_not_block_identity_switch() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-identity-test-{}", new_id().unwrap()));
    let home = folder.join("home");
    let debug = home.join(".claude/debug");
    private::directory(&debug).unwrap();
    fixture_write(debug.join("session.log"), "inside").unwrap();
    fixture_write(folder.join("outside.log"), "outside").unwrap();
    std::os::unix::fs::symlink(folder.join("outside.log"), debug.join("latest")).unwrap();

    let mut store = store::Store::open(&folder.join("vault"), &home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .switch("empty", "saved", &Policy::default(), &reporter)
        .unwrap();
    let saved = store
        .records()
        .into_iter()
        .find(|record| !record.empty)
        .unwrap();

    assert!(!debug.exists());
    assert_eq!(
        std::fs::read_to_string(folder.join("outside.log")).unwrap(),
        "outside"
    );
    store
        .switch(&saved.id, "empty", &Policy::default(), &reporter)
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(debug.join("session.log")).unwrap(),
        "inside"
    );
    assert!(std::fs::symlink_metadata(debug.join("latest")).is_err());
    assert_eq!(
        std::fs::read_to_string(folder.join("outside.log")).unwrap(),
        "outside"
    );

    drop(store);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn archived_root_backup_remains_allowed_after_source_is_cleared() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-identity-test-{}", new_id().unwrap()));
    let home = folder.join("home");
    private::directory(&home).unwrap();
    fixture_write(home.join(".claude.json.backup"), "backup").unwrap();

    let mut store = store::Store::open(&folder.join("vault"), &home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .switch("empty", "saved", &Policy::default(), &reporter)
        .unwrap();
    let saved = store
        .records()
        .into_iter()
        .find(|record| !record.empty)
        .unwrap();
    assert!(!home.join(".claude.json.backup").exists());

    store
        .switch(&saved.id, "empty", &Policy::default(), &reporter)
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(home.join(".claude.json.backup")).unwrap(),
        "backup"
    );

    drop(store);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn direct_delete_respects_kept_data_and_removes_only_selected_archive() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-identity-test-{}", new_id().unwrap()));
    let home = folder.join("home");
    let vault = folder.join("vault");
    private::directory(&home.join(".claude/skills/private")).unwrap();
    fixture_write(
        home.join(".claude/skills/private/SKILL.md"),
        "personal asset",
    )
    .unwrap();
    fixture_write(
        home.join(".claude/.credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"fixture"},"mcpOAuth":"shared"}"#,
    )
    .unwrap();
    let mut store = store::Store::open(&vault, &home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .switch("empty", "saved", &Policy::default(), &reporter)
        .unwrap();
    let saved = store.records().into_iter().find(|r| !r.empty).unwrap();
    store
        .switch(&saved.id, "empty", &Policy::default(), &reporter)
        .unwrap();
    let count = store.records().len();
    store
        .delete("current", &Policy::default(), &reporter)
        .unwrap();
    assert_eq!(count, store.records().len());
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(home.join(".claude/.credentials.json")).unwrap())
            .unwrap();
    assert!(value.get("claudeAiOauth").is_none());
    assert_eq!(value["mcpOAuth"], "shared");
    assert!(home.join(".claude/skills/private/SKILL.md").exists());
    store
        .delete(&saved.id, &Policy::default(), &reporter)
        .unwrap();
    assert!(!vault.join(&saved.archive).exists());
    assert!(!store.records().iter().any(|r| r.id == saved.id));
    assert_eq!(
        std::fs::read_to_string(home.join(".claude/skills/private/SKILL.md")).unwrap(),
        "personal asset"
    );
    assert!(store
        .delete(&saved.id, &Policy::default(), &reporter)
        .is_err());
    drop(store);
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn restoring_full_settings_preserves_newer_shared_settings_when_requested() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-identity-test-{}", new_id().unwrap()));
    let home = folder.join("home");
    private::directory(&home.join(".claude")).unwrap();
    fixture_write(
        home.join(".claude.json"),
        r#"{"oauthAccount":{"id":"first"},"theme":"old","mcpServers":{"private":"old"}}"#,
    )
    .unwrap();
    fixture_write(
        home.join(".claude/settings.json"),
        r#"{"theme":"old","env":{"ANTHROPIC_API_KEY":"first"}}"#,
    )
    .unwrap();
    let mut store = store::Store::open(&folder.join("vault"), &home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .switch("empty", "all", &Policy::all(), &reporter)
        .unwrap();
    let saved = store.records().into_iter().find(|r| !r.empty).unwrap();
    fixture_write(
        home.join(".claude.json"),
        r#"{"theme":"latest","mcpServers":{"private":"latest"}}"#,
    )
    .unwrap();
    fixture_write(
        home.join(".claude/settings.json"),
        r#"{"theme":"latest","env":{"OTHER":"keep"}}"#,
    )
    .unwrap();
    store
        .switch(&saved.id, "shared", &Policy::default(), &reporter)
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(home.join(".claude.json")).unwrap()).unwrap();
    assert_eq!(value["oauthAccount"]["id"], "first");
    assert_eq!(value["theme"], "latest");
    assert_eq!(value["mcpServers"]["private"], "latest");
    let settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(home.join(".claude/settings.json")).unwrap())
            .unwrap();
    assert_eq!(settings["theme"], "latest");
    assert_eq!(settings["env"]["OTHER"], "keep");
    assert_eq!(settings["env"]["ANTHROPIC_API_KEY"], "first");
    drop(store);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn missing_master_key_never_replaces_a_retired_registered_archive() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-identity-test-{}", new_id().unwrap()));
    let home = folder.join("home");
    let vault = folder.join("vault");
    private::directory(&home.join(".claude")).unwrap();
    fixture_write(
        home.join(".claude/.credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"fixture"}}"#,
    )
    .unwrap();
    let mut store = store::Store::open(&vault, &home).unwrap();
    store
        .switch(
            "empty",
            "saved",
            &Policy::default(),
            &Reporter::channel(None),
        )
        .unwrap();
    let saved = store.records().into_iter().find(|r| !r.empty).unwrap();
    drop(store);
    let retired = vault.join(format!(".trash-{}", saved.id));
    std::fs::rename(vault.join(saved.archive), &retired).unwrap();
    std::fs::remove_file(vault.join("key.bin")).unwrap();
    assert!(store::Store::open(&vault, &home).is_err());
    assert!(!vault.join("key.bin").exists());
    assert!(retired.exists());
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn standard_fs_written_files_are_accepted_by_profile_catalog_and_switch() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-identity-test-{}", new_id().unwrap()));
    let home = folder.join("home");
    let vault = folder.join("vault");
    std::fs::create_dir_all(home.join(".claude/projects/demo")).unwrap();
    std::fs::write(
        home.join(".claude.json"),
        r#"{"oauthAccount":{"accountUuid":"std-fs-user","organizationUuid":"std-org","emailAddress":"stdfs@example.invalid","displayName":"StdFsUser"},"mcpServers":{"keep":{}}}"#,
    )
    .unwrap();
    std::fs::write(
        home.join(".claude/.credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"std-fs-token"}}"#,
    )
    .unwrap();
    std::fs::write(
        home.join(".claude/history.jsonl"),
        "{\"display\":\"hello\"}\n",
    )
    .unwrap();
    std::fs::write(
        home.join(".claude/projects/demo/session.jsonl"),
        "{\"session\":1}\n",
    )
    .unwrap();

    let current = profile::read(&home);
    assert!(current.present);
    assert!(!current.unavailable);
    assert_eq!(current.display_name.as_deref(), Some("StdFsUser"));
    assert_eq!(current.email.as_deref(), Some("stdfs@example.invalid"));

    let mut store = store::Store::open(&vault, &home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .switch("empty", "默认写入身份", &Policy::default(), &reporter)
        .unwrap();
    let saved = store.records().into_iter().find(|r| !r.empty).unwrap();
    assert_eq!(saved.name, "默认写入身份");
    assert!(catalog::current_id(&vault, &home, &store.db)
        .unwrap()
        .is_none());

    store
        .switch(&saved.id, "空身份", &Policy::default(), &reporter)
        .unwrap();
    assert_eq!(
        catalog::current_id(&vault, &home, &store.db)
            .unwrap()
            .as_deref(),
        Some(saved.id.as_str())
    );
    let restored = profile::read(&home);
    assert!(restored.present);
    assert!(!restored.unavailable);
    assert_eq!(restored.display_name.as_deref(), Some("StdFsUser"));
    drop(store);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
#[cfg(windows)]
fn windows_long_stage_paths_round_trip_beyond_max_path() {
    // 按 UTF-16 实际长度构造路径；TEMP 可以是不同长度/含非 ASCII 的目录。
    use std::os::windows::ffi::OsStrExt;
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-longpath-{}", new_id().unwrap()));
    let home = folder.join("home");
    let vault = folder.join("vault");
    let session = format!("{}.jsonl", "s".repeat(23));
    let base = home.join(".claude").join("projects");
    let required = 245usize
        .saturating_sub(base.as_os_str().encode_wide().count() + session.len() + 2)
        .max(32);
    let mut deep = std::path::PathBuf::new();
    let mut remaining = required;
    while remaining > 0 {
        let segment = remaining.min(80);
        deep.push("l".repeat(segment));
        remaining = remaining.saturating_sub(segment + 1);
    }
    let project = base.join(&deep);
    private::directory(&project).unwrap();
    fixture_write(project.join(&session), "long path fixture").unwrap();
    fixture_write(
        home.join(".claude.json"),
        r#"{"oauthAccount":{"accountUuid":"long-path-fixture"}}"#,
    )
    .unwrap();
    let home_path = project.join(&session);
    let stage_path = vault
        .join(format!(".stage-{}", "0".repeat(32)))
        .join("data")
        .join("0")
        .join(&deep)
        .join(&session);
    assert!(stage_path.as_os_str().encode_wide().count() >= 260);
    let mut store = store::Store::open(&vault, &home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .switch("empty", "长路径", &Policy::default(), &reporter)
        .unwrap();
    assert!(!home_path.exists());
    let saved = store.records().into_iter().find(|r| !r.empty).unwrap();
    store
        .switch(&saved.id, "恢复", &Policy::default(), &reporter)
        .unwrap();
    assert_eq!(
        std::fs::read(&home_path).unwrap(),
        b"long path fixture".to_vec()
    );
    drop(store);
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn existing_directories_keep_their_permissions() {
    let folder = canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("baga-parent-{}", new_id().unwrap()));
    std::fs::create_dir(&folder).unwrap();
    #[cfg(unix)]
    fn permissions(path: &std::path::Path) -> String {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .unwrap()
            .permissions()
            .mode()
            .to_string()
    }
    #[cfg(windows)]
    fn permissions(path: &std::path::Path) -> String {
        use windows_sys::Win32::{
            Foundation::LocalFree,
            Security::{Authorization::*, *},
        };
        let mut descriptor = std::ptr::null_mut();
        let mut text = std::ptr::null_mut();
        let mut length = 0;
        let wide = crate::windows::wide_path(path);
        unsafe {
            let flags = OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
            assert_eq!(
                GetNamedSecurityInfoW(
                    wide.as_ptr(),
                    SE_FILE_OBJECT,
                    flags,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut descriptor
                ),
                0
            );
            assert_ne!(
                ConvertSecurityDescriptorToStringSecurityDescriptorW(
                    descriptor,
                    1,
                    flags,
                    &mut text,
                    &mut length
                ),
                0
            );
            let result =
                String::from_utf16_lossy(std::slice::from_raw_parts(text, length as usize))
                    .trim_end_matches('\0')
                    .to_string();
            LocalFree(text.cast());
            LocalFree(descriptor.cast());
            result
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let before = permissions(&folder);
    private::directory(&folder).unwrap();
    assert_eq!(permissions(&folder), before);
    private::directory(&folder.join("child")).unwrap();
    assert_eq!(permissions(&folder), before);
    std::fs::remove_dir_all(folder).unwrap();
}
