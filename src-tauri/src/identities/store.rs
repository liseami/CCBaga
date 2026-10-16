// [INPUT]: SQLite 目录、用户策略、白名单源、已验收归档代次、认证加密与平台凭据桥。
// [OUTPUT]: 已验证身份归档、先暂存后切换、持久恢复记录、原位配置合并、直接删除与清除/恢复/凭据/提交/回滚真实进度。
// [POS]: 身份的文件事务；SQLite 与文件系统由恢复日志协调，不能假装是同一原子事务。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::{
    archive::{walk, Stage},
    codec,
    database::{Database, Record},
    private, Policy,
};
use crate::{
    cleaner::identity::{self, Source},
    native,
    progress::Reporter,
};
use std::{
    fs,
    io::{self},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;
pub struct Store {
    pub(super) vault: PathBuf,
    pub(super) home: PathBuf,
    pub(super) key: Zeroizing<Vec<u8>>,
    pub(super) db: Database,
    process_check: std::cell::Cell<std::time::Instant>,
    #[cfg(test)]
    pub(super) fail_after_clear: bool,
}
pub(super) fn problem(_: impl std::fmt::Display) -> String {
    "身份归档读写或验证失败；请保留暂存文件并重试。".into()
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
                    if !super::catalog::archive_name(&record.archive) {
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
        let store = Self {
            vault: vault.into(),
            home: home.into(),
            key,
            db,
            process_check: std::cell::Cell::new(std::time::Instant::now()),
            #[cfg(test)]
            fail_after_clear: false,
        };
        if store.db.get("recovery")?.is_none() {
            store.index_legacy()?;
            store.collect_generations()?;
        }
        Ok(store)
    }
    pub(super) fn stopped(&self) -> Result<(), String> {
        #[cfg(test)]
        if identity::root().ok().as_ref() != Some(&self.home) {
            return Ok(());
        }
        self.process_check.set(std::time::Instant::now());
        identity::ensure_stopped()
    }
    pub(super) fn during_io(&self) -> Result<(), String> {
        if self.process_check.get().elapsed() >= std::time::Duration::from_millis(250) {
            self.stopped()?;
        }
        Ok(())
    }
    pub(super) fn snapshot_keys(&self) -> Result<Vec<serde_json::Value>, String> {
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
                    s.keys = crate::cleaner::identity::SETTINGS_IDENTITY_KEYS
                        .iter()
                        .map(|k| (*k).into())
                        .collect();
                }
                if s.path == stage.manifest.home.join(".claude.json") && policy.settings {
                    s.keys.retain(|k| {
                        crate::cleaner::identity::account_key(k)
                            || k == "mcpServers"
                            || k == "projects"
                            || k == "githubRepoPaths"
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
                    s.keys.retain(|k| k != "projects" && k != "githubRepoPaths");
                }
                if !stage.manifest.sources[n].keys.is_empty() && s.keys.is_empty() {
                    return None;
                }
                Some((n, s))
            })
            .collect()
    }
    pub(super) fn clear(
        &self,
        sources: &[(usize, Source)],
        reporter: &Reporter,
        phase: &str,
    ) -> Result<(), String> {
        reporter.log(
            phase,
            "running",
            "正在清除所选工作区文件。",
            0,
            Some(sources.len()),
        );
        for (index, (_, source)) in sources.iter().enumerate() {
            self.stopped()?;
            if !identity::allowed(&self.home, source) {
                return Err("身份路径超出允许范围。".into());
            }
            match fs::symlink_metadata(&source.path) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(problem(error)),
                Ok(_) => {
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
            }
            reporter.log(
                phase,
                "running",
                "已清除工作区文件。",
                index + 1,
                Some(sources.len()),
            );
        }
        reporter.log(
            phase,
            "ok",
            "所选工作区文件已清除。",
            sources.len(),
            Some(sources.len()),
        );
        Ok(())
    }
    fn restore(
        &self,
        stage: &Stage,
        sources: &[(usize, Source)],
        policy: &Policy,
        reporter: &Reporter,
        phase: &str,
    ) -> Result<(), String> {
        reporter.log(
            phase,
            "running",
            "正在恢复身份文件。",
            0,
            Some(sources.len()),
        );
        for (index, (n, source)) in sources.iter().enumerate() {
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
                        self.during_io()?;
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
            reporter.log(
                phase,
                "running",
                "已恢复身份文件。",
                index + 1,
                Some(sources.len()),
            );
        }
        reporter.log(
            phase,
            "ok",
            "所选身份文件已恢复。",
            sources.len(),
            Some(sources.len()),
        );
        self.stopped()?;
        let phase = if phase == "rollback" {
            phase
        } else {
            "credentials"
        };
        reporter.log(phase, "running", "正在恢复本地登录凭据。", 0, None);
        self.keys(&stage.manifest.credentials, true, policy)?;
        reporter.log(phase, "ok", "本地登录凭据已恢复。", 1, Some(1));
        Ok(())
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
            self.clear(&selected, reporter, "clear")
                .and_then(|_| {
                    reporter.log(
                        "credentials",
                        "running",
                        "正在清除所选本地登录凭据。",
                        0,
                        None,
                    );
                    self.keys(&credentials, false, policy)?;
                    reporter.log("credentials", "ok", "所选本地登录凭据已清除。", 1, Some(1));
                    Ok(())
                })
                .map_err(|_| "直接删除未完成；部分数据可能已删除，请刷新检查。".to_string())?;
            reporter.log("commit", "running", "正在保存空身份状态。", 0, Some(1));
            self.db.complete("empty")?;
            reporter.log("commit", "ok", "空身份状态已保存。", 1, Some(1));
        } else {
            let record = self.record(id)?;
            if !super::catalog::archive_name(&record.archive) {
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
    pub fn save_current(
        &mut self,
        name: &str,
        policy: &Policy,
        reporter: &Reporter,
    ) -> Result<(), String> {
        if self.db.get("recovery")?.is_some() {
            return Err("请先恢复未完成的身份切换。".into());
        }
        self.stopped()?;
        identity::clear_stale_locks(&self.home);
        let (mut record, stage) = self.save(name, policy, reporter)?;
        if record.empty {
            return Err("当前未检测到可暂存的登录凭据或身份数据。".into());
        }
        record.name = name.into();
        reporter.log(
            "commit",
            "running",
            "正在更新身份保险库与浏览器绑定。",
            0,
            Some(1),
        );
        self.db.remember(&record)?;
        self.db.rename(&record.id, &record.name)?;
        let _ = self
            .db
            .set_summary(&record.id, &super::profile::summarize_stage(&stage));
        self.db.complete(&record.id)?;
        self.db.compact()?;
        let _ = self.collect_generations();
        reporter.log("commit", "ok", "身份保险库与浏览器绑定已更新。", 1, Some(1));
        reporter.log(
            "verify",
            "ok",
            "当前身份已加密备份到保险库，无需退出当前工作区。",
            1,
            Some(1),
        );
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
        reporter.log("validate", "running", "正在核对当前身份。", 0, None);
        let live_key = super::catalog::live_key(&self.home, &self.key)?;
        reporter.log("validate", "ok", "当前本地身份已核对。", 1, Some(1));
        if target != "empty" && live_key.is_some() && self.record(target)?.account_key == live_key {
            // 当前账号仍是最新状态；避免用自己的旧快照覆盖当前刷新过的登录。
            reporter.log("commit", "running", "正在保存当前身份选择。", 0, Some(1));
            self.db.complete(target)?;
            self.db.compact()?;
            let _ = self.collect_generations();
            reporter.log("commit", "ok", "当前身份选择已保存。", 1, Some(1));
            reporter.log("verify", "ok", "当前身份已经是所选身份。", 1, Some(1));
            return Ok(());
        }
        let incoming = if target == "empty" {
            None
        } else {
            reporter.log(
                "verify_archive",
                "running",
                "正在验收目标身份归档。",
                0,
                None,
            );
            let stage = self.load(&self.record(target)?)?;
            reporter.log(
                "verify_archive",
                "ok",
                "目标身份归档已完整验收。",
                1,
                Some(1),
            );
            Some(stage)
        };
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
            reporter.log(
                "validate",
                "running",
                "正在复核切换前的工作区文件。",
                0,
                Some(outgoing.manifest.sources.len()),
            );
            for (index, (source, stamp)) in outgoing
                .manifest
                .sources
                .iter()
                .zip(&outgoing.manifest.fingerprints)
                .enumerate()
            {
                if identity::fingerprint(&source.path)? != *stamp {
                    return Err("身份数据发生变化；未切换身份。".into());
                }
                reporter.log(
                    "validate",
                    "running",
                    "已复核切换前的工作区文件。",
                    index + 1,
                    Some(outgoing.manifest.sources.len()),
                );
            }
            reporter.log(
                "validate",
                "ok",
                "切换前工作区文件已完整复核。",
                outgoing.manifest.sources.len(),
                Some(outgoing.manifest.sources.len()),
            );
            mutated = true;
            self.clear(&current, reporter, "clear")?;
            identity::clear_stale_locks(&self.home);
            #[cfg(test)]
            if self.fail_after_clear {
                return Err("fixture rollback".into());
            }
            reporter.log(
                "credentials",
                "running",
                "正在清除当前本地登录凭据。",
                0,
                None,
            );
            self.keys(&outgoing.manifest.credentials, false, policy)?;
            reporter.log("credentials", "ok", "当前本地登录凭据已清除。", 1, Some(1));
            if let Some(stage) = incoming.as_ref() {
                self.restore(stage, &next, policy, reporter, "restore")?;
            }
            reporter.log(
                "commit",
                "running",
                "正在提交身份与浏览器选择。",
                0,
                Some(1),
            );
            self.db.complete(target)?;
            if !record.empty {
                let _ = self
                    .db
                    .set_summary(&record.id, &super::profile::summarize_stage(&outgoing));
            }
            if let Some(stage) = incoming.as_ref() {
                let _ = self
                    .db
                    .set_summary(target, &super::profile::summarize_stage(stage));
            }
            reporter.log("commit", "ok", "身份与浏览器选择已提交。", 1, Some(1));
            Ok(())
        })();
        if let Err(error) = result {
            if !mutated {
                reporter.log(
                    "rollback",
                    "running",
                    "身份未修改，正在撤销切换记录。",
                    0,
                    Some(1),
                );
                self.db.complete(&previous)?;
                reporter.log("rollback", "ok", "原身份选择已保留。", 1, Some(1));
                return Err(error);
            }
            reporter.log("rollback", "running", "切换失败，正在恢复原身份。", 0, None);
            if self.stopped().is_ok() {
                let rollback = (|| {
                    self.clear(&current, reporter, "rollback")?;
                    self.clear(&next, reporter, "rollback")?;
                    let live = self.snapshot_keys()?;
                    self.keys(&live, false, &outgoing.manifest.policy)?;
                    self.restore(
                        &outgoing,
                        &current,
                        &outgoing.manifest.policy,
                        reporter,
                        "rollback",
                    )?;
                    self.db.complete(&record.id)
                })();
                if rollback.is_ok() {
                    reporter.log("rollback", "ok", "已恢复切换前的身份。", 1, Some(1));
                    return Err(format!("{error} 已恢复切换前的身份。"));
                }
            }
            reporter.log(
                "rollback",
                "error",
                "自动恢复未完成，原身份已暂存。",
                0,
                None,
            );
            return Err(format!("{error} 原身份已暂存，请使用恢复操作。"));
        }
        self.db.compact()?;
        // 目录已提交后才回收旧代次；回收失败保留加密文件，不把成功切换报为失败。
        let _ = self.collect_generations();
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
        reporter.log(
            "verify_archive",
            "running",
            "正在验收待恢复的身份归档。",
            0,
            None,
        );
        let outgoing = self.load(&record)?;
        reporter.log(
            "verify_archive",
            "ok",
            "待恢复的身份归档已验收。",
            1,
            Some(1),
        );
        let target = self.db.get("target")?.unwrap_or_else(|| "empty".into());
        let incoming = if target == "empty" {
            None
        } else {
            reporter.log(
                "verify_archive",
                "running",
                "正在验收未完成的目标归档。",
                0,
                None,
            );
            let stage = self.load(&self.record(&target)?)?;
            reporter.log(
                "verify_archive",
                "ok",
                "未完成的目标归档已验收。",
                1,
                Some(1),
            );
            Some(stage)
        };
        self.stopped()?;
        reporter.log("switch", "running", "正在恢复未完成的身份切换。", 0, None);
        // 恢复前再保存现状，避免崩溃后用户新产生的数据被恢复操作覆盖。
        let (checkpoint, current_stage) =
            self.save_checkpoint(name, &outgoing.manifest.policy, reporter)?;
        if !checkpoint.empty {
            self.db.remember(&checkpoint)?;
            let _ = self.db.set_summary(
                &checkpoint.id,
                &super::profile::summarize_stage(&current_stage),
            );
        }
        self.clear(
            &Self::selected(&current_stage, &outgoing.manifest.policy),
            reporter,
            "clear",
        )?;
        let current = Self::selected(&outgoing, &outgoing.manifest.policy);
        self.clear(&current, reporter, "clear")?;
        if let Some(stage) = incoming.as_ref() {
            self.clear(
                &Self::selected(stage, &outgoing.manifest.policy),
                reporter,
                "clear",
            )?;
        }
        let live = self.snapshot_keys()?;
        reporter.log(
            "credentials",
            "running",
            "正在清除待恢复工作区的登录凭据。",
            0,
            None,
        );
        self.keys(&live, false, &outgoing.manifest.policy)?;
        reporter.log(
            "credentials",
            "ok",
            "待恢复工作区的登录凭据已清除。",
            1,
            Some(1),
        );
        self.restore(
            &outgoing,
            &current,
            &outgoing.manifest.policy,
            reporter,
            "restore",
        )?;
        reporter.log(
            "commit",
            "running",
            "正在提交恢复后的身份选择。",
            0,
            Some(1),
        );
        self.db.complete(&record.id)?;
        if !record.empty {
            let _ = self
                .db
                .set_summary(&record.id, &super::profile::summarize_stage(&outgoing));
        }
        self.db.compact()?;
        let _ = self.collect_generations();
        reporter.log("commit", "ok", "恢复后的身份选择已提交。", 1, Some(1));
        reporter.log("verify", "ok", "原身份已恢复。", 1, Some(1));
        Ok(())
    }
}
