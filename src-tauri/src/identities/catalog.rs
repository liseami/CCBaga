// [INPUT]: 有界 OAuth 账号元数据、本机密钥、已认证旧归档与 SQLite 目录。
// [OUTPUT]: 稳定的本机 HMAC 账号索引、显式事务内旧账号索引迁移与已提交归档代次回收；缺摘要不主动解包。
// [POS]: 只按账号 UUID/组织（缺失时邮箱/组织）匹配；Claude Code 缺账号元数据时回退到 Claude Desktop 的 lastKnownAccountUuid；令牌刷新、名称变更不产生新身份。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::{database::Database, private, store::Store};
use ring::hmac;
use serde_json::Value;
use std::{fs, io::Read, path::Path};
pub(super) fn archive_name(name: &str) -> bool {
    name.strip_suffix(".baga").is_some_and(super::valid_id)
}
pub(super) fn account_key(value: &Value, key: &[u8]) -> Option<String> {
    let account = &value["oauthAccount"];
    let field = |name: &str| {
        account[name]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.len() <= 254 && !s.chars().any(char::is_control))
    };
    let org = field("organizationUuid")
        .or_else(|| field("organizationName"))
        .unwrap_or("");
    let identity = if let Some(uuid) = field("accountUuid") {
        ("uuid", uuid.to_string(), org)
    } else {
        let email = field("emailAddress")?.to_lowercase();
        if !email.contains('@') {
            return None;
        }
        ("email", email, org)
    };
    let bytes = serde_json::to_vec(&identity).ok()?;
    let tag = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, key), &bytes);
    Some(tag.as_ref().iter().map(|b| format!("{b:02x}")).collect())
}
/// 有界读取一个 JSON 账号文件并计算 HMAC 摘要；`extract` 决定从哪个结构与字段取标识。
fn key_from_file(
    path: &Path,
    key: &[u8],
    extract: fn(&Value, &[u8]) -> Option<String>,
) -> Result<Option<String>, String> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("身份账号索引无法读取。".into()),
    };
    private::parents(path.parent().ok_or("身份路径无效。")?)?;
    private::safe(path)?;
    if !meta.is_file() || meta.len() > 8 * 1024 * 1024 {
        return Ok(None);
    }
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    fs::File::open(path)
        .map_err(|_| "身份账号索引无法读取。")?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "身份账号索引无法读取。")?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Ok(None);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "身份配置不是有效 JSON。")?;
    Ok(extract(&value, key))
}

fn file_key(path: &Path, key: &[u8]) -> Result<Option<String>, String> {
    key_from_file(path, key, account_key)
}
/// Claude Desktop 的 config.json 是扁平结构，没有 oauthAccount。
/// 用独立的 `desktop-uuid` 命名空间，避免与基于 Claude Code 的 uuid/email 摘要互相污染。
fn desktop_account_key(value: &Value, key: &[u8]) -> Option<String> {
    let field = |name: &str| {
        value[name]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.len() <= 254 && !s.chars().any(char::is_control))
    };
    let uuid = field("lastKnownAccountUuid")?;
    let org = field("lastKnownOrgUuid")
        .or_else(|| field("orgUuid"))
        .unwrap_or("");
    let identity = ("desktop-uuid", uuid.to_string(), org.to_string());
    let bytes = serde_json::to_vec(&identity).ok()?;
    let tag = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, key), &bytes);
    Some(tag.as_ref().iter().map(|b| format!("{b:02x}")).collect())
}

pub(super) fn live_key(home: &Path, key: &[u8]) -> Result<Option<String>, String> {
    if let Some(found) = file_key(&home.join(".claude.json"), key)? {
        return Ok(Some(found));
    }
    // Claude Code 侧没有 oauthAccount 时（典型情况：只用 Claude Desktop），
    // 回退到桌面端 config.json 的 lastKnownAccountUuid，否则每次保存都会新建槽位。
    // 桌面端配置缺失、损坏或不属于当前用户一律按“无摘要”处理，不应让身份操作失败。
    for root in crate::cleaner::desktop_roots(home) {
        if let Ok(Some(found)) = key_from_file(&root.join("config.json"), key, desktop_account_key) {
            return Ok(Some(found));
        }
    }
    Ok(None)
}
pub(super) fn current_id(
    vault: &Path,
    home: &Path,
    db: &Database,
) -> Result<Option<String>, String> {
    let key_path = vault.join("key.bin");
    if !key_path.exists() {
        return Ok(None);
    }
    private::parents(vault)?;
    private::safe(&key_path)?;
    if fs::metadata(&key_path)
        .map_err(|_| "身份密钥无法读取。")?
        .len()
        != 32
    {
        return Err("身份密钥无效。".into());
    }
    let key = zeroize::Zeroizing::new(fs::read(key_path).map_err(|_| "身份密钥无法读取。")?);
    Ok(match live_key(home, &key)? {
        Some(key) => db.by_account(&key)?.map(|r| r.id),
        None => None,
    })
}
impl Store {
    pub(super) fn index_legacy(&self) -> Result<(), String> {
        for record in self
            .db
            .records()?
            .into_iter()
            .filter(|r| r.account_key.is_none())
        {
            // 损坏的旧副本不自动删除；用户仍可单独删除或保留排查。
            let Ok(stage) = self.load(&record) else {
                continue;
            };
            if record.account_key.is_none() {
                let key = if let Some(key) = &stage.manifest.account_key {
                    Some(key.clone())
                } else if let Some(n) = stage
                    .manifest
                    .sources
                    .iter()
                    .position(|s| s.path == self.home.join(".claude.json"))
                {
                    file_key(&stage.path.join(format!("data/{n}")), &self.key)?
                } else {
                    None
                };
                self.db.index(&record.id, key.as_deref().unwrap_or(""))?;
            }
            if !record.empty {
                let _ = self
                    .db
                    .set_summary(&record.id, &super::profile::summarize_stage(&stage));
            }
        }
        Ok(())
    }
    pub(super) fn collect_generations(&self) -> Result<(), String> {
        let records = self.db.records()?;
        for entry in fs::read_dir(&self.vault).map_err(|_| "身份保险库无法读取。")? {
            let entry = entry.map_err(|_| "身份保险库无法读取。")?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if (archive_name(&name) && !records.iter().any(|r| r.archive == name))
                || name.strip_prefix(".pending-").is_some_and(super::valid_id)
            {
                private::safe(&entry.path())?;
                fs::remove_file(entry.path()).map_err(|_| "旧身份归档回收失败。")?;
            }
        }
        private::sync_parent(&self.vault.join("identities.sqlite"));
        Ok(())
    }
}
