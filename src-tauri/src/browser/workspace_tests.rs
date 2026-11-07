// [INPUT]: 临时应用数据目录、虚构浏览器资料与身份绑定配置。
// [OUTPUT]: SQLite 句柄释放后的三平台临时目录清理、Cookie/缓存/种子隔离、手动换指纹持久化、旧资料保留与路径逃逸拒绝回归。
// [POS]: 浏览器存储边界的离线证明，不启动真实用户的浏览器或访问钥匙串。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
#[test]
fn retained_saved_profile_reports_its_relative_location_without_changing_data() {
    let fixture = tests::sample();
    let root = fixture.join("browser");
    let slot = crate::identities::new_id().unwrap();
    let cookie = root.join("contexts").join(&slot).join("profile/Default/Cookies");
    fs::create_dir_all(cookie.parent().unwrap()).unwrap();
    fs::write(&cookie, b"synthetic cookie").unwrap();

    assert_eq!(
        workspaces::retained_profile_path(&root, &slot).unwrap(),
        Some(format!("browser/contexts/{slot}/profile"))
    );
    assert_eq!(fs::read(&cookie).unwrap(), b"synthetic cookie");
    fs::remove_dir_all(fixture).unwrap();
}

#[test]
fn linked_profiles_keep_separate_data_and_configs_without_compressing_or_copying() {
    let fixture = tests::sample();
    let root = fixture.join("browser");
    crate::identities::private::directory(&root).unwrap();
    let legacy = root.join("profile/Default");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join("Bookmarks"), b"old").unwrap();
    let mut db = workspaces::database(&root).unwrap();
    let a = crate::identities::new_id().unwrap();
    let b = crate::identities::new_id().unwrap();
    db.configure_browser(true, false, Some(&a)).unwrap();
    let scope_a = workspaces::ensure(&root).unwrap();
    assert_eq!(
        config::load(&scope_a).unwrap().seed,
        crate::identities::browser_link::seed(&a, 0)
    );
    let data_a = scope_a.join("profile/Default");
    fs::create_dir_all(&data_a).unwrap();
    fs::write(data_a.join("Cookies"), b"A").unwrap();
    let mut config_a = config::load(&scope_a).unwrap();
    config_a.seed = 12345;
    config::save(&scope_a, &config_a).unwrap();
    db.configure_browser(true, false, Some(&b)).unwrap();
    let scope_b = workspaces::ensure(&root).unwrap();
    assert_ne!(scope_a, scope_b);
    let data_b = scope_b.join("profile/Default");
    assert!(!data_b.exists());
    fs::create_dir_all(data_b.join("Cache")).unwrap();
    fs::write(data_b.join("Cookies"), b"B").unwrap();
    fs::write(data_b.join("Cache/data"), b"cache").unwrap();
    profile::clear(&scope_b.join("profile"), false, |_, _| {}).unwrap();
    assert_eq!(fs::read(data_a.join("Cookies")).unwrap(), b"A");
    assert_eq!(fs::read(data_b.join("Cookies")).unwrap(), b"B");
    db.configure_browser(true, false, Some(&a)).unwrap();
    assert_eq!(workspaces::ensure(&root).unwrap(), scope_a);
    assert_eq!(config::load(&scope_a).unwrap().seed, 12345);
    db.configure_browser(false, false, None).unwrap();
    assert_eq!(workspaces::scope(&root).unwrap(), root);
    assert_eq!(fs::read(legacy.join("Bookmarks")).unwrap(), b"old");
    for bad in ["../escape", "/tmp", "", "Default", "AABB"] {
        assert!(workspaces::slot_path(&root, bad).is_err());
    }
    drop(db);
    fs::remove_dir_all(fixture).unwrap();
}
#[test]
fn recovery_refuses_profile_launch_preparation() {
    let fixture = tests::sample();
    let root = fixture.join("browser");
    let db = workspaces::database(&root).unwrap();
    drop(db);
    let connection =
        rusqlite::Connection::open(fixture.join("identity-vault/identities.sqlite")).unwrap();
    connection
        .execute(
            "INSERT INTO state VALUES('recovery',?1)",
            [crate::identities::new_id().unwrap()],
        )
        .unwrap();
    assert!(workspaces::ensure(&root).unwrap_err().contains("恢复"));
    drop(connection);
    fs::remove_dir_all(fixture).unwrap();
}
#[test]
fn portable_browser_config_preserves_custom_seed_without_touching_active_data() {
    let fixture = tests::sample();
    let root = fixture.join("browser");
    let mut db = workspaces::database(&root).unwrap();
    let identity = crate::identities::new_id().unwrap();
    db.configure_browser(true, false, Some(&identity)).unwrap();
    let scope = workspaces::ensure(&root).unwrap();
    let active = db.browser_slot().unwrap();
    let cookie = scope.join("profile/Default/Cookies");
    crate::identities::private::directory(cookie.parent().unwrap()).unwrap();
    crate::identities::private::write(&cookie, b"private local cookie").unwrap();
    let mut config = config::load(&scope).unwrap();
    config.seed = 12345;
    config::save(&scope, &config).unwrap();
    let exported = workspaces::portable_config_at(&root, &identity)
        .unwrap()
        .unwrap();
    assert_eq!(exported.seed, 12345);
    let new_slot = crate::identities::new_id().unwrap();
    let prepared =
        workspaces::portable_prepare_at(&root, &new_slot, &exported, &tests::machine()).unwrap();
    let mut names: Vec<_> = fs::read_dir(&prepared)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["config.json", "seed.owner"]);
    assert!(!prepared.join("profile").exists());
    assert_eq!(fs::read(prepared.join("seed.owner")).unwrap(), b"12345");
    assert_eq!(db.browser_slot().unwrap(), active);
    assert!(db.browser_seed(&new_slot).unwrap().is_none());
    assert_eq!(fs::read(&cookie).unwrap(), b"private local cookie");
    assert!(
        workspaces::portable_prepare_at(&root, &new_slot, &exported, &tests::machine()).is_err()
    );
    crate::identities::private::write(&scope.join("seed.owner"), b"invalid owner").unwrap();
    assert!(workspaces::portable_config_at(&root, &identity).is_err());
    assert!(
        workspaces::portable_config_at(&root, &crate::identities::new_id().unwrap())
            .unwrap()
            .is_none()
    );
    drop(db);
    fs::remove_dir_all(fixture).unwrap();
}
