// [INPUT]: Claude 白名单、所选文件夹发现、配置变换、原子写入、官方原生 CLI 版本路径及 macOS 原生桥。
// [OUTPUT]: 全机/文件夹扫描、旧组织发现、直接清理、实时 Claude 进程检查与结束全部已识别进程；CLI 安装管理共用互斥锁与进程核验。
// [POS]: 本地清理事务编排；界面只提交快照编号，凭据正文不离开原生层。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use crate::native;
use crate::progress::{Progress, Reporter};
use serde::Serialize;
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{ipc::Channel, Manager};
#[path = "cleaner_config.rs"]
mod config;
#[path = "cleaner_io.rs"]
mod file_io;
#[path = "cleaner_folder.rs"]
mod folder;
#[path = "cleaner_targets.rs"]
mod targets;
#[path = "cleaner_transaction.rs"]
mod transaction;
#[cfg(test)]
use transaction::perform_items;
#[cfg(test)]
#[path = "cleaner_tests.rs"]
mod tests;
extern "C" {
    fn getuid() -> u32;
    fn kill(pid: i32, signal: i32) -> i32;
}
#[derive(Clone)]
enum Operation {
    Delete,
    Edit(config::EditKind),
    Keychain(String),
}
#[derive(Clone, Serialize)]
pub struct Item {
    id: String,
    path: String,
    title: String,
    group: &'static str,
    risk: &'static str,
    impact: String,
    bytes: u64,
    files: usize,
    exact: bool,
    keys: Vec<String>,
    #[serde(skip)]
    operation: Operation,
    #[serde(skip)]
    directory: bool,
    #[serde(skip)]
    stamp: Option<Stamp>,
    #[serde(skip)]
    tree_stamp: Vec<u8>,
}
#[derive(Clone, PartialEq)]
struct Stamp {
    dev: u64,
    inode: u64,
    modified: i64,
    nanos: i64,
    changed: i64,
    changed_nanos: i64,
    size: u64,
}
fn stamp(m: &fs::Metadata) -> Stamp {
    Stamp {
        dev: m.dev(),
        inode: m.ino(),
        modified: m.mtime(),
        nanos: m.mtime_nsec(),
        changed: m.ctime(),
        changed_nanos: m.ctime_nsec(),
        size: m.len(),
    }
}
fn generation() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
#[derive(Clone, Serialize)]
struct Diagnostic {
    title: String,
    detail: String,
    status: &'static str,
}
#[derive(Clone, Serialize, Default)]
pub struct Scan {
    folder: String,
    items: Vec<Item>,
    running: bool,
    processes: Vec<ClaudeProcess>,
    message: String,
    warnings: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    org_id: String,
}
#[derive(Default)]
pub struct Cleaner {
    plan: Mutex<Option<Scan>>,
    active: AtomicBool,
}
impl Cleaner {
    pub(crate) fn enter(&self) -> Result<(), String> {
        if self.active.swap(true, Ordering::SeqCst) {
            Err("已有清理操作正在运行。".into())
        } else {
            Ok(())
        }
    }
    pub(crate) fn leave(&self) {
        self.active.store(false, Ordering::SeqCst);
    }
}
fn home() -> Result<PathBuf, String> {
    fs::canonicalize(PathBuf::from(
        std::env::var_os("HOME").ok_or("无法定位用户目录。")?,
    ))
    .map_err(|_| "无法读取用户目录。".into())
}
fn add_target(scan: &mut Scan, target: targets::Target, batch: u128) {
    let path = target.path;
    let meta = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                scan.warnings.push(format!("无法检查 {}。", path.display()));
            }
            return;
        }
    };
    if meta.file_type().is_symlink()
        || meta.uid() != unsafe { getuid() }
        || (!meta.is_dir() && !meta.is_file())
        || fs::canonicalize(&path).ok().as_deref() != Some(path.as_path())
    {
        scan.warnings.push(format!(
            "已跳过链接、其他用户或特殊路径：{}",
            path.display()
        ));
        return;
    }
    let mut keys = Vec::new();
    if let Operation::Edit(kind) = target.operation {
        if !meta.is_file() || meta.len() > 4 * 1024 * 1024 {
            scan.warnings
                .push(format!("配置过大或类型不正确：{}", path.display()));
            return;
        }
        if let Ok(text) = fs::read_to_string(&path) {
            if config::needs_manual_review(&text, kind) {
                scan.warnings.push(format!(
                    "{}：包含复合赋值或外部配置引用，未自动展开或改写这些内容。",
                    path.display()
                ));
            }
        }
        match fs::read_to_string(&path)
            .map_err(|_| "无法读取配置。".to_string())
            .and_then(|s| config::transform(&s, kind))
        {
            Ok((_, names)) => {
                if names.is_empty() {
                    return;
                }
                keys = names;
            }
            Err(e) => {
                scan.warnings.push(format!("{}：{e}", path.display()));
                return;
            }
        }
    }
    let (bytes, files, exact, tree_stamp) = targets::measure(&path);
    scan.items.push(Item {
        id: format!("{batch}-{}", scan.items.len()),
        path: path.to_string_lossy().into_owned(),
        title: target.title,
        group: target.group,
        risk: target.risk,
        impact: target.impact,
        bytes,
        files,
        exact,
        keys,
        operation: target.operation,
        directory: meta.is_dir(),
        stamp: Some(stamp(&meta)),
        tree_stamp,
    });
}
fn collect(root: &Path, org: Option<&str>) -> Result<Scan, String> {
    let batch = generation();
    let mut scan = Scan::default();
    for target in targets::definitions(root) {
        add_target(&mut scan, target, batch);
    }
    if let Some(org) = org.filter(|s| !s.is_empty()) {
        let (matches, complete) = targets::org_files(root, org)?;
        scan.org_id = org.into();
        for path in matches {
            if scan.items.iter().any(|i| i.path == path.to_string_lossy()) {
                continue;
            }
            add_target(
                &mut scan,
                targets::Target {
                    title: "旧组织会话文件".into(),
                    path,
                    group: "organization",
                    risk: "history",
                    impact: "文件正文匹配所填组织编号；请核对路径，只清理确认不再需要的旧会话。"
                        .into(),
                    operation: Operation::Delete,
                },
                batch,
            );
        }
        if !complete {
            scan.warnings
                .push("组织检索达到时间或条目上限；未扫描全部文件。".into());
        }
    }
    scan.message = format!("已发现 {} 个可选项目。", scan.items.len());
    Ok(scan)
}
fn diagnostics(scan: &mut Scan) {
    let names: Vec<_> = std::env::vars_os()
        .filter_map(|(key, _)| key.into_string().ok())
        .filter(|key| {
            matches!(
                key.to_ascii_uppercase().as_str(),
                "HTTP_PROXY"
                    | "HTTPS_PROXY"
                    | "ALL_PROXY"
                    | "NO_PROXY"
                    | "CLAUDE_CODE_OAUTH_TOKEN"
                    | "CLAUDE_CONFIG_DIR"
            ) || key.starts_with("ANTHROPIC_")
        })
        .collect();
    scan.diagnostics.push(Diagnostic {
        title: "当前应用环境".into(),
        detail: if names.is_empty() {
            "未发现代理或认证变量。终端已启动的会话需单独检查。".into()
        } else {
            format!(
                "发现变量：{}。清理启动文件后，请重开终端。",
                names.join("、")
            )
        },
        status: if names.is_empty() { "ok" } else { "review" },
    });
    if std::env::var_os("CLAUDE_CONFIG_DIR").is_some() {
        scan.warnings.push(
            "检测到自定义 CLAUDE_CONFIG_DIR；本次仅检查默认目录，自定义目录未纳入清理。".into(),
        );
    }
    let proxy = std::process::Command::new("/usr/sbin/scutil")
        .arg("--proxy")
        .output();
    let enabled = proxy
        .as_ref()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.contains("Enable : 1"))
        })
        .unwrap_or(false);
    scan.diagnostics.push(Diagnostic {
        title: "系统代理".into(),
        detail: if enabled {
            "系统代理已开启。此处仅检查，不改动现有 Clash 路由。".into()
        } else {
            "未发现已开启的系统代理；仍需在客户端验证实际出口和断线保护。".into()
        },
        status: if enabled { "review" } else { "info" },
    });
    let timezone = fs::read_link("/etc/localtime")
        .ok()
        .map(|p| {
            p.to_string_lossy()
                .split("/zoneinfo/")
                .last()
                .unwrap_or("未知")
                .to_string()
        })
        .unwrap_or("未知".into());
    scan.diagnostics.push(Diagnostic {
        title: "设备时区".into(),
        detail: format!("{timezone} · 可在电脑设置中修改。"),
        status: "info",
    });
    scan.diagnostics.push(Diagnostic {
        title: "保留个人资产".into(),
        detail: "skills、plugins、hooks、commands、CLAUDE.md、MCP、权限、模型及遥测设置保持不变。"
            .into(),
        status: "ok",
    });
}
#[derive(Clone, Serialize)]
pub struct ClaudeProcess {
    pid: i32,
    name: String,
    #[serde(skip)]
    executable: String,
}
fn process_row(line: &str, desktop: &[i32], uid: u32) -> Option<ClaudeProcess> {
    let mut fields = line.trim().splitn(2, char::is_whitespace);
    if fields.next()?.parse::<u32>().ok()? != uid {
        return None;
    }
    let mut fields = fields.next()?.trim_start().splitn(2, char::is_whitespace);
    let pid = fields.next()?.parse::<i32>().ok()?;
    if pid <= 1 {
        return None;
    }
    let executable = fields.next()?.trim().to_owned();
    let name = Path::new(&executable).file_name()?.to_str()?;
    let desktop_app = desktop.contains(&pid);
    let native_cli = name.split('.').count() == 3
        && name
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        && home().ok().is_some_and(|root| {
            Path::new(&executable).parent()
                == Some(root.join(".local/share/claude/versions").as_path())
        });
    if !desktop_app
        && !native_cli
        && !matches!(
            name,
            "claude" | "claude.exe" | "claude-code" | "claude-code-vm"
        )
    {
        return None;
    }
    Some(ClaudeProcess {
        pid,
        name: if native_cli {
            "Claude Code".into()
        } else if desktop_app {
            if matches!(name, "node" | "nodejs" | "bun") {
                "Claude Code".into()
            } else if name == "Claude" {
                "Claude Desktop".into()
            } else {
                name.into()
            }
        } else {
            name.into()
        },
        executable,
    })
}
fn processes() -> Result<Vec<ClaudeProcess>, String> {
    let desktop = native::claude_pids()?;
    let output = std::process::Command::new("/bin/ps")
        .args(["-axo", "uid=,pid=,comm="])
        .output()
        .map_err(|_| "无法检查 Claude 进程，请刷新重试。")?;
    if !output.status.success() {
        return Err("无法检查 Claude 进程，请刷新重试。".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| process_row(line, &desktop, unsafe { getuid() }))
        .collect())
}
fn running() -> Result<bool, String> {
    Ok(!processes()?.is_empty())
}
pub(crate) fn cli_running() -> Result<bool, String> {
    Ok(processes()?.iter().any(|process| {
        matches!(
            process.name.as_str(),
            "Claude Code" | "claude" | "claude.exe" | "claude-code" | "claude-code-vm"
        )
    }))
}
#[tauri::command]
pub async fn cleaner_processes() -> Result<Vec<ClaudeProcess>, String> {
    tauri::async_runtime::spawn_blocking(processes)
        .await
        .map_err(|_| "进程检查被中断。".to_string())?
}
fn signal_same_process(target: &ClaudeProcess, signal: i32) -> Result<(), String> {
    if processes()?
        .iter()
        .any(|p| p.pid == target.pid && p.executable == target.executable)
    {
        if unsafe { kill(target.pid, signal) } != 0
            && processes()?
                .iter()
                .any(|p| p.pid == target.pid && p.executable == target.executable)
        {
            return Err(format!("无法结束 Claude 进程 {}。", target.pid));
        }
    }
    Ok(())
}

fn current_scan(root: &Path, org: Option<&str>) -> Result<Scan, String> {
    let mut scan = collect(root, org)?;
    match native::credentials() {
        Ok(credentials) => {
            let batch = generation();
            for credential in credentials {
                scan.items.push(Item {
                    id: format!("keychain-{batch}-{}", scan.items.len()),
                    path: format!("macOS 钥匙串 / {}", credential.service),
                    title: format!("登录凭据 · {}", credential.service),
                    group: "account",
                    risk: "login",
                    impact: "直接删除后需重新登录；不读取或保留凭据正文。".into(),
                    bytes: 0,
                    files: 0,
                    exact: true,
                    keys: vec![],
                    operation: Operation::Keychain(credential.reference),
                    directory: false,
                    stamp: None,
                    tree_stamp: vec![],
                });
            }
        }
        Err(e) => scan.warnings.push(e),
    }
    scan.processes = processes()?;
    scan.running = !scan.processes.is_empty();
    diagnostics(&mut scan);
    scan.message = format!("已发现 {} 个可选项目。", scan.items.len());
    Ok(scan)
}
fn scan_scope(
    folder: Option<&str>,
    org: Option<&str>,
    reporter: &Reporter,
) -> Result<Scan, String> {
    if let Some(path) = folder.filter(|p| !p.is_empty()) {
        let mut scan = folder::collect(Path::new(path), reporter)?;
        scan.processes = processes()?;
        scan.running = !scan.processes.is_empty();
        scan.diagnostics.push(Diagnostic { title: "文件夹范围".into(), detail: "仅检查所选文件夹内的 Claude 会话、缓存与既有备份。项目文档、设置、凭据、技能和插件保留；不检查目录外钥匙串或全局会话。".into(), status: "info" });
        Ok(scan)
    } else {
        current_scan(&home()?, org)
    }
}
#[tauri::command]
pub async fn cleaner_scan(
    app: tauri::AppHandle,
    org_id: Option<String>,
    folder: Option<String>,
    on_progress: Channel<Progress>,
) -> Result<Scan, String> {
    let state = app.state::<Cleaner>();
    state.enter()?;
    let worker = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let reporter = Reporter::channel(Some(on_progress));
        reporter.log(
            "scan",
            "running",
            "正在发现文件、检查配置与钥匙串；凭据内容不进入日志。",
            0,
            None,
        );
        let scan = match scan_scope(
            folder.as_deref(),
            org_id.as_deref().map(str::trim),
            &reporter,
        ) {
            Ok(scan) => scan,
            Err(error) => {
                reporter.log("scan", "error", &error, 0, None);
                return Err(error);
            }
        };
        reporter.log(
            "scan",
            "ok",
            &scan.message,
            scan.items.len(),
            Some(scan.items.len()),
        );
        for warning in &scan.warnings {
            reporter.log("scan", "warning", warning, 0, None);
        }
        let state = worker.state::<Cleaner>();
        *state.plan.lock().unwrap() = Some(scan.clone());
        Ok(scan)
    })
    .await
    .map_err(|_| "深度扫描被中断，请重试。".to_string())
    .and_then(|r| r);
    state.active.store(false, Ordering::SeqCst);
    result
}
#[tauri::command]
pub async fn cleaner_stop(app: tauri::AppHandle) -> Result<Scan, String> {
    let state = app.state::<Cleaner>();
    state.enter()?;
    let worker = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut targets = std::collections::HashSet::new();
        // 主进程退出期间辅助进程可能重生；每轮重新识别，最多三轮，不扩大到其他 Node/MCP/终端。
        for _ in 0..3 {
            let current = processes()?;
            if current.is_empty() {
                break;
            }
            for target in &current {
                targets.insert(target.pid);
                signal_same_process(target, 15)?;
            }
            std::thread::sleep(std::time::Duration::from_millis(800));
            for target in processes()? {
                targets.insert(target.pid);
                signal_same_process(&target, 9)?;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        let plan = worker.state::<Cleaner>().plan.lock().unwrap().clone();
        let mut scan = scan_scope(
            plan.as_ref().map(|p| p.folder.as_str()),
            plan.as_ref().map(|p| p.org_id.as_str()),
            &Reporter::channel(None),
        )?;
        scan.message = if scan.running {
            "仍有 Claude 进程在运行。".into()
        } else {
            format!("已结束 {} 个 Claude 进程，可以清理。", targets.len())
        };
        *worker.state::<Cleaner>().plan.lock().unwrap() = Some(scan.clone());
        Ok(scan)
    })
    .await
    .map_err(|_| "结束进程被中断，请刷新。".to_string())
    .and_then(|r| r);
    state.active.store(false, Ordering::SeqCst);
    result
}
fn selected(plan: &Scan, ids: &[String]) -> Result<Vec<Item>, String> {
    if ids.is_empty() {
        return Err("请选择清理项目。".into());
    }
    let mut chosen: Vec<Item> = Vec::new();
    for id in ids {
        let item = plan
            .items
            .iter()
            .find(|i| &i.id == id)
            .ok_or("清理快照已过期，请重新扫描。")?;
        if !chosen.iter().any(|i| i.id == *id) {
            chosen.push(item.clone());
        }
    }
    Ok(chosen
        .iter()
        .filter(|item| {
            !chosen.iter().any(|parent| {
                parent.id != item.id
                    && matches!(parent.operation, Operation::Delete)
                    && parent.directory
                    && !matches!(item.operation, Operation::Keychain(_))
                    && Path::new(&item.path).starts_with(&parent.path)
            })
        })
        .cloned()
        .collect())
}
fn identity(item: &Item, root: &Path) -> Result<(), String> {
    if let Operation::Keychain(reference) = &item.operation {
        return if native::credentials()?
            .iter()
            .any(|c| &c.reference == reference)
        {
            Ok(())
        } else {
            Err("登录凭据已变化，请重新扫描。".into())
        };
    }
    let path = Path::new(&item.path);
    if !path.starts_with(root) || fs::canonicalize(path).ok().as_deref() != Some(path) {
        return Err("路径已变化或超出当前用户范围。".into());
    }
    let m = fs::symlink_metadata(path).map_err(|_| "项目已变化，请重新扫描。")?;
    if m.file_type().is_symlink()
        || m.uid() != unsafe { getuid() }
        || m.is_dir() != item.directory
        || Some(stamp(&m)) != item.stamp
    {
        return Err("数据已变化，请退出 Claude 后重新扫描。".into());
    }
    if item.directory {
        let (_, _, complete, tree) = targets::measure(path);
        if !complete || !item.exact {
            return Err("目录统计不完整，拒绝直接清理；请减少目录规模后重新扫描。".into());
        }
        if tree != item.tree_stamp {
            return Err("目录内的文件已经变化，请重新扫描。".into());
        }
    }

    Ok(())
}
#[derive(Serialize)]
pub struct CleanResult {
    cleaned: usize,
    errors: Vec<String>,
    completed: bool,
}
#[tauri::command]
pub async fn cleaner_clean(
    app: tauri::AppHandle,
    ids: Vec<String>,
    on_progress: Channel<Progress>,
) -> Result<CleanResult, String> {
    let state = app.state::<Cleaner>();
    state.enter()?;
    let worker = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let reporter = Reporter::channel(Some(on_progress));
        let result = transaction::perform_clean(&worker, ids, &reporter);
        if let Err(error) = &result {
            reporter.fail(error);
        }
        result
    })
    .await
    .map_err(|_| "清理被中断；已删除的数据不可撤回，请重新扫描剩余项目。".to_string())
    .and_then(|r| r);
    state.active.store(false, Ordering::SeqCst);
    result
}
