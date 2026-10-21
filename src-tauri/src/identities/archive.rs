// [INPUT]: 白名单身份源、SQLite 账号目录、Zstandard/旧 gzip 与认证加密流。
// [OUTPUT]: 不可变归档代次与短期解包区，逐源完成/压缩落盘/完整归档验收的真实局部进度；排除明确瞬态锁和可重建调试指针但不删除原件，首身份复用空浏览器编号。
// [POS]: 保险库的数据格式边界，与 store 的工作区切换事务分离，旧 gzip 归档保持可读。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::{
    codec, compression,
    database::Record,
    private,
    store::{problem, Store},
    Policy,
};
use crate::{
    cleaner::identity::{self, Source},
    progress::Reporter,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, BufWriter, Write},
    path::{Component, Path, PathBuf},
};
#[derive(Serialize, Deserialize)]
pub(super) struct Manifest {
    pub(super) version: u32,
    pub(super) id: String,
    pub(super) home: PathBuf,
    pub(super) platform: String,
    pub(super) policy: Policy,
    pub(super) sources: Vec<Source>,
    pub(super) credentials: Vec<serde_json::Value>,
    pub(super) fingerprints: Vec<Vec<u8>>,
    #[serde(default)]
    pub(super) account_key: Option<String>,
}
pub(super) struct Stage {
    pub(super) path: PathBuf,
    pub(super) manifest: Manifest,
}
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
pub(super) fn walk(
    path: &Path,
    base: &Path,
    visit: &mut impl FnMut(&Path, &Path, bool) -> Result<(), String>,
) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(problem)?;
    if meta.file_type().is_symlink()
        && path.ends_with(Path::new(".claude").join("debug").join("latest"))
    {
        return Ok(());
    }
    private::safe(path)?;
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
fn append<W: Write>(
    builder: &mut tar::Builder<W>,
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
    pub(super) fn load(&self, record: &Record) -> Result<Stage, String> {
        if !super::valid_id(&record.id) || !super::catalog::archive_name(&record.archive) {
            return Err("身份归档编号无效。".into());
        }
        let archive = self.vault.join(&record.archive);
        private::parents(&self.vault)?;
        private::safe(&archive)?;
        let folder = self.vault.join(format!(".stage-{}", super::new_id()?));
        private::directory(&folder)?;
        let result = (|| {
            let mut input = compression::decode(
                codec::Open::new(fs::File::open(archive).map_err(problem)?, &self.key)
                    .map_err(problem)?,
            )
            .map_err(problem)?;
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
                        // 工作区是可丢弃的；恢复依据是已 fsync 的完整认证归档。
                    }
                }
            }
            // tar 结束不能替代认证结尾；排空压缩和加密流，拒绝截断或附加内容。
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
                let key = format!("data/{}", second.as_os_str().to_string_lossy());
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
        self.save_inner(name, policy, reporter, true)
    }
    pub(super) fn save_checkpoint(
        &self,
        name: &str,
        policy: &Policy,
        reporter: &Reporter,
    ) -> Result<(Record, Stage), String> {
        self.save_inner(name, policy, reporter, false)
    }
    fn save_inner(
        &self,
        name: &str,
        policy: &Policy,
        reporter: &Reporter,
        reuse: bool,
    ) -> Result<(Record, Stage), String> {
        self.stopped()?;
        reporter.log("validate", "running", "正在检查当前身份文件。", 0, None);
        let sources = identity::sources(&self.home, policy)?
            .into_iter()
            .filter(|s| !identity::transient_lock(&self.home, &s.path, &s.path))
            .collect::<Vec<_>>();
        let mut fingerprints = Vec::with_capacity(sources.len());
        for (index, source) in sources.iter().enumerate() {
            fingerprints.push(identity::fingerprint(&source.path)?);
            reporter.log(
                "validate",
                "running",
                "已检查身份文件快照。",
                index + 1,
                Some(sources.len()),
            );
        }
        reporter.log(
            "validate",
            "ok",
            "当前身份文件快照已完整检查。",
            sources.len(),
            Some(sources.len()),
        );
        reporter.log(
            "credentials",
            "running",
            "正在保存所选本地登录凭据。",
            0,
            None,
        );
        let credentials = self.snapshot_keys()?;
        super::credentials::clear(&credentials, policy)?;
        reporter.log(
            "credentials",
            "ok",
            "本地登录凭据已读取并核验。",
            1,
            Some(1),
        );
        let account_key = if reuse {
            super::catalog::live_key(&self.home, &self.key)?
        } else {
            None
        };
        let previous = account_key
            .as_deref()
            .map(|key| self.db.by_account(key))
            .transpose()?
            .flatten();
        // 空资料的随机编号预留给首次保存的身份，避免登录后第一次暂存又改变种子。
        let draft = if reuse {
            self.db.browser_draft()?
        } else {
            None
        };
        let id = previous
            .as_ref()
            .map(|r| r.id.clone())
            .or(draft)
            .unwrap_or(super::new_id()?);
        let generation = super::new_id()?;
        let manifest = Manifest {
            version: 1,
            id: id.clone(),
            home: self.home.clone(),
            platform: std::env::consts::OS.into(),
            policy: policy.clone(),
            sources,
            credentials,
            fingerprints,
            account_key: account_key.clone(),
        };
        let pending = self.vault.join(format!(".pending-{generation}"));
        let destination = self.vault.join(format!("{generation}.baga"));
        let result = (|| {
            let output =
                codec::Seal::new(private::create(&pending)?, &self.key).map_err(problem)?;
            let buffered = BufWriter::with_capacity(64 * 1024, output);
            let mut tar = tar::Builder::new(compression::encode(buffered).map_err(problem)?);
            reporter.log(
                "archive",
                "running",
                "正在压缩暂存身份文件。",
                0,
                Some(manifest.sources.len()),
            );
            for (n, source) in manifest.sources.iter().enumerate() {
                self.stopped()?;
                let archive_path = PathBuf::from(format!("data/{n}"));
                if !source.keys.is_empty() {
                    let text = fs::read_to_string(&source.path).map_err(problem)?;
                    let bytes =
                        serde_json::to_vec(&identity::extract(source, &text)?).map_err(problem)?;
                    append(&mut tar, &archive_path, &bytes)?;
                } else if source.path.is_dir() {
                    let parent = source.path.parent().ok_or("身份路径无效。")?;
                    walk(&source.path, parent, &mut |path, relative, directory| {
                        self.during_io()?;
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
                reporter.log(
                    "archive",
                    "running",
                    "已写入身份归档文件。",
                    n + 1,
                    Some(manifest.sources.len()),
                );
            }
            append(
                &mut tar,
                Path::new("manifest.json"),
                &serde_json::to_vec(&manifest).map_err(problem)?,
            )?;
            reporter.log(
                "archive",
                "ok",
                "身份文件已写入压缩流。",
                manifest.sources.len(),
                Some(manifest.sources.len()),
            );
            reporter.log(
                "compress",
                "running",
                "正在完成压缩、认证加密与落盘。",
                0,
                None,
            );
            let encoder = tar.into_inner().map_err(problem)?;
            let buffered = encoder.finish().map_err(problem)?;
            let seal = buffered.into_inner().map_err(problem)?;
            let file = seal.finish().map_err(problem)?;
            file.sync_all().map_err(problem)?;
            drop(file);
            reporter.log("compress", "ok", "压缩归档已完整落盘。", 1, Some(1));
            reporter.log(
                "validate",
                "running",
                "正在复核身份源文件。",
                0,
                Some(manifest.sources.len()),
            );
            for (index, (source, stamp)) in manifest
                .sources
                .iter()
                .zip(&manifest.fingerprints)
                .enumerate()
            {
                if identity::fingerprint(&source.path)? != *stamp {
                    return Err("身份数据发生变化；未切换身份。".into());
                }
                reporter.log(
                    "validate",
                    "running",
                    "已复核身份源文件。",
                    index + 1,
                    Some(manifest.sources.len()),
                );
            }
            reporter.log(
                "validate",
                "ok",
                "身份源文件已完整复核。",
                manifest.sources.len(),
                Some(manifest.sources.len()),
            );
            self.stopped()?;
            fs::rename(&pending, &destination).map_err(problem)?;
            private::sync_parent(&destination);
            let record = Record {
                id,
                name: previous
                    .as_ref()
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| name.into()),
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
                account_key: Some(account_key.unwrap_or_default()),
            };
            reporter.log(
                "verify_archive",
                "running",
                "正在解包并验收认证归档。",
                0,
                None,
            );
            let stage = self.load(&record)?;
            reporter.log("verify_archive", "ok", "认证归档已完整验收。", 1, Some(1));
            Ok((record, stage))
        })();
        if result.is_err() {
            let _ = fs::remove_file(pending);
            let _ = fs::remove_file(destination);
        }
        result
    }
}
