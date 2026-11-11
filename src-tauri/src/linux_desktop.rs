// [INPUT]: 明确的 Linux Desktop OAuth JSON 键、Cookie 数据库和当前用户文件边界。
// [OUTPUT]: 仅登录 Cookie 的一致性快照/事务清除/合并恢复，约束冲突回滚，保留其他 Cookie、配置和历史。
// [POS]: 身份事务专用；SQLite 正文只进入私有暂存和加密归档，不进入 UI 或通用残留删除。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::identity::Source;
use crate::identities::{new_id, private};
use rusqlite::{params_from_iter, types::Value, Connection, OpenFlags};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) const COOKIE_CATEGORY: &str = "desktop-cookies";
const AUTH: &str = "(lower(ltrim(host_key,'.')) IN ('claude.ai','claude.com','anthropic.com') OR lower(host_key) LIKE '%.claude.ai' OR lower(host_key) LIKE '%.claude.com') AND (name IN ('sessionKey','lastActiveOrg') OR name GLOB 'intercom-session-*')";
const KEYS: &[&str] = &[
    "oauth:tokenCache",
    "oauth:tokenCacheV2",
    "oauth:staleSessionLatch",
    "lastKnownAccountUuid",
    "coworkTrustedDeviceToken",
];
fn problem(_: impl std::fmt::Display) -> String {
    "Desktop Cookie 数据库无法安全读取或合并；请退出 Desktop 后重试。".into()
}
pub(crate) fn config_key(key: &str) -> bool {
    KEYS.contains(&key)
        || key
            .strip_prefix("coworkTrustedDeviceToken:")
            .is_some_and(|id| {
                id.len() == 36
                    && id.bytes().enumerate().all(|(n, b)| {
                        if [8, 13, 18, 23].contains(&n) {
                            b == b'-'
                        } else {
                            b.is_ascii_hexdigit()
                        }
                    })
            })
}
pub(crate) fn sources(root: &Path) -> Vec<Source> {
    let desktop = super::targets::xdg_dir(root, "XDG_CONFIG_HOME", ".config").join("Claude");
    let mut sources = vec![Source {
        path: desktop.join("config.json"),
        category: "desktop".into(),
        keys: KEYS.iter().map(|s| (*s).into()).collect(),
    }];
    for relative in ["Cookies", "Network/Cookies"] {
        sources.push(Source {
            path: desktop.join(relative),
            category: COOKIE_CATEGORY.into(),
            keys: Vec::new(),
        });
    }
    sources
}
fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    name.into()
}
pub(crate) fn check(path: &Path) -> Result<(), String> {
    private::parents(path.parent().ok_or_else(|| problem("parent"))?)?;
    private::safe(path)?;
    if !path.is_file() {
        return Err(problem("file"));
    }
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = sidecar(path, suffix);
        match fs::symlink_metadata(&sidecar) {
            Ok(_) => {
                private::safe(&sidecar)?;
                if !sidecar.is_file() {
                    return Err(problem("sidecar"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(problem(e)),
        }
    }
    Ok(())
}
pub(crate) fn fingerprint(path: &Path) -> Result<Vec<u8>, String> {
    check(path)?;
    let mut stamp = super::identity::fingerprint(path)?;
    for suffix in ["-wal", "-journal"] {
        let sidecar = sidecar(path, suffix);
        if sidecar.exists() {
            stamp.push(1);
            stamp.extend(super::identity::fingerprint(&sidecar)?);
        } else {
            stamp.push(0);
        }
    }
    Ok(stamp)
}
fn connection(path: &Path, write: bool) -> Result<Connection, String> {
    check(path)?;
    if write {
        private::protect(path)?;
    }
    let flags = if write {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    } else {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    };
    let db = Connection::open_with_flags(path, flags | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map_err(problem)?;
    db.execute_batch("PRAGMA trusted_schema=OFF;")
        .map_err(problem)?;
    Ok(db)
}
pub(crate) fn has_login(path: &Path) -> Result<bool, String> {
    connection(path, false)?
        .query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM cookies WHERE {AUTH})"),
            [],
            |row| row.get(0),
        )
        .map_err(problem)
}
pub(crate) fn present(root: &Path) -> Result<bool, String> {
    for source in sources(root) {
        match fs::symlink_metadata(&source.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(problem(e)),
            Ok(_) => {}
        }
        private::parents(source.path.parent().unwrap())?;
        private::safe(&source.path)?;
        if source.category == COOKIE_CATEGORY {
            // Only the presence of known cookie names is queried, never credential values.
            if has_login(&source.path)? {
                return Ok(true);
            }
            continue;
        }
        if !fs::metadata(&source.path).is_ok_and(|m| m.is_file() && m.len() <= 8 * 1024 * 1024) {
            return Err(problem("configuration size"));
        }
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        fs::File::open(&source.path)
            .map_err(problem)?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(problem)?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(problem("configuration size"));
        }
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(problem)?;
        if value
            .as_object()
            .ok_or_else(|| problem("configuration"))?
            .iter()
            .any(|(key, value)| config_key(key) && !value.is_null() && value.as_str() != Some(""))
        {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(crate) struct Snapshot {
    folder: PathBuf,
    pub(crate) path: PathBuf,
}
impl Drop for Snapshot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.folder);
    }
}
pub(crate) fn snapshot(path: &Path, vault: &Path) -> Result<Snapshot, String> {
    let folder = vault.join(format!(".stage-{}", new_id()?));
    private::create_directory(&folder)?;
    let snapshot = Snapshot {
        path: folder.join("Cookies"),
        folder,
    };
    drop(private::create(&snapshot.path)?);
    // SQLite backup includes committed WAL pages; copying the main file alone does not.
    let source = connection(path, false)?;
    schema(&source)?;
    source
        .backup(rusqlite::MAIN_DB, &snapshot.path, None)
        .map_err(problem)?;
    let db = connection(&snapshot.path, true)?;
    // VACUUM also removes previously freed pages, which may contain unrelated cookies.
    db.execute_batch(&format!("PRAGMA journal_mode=DELETE; PRAGMA secure_delete=ON; DELETE FROM cookies WHERE NOT ({AUTH}); VACUUM;")).map_err(problem)?;
    drop(db);
    Ok(snapshot)
}
pub(crate) fn clear(path: &Path) -> Result<(), String> {
    let db = connection(path, true)?;
    schema(&db)?;
    db.execute(&format!("DELETE FROM cookies WHERE {AUTH}"), [])
        .map_err(problem)?;
    Ok(())
}
#[derive(PartialEq)]
struct CookieSchema {
    objects: Vec<(String, String, Option<String>)>,
    versions: Vec<(String, Value)>,
}
fn schema(db: &Connection) -> Result<CookieSchema, String> {
    let objects = db.prepare("SELECT type,name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_stat%' ORDER BY type,name").map_err(problem)?
        .query_map([],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,Option<String>>(2)?))).map_err(problem)?
        .collect::<rusqlite::Result<Vec<_>>>().map_err(problem)?;
    if !objects.iter().all(|(kind, name, sql)| match kind.as_str() {
        "table" => {
            matches!(name.as_str(), "cookies" | "meta")
                && sql.as_ref().is_some_and(|sql| {
                    sql.trim_start()
                        .to_ascii_uppercase()
                        .starts_with("CREATE TABLE ")
                })
        }
        "index" => {
            name.starts_with("sqlite_autoindex_")
                || sql
                    .as_ref()
                    .is_some_and(|sql| sql.trim_start().to_ascii_uppercase().starts_with("CREATE "))
        }
        _ => false,
    }) {
        return Err(problem("unsupported cookie schema"));
    }
    let versions = db.prepare("SELECT key,value FROM meta WHERE key IN ('version','last_compatible_version') ORDER BY key").map_err(problem)?
        .query_map([],|row|Ok((row.get::<_,String>(0)?,row.get::<_,Value>(1)?))).map_err(problem)?
        .collect::<rusqlite::Result<Vec<_>>>().map_err(problem)?;
    if !versions.iter().any(|(key, _)| key == "version") {
        return Err(problem("cookie version"));
    }
    Ok(CookieSchema { objects, versions })
}
pub(crate) fn restore(path: &Path, saved: &Path) -> Result<(), String> {
    let source = connection(saved, false)?;
    let saved_schema = schema(&source)?;
    let mut query = source
        .prepare(&format!("SELECT * FROM cookies WHERE {AUTH}"))
        .map_err(problem)?;
    let columns = query
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    if !path.exists() {
        private::parents(path.parent().ok_or_else(|| problem("parent"))?)?;
        for suffix in ["-wal", "-shm", "-journal"] {
            match fs::symlink_metadata(sidecar(path, suffix)) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                _ => return Err(problem("orphaned cookie sidecar")),
            }
        }
        // A new profile has no unrelated cookies to preserve; retain the saved Chromium schema.
        let unrelated: bool = source
            .query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM cookies WHERE NOT ({AUTH}))"),
                [],
                |row| row.get(0),
            )
            .map_err(problem)?;
        if unrelated {
            return Err(problem("unexpected cookies"));
        }
        private::directory(path.parent().ok_or_else(|| problem("parent"))?)?;
        private::write(path, &fs::read(saved).map_err(problem)?)?;
        return Ok(());
    }
    let mut destination = connection(path, true)?;
    if schema(&destination)? != saved_schema {
        return Err(problem("incompatible schema"));
    }
    let target_columns = destination
        .prepare("SELECT * FROM cookies LIMIT 0")
        .map_err(problem)?
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    if columns != target_columns {
        return Err(problem("incompatible schema"));
    }
    let columns_sql = columns
        .iter()
        .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(",");
    let placeholders = vec!["?"; columns.len()].join(",");
    let transaction = destination.transaction().map_err(problem)?;
    transaction
        .execute(&format!("DELETE FROM cookies WHERE {AUTH}"), [])
        .map_err(problem)?;
    let mut rows = query.query([]).map_err(problem)?;
    while let Some(row) = rows.next().map_err(problem)? {
        let values = (0..columns.len())
            .map(|n| row.get::<_, Value>(n))
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(problem)?;
        transaction
            .execute(
                &format!("INSERT OR ABORT INTO cookies ({columns_sql}) VALUES ({placeholders})"),
                params_from_iter(values),
            )
            .map_err(problem)?;
    }
    transaction.commit().map_err(problem)
}
