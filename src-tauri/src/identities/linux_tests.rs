// [INPUT]: Linux 临时 Desktop 配置、稀疏文件与合成 SQLite Cookie/WAL。
// [OUTPUT]: 身份往返/直接删除保留设置、历史、工作区和非登录 Cookie，拒绝数据库链接。
// [POS]: Linux 身份事务回归；不读取真实账号、运行应用或修改 HOME。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::{optimization_tests::Fixture, *};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{
    fs,
    io::{Seek, SeekFrom, Write},
    os::unix::fs::{symlink, MetadataExt},
};

fn policy() -> Policy {
    Policy {
        history: true,
        cache: true,
        ..Policy::default()
    }
}
fn config(f: &Fixture) -> Value {
    serde_json::from_slice(&fs::read(f.home.join(".config/Claude/config.json")).unwrap()).unwrap()
}
fn setup(f: &Fixture, relative: &str) -> Connection {
    f.write(".config/Claude/config.json", br#"{"oauth:tokenCache":"legacy-token","oauth:tokenCacheV2":"current-token","lastKnownAccountUuid":"account-a","theme":"dark","custom.setting":{"keep":true}}"#);
    for (path, content) in [
        (
            ".config/Claude/claude_desktop_config.json",
            "MCP and preferences",
        ),
        (".config/Claude/Local Storage/state", "keep web preferences"),
        (".config/Claude/IndexedDB/state", "keep web history"),
        (
            ".config/Claude/Session Storage/state",
            "keep web session data",
        ),
        (
            ".config/Claude/claude-code-sessions/account-a/session.json",
            "keep Code history",
        ),
        (
            ".config/Claude/local-agent-mode-sessions/account-a/session.json",
            "keep Cowork history",
        ),
        (".config/Claude/Local State", "keep encryption key"),
        ("workspace/project.txt", "keep source"),
        (".claude/history.jsonl", "keep CLI history"),
    ] {
        f.write(path, content.as_bytes());
    }
    let path = f.home.join(relative);
    private::directory(path.parent().unwrap()).unwrap();
    let db = Connection::open(path).unwrap();
    db.execute_batch("CREATE TABLE meta(key TEXT PRIMARY KEY,value TEXT); INSERT INTO meta VALUES('version','24'); CREATE TABLE cookies(host_key TEXT NOT NULL,name TEXT NOT NULL,value TEXT NOT NULL,encrypted_value BLOB NOT NULL,path TEXT NOT NULL,PRIMARY KEY(host_key,name,path));").unwrap();
    for (host, name, value) in [
        (".claude.ai", "sessionKey", "session-a"),
        (".claude.ai", "lastActiveOrg", "org-a"),
        (".claude.ai", "intercom-session-test", "support-a"),
        (".claude.ai", "CH-prefers-color-scheme", "dark"),
        ("unrelated.invalid", "sessionKey", "keep-other-site"),
        ("evilclaude.ai", "sessionKey", "keep-lookalike"),
    ] {
        db.execute(
            "INSERT INTO cookies VALUES(?1,?2,?3,?4,'/')",
            rusqlite::params![host, name, value, vec![1u8, 2, 3]],
        )
        .unwrap();
    }
    db
}
fn cookie(db: &Connection, host: &str, name: &str) -> Option<String> {
    use rusqlite::OptionalExtension;
    db.query_row(
        "SELECT value FROM cookies WHERE host_key=?1 AND name=?2",
        [host, name],
        |row| row.get(0),
    )
    .optional()
    .unwrap()
}
fn retained(f: &Fixture) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    [
        "claude_desktop_config.json",
        "Local Storage/state",
        "IndexedDB/state",
        "Session Storage/state",
        "claude-code-sessions/account-a/session.json",
        "local-agent-mode-sessions/account-a/session.json",
        "Local State",
    ]
    .into_iter()
    .map(|path| f.home.join(".config/Claude").join(path))
    .chain([
        f.home.join("workspace/project.txt"),
        f.home.join(".claude/history.jsonl"),
    ])
    .map(|path| {
        let bytes = fs::read(&path).unwrap();
        (path, bytes)
    })
    .collect()
}

#[test]
fn linux_sparse_gpu_cache_round_trip() {
    let f = Fixture::new();
    let path = f.home.join(".config/Claude/GPUCache/data_1");
    private::directory(path.parent().unwrap()).unwrap();
    let mut file = private::create(&path).unwrap();
    file.write_all(b"start").unwrap();
    file.seek(SeekFrom::Start(1024 * 1024)).unwrap();
    file.write_all(b"end").unwrap();
    file.sync_all().unwrap();
    assert!(file.metadata().unwrap().blocks() * 512 < file.metadata().unwrap().len());
    drop(file);
    let original = fs::read(&path).unwrap();
    let mut store = store::Store::open(&f.vault, &f.home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .switch("empty", "Sparse cache", &Policy::default(), &reporter)
        .unwrap();
    assert!(!path.exists());
    let saved = store.records().into_iter().find(|r| !r.empty).unwrap();
    store
        .switch(&saved.id, "Empty", &Policy::default(), &reporter)
        .unwrap();
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn linux_desktop_switch_restores_only_login_data_and_preserves_personal_assets() {
    for relative in [".config/Claude/Cookies", ".config/Claude/Network/Cookies"] {
        let f = Fixture::new();
        drop(setup(&f, relative));
        let original = config(&f);
        let retained = retained(&f);
        let mut store = store::Store::open(&f.vault, &f.home).unwrap();
        let reporter = Reporter::channel(None);
        assert!(profile::read(&f.home).present);
        store
            .switch("empty", "Desktop A", &policy(), &reporter)
            .unwrap();
        assert!(!profile::read(&f.home).present);
        assert_eq!(
            config(&f),
            json!({"theme":"dark","custom.setting":{"keep":true}})
        );
        let db = Connection::open(f.home.join(relative)).unwrap();
        assert_eq!(cookie(&db, ".claude.ai", "sessionKey"), None);
        assert_eq!(cookie(&db, ".claude.ai", "lastActiveOrg"), None);
        assert_eq!(cookie(&db, ".claude.ai", "intercom-session-test"), None);
        assert_eq!(
            cookie(&db, "unrelated.invalid", "sessionKey").as_deref(),
            Some("keep-other-site")
        );
        assert_eq!(
            cookie(&db, "evilclaude.ai", "sessionKey").as_deref(),
            Some("keep-lookalike")
        );
        db.execute(
            "UPDATE cookies SET value='light' WHERE name='CH-prefers-color-scheme'",
            [],
        )
        .unwrap();
        drop(db);
        f.write(
            ".config/Claude/config.json",
            br#"{"theme":"light","custom.setting":{"keep":true}}"#,
        );
        let saved = store.records().into_iter().find(|r| !r.empty).unwrap();
        store
            .switch(&saved.id, "Empty", &policy(), &reporter)
            .unwrap();
        assert!(profile::read(&f.home).present);
        let mut expected = original;
        expected["theme"] = json!("light");
        assert_eq!(config(&f), expected);
        let db = Connection::open(f.home.join(relative)).unwrap();
        assert_eq!(
            cookie(&db, ".claude.ai", "sessionKey").as_deref(),
            Some("session-a")
        );
        assert_eq!(
            cookie(&db, ".claude.ai", "CH-prefers-color-scheme").as_deref(),
            Some("light")
        );
        for (path, bytes) in retained {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }
}

#[test]
fn linux_desktop_direct_delete_preserves_history_and_non_login_cookies() {
    let f = Fixture::new();
    drop(setup(&f, ".config/Claude/Cookies"));
    let retained = retained(&f);
    let mut store = store::Store::open(&f.vault, &f.home).unwrap();
    store
        .delete("current", &policy(), &Reporter::channel(None))
        .unwrap();
    assert_eq!(
        config(&f),
        json!({"theme":"dark","custom.setting":{"keep":true}})
    );
    let db = Connection::open(f.home.join(".config/Claude/Cookies")).unwrap();
    assert_eq!(cookie(&db, ".claude.ai", "sessionKey"), None);
    assert_eq!(
        cookie(&db, ".claude.ai", "CH-prefers-color-scheme").as_deref(),
        Some("dark")
    );
    assert!(store.records().is_empty());
    for (path, bytes) in retained {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn linux_desktop_snapshot_includes_committed_cookie_wal() {
    let f = Fixture::new();
    let db = setup(&f, ".config/Claude/Cookies");
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; UPDATE cookies SET value='wal-token' WHERE name='sessionKey' AND host_key='.claude.ai';").unwrap();
    assert!(f.home.join(".config/Claude/Cookies-wal").exists());
    let mut store = store::Store::open(&f.vault, &f.home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .save_current("WAL snapshot", &policy(), &reporter)
        .unwrap();
    drop(db);
    store
        .switch("empty", "Desktop A", &policy(), &reporter)
        .unwrap();
    let saved = store
        .records()
        .into_iter()
        .find(|r| r.name == "WAL snapshot")
        .unwrap();
    store
        .switch(&saved.id, "Empty", &policy(), &reporter)
        .unwrap();
    let db = Connection::open(f.home.join(".config/Claude/Cookies")).unwrap();
    assert_eq!(
        cookie(&db, ".claude.ai", "sessionKey").as_deref(),
        Some("wal-token")
    );
}

#[test]
fn linux_desktop_rejects_linked_cookie_sidecars_before_clearing_identity() {
    let f = Fixture::new();
    drop(setup(&f, ".config/Claude/Cookies"));
    let original = fs::read(f.home.join(".config/Claude/config.json")).unwrap();
    f.write("outside", b"do not touch");
    symlink(
        f.home.join("outside"),
        f.home.join(".config/Claude/Cookies-wal"),
    )
    .unwrap();
    let mut store = store::Store::open(&f.vault, &f.home).unwrap();
    assert!(store
        .switch("empty", "Desktop A", &policy(), &Reporter::channel(None))
        .is_err());
    assert_eq!(
        fs::read(f.home.join(".config/Claude/config.json")).unwrap(),
        original
    );
    assert_eq!(fs::read(f.home.join("outside")).unwrap(), b"do not touch");
}

#[test]
fn linux_desktop_failed_switch_rolls_back_login_without_touching_history() {
    let f = Fixture::new();
    drop(setup(&f, ".config/Claude/Cookies"));
    let retained = retained(&f);
    let mut store = store::Store::open(&f.vault, &f.home).unwrap();
    let reporter = Reporter::channel(None);
    store
        .switch("empty", "Desktop A", &policy(), &reporter)
        .unwrap();
    let a = store.records().into_iter().find(|r| !r.empty).unwrap();
    f.write(
        ".config/Claude/config.json",
        br#"{"oauth:tokenCacheV2":"token-b","theme":"light"}"#,
    );
    let db = Connection::open(f.home.join(".config/Claude/Cookies")).unwrap();
    db.execute(
        "INSERT INTO cookies VALUES('.claude.ai','sessionKey','session-b',X'010203','/')",
        [],
    )
    .unwrap();
    drop(db);
    store.fail_after_clear = true;
    assert!(store
        .switch(&a.id, "Desktop B", &policy(), &reporter)
        .is_err());
    assert_eq!(
        config(&f),
        json!({"oauth:tokenCacheV2":"token-b","theme":"light"})
    );
    let db = Connection::open(f.home.join(".config/Claude/Cookies")).unwrap();
    assert_eq!(
        cookie(&db, ".claude.ai", "sessionKey").as_deref(),
        Some("session-b")
    );
    for (path, bytes) in retained {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn unreadable_desktop_login_is_unknown_instead_of_empty() {
    let f = Fixture::new();
    f.write(".config/Claude/config.json", br#"{"theme":"dark"}"#);
    f.write(".config/Claude/Cookies", b"not a SQLite database");
    let current = profile::read(&f.home);
    assert!(current.present && current.unavailable);
}

#[test]
fn missing_cookie_database_rejects_all_orphaned_sidecars() {
    let f = Fixture::new();
    drop(setup(&f, ".config/Claude/Cookies"));
    let _store = store::Store::open(&f.vault, &f.home).unwrap();
    let snapshot =
        crate::cleaner::desktop::snapshot(&f.home.join(".config/Claude/Cookies"), &f.vault)
            .unwrap();
    f.write("outside", b"do not touch");
    for (n, suffix) in ["-wal", "-shm", "-journal"].into_iter().enumerate() {
        for linked in [false, true] {
            let folder = f.home.join(format!("missing-{n}-{linked}"));
            private::directory(&folder).unwrap();
            let path = folder.join("Cookies");
            let sidecar = folder.join(format!("Cookies{suffix}"));
            if linked {
                symlink(f.home.join("outside"), &sidecar).unwrap();
            } else {
                private::write(&sidecar, b"orphan").unwrap();
            }
            assert!(crate::cleaner::desktop::restore(&path, &snapshot.path).is_err());
            assert!(!path.exists());
            assert_eq!(fs::read(f.home.join("outside")).unwrap(), b"do not touch");
        }
    }
}

#[test]
fn incompatible_cookie_schema_or_version_never_replaces_unrelated_rows() {
    let f = Fixture::new();
    drop(setup(&f, ".config/Claude/Cookies"));
    let _store = store::Store::open(&f.vault, &f.home).unwrap();
    let snapshot =
        crate::cleaner::desktop::snapshot(&f.home.join(".config/Claude/Cookies"), &f.vault)
            .unwrap();
    for (n, constraint, version, trigger) in [
        (0, "UNIQUE(name)", "24", false),
        (1, "PRIMARY KEY(host_key,name,path)", "25", false),
        (2, "PRIMARY KEY(host_key,name,path)", "24", true),
    ] {
        let target = f.home.join(format!("target-{n}"));
        let db = Connection::open(&target).unwrap();
        db.execute_batch(&format!("CREATE TABLE meta(key TEXT PRIMARY KEY,value TEXT); INSERT INTO meta VALUES('version','{version}'); CREATE TABLE cookies(host_key TEXT NOT NULL,name TEXT NOT NULL,value TEXT NOT NULL,encrypted_value BLOB NOT NULL,path TEXT NOT NULL,{constraint}); INSERT INTO cookies VALUES('unrelated.invalid','lastActiveOrg','keep',X'010203','/');")).unwrap();
        if trigger {
            db.execute_batch("CREATE TRIGGER cookie_insert AFTER INSERT ON cookies BEGIN DELETE FROM cookies WHERE host_key='unrelated.invalid'; END;").unwrap();
        }
        drop(db);
        assert!(crate::cleaner::desktop::restore(&target, &snapshot.path).is_err());
        let db = Connection::open(target).unwrap();
        assert_eq!(
            cookie(&db, "unrelated.invalid", "lastActiveOrg").as_deref(),
            Some("keep")
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM cookies", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}

#[test]
fn filtered_cookie_snapshot_does_not_export_previously_deleted_cookie_bytes() {
    let f = Fixture::new();
    let db = setup(&f, ".config/Claude/Cookies");
    let marker = "private-cookie-previously-deleted-".repeat(500);
    db.execute_batch("PRAGMA secure_delete=OFF;").unwrap();
    db.execute(
        "INSERT INTO cookies VALUES('unrelated.invalid','deleted',?1,X'010203','/')",
        [&marker],
    )
    .unwrap();
    db.execute("DELETE FROM cookies WHERE name='deleted'", [])
        .unwrap();
    drop(db);
    let bytes = fs::read(f.home.join(".config/Claude/Cookies")).unwrap();
    assert!(bytes
        .windows(32)
        .any(|bytes| bytes == &marker.as_bytes()[..32]));
    let _store = store::Store::open(&f.vault, &f.home).unwrap();
    let snapshot =
        crate::cleaner::desktop::snapshot(&f.home.join(".config/Claude/Cookies"), &f.vault)
            .unwrap();
    let bytes = fs::read(&snapshot.path).unwrap();
    assert!(!bytes
        .windows(32)
        .any(|bytes| bytes == &marker.as_bytes()[..32]));
    let db = Connection::open(&snapshot.path).unwrap();
    assert_eq!(
        cookie(&db, ".claude.ai", "sessionKey").as_deref(),
        Some("session-a")
    );
}
