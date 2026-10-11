// [INPUT]: 有效全机/目录清理快照、原子配置写入与原生凭据删除，以及进度通道。
// [OUTPUT]: 直接删除结果和真实逐项日志；不生成备份、废纸篓副本或撤回记录。
// [POS]: 清理事务的唯一写入边界；校验后执行，失败停止并报告剩余项未执行。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
pub(super) fn perform_clean(
    app: &tauri::AppHandle,
    ids: Vec<String>,
    reporter: &Reporter,
) -> Result<CleanResult, String> {
    reporter.log("validate", "running", "正在核验进程与所选快照。", 0, None);
    if running()? {
        return Err("请先退出 Claude Desktop 和 Claude Code，再清理。".into());
    }
    let state = app.state::<Cleaner>();
    let plan = state.plan.lock().unwrap().clone().ok_or("请先深度扫描。")?;
    let items = selected(&plan, &ids)?;
    let root = if plan.folder.is_empty() {
        home()?
    } else {
        folder::validate(Path::new(&plan.folder))?
    };
    perform_items(&root, &items, &state, running, reporter)
}
pub(super) fn perform_items(
    root: &Path,
    items: &[Item],
    state: &Cleaner,
    check_running: impl Fn() -> Result<bool, String>,
    reporter: &Reporter,
) -> Result<CleanResult, String> {
    for item in items {
        identity(item, root)?;
    }
    reporter.log(
        "validate",
        "ok",
        "快照核验通过，直接删除所选数据。",
        items.len(),
        Some(items.len()),
    );
    let mut cleaned = 0;
    let mut errors = Vec::new();
    for item in items {
        reporter.log(
            "clean",
            "running",
            &format!("正在清理：{}", item.title),
            cleaned,
            Some(items.len()),
        );
        let action = (|| -> Result<(), String> {
            if check_running()? {
                return Err("Claude 重新启动，已停止余下清理。".into());
            }
            identity(item, root)?;
            let path = Path::new(&item.path);
            match &item.operation {
                Operation::Delete => {
                    // 标准库目录删除不跟随符号链接；不经过废纸篓。
                    let result = if item.directory {
                        fs::remove_dir_all(path)
                    } else {
                        fs::remove_file(path)
                    };
                    result.map_err(|_| "直接删除失败；此项目可能部分删除，请重新扫描。".to_string())
                }
                Operation::Keychain(reference) => native::remove_credential(reference),
                Operation::Edit(kind) => {
                    let meta = fs::metadata(path).map_err(|_| "无法读取配置属性。")?;
                    let text = fs::read_to_string(path).map_err(|_| "无法读取配置。")?;
                    let (updated, names) = config::transform(&text, *kind)?;
                    if names != item.keys {
                        return Err("配置键发生变化，请重新扫描。".into());
                    }
                    identity(item, root)?;
                    file_io::atomic_write(path, updated.as_bytes(), meta.permissions().mode())
                }
            }
        })();
        match action {
            Ok(()) => {
                cleaned += 1;
                reporter.log(
                    "clean",
                    "ok",
                    &format!("已清理：{}", item.title),
                    cleaned,
                    Some(items.len()),
                );
            }
            Err(error) => {
                let message = format!("{}：{error} 余下项目未执行。", item.title);
                reporter.log("clean", "error", &message, cleaned, Some(items.len()));
                errors.push(message);
                break;
            }
        }
    }
    *state.plan.lock().unwrap() = None;
    reporter.log(
        "clean",
        if errors.is_empty() { "ok" } else { "error" },
        &format!(
            "已清理 {cleaned}/{} 项；{}。",
            items.len(),
            if errors.is_empty() {
                "准备复查"
            } else {
                "后续清理已停止"
            }
        ),
        cleaned,
        Some(items.len()),
    );
    Ok(CleanResult {
        cleaned,
        completed: errors.is_empty(),
        errors,
    })
}
