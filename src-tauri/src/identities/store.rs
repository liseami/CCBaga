// [INPUT]: SQLite 目录、用户策略、白名单源、tar/gzip、认证加密与平台凭据桥。
// [OUTPUT]: 已验证身份归档、先暂存后切换、持久恢复记录、原位配置合并及明确的直接删除。
// [POS]: 身份的文件事务；SQLite 与文件系统由恢复日志协调，不能假装是同一原子事务。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::{
    codec,
    database::{Database, Record},
    private, Policy,
};
use crate::{
    cleaner::identity::{self, Source},
    native,
    progress::Reporter,
};
use flate2::{read::MultiGzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self},
    path::{Component, Path, PathBuf},
};
use zeroize::Zeroizing;
#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u32,
    id: String,
    home: PathBuf,
    platform: String,
    policy: Policy,
    sources: Vec<Source>,
    credentials: Vec<serde_json::Value>,
    fingerprints: Vec<Vec<u8>>,
}
pub(super) struct Stage {
    path: PathBuf,
    manifest: Manifest,
}
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
pub struct Store {
    vault: PathBuf,
    home: PathBuf,
    key: Zeroizing<Vec<u8>>,
    pub(super) db: Database,
    #[cfg(test)]
    pub(super) fail_after_clear: bool,
}
fn problem(_: impl std::fmt::Display) -> String {
    "身份归档读写或验证失败；请保留暂存文件并重试。".into()
}
fn walk(
    path: &Path,
    base: &Path,
    visit: &mut impl FnMut(&Path, &Path, bool) -> Result<(), String>,
) -> Result<(), String> {
    private::safe(path)?;
    let meta = fs::symlink_metadata(path).map_err(problem)?;
    visit(
        path,
        path.strip_prefix(base).map_err(problem)?,
        meta.is_dir(),
    )?;
    if meta.is_dir() {
        let mut entries = fs::read_dir(path)
            .map_err(problem)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(problem)?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            walk(&entry.path(), base, visit)?;
        }
    }
    Ok(())
}
fn append(
    builder: &mut tar::Builder<GzEncoder<codec::Seal<fs::File>>>,
    name: &Path,
    bytes: &[u8],
) -> Result<(), String> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o600);
    header.set_cksum();
    builder
        .append_data(&mut header, name, bytes)
        .map_err(problem)
}
impl Store {
    pub fn open(vault: &Path, home: &Path) -> Result<Self, String> {
        let db = Database::open(vault)?;
        let key_path = vault.join("key.bin");
        if !key_path.exists() {
            if !db.records()?.is_empty()
                || fs::read_dir(vault).map_err(problem)?.any(|e| {
                    e.ok()
                        .is_some_and(|e| e.path().extension().is_some_and(|x| x == "baga"))
                })
            {
                return Err("身份密钥缺失；不能创建新密钥覆盖已有身份。".into());
            }
            private::write(&key_path, &codec::random::<32>()?)?;
        }
        private::protect(&key_path)?;
        let key = Zeroizing::new(fs::read(&key_path).map_err(problem)?);
        if key.len() != 32 {
            return Err("身份密钥无效。".into());
        }
        for entry in fs::read_dir(vault).map_err(problem)? {
            let p = entry.map_err(problem)?.path();
            if p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.strip_prefix(".stage-").is_some_and(super::valid_id))
            {
                private::safe(&p)?;
                fs::remove_dir_all(p).map_err(problem)?;
            }
        }
        let records = db.records()?;
        for entry in fs::read_dir(vault).map_err(problem)? {
            let path = entry.map_err(problem)?.path();
            if let Some(id) = path
                .file_name()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix(".trash-"))
                .filter(|s| super::valid_id(s))
            {
                private::safe(&path)?;
                if let Some(record) = records.iter().find(|r| r.id == id) {
                    if record.archive != format!("{id}.baga") {
                        return Err("身份归档编号无效。".into());
                    }
                    let original = vault.join(&record.archive);
                    if !original.exists() {
                        fs::rename(&path, original).map_err(problem)?;
                    }
                } else {
                    fs::remove_file(path).map_err(problem)?;
                }
            }
        }
        Ok(Self {
            vault: vault.into(),
            home: home.into(),
            key,
            db,
            #[cfg(test)]
            fail_after_clear: false,
        })
    }
    fn stopped(&self) -> Result<(), String> {
        #[cfg(test)]
        if identity::root().ok().as_ref() != Some(&self.home) {
            return Ok(());
        }
        identity::ensure_stopped()
    }
    fn snapshot_keys(&self) -> Result<Vec<serde_json::Value>, String> {
        #[cfg(test)]
        if identity::root().ok().as_ref() != Some(&self.home) {
            return Ok(Vec::new());
        }
        native::identity_snapshot()
    }
    fn keys(
        &self,
        items: &[serde_json::Value],
        restore: bool,
        policy: &Policy,
    ) -> Result<(), String> {
        #[cfg(test)]
        if identity::root().ok().as_ref() != Some(&self.home) {
            return Ok(());
        }
        let transformed = if restore {
            super::credentials::restore(items, &native::identity_snapshot()?, policy)?
        } else {
            super::credentials::clear(items, policy)?
        };
        native::identity_keys(&transformed, restore)
    }
    #[cfg(test)]
    pub(super) fn records(&self) -> Vec<Record> {
        self.db.records().unwrap()
    }
    fn record(&self, id: &str) -> Result<Record, String> {
        if !super::valid_id(id) {
            return Err("身份编号无效。".into());
        }
        self.db
            .records()?
            .into_iter()
            .find(|r| r.id == id)
            .ok_or("身份不存在。".into())
    }
    fn load(&self, record: &Record) -> Result<Stage, String> {
        if !super::valid_id(&record.id) || record.archive != format!("{}.baga", record.id) {
            return Err("身份归档编号无效。".into());
        }
        let archive = self.vault.join(&record.archive);
        private::parents(&self.vault)?;
        private::safe(&archive)?;
        let folder = self.vault.join(format!(".stage-{}", super::new_id()?));
        private::directory(&folder)?;
        let result = (|| {
            let mut input = MultiGzDecoder::new(
                codec::Open::new(fs::File::open(archive).map_err(problem)?, &self.key)
                    .map_err(problem)?,
            );
            let mut files = std::collections::HashSet::new();
            {
                let mut tar = tar::Archive::new(&mut input);
                for entry in tar.entries().map_err(problem)? {
                    let mut entry = entry.map_err(problem)?;
                    let path = entry.path().map_err(problem)?.into_owned();
                    if path.as_os_str().is_empty()
                        || path
                            .components()
                            .any(|c| !matches!(c, Component::Normal(_)))
                        || !files.insert(path.clone())
                    {
                        return Err("身份归档成员无效。".into());
                    }
                    let kind = entry.header().entry_type();
                    if !kind.is_file() && !kind.is_dir() {
                        return Err("身份归档包含链接或特殊文件。".into());
                    }
                    let destination = folder.join(&path);
                    if kind.is_dir() {
                        private::directory(&destination)?;
                    } else {
                        private::directory(destination.parent().unwrap())?;
                        let mut output = private::create(&destination)?;
                        io::copy(&mut entry, &mut output).map_err(problem)?;
                        output.sync_all().map_err(problem)?;
                    }
                }
            }
            // tar 结束不能替代认证结尾；排空 gzip 和加密流，拒绝截断或附加内容。
            io::copy(&mut input, &mut io::sink()).map_err(problem)?;
            let manifest: Manifest =
                serde_json::from_slice(&fs::read(folder.join("manifest.json")).map_err(problem)?)
                    .map_err(problem)?;
            if manifest.version != 1
                || manifest.id != record.id
                || manifest.home != self.home
                || manifest.platform != std::env::consts::OS
                || manifest.sources.len() != manifest.fingerprints.len()
            {
                return Err("身份不属于当前电脑或归档版本无效。".into());
            }
            if manifest
                .sources
                .iter()
                .any(|s| !identity::allowed(&self.home, s))
            {
                return Err("身份路径超出允许范围。".into());
            }
            let expected: std::collections::HashSet<_> = manifest
                .sources
                .iter()
                .enumerate()
                .map(|(n, _)| format!("data/{n}"))
                .collect();
            for path in files {
                if path == Path::new("manifest.json") {
                    continue;
                }
                let mut parts = path.components();
                if parts.next() != Some(Component::Normal("data".as_ref())) {
                    return Err("身份归档成员无效。".into());
                }
                let second = parts.next().ok_or("身份归档成员无效。")?;
                let key = PathBuf::from("data")
                    .join(second.as_os_str())
                    .to_string_lossy()
                    .into_owned();
                if !expected.contains(&key) {
                    return Err("身份归档成员无效。".into());
                }
            }
            for (n, source) in manifest.sources.iter().enumerate() {
                let data = folder.join(format!("data/{n}"));
                if !data.exists() {
                    return Err("身份归档不完整。".into());
                }
                if !source.keys.is_empty() {
                    let value: serde_json::Value =
                        serde_json::from_slice(&fs::read(data).map_err(problem)?)
                            .map_err(problem)?;
                    if !value.is_object() {
                        return Err("身份配置不是对象。".into());
                    }
                }
            }
            Ok(manifest)
        })();
        match result {
            Ok(manifest) => Ok(Stage {
                path: folder,
                manifest,
            }),
            Err(e) => {
                let _ = fs::remove_dir_all(folder);
                Err(e)
            }
        }
    }
    pub(super) fn save(
        &self,
        name: &str,
        policy: &Policy,
        reporter: &Reporter,
    ) -> Result<(Record, Stage), String> {
        self.stopped()?;
        let sources = identity::sources(&self.home, policy)?;
        let fingerprints = sources
            .iter()
            .map(|s| identity::fingerprint(&s.path))
            .collect::<Result<Vec<_>, _>>()?;
        let credentials = self.snapshot_keys()?;
        super::credentials::clear(&credentials, policy)?;
        let id = super::new_id()?;
        let manifest = Manifest {
            version: 1,
            id: id.clone(),
            home: self.home.clone(),
            platform: std::env::consts::OS.into(),
            policy: policy.clone(),
            sources,
            credentials,
            fingerprints,
        };
        let pending = self.vault.join(format!(".pending-{id}"));
        let destination = self.vault.join(format!("{id}.baga"));
        let result = (|| {
            let output =
                codec::Seal::new(private::create(&pending)?, &self.key).map_err(problem)?;
            let mut tar = tar::Builder::new(GzEncoder::new(output, Compression::default()));
            for (n, source) in manifest.sources.iter().enumerate() {
                self.stopped()?;
                reporter.log(
                    "archive",
                    "running",
                    "正在压缩暂存身份。",
                    n,
                    Some(manifest.sources.len()),
                );
                let archive_path = PathBuf::from(format!("data/{n}"));
                if !source.keys.is_empty() {
                    let text = fs::read_to_string(&source.path).map_err(problem)?;
                    let bytes =
                        serde_json::to_vec(&identity::extract(source, &text)?).map_err(problem)?;
                    append(&mut tar, &archive_path, &bytes)?;
                } else if source.path.is_dir() {
                    let parent = source.path.parent().ok_or("身份路径无效。")?;
                    walk(&source.path, parent, &mut |path, relative, directory| {
                        self.stopped()?;
                        let suffix = relative
                            .strip_prefix(source.path.file_name().unwrap())
                            .map_err(problem)?;
                        let target = archive_path.join(suffix);
                        if directory {
                            tar.append_dir(target, path).map_err(problem)
                        } else {
                            let mut file = fs::File::open(path).map_err(problem)?;
                            tar.append_file(target, &mut file).map_err(problem)
                        }
                    })?;
                } else {
                    let mut file = fs::File::open(&source.path).map_err(problem)?;
                    tar.append_file(archive_path, &mut file).map_err(problem)?;
                }
            }
            append(
                &mut tar,
                Path::new("manifest.json"),
                &serde_json::to_vec(&manifest).map_err(problem)?,
            )?;
            let gzip = tar.into_inner().map_err(problem)?;
            let seal = gzip.finish().map_err(problem)?;
            let file = seal.finish().map_err(problem)?;
            file.sync_all().map_err(problem)?;
            drop(file);
            for (source, stamp) in manifest.sources.iter().zip(&manifest.fingerprints) {
                if identity::fingerprint(&source.path)? != *stamp {
                    return Err("身份数据发生变化；未切换身份。".into());
                }
            }
            self.stopped()?;
            fs::rename(&pending, &destination).map_err(problem)?;
            private::sync_parent(&destination);
            let record = Record {
                id,
                name: name.into(),
                archive: destination
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                saved_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(problem)?
                    .as_millis() as u64,
                bytes: fs::metadata(&destination).map_err(problem)?.len(),
                policy: policy.clone(),
                empty: manifest.sources.is_empty() && manifest.credentials.is_empty(),
            };
            let stage = self.load(&record)?;
            Ok((record, stage))
        })();
        if result.is_err() {
            let _ = fs::remove_file(pending);
        }
        result
    }
    pub(super) fn selected(stage: &Stage, policy: &Policy) -> Vec<(usize, Source)> {
        stage
            .manifest
            .sources
            .iter()
            .enumerate()
            .filter_map(|(n, s)| {
                if s.keys.is_empty() && s.category != "settings" && policy.keeps(&s.category) {
                    return None;
                }
                let mut s = s.clone();
                if s.keys.is_empty() && s.category == "settings" && policy.settings {
                    s.keys = vec![
                        "env.CLAUDE_CODE_OAUTH_TOKEN".into(),
                        "env.ANTHROPIC_API_KEY".into(),
                        "env.ANTHROPIC_AUTH_TOKEN".into(),
                        "apiKeyHelper".into(),
                    ];
                }
                if s.path == stage.manifest.home.join(".claude.json") && policy.settings {
                    s.keys.retain(|k| {
                        crate::cleaner::identity::account_key(k)
                            || k == "mcpServers"
                            || k == "projects"
                    });
                }
                if policy.mcp {
                    s.keys.retain(|k| {
                        ![
                            "mcpServers",
                            "mcpOAuth",
                            "mcpOAuthClientConfig",
                            "mcpXaaIdp",
                            "mcpXaaIdpConfig",
                        ]
                        .contains(&k.as_str())
                    });
                }
                if policy.plugins {
                    s.keys.retain(|k| k != "pluginSecrets");
                }
                if policy.projects {
                    s.keys.retain(|k| k != "projects");
                }
                if !stage.manifest.sources[n].keys.is_empty() && s.keys.is_empty() {
                    return None;
                }
                Some((n, s))
            })
            .collect()
    }
    pub(super) fn clear(&self, sources: &[(usize, Source)]) -> Result<(), String> {
        for (_, source) in sources {
            self.stopped()?;
            if !identity::allowed(&self.home, source) {
                return Err("身份路径超出允许范围。".into());
            }
            if fs::symlink_metadata(&source.path).is_err() {
                continue;
            }
            private::parents(source.path.parent().unwrap())?;
            private::safe(&source.path)?;
            if !source.keys.is_empty() {
                identity::apply_config(source, &serde_json::json!({}))?;
            } else if source.path.is_dir() {
                fs::remove_dir_all(&source.path).map_err(problem)?;
            } else {
                fs::remove_file(&source.path).map_err(problem)?;
            }
        }
        Ok(())
    }
    fn restore(
        &self,
        stage: &Stage,
        sources: &[(usize, Source)],
        policy: &Policy,
    ) -> Result<(), String> {
        for (n, source) in sources {
            self.stopped()?;
            private::parents(source.path.parent().unwrap())?;
            if source.path.exists() {
                private::safe(&source.path)?;
            }
            let data = stage.path.join(format!("data/{n}"));
            if !source.keys.is_empty() {
                let bytes = fs::read(data).map_err(problem)?;
                let saved = if stage.manifest.sources[*n].keys.is_empty() {
                    identity::extract(source, std::str::from_utf8(&bytes).map_err(problem)?)?
                } else {
                    serde_json::from_slice(&bytes).map_err(problem)?
                };
                identity::apply_config(source, &saved)?;
            } else {
                if source.path.exists() {
                    return Err("身份恢复位置已有数据；请使用恢复操作。".into());
                }
                if data.is_dir() {
                    walk(&data, &data, &mut |path, relative, directory| {
                        self.stopped()?;
                        let target = source.path.join(relative);
                        if directory {
                            private::directory(&target)
                        } else {
                            private::directory(target.parent().unwrap())?;
                            let mut dest = private::create(&target)?;
                            io::copy(&mut fs::File::open(path).map_err(problem)?, &mut dest)
                                .map_err(problem)?;
                            dest.sync_all().map_err(problem)
                        }
                    })?;
                } else {
                    private::directory(source.path.parent().unwrap())?;
                    let mut output = private::create(&source.path)?;
                    io::copy(&mut fs::File::open(data).map_err(problem)?, &mut output)
                        .map_err(problem)?;
                    output.sync_all().map_err(problem)?;
                }
            }
        }
        self.stopped()?;
        self.keys(&stage.manifest.credentials, true, policy)
    }
    pub fn delete(&mut self, id: &str, policy: &Policy, reporter: &Reporter) -> Result<(), String> {
        if self.db.get("recovery")?.is_some() {
            return Err("请先恢复未完成的身份切换。".into());
        }
        if id == "current" {
            self.stopped()?;
            let sources = identity::sources(&self.home, policy)?;
            let fingerprints = sources
                .iter()
                .map(|s| identity::fingerprint(&s.path))
                .collect::<Result<Vec<_>, _>>()?;
            let credentials = self.snapshot_keys()?;
            // 预先验证凭据格式和范围，删除没有隐式恢复副本。
            super::credentials::clear(&credentials, policy)?;
            for (s, stamp) in sources.iter().zip(&fingerprints) {
                if identity::fingerprint(&s.path)? != *stamp {
                    return Err("身份数据发生变化；未修改身份。".into());
                }
            }
            reporter.log(
                "delete",
                "running",
                "正在直接删除所选本地身份数据。",
                0,
                None,
            );
            let selected: Vec<_> = sources.into_iter().enumerate().collect();
            self.clear(&selected)
                .and_then(|_| self.keys(&credentials, false, policy))
                .map_err(|_| "直接删除未完成；部分数据可能已删除，请刷新检查。".to_string())?;
            self.db.complete("empty")?;
        } else {
            let record = self.record(id)?;
            if record.archive != format!("{id}.baga") {
                return Err("身份归档编号无效。".into());
            }
            let path = self.vault.join(&record.archive);
            private::safe(&path)?;
            let trash = self.vault.join(format!(".trash-{id}"));
            if trash.exists() {
                return Err("身份删除事务尚未完成，请重新打开身份面板。".into());
            }
            fs::rename(&path, &trash).map_err(problem)?;
            private::sync_parent(&trash);
            if let Err(e) = self.db.forget(id) {
                let _ = fs::rename(&trash, &path);
                return Err(e);
            }
            // 数据库提交后，这个仅由自身命名的加密文件可安全回收；失败留待下次回收。
            fs::remove_file(&trash)
                .map_err(|_| "身份已从目录移除，但暂存文件回收失败；请重新打开面板。")?;
            private::sync_parent(&trash);
        }
        reporter.log("verify", "ok", "所选本地身份已直接删除。", 1, Some(1));
        Ok(())
    }
    pub fn switch(
        &mut self,
        target: &str,
        name: &str,
        policy: &Policy,
        reporter: &Reporter,
    ) -> Result<(), String> {
        if self.db.get("recovery")?.is_some() {
            return Err("请先恢复未完成的身份切换。".into());
        }
        let incoming = if target == "empty" {
            None
        } else {
            Some(self.load(&self.record(target)?)?)
        };
        reporter.log("archive", "running", "正在验证并暂存当前身份。", 0, None);
        let (record, outgoing) = self.save(name, policy, reporter)?;
        let previous = self.db.get("active")?.unwrap_or_else(|| "current".into());
        self.db.pending(&record, target)?;
        let current = Self::selected(&outgoing, policy);
        let next = incoming
            .as_ref()
            .map(|s| Self::selected(s, policy))
            .unwrap_or_default();
        let mut mutated = false;
        let result: Result<(), String> = (|| {
            self.stopped()?;
            reporter.log("switch", "running", "暂存已验证，正在切换身份。", 0, None);
            for (source, stamp) in outgoing
                .manifest
                .sources
                .iter()
                .zip(&outgoing.manifest.fingerprints)
            {
                if identity::fingerprint(&source.path)? != *stamp {
                    return Err("身份数据发生变化；未切换身份。".into());
                }
            }
            mutated = true;
            self.clear(&current)?;
            #[cfg(test)]
            if self.fail_after_clear {
                return Err("fixture rollback".into());
            }
            self.keys(&outgoing.manifest.credentials, false, policy)?;
            if let Some(stage) = incoming.as_ref() {
                self.restore(stage, &next, policy)?;
            }
            self.db.complete(target)?;
            Ok(())
        })();
        if let Err(error) = result {
            if !mutated {
                self.db.complete(&previous)?;
                return Err(error);
            }
            if self.stopped().is_ok() {
                let rollback = (|| {
                    self.clear(&current)?;
                    self.clear(&next)?;
                    let live = self.snapshot_keys()?;
                    self.keys(&live, false, &outgoing.manifest.policy)?;
                    self.restore(&outgoing, &current, &outgoing.manifest.policy)?;
                    self.db.complete(&record.id)
                })();
                if rollback.is_ok() {
                    return Err(format!("{error} 已恢复切换前的身份。"));
                }
            }
            return Err(format!("{error} 原身份已暂存，请使用恢复操作。"));
        }
        reporter.log(
            "verify",
            "ok",
            "身份切换完成，原身份已压缩暂存。",
            1,
            Some(1),
        );
        Ok(())
    }
    pub fn recover(&mut self, name: &str, reporter: &Reporter) -> Result<(), String> {
        let id = self.db.get("recovery")?.ok_or("没有需要恢复的身份事务。")?;
        let record = self.record(&id)?;
        let outgoing = self.load(&record)?;
        let target = self.db.get("target")?.unwrap_or_else(|| "empty".into());
        let incoming = if target == "empty" {
            None
        } else {
            Some(self.load(&self.record(&target)?)?)
        };
        self.stopped()?;
        reporter.log("switch", "running", "正在恢复未完成的身份切换。", 0, None);
        // 恢复前再保存现状，避免崩溃后用户新产生的数据被恢复操作覆盖。
        let (checkpoint, current_stage) = self.save(name, &outgoing.manifest.policy, reporter)?;
        self.db.remember(&checkpoint)?;
        self.clear(&Self::selected(&current_stage, &outgoing.manifest.policy))?;
        let current = Self::selected(&outgoing, &outgoing.manifest.policy);
        self.clear(&current)?;
        if let Some(stage) = incoming.as_ref() {
            self.clear(&Self::selected(stage, &outgoing.manifest.policy))?;
        }
        let live = self.snapshot_keys()?;
        self.keys(&live, false, &outgoing.manifest.policy)?;
        self.restore(&outgoing, &current, &outgoing.manifest.policy)?;
        self.db.complete(&record.id)?;
        reporter.log("verify", "ok", "原身份已恢复。", 1, Some(1));
        Ok(())
    }
}
