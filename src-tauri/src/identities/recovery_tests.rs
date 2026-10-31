// [INPUT]: Synthetic identities, disposable vaults and SQLite failure injection.
// [OUTPUT]: Empty-identity round trips and preservation of unindexed recovery archives.
// [POS]: Offline regression coverage; never opens a real Claude profile.
// [PROTOCOL]: Update this header when changing behavior, then check CLAUDE.md.
use super::{optimization_tests::Fixture, *};
use std::fs;

#[cfg(windows)]
#[test]
fn desktop_3p_round_trip_survives_reopening_the_vault() {
    let f = Fixture::new();
    f.login("synthetic-code", "synthetic-org", "synthetic-token");
    let desktop = "AppData/Local/Claude-3p";
    f.write(
        &format!("{desktop}/Local Storage/state"),
        b"original-desktop",
    );
    f.write(&format!("{desktop}/ant-did"), b"original-device");
    f.write(
        &format!("{desktop}/local-agent-mode-sessions/original/task.json"),
        b"synthetic-cowork-history",
    );
    f.write(
        ".claude/projects/example/session.jsonl",
        b"synthetic-code-history",
    );
    let reporter = Reporter::channel(None);
    let policy = Policy::default();
    let mut s = store::Store::open(&f.vault, &f.home).unwrap();
    s.switch("empty", "Original", &policy, &reporter).unwrap();
    let original = s.records().into_iter().find(|row| !row.empty).unwrap();
    assert!(!f.home.join(desktop).join("Local Storage/state").exists());
    drop(s);
    let mut s = store::Store::open(&f.vault, &f.home).unwrap();
    assert!(s.records().iter().any(|row| row.id == original.id));
    s.switch(&original.id, "Empty", &policy, &reporter).unwrap();
    assert_eq!(
        fs::read(f.home.join(desktop).join("Local Storage/state")).unwrap(),
        b"original-desktop"
    );
    assert_eq!(
        fs::read(f.home.join(desktop).join("ant-did")).unwrap(),
        b"original-device"
    );
    assert_eq!(
        fs::read(
            f.home
                .join(desktop)
                .join("local-agent-mode-sessions/original/task.json")
        )
        .unwrap(),
        b"synthetic-cowork-history"
    );
    assert_eq!(
        fs::read(f.home.join(".claude/projects/example/session.jsonl")).unwrap(),
        b"synthetic-code-history"
    );
}

#[test]
fn missing_catalog_rows_do_not_destroy_the_only_recovery_archive() {
    let f = Fixture::new();
    f.login("synthetic-account", "synthetic-org", "synthetic-token");
    f.write(".claude/history.jsonl", b"synthetic-history");
    let mut s = store::Store::open(&f.vault, &f.home).unwrap();
    s.switch(
        "empty",
        "Original",
        &Policy::default(),
        &Reporter::channel(None),
    )
    .unwrap();
    let original = s.records().into_iter().find(|row| !row.empty).unwrap();
    let archive = f.vault.join(&original.archive);
    let bytes = fs::read(&archive).unwrap();
    // Model the observed inconsistent catalog, without claiming its cause.
    s.db.connection
        .execute_batch("DELETE FROM identities; DELETE FROM identity_summaries;")
        .unwrap();
    drop(s);
    let s = store::Store::open(&f.vault, &f.home).unwrap();
    assert_eq!(
        fs::read(&archive).expect("unindexed authenticated archive must survive reopening"),
        bytes
    );
    assert!(s.load(&original).is_ok());
    assert!(!f.home.join(".claude/history.jsonl").exists());
}

#[test]
fn rejected_catalog_commit_preserves_its_archive_on_the_next_open() {
    let f = Fixture::new();
    f.login("synthetic-account", "synthetic-org", "synthetic-token");
    f.write(".claude/history.jsonl", b"synthetic-history");
    let mut s = store::Store::open(&f.vault, &f.home).unwrap();
    s.db.connection.execute_batch("CREATE TRIGGER reject_identity BEFORE INSERT ON identities BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(s
        .switch(
            "empty",
            "Original",
            &Policy::default(),
            &Reporter::channel(None)
        )
        .is_err());
    assert_eq!(f.token(), "synthetic-token");
    assert_eq!(
        fs::read(f.home.join(".claude/history.jsonl")).unwrap(),
        b"synthetic-history"
    );
    let archive = fs::read_dir(&f.vault)
        .unwrap()
        .flatten()
        .find(|entry| catalog::archive_name(&entry.file_name().to_string_lossy()))
        .unwrap()
        .path();
    let bytes = fs::read(&archive).unwrap();
    drop(s);
    let _s = store::Store::open(&f.vault, &f.home).unwrap();
    assert_eq!(
        fs::read(&archive).expect("failed commit must not authorize deleting the archive"),
        bytes
    );
}

#[test]
fn successful_operations_collect_only_known_superseded_generations() {
    let f = Fixture::new();
    let reporter = Reporter::channel(None);
    let policy = Policy::default();
    f.login("synthetic-account", "synthetic-org", "old-token");
    let mut s = store::Store::open(&f.vault, &f.home).unwrap();
    s.save_current("Original", &policy, &reporter).unwrap();
    let old = s.records()[0].clone();
    f.login("synthetic-account", "synthetic-org", "uncommitted-token");
    let (unindexed, stage) = s.save("Uncommitted", &policy, &reporter).unwrap();
    drop(stage);
    let unindexed_bytes = fs::read(f.vault.join(&unindexed.archive)).unwrap();
    f.login("synthetic-account", "synthetic-org", "latest-token");
    s.save_current("Latest", &policy, &reporter).unwrap();
    let current = s.records()[0].clone();
    assert_eq!(current.id, old.id);
    assert_ne!(current.archive, old.archive);
    assert!(!f.vault.join(&old.archive).exists());
    assert!(f.vault.join(&current.archive).exists());
    assert_eq!(
        fs::read(f.vault.join(&unindexed.archive)).unwrap(),
        unindexed_bytes
    );
    // Both the same-account shortcut and a normal switch perform collection.
    s.switch(&current.id, "Current", &policy, &reporter)
        .unwrap();
    assert!(f.vault.join(&unindexed.archive).exists());
    s.switch("empty", "Latest", &policy, &reporter).unwrap();
    let saved = s.records()[0].clone();
    assert!(!f.vault.join(&current.archive).exists());
    assert!(f.vault.join(&unindexed.archive).exists());
    // Simulate the persisted marker at an interruption after clearing the workspace.
    s.db.pending(&saved, "empty").unwrap();
    s.recover("Empty checkpoint", &reporter).unwrap();
    assert!(f.vault.join(&saved.archive).exists());
    // A committed rollback may collect its superseded generation, not unknown ones.
    s.fail_after_clear = true;
    assert!(s.switch("empty", "Rollback", &policy, &reporter).is_err());
    assert_eq!(f.token(), "latest-token");
    assert!(!f.vault.join(&saved.archive).exists());
    assert_eq!(
        fs::read_dir(&f.vault)
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| catalog::archive_name(&entry.file_name().to_string_lossy()))
            .count(),
        2,
        "only the indexed generation and the unindexed recovery copy should remain"
    );
    drop(s);
    let s = store::Store::open(&f.vault, &f.home).unwrap();
    assert_eq!(
        fs::read(f.vault.join(&unindexed.archive)).unwrap(),
        unindexed_bytes
    );
    assert!(s.load(&unindexed).is_ok());
    assert_eq!(f.token(), "latest-token");
}
