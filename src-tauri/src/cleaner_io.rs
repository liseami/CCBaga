// [INPUT]: 待写入的清理后配置、目标路径与原权限。
// [OUTPUT]: 原子配置替换；失败移除临时文件，不留原配置备份。
// [POS]: 清理写入边界；临时文件仅保存清理后的内容。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
use std::io::Write;
pub(super) fn atomic_write(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let temporary = path.with_file_name(format!(".rf-cleanup-{}", generation()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(mode & 0o777)
            .open(&temporary)
            .map_err(|_| "无法创建配置临时文件。")?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "无法保存完整配置。")?;
        fs::rename(&temporary, path).map_err(|_| "无法替换配置。")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
use std::os::unix::fs::OpenOptionsExt;
