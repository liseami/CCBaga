// [INPUT]: 用户选择的 pem/ppk/pub 本机路径与系统 ssh-keygen；只接受完整同组金钥。
// [OUTPUT]: 文件类型、PPK 公钥及 PEM/PUB 配对核验，作用域内 0600 临时 PEM 私钥。
// [POS]: VPS 的单一密钥登录边界；私钥不进入 WebView，源文件不修改，临时副本随事务释放。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde::Deserialize;
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    process::{Command, Stdio},
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keys {
    pub pem: String,
    pub ppk: String,
    pub public_key: String,
}
pub struct PrivateKey(pub PathBuf);
impl Drop for PrivateKey {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn read(path: &str, extension: &str) -> Result<String, String> {
    let path = PathBuf::from(path);
    if !path.is_absolute() || path.extension().and_then(|s| s.to_str()) != Some(extension) {
        return Err("请提供 .pem、.ppk、.pub 三个密钥文件。".into());
    }
    if !fs::metadata(&path)
        .map_err(|_| "无法读取密钥文件，请重新选择。")?
        .is_file()
    {
        return Err("密钥文件为空、过大或不是普通文件。".into());
    }
    let file = fs::File::open(path).map_err(|_| "无法读取密钥文件，请重新选择。")?;
    let metadata = file
        .metadata()
        .map_err(|_| "无法读取密钥文件，请重新选择。")?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 65536 {
        return Err("密钥文件为空、过大或不是普通文件。".into());
    }
    let mut text = String::new();
    file.take(65537)
        .read_to_string(&mut text)
        .map_err(|_| "密钥文件格式无效。")?;
    if text.len() > 65536 {
        return Err("密钥文件为空、过大或不是普通文件。".into());
    }
    Ok(text)
}
fn public_parts(text: &str) -> Option<(&str, &str)> {
    let mut parts = text.split_whitespace();
    Some((parts.next()?, parts.next()?))
}
impl Keys {
    pub fn prepare(&self) -> Result<PrivateKey, String> {
        let pem = read(&self.pem, "pem")?;
        let ppk = read(&self.ppk, "ppk")?;
        let public = read(&self.public_key, "pub")?;
        let expected = public_parts(&public).ok_or("密钥文件格式无效。")?;
        // ----- PPK 与 PUB 必须携带同一公钥；不转换或使用 PPK 私钥 -----
        let mut lines = ppk.lines();
        let header = lines.next().ok_or("密钥文件格式无效。")?;
        let algorithm = header
            .strip_prefix("PuTTY-User-Key-File-2: ")
            .or_else(|| header.strip_prefix("PuTTY-User-Key-File-3: "))
            .ok_or("密钥文件格式无效。")?;
        let count = lines
            .by_ref()
            .find_map(|line| line.strip_prefix("Public-Lines: "))
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0 && *n <= 1000)
            .ok_or("密钥文件格式无效。")?;
        let blob: String = lines.take(count).collect();
        if algorithm != expected.0 || blob != expected.1 {
            return Err("三个密钥文件不属于同一组，请重新下载完整密钥包。".into());
        }
        // ----- 限制副本权限，不更改 Downloads 中的原始文件 -----
        let path =
            std::env::temp_dir().join(format!("cc-ssh-key-{}", crate::browser::server::nonce()?));
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .map_err(|_| "无法准备 SSH 私钥。")?;
        let key = PrivateKey(path);
        file.write_all(pem.as_bytes())
            .map_err(|_| "无法准备 SSH 私钥。")?;
        drop(file);
        let derived = Command::new("/usr/bin/ssh-keygen")
            .args(["-y", "-P", "", "-f"])
            .arg(&key.0)
            .stdin(Stdio::null())
            .output()
            .map_err(|_| "无法核验 SSH 私钥。")?;
        if !derived.status.success() {
            return Err("PEM 私钥无效或带有口令，请使用 DMIT 下载的无口令密钥包。".into());
        }
        let derived = String::from_utf8_lossy(&derived.stdout);
        if public_parts(&derived) != Some(expected) {
            return Err("三个密钥文件不属于同一组，请重新下载完整密钥包。".into());
        }
        Ok(key)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn complete_bundle_is_paired_and_temporary_private_key_is_removed() {
        let dir = std::env::temp_dir().join(format!(
            "cc-keys-test-{}",
            crate::browser::server::nonce().unwrap()
        ));
        fs::create_dir(&dir).unwrap();
        let pem = dir.join("id_rsa.pem");
        assert!(Command::new("/usr/bin/ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&pem)
            .status()
            .unwrap()
            .success());
        let public_key = dir.join("id_rsa.pub");
        fs::rename(dir.join("id_rsa.pem.pub"), &public_key).unwrap();
        let public = fs::read_to_string(&public_key).unwrap();
        let (algorithm, blob) = public_parts(&public).unwrap();
        let ppk = dir.join("id_rsa.ppk");
        fs::write(&ppk, format!("PuTTY-User-Key-File-3: {algorithm}\nEncryption: none\nComment: fixture\nPublic-Lines: 1\n{blob}\nPrivate-Lines: 1\nfixture\n")).unwrap();
        let mut keys = Keys {
            pem: pem.to_string_lossy().into(),
            ppk: ppk.to_string_lossy().into(),
            public_key: public_key.to_string_lossy().into(),
        };
        fs::set_permissions(&pem, fs::Permissions::from_mode(0o644)).unwrap();
        let guard = keys.prepare().unwrap();
        let temporary = guard.0.clone();
        assert_eq!(
            fs::metadata(&temporary).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&pem).unwrap().permissions().mode() & 0o777,
            0o644
        );
        assert_eq!(fs::read(&temporary).unwrap(), fs::read(&pem).unwrap());
        drop(guard);
        assert!(!temporary.exists());
        keys.ppk = String::new();
        assert!(keys.prepare().is_err());
        keys.ppk = ppk.to_string_lossy().into();
        fs::write(
            &ppk,
            "PuTTY-User-Key-File-3: ssh-ed25519\nPublic-Lines: 1\nwrong\n",
        )
        .unwrap();
        assert!(keys.prepare().is_err());
        fs::write(
            &ppk,
            format!("PuTTY-User-Key-File-3: {algorithm}\nPublic-Lines: 1\n{blob}\n"),
        )
        .unwrap();
        // ----- PPK/PUB 相符而 PEM 来自另一组，也必须拒绝 -----
        let other = dir.join("other.pem");
        assert!(Command::new("/usr/bin/ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&other)
            .status()
            .unwrap()
            .success());
        keys.pem = other.to_string_lossy().into();
        assert!(keys.prepare().is_err());
        keys.pem = pem.to_string_lossy().into();
        fs::write(&pem, "not a private key").unwrap();
        assert!(keys.prepare().is_err());
        fs::write(&pem, vec![b'x'; 65537]).unwrap();
        assert!(keys.prepare().is_err());
        assert!(read(dir.to_str().unwrap(), "pem").is_err());
        assert!(serde_json::from_str::<Keys>(
            r#"{"pem":"x","ppk":"y","public_key":"z","password":"secret"}"#
        )
        .is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
