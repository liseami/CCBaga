// [INPUT]: 用户明确选择的目录、既有 Claude 白名单、快照统计与操作日志。
// [OUTPUT]: 目录内 Claude 会话、缓存与既有备份快照；不读取或改写权限和凭据。
// [POS]: 文件夹扫描边界；复用清理事务，不扩展到目录外，不跟随链接或进入依赖/源码工作树。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
use std::time::{Duration, Instant};

pub(super) fn validate(path: &Path) -> Result<PathBuf, String> {
    let meta =
        fs::symlink_metadata(path).map_err(|_| "无法读取文件夹；请拖入一个可访问的目录。")?;
    if !path.is_absolute()
        || !meta.is_dir()
        || meta.file_type().is_symlink()
        || meta.uid() != unsafe { getuid() }
        || fs::canonicalize(path).ok().as_deref() != Some(path)
    {
        return Err("仅支持当前用户的真实文件夹；不接受文件、链接或其他用户目录。".into());
    }
    Ok(path.to_path_buf())
}
pub(super) fn collect(root: &Path, reporter: &Reporter) -> Result<Scan, String> {
    let root = validate(root)?;
    let mut scan = Scan {
        folder: root.to_string_lossy().into_owned(),
        ..Scan::default()
    };
    let batch = generation();
    // 直接拖入已知数据目录的子目录时，仍沿用相同白名单，不扩大到其父目录。
    for ancestor in root
        .ancestors()
        .filter(|p| p.file_name().and_then(|n| n.to_str()) == Some(".claude"))
    {
        if let Some(parent) = ancestor.parent() {
            if let Some(mut target) = targets::definitions(parent).into_iter().find(|t| {
                root.starts_with(&t.path)
                    && matches!(t.operation, Operation::Delete)
                    && t.path.file_name().and_then(|s| s.to_str()) != Some(".credentials.json")
            }) {
                target.path = root.clone();
                add_target(&mut scan, target, batch);
                scan.message = format!(
                    "文件夹扫描完成：发现 {} 个 Claude 清理项目。",
                    scan.items.len()
                );
                return Ok(scan);
            }
        }
    }
    let mut pending = vec![(root.clone(), 0)];
    let deadline = Instant::now() + Duration::from_secs(5);
    let (mut seen, mut entries_seen) = (0, 0);
    while let Some((dir, depth)) = pending.pop() {
        seen += 1;
        if seen > 100_000 || Instant::now() > deadline {
            scan.warnings
                .push("文件夹扫描达到条目或时间上限，仅列出已发现的项目。".into());
            break;
        }
        if seen % 500 == 0 {
            reporter.log(
                "scan",
                "running",
                &format!("已检查 {seen} 个目录。"),
                seen,
                None,
            );
        }
        if dir.file_name().and_then(|n| n.to_str()) == Some(".claude") {
            if let Some(parent) = dir.parent() {
                for target in targets::definitions(parent).into_iter().filter(|t| {
                    t.path.starts_with(&dir)
                        && matches!(t.operation, Operation::Delete)
                        && t.path.file_name().and_then(|s| s.to_str()) != Some(".credentials.json")
                }) {
                    add_target(&mut scan, target, batch);
                }
            }
            continue;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => {
                scan.warnings.push(format!("无法检查 {}。", dir.display()));
                continue;
            }
        };
        for entry in entries {
            entries_seen += 1;
            if entries_seen > 100_000 || Instant::now() > deadline {
                scan.warnings
                    .push("文件夹扫描达到条目或时间上限，仅列出已发现的项目。".into());
                pending.clear();
                break;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(_) => {
                    scan.warnings.push("有目录条目无法读取。".into());
                    continue;
                }
            };
            let path = entry.path();
            let meta = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.file_type().is_symlink() || meta.uid() != unsafe { getuid() } {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if (meta.is_file()
                && (name == ".claude.json.backup" || name.starts_with(".claude.json.backup.")))
                || (meta.is_dir() && name == "ClaudeBackups")
            {
                add_target(
                    &mut scan,
                    targets::Target {
                        path,
                        title: "文件夹内 Claude 既有备份".into(),
                        group: "backups",
                        risk: "backup",
                        impact: "直接删除已存在的 Claude 备份，不保留恢复副本。".into(),
                        operation: Operation::Delete,
                    },
                    batch,
                );
            } else if meta.is_dir()
                && !matches!(
                    name.as_str(),
                    ".git"
                        | "node_modules"
                        | "target"
                        | "dist"
                        | "build"
                        | ".gradle"
                        | ".next"
                        | ".venv"
                        | "vendor"
                        | "reference"
                        | "browser-runtime"
                )
            {
                if depth >= 64 {
                    scan.warnings
                        .push(format!("目录层级过深，已跳过 {}。", path.display()));
                } else if pending.len() >= 100_000 {
                    scan.warnings
                        .push("目录数量达到上限，未展开全部子目录。".into());
                } else {
                    pending.push((path, depth + 1));
                }
            }
        }
    }
    scan.message = format!(
        "文件夹扫描完成：发现 {} 个 Claude 清理项目。",
        scan.items.len()
    );
    Ok(scan)
}
