// [INPUT]: rusqlite/内嵌 SQLite、私有保险库目录和脱敏身份元数据与本机加盐的账号索引。
// [OUTPUT]: 默认联动/显式关闭保留、首身份预留编号与身份目录、已保存身份脱敏摘要、保留策略、当前身份、SQLite v3 浏览器绑定与持久切换/恢复状态和身份移除事务；仅在互斥的提交后回收空归档记录。
// [POS]: 保险库的索引事务；凭据和会话正文仅进入加密归档，不进入数据库。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
#[derive(Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub name: String,
    pub archive: String,
    pub saved_at: u64,
    pub bytes: u64,
    pub policy: super::Policy,
    pub empty: bool,
    #[serde(skip)]
    pub account_key: Option<String>,
}
pub struct Database {
    pub(super) connection: Connection,
}
pub(super) fn error(_: rusqlite::Error) -> String {
    "身份数据库无法读写；未提交身份切换。".into()
}
impl Database {
    pub fn open(root: &Path) -> Result<Self, String> {
        super::private::directory(root)?;
        super::private::protect(root)?;
        let path = root.join("identities.sqlite");
        if !path.exists() {
            super::private::create(&path)?
                .sync_all()
                .map_err(|_| "无法创建身份数据库。")?;
        }
        super::private::parents(root)?;
        super::private::protect(&path)?;
        let connection = Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(error)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(error)?;
        connection.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS identities(id TEXT PRIMARY KEY, name TEXT NOT NULL, archive TEXT NOT NULL, saved_at INTEGER NOT NULL, bytes INTEGER NOT NULL, policy TEXT NOT NULL,empty INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS state(key TEXT PRIMARY KEY,value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS identity_summaries(id TEXT PRIMARY KEY,summary TEXT NOT NULL);
").map_err(error)?;
        let tx = connection.unchecked_transaction().map_err(error)?;
        let version: i64 = tx
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(error)?;
        if version < 2 {
            tx.execute_batch(
                "ALTER TABLE identities ADD COLUMN account_key TEXT; PRAGMA user_version=2;",
            )
            .map_err(error)?;
        } else if version > 3 {
            return Err("身份目录版本较新，请更新应用。".into());
        }
        tx.execute_batch("CREATE TABLE IF NOT EXISTS browser_bindings(identity TEXT PRIMARY KEY,slot TEXT NOT NULL UNIQUE,seed INTEGER NOT NULL UNIQUE); PRAGMA user_version=3;").map_err(error)?;
        // 未设置过的用户默认联动；显式关闭和已有资料指针始终保留。
        tx.execute_batch("INSERT OR IGNORE INTO state VALUES('browser.enabled','1'); INSERT OR IGNORE INTO state VALUES('browser.active','legacy');").map_err(error)?;
        tx.execute(
            "INSERT OR IGNORE INTO state VALUES('browser.draft',?1)",
            [super::new_id()?],
        )
        .map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(Self { connection })
    }
    pub fn summaries(
        &self,
    ) -> Result<std::collections::BTreeMap<String, super::profile::Summary>, String> {
        let mut query = self
            .connection
            .prepare("SELECT id,summary FROM identity_summaries")
            .map_err(error)?;
        let rows = query
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(error)?;
        let mut map = std::collections::BTreeMap::new();
        for row in rows {
            let (id, raw) = row.map_err(error)?;
            if let Ok(summary) = serde_json::from_str::<super::profile::Summary>(&raw) {
                map.insert(id, summary);
            }
        }
        Ok(map)
    }
    pub fn set_summary(&self, id: &str, summary: &super::profile::Summary) -> Result<(), String> {
        let text = serde_json::to_string(summary).map_err(|_| "身份摘要无法保存。")?;
        self.connection
            .execute(
                "INSERT INTO identity_summaries(id,summary) VALUES(?1,?2)
                 ON CONFLICT(id) DO UPDATE SET summary=excluded.summary",
                params![id, text],
            )
            .map_err(error)?;
        Ok(())
    }
    pub fn records(&self) -> Result<Vec<Record>, String> {
        let mut query=self.connection.prepare("SELECT id,name,archive,saved_at,bytes,policy,empty,account_key FROM identities ORDER BY saved_at DESC,id").map_err(error)?;
        let rows = query
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, bool>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(error)?;
        rows.map(|r| {
            let (id, name, archive, saved_at, bytes, policy, empty, account_key) =
                r.map_err(error)?;
            let saved_at = u64::try_from(saved_at).map_err(|_| "身份元数据无效。")?;
            let bytes = u64::try_from(bytes).map_err(|_| "身份元数据无效。")?;
            Ok(Record {
                id,
                name,
                archive,
                saved_at,
                bytes,
                empty,
                account_key,
                policy: serde_json::from_str(&policy).map_err(|_| "身份策略无法读取。")?,
            })
        })
        .collect()
    }
    pub fn by_account(&self, key: &str) -> Result<Option<Record>, String> {
        Ok(self
            .records()?
            .into_iter()
            .find(|r| r.account_key.as_deref() == Some(key)))
    }
    pub fn index(&self, id: &str, key: &str) -> Result<(), String> {
        self.connection
            .execute(
                "UPDATE identities SET account_key=?2 WHERE id=?1",
                params![id, key],
            )
            .map_err(error)?;
        Ok(())
    }
    pub fn compact(&mut self) -> Result<(), String> {
        if self.get("recovery")?.is_some() {
            return Ok(());
        }
        let records = self.records()?;
        let tx = self.connection.transaction().map_err(error)?;
        tx.execute(
            "DELETE FROM identity_summaries WHERE id IN (SELECT id FROM identities WHERE empty != 0)",
            [],
        )
        .map_err(error)?;
        tx.execute("DELETE FROM identities WHERE empty != 0", [])
            .map_err(error)?;
        let mut seen = std::collections::HashMap::<String, String>::new();
        for record in records.into_iter().filter(|r| !r.empty) {
            if let Some(key) = record.account_key.filter(|k| !k.is_empty()) {
                if let Some(id) = seen.get(&key) {
                    super::browser_link::merge(&tx, &record.id, id)?;
                    tx.execute(
                        "UPDATE state SET value=?2 WHERE key='active' AND value=?1",
                        params![record.id, id],
                    )
                    .map_err(error)?;
                    tx.execute("DELETE FROM identity_summaries WHERE id=?1", [&record.id])
                        .map_err(error)?;
                    tx.execute("DELETE FROM identities WHERE id=?1", [record.id])
                        .map_err(error)?;
                } else {
                    seen.insert(key, record.id);
                }
            }
        }
        tx.commit().map_err(error)
    }
    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        self.connection
            .query_row("SELECT value FROM state WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .map_err(error)
    }
    pub fn remember(&self, record: &Record) -> Result<(), String> {
        let saved_at = i64::try_from(record.saved_at).map_err(|_| "身份元数据无效。")?;
        let bytes = i64::try_from(record.bytes).map_err(|_| "身份元数据无效。")?;
        self.connection
            .execute(
                "INSERT INTO identities(id,name,archive,saved_at,bytes,policy,empty,account_key) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                 ON CONFLICT(id) DO UPDATE SET archive=excluded.archive,saved_at=excluded.saved_at,bytes=excluded.bytes,policy=excluded.policy,empty=excluded.empty,account_key=excluded.account_key",
                params![
                    record.id,
                    record.name,
                    record.archive,
                    saved_at,
                    bytes,
                    serde_json::to_string(&record.policy).unwrap(),
                    record.empty,
                    record.account_key
                ],
            )
            .map_err(error)?;
        Ok(())
    }
    pub fn pending(&mut self, record: &Record, target: &str) -> Result<(), String> {
        let saved_at = i64::try_from(record.saved_at).map_err(|_| "身份元数据无效。")?;
        let bytes = i64::try_from(record.bytes).map_err(|_| "身份元数据无效。")?;
        let tx = self.connection.transaction().map_err(error)?;
        tx.execute(
            "INSERT INTO identities(id,name,archive,saved_at,bytes,policy,empty,account_key) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                 ON CONFLICT(id) DO UPDATE SET archive=excluded.archive,saved_at=excluded.saved_at,bytes=excluded.bytes,policy=excluded.policy,empty=excluded.empty,account_key=excluded.account_key",
            params![
                record.id,
                record.name,
                record.archive,
                saved_at,
                bytes,
                serde_json::to_string(&record.policy).unwrap(),
                record.empty,
                record.account_key
            ],
        )
        .map_err(error)?;
        tx.execute(
            "INSERT OR REPLACE INTO state VALUES('recovery',?1)",
            [&record.id],
        )
        .map_err(error)?;
        tx.execute("INSERT OR REPLACE INTO state VALUES('target',?1)", [target])
            .map_err(error)?;
        super::browser_link::pending(&tx, record, target)?;
        tx.commit().map_err(error)
    }
    pub fn complete(&mut self, active: &str) -> Result<(), String> {
        let tx = self.connection.transaction().map_err(error)?;
        super::browser_link::complete(&tx, active)?;
        tx.execute("INSERT OR REPLACE INTO state VALUES('active',?1)", [active])
            .map_err(error)?;
        tx.execute("DELETE FROM state WHERE key IN ('recovery','target')", [])
            .map_err(error)?;
        tx.commit().map_err(error)
    }
    pub fn forget(&mut self, id: &str) -> Result<(), String> {
        let tx = self.connection.transaction().map_err(error)?;
        if tx
            .execute("DELETE FROM identities WHERE id=?1", [id])
            .map_err(error)?
            == 0
        {
            return Err("身份不存在。".into());
        }
        tx.execute("DELETE FROM identity_summaries WHERE id=?1", [id])
            .map_err(error)?;
        tx.execute(
            "UPDATE state SET value='current' WHERE key='active' AND value=?1",
            [id],
        )
        .map_err(error)?;
        tx.commit().map_err(error)
    }
    pub fn rename(&self, id: &str, name: &str) -> Result<(), String> {
        if self
            .connection
            .execute(
                "UPDATE identities SET name=?2 WHERE id=?1",
                params![id, name],
            )
            .map_err(error)?
            == 0
        {
            return Err("身份不存在。".into());
        }
        Ok(())
    }
}
