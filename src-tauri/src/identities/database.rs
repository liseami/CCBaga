// [INPUT]: rusqlite/内嵌 SQLite、私有保险库目录和脱敏身份元数据。
// [OUTPUT]: 身份目录、保留策略、当前身份、持久切换/恢复状态与身份移除事务。
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
}
pub struct Database {
    connection: Connection,
}
fn error(_: rusqlite::Error) -> String {
    "身份数据库无法读写；未提交身份切换。".into()
}
impl Database {
    pub fn open(root: &Path) -> Result<Self, String> {
        super::private::directory(root)?;
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
            PRAGMA user_version=1;").map_err(error)?;
        Ok(Self { connection })
    }
    pub fn records(&self) -> Result<Vec<Record>, String> {
        let mut query=self.connection.prepare("SELECT id,name,archive,saved_at,bytes,policy,empty FROM identities ORDER BY saved_at DESC,id").map_err(error)?;
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
                ))
            })
            .map_err(error)?;
        rows.map(|r| {
            let (id, name, archive, saved_at, bytes, policy, empty) = r.map_err(error)?;
            let saved_at = u64::try_from(saved_at).map_err(|_| "身份元数据无效。")?;
            let bytes = u64::try_from(bytes).map_err(|_| "身份元数据无效。")?;
            Ok(Record {
                id,
                name,
                archive,
                saved_at,
                bytes,
                empty,
                policy: serde_json::from_str(&policy).map_err(|_| "身份策略无法读取。")?,
            })
        })
        .collect()
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
                "INSERT INTO identities VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    record.id,
                    record.name,
                    record.archive,
                    saved_at,
                    bytes,
                    serde_json::to_string(&record.policy).unwrap(),
                    record.empty
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
            "INSERT INTO identities VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                record.id,
                record.name,
                record.archive,
                saved_at,
                bytes,
                serde_json::to_string(&record.policy).unwrap(),
                record.empty
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
        tx.commit().map_err(error)
    }
    pub fn complete(&mut self, active: &str) -> Result<(), String> {
        let tx = self.connection.transaction().map_err(error)?;
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
