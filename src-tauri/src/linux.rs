// [INPUT]: 当前用户 procfs、zoneinfo、桌面偏好和本地会话工具。
// [OUTPUT]: Linux 原生桥；已支持 Desktop 精确进程入口、独立浏览器归属与区域检查，未知不冒充成功。
// [POS]: 仅原生识别与用户触发的系统操作；不含网络客户端，不向 UI 输出进程参数或代理秘密。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde_json::{json, Value};
use std::{
    ffi::{c_char, CStr, CString},
    fs,
    io::Write,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn response(value: Result<Value, String>) -> *mut c_char {
    let value = value.unwrap_or_else(|error| json!({"error": error}));
    CString::new(value.to_string())
        .expect("JSON escapes NUL")
        .into_raw()
}
unsafe fn input<'a>(value: *const c_char) -> &'a str {
    CStr::from_ptr(value).to_str().unwrap_or("")
}
pub unsafe fn rf_free(value: *mut c_char) {
    drop(CString::from_raw(value));
}
fn output(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|_| format!("Linux: {program} is unavailable."))?;
    if !out.status.success() {
        return Err(format!(
            "Linux: {program} failed; check desktop permissions and service availability."
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}
fn exists(program: &str) -> bool {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .any(|dir| dir.join(program).is_file())
}
fn spawn(program: &str, args: &[&str]) -> Result<Value, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| format!("Linux: {program} is unavailable."))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(json!({"ok": true}))
}
pub fn open_settings(pane: &str) -> Result<Value, String> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();
    if desktop.contains("gnome") {
        let panel = match pane {
            "timezone" => "datetime",
            "language" => "region",
            "location" => "location",
            _ => return Err("Invalid settings panel.".into()),
        };
        spawn("gnome-control-center", &[panel])
    } else if desktop.contains("kde") {
        let panel =
            match pane {
                "timezone" => "kcm_clock",
                "language" => "kcm_regionandlang",
                "location" => return Err(
                    "Manage location permissions in KDE System Settings and the desktop portal."
                        .into(),
                ),
                _ => return Err("Invalid settings panel.".into()),
            };
        spawn("systemsettings", &[panel])
    } else {
        Err("Open your desktop settings to manage time zone, language or location; this desktop has no supported settings launcher.".into())
    }
}

pub unsafe fn rf_copy(text: *const c_char) -> Result<(), String> {
    let (program, args): (&str, &[&str]) = if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        ("wl-copy", &[])
    } else {
        ("xclip", &["-selection", "clipboard"])
    };
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| format!("Install {program} to use the clipboard."))?;
    child
        .stdin
        .take()
        .ok_or("Clipboard input unavailable.")?
        .write_all(input(text).as_bytes())
        .map_err(|_| "Clipboard write failed.")?;
    if !child
        .wait()
        .map_err(|_| "Clipboard write failed.")?
        .success()
    {
        return Err("Clipboard write failed.".into());
    }
    Ok(())
}
pub unsafe fn rf_credentials_list() -> *mut c_char {
    response(Err("Linux keyring cleanup is unavailable; file credentials are scanned, but Secret Service/KWallet entries must be managed separately.".into()))
}
pub unsafe fn rf_credentials_remove(_: *const c_char) -> *mut c_char {
    response(Err("Linux keyring cleanup is unavailable.".into()))
}
pub unsafe fn rf_keychain_unlock() -> *mut c_char {
    response(Err(
        "Unlock the session keyring using your desktop's keyring manager.".into(),
    ))
}

#[derive(Debug)]
pub struct Process {
    pub pid: i32,
    pub executable: PathBuf,
    pub args: Vec<String>,
}
extern "C" {
    fn getuid() -> u32;
    fn kill(pid: i32, signal: i32) -> i32;
}
pub fn processes() -> Result<Vec<Process>, String> {
    let mut result = Vec::new();
    for entry in fs::read_dir("/proc")
        .map_err(|_| "Cannot read Linux process list.")?
        .flatten()
    {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<i32>() else {
            continue;
        };
        if pid <= 1
            || entry
                .metadata()
                .map(|m| m.uid() != unsafe { getuid() })
                .unwrap_or(true)
        {
            continue;
        }
        let Ok(executable) = fs::read_link(entry.path().join("exe")) else {
            continue;
        };
        let Ok(bytes) = fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        let args = bytes
            .split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect();
        result.push(Process {
            pid,
            executable,
            args,
        });
    }
    Ok(result)
}
pub fn node_cli(process: &Process) -> bool {
    if !matches!(
        process.executable.file_name().and_then(|s| s.to_str()),
        Some("node" | "nodejs" | "bun")
    ) {
        return false;
    }
    for arg in process.args.iter().skip(1) {
        if arg.starts_with("-e")
            || arg.starts_with("-p")
            || arg.starts_with("--eval")
            || arg.starts_with("--print")
        {
            return false;
        }
        if arg.starts_with('-') {
            continue;
        }
        let path = Path::new(arg);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            let Ok(cwd) = fs::read_link(format!("/proc/{}/cwd", process.pid)) else {
                return false;
            };
            cwd.join(path)
        };
        return path.ends_with("node_modules/@anthropic-ai/claude-code/cli.js");
    }
    false
}
pub fn desktop_process(process: &Process) -> bool {
    matches!(
        process.executable.to_str().map(|path|path.strip_suffix(" (deleted)").unwrap_or(path)),
        Some(
            "/usr/lib/claude-desktop/claude-desktop"
                | "/usr/lib/claude-desktop/chrome_crashpad_handler"
                | "/usr/lib/claude-desktop/resources/cowork-linux-helper"
        )
    )
}
const PROFILE_UNKNOWN: &str = "无法确认浏览器进程的资料归属，未操作资料。";
#[derive(PartialEq, Eq)]
struct ProfileProcess {
    pid: i32,
    executable: PathBuf,
    started: u64,
}
fn browser_name(value: &str) -> bool {
    matches!(
        value.strip_suffix(" (deleted)").unwrap_or(value),
        "chrome" | "chromium" | "chromium-browser" | "chromium-browse"
    )
}
fn browser_profile_argument(args: &[String]) -> Result<Option<&Path>, String> {
    let mut selected = None;
    let mut args = args.iter().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--" {
            break;
        }
        if arg
            .strip_prefix("--type=")
            .is_some_and(|value| !value.is_empty())
        {
            return Ok(None);
        }
        let value = if arg == "--user-data-dir" {
            Some(args.next().ok_or(PROFILE_UNKNOWN)?.as_str())
        } else {
            arg.strip_prefix("--user-data-dir=")
        };
        if let Some(value) = value {
            if selected.is_some() || !Path::new(value).is_absolute() {
                return Err(PROFILE_UNKNOWN.into());
            }
            selected = Some(Path::new(value));
        }
    }
    Ok(selected)
}
fn browser_stamp(process: &Path) -> Result<Option<(u32, u64)>, String> {
    let uid = fs::metadata(process).map_err(|_| PROFILE_UNKNOWN)?.uid();
    if uid != unsafe { getuid() } {
        return Ok(None);
    }
    let stat = fs::read_to_string(process.join("stat")).map_err(|_| PROFILE_UNKNOWN)?;
    let mut fields = stat
        .rsplit_once(')')
        .ok_or(PROFILE_UNKNOWN)?
        .1
        .split_whitespace();
    if matches!(fields.next(), Some("Z" | "X")) {
        return Ok(None);
    }
    let started = fields
        .nth(18)
        .and_then(|s| s.parse().ok())
        .ok_or(PROFILE_UNKNOWN)?;
    Ok(Some((uid, started)))
}
// ----- 浏览器独立扫描；不改变 Claude 通用枚举的匹配规则，不因无关进程读失败阻断 -----
fn profile_processes_at(proc_root: &Path, profile: &Path) -> Result<Vec<ProfileProcess>, String> {
    match fs::metadata(profile) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Ok(metadata) if metadata.is_dir() => {}
        _ => return Err(PROFILE_UNKNOWN.into()),
    }
    let mut result = Vec::new();
    for entry in fs::read_dir(proc_root)
        .map_err(|_| PROFILE_UNKNOWN)?
        .flatten()
    {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<i32>() else {
            continue;
        };
        if pid <= 1 {
            continue;
        }
        let process = entry.path();
        let executable = fs::read_link(process.join("exe"));
        let candidate = match &executable {
            Ok(path) => path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(browser_name),
            Err(_) => {
                fs::read_to_string(process.join("comm")).is_ok_and(|name| browser_name(name.trim()))
            }
        };
        if !candidate {
            continue;
        }
        let read = (|| -> Result<Option<ProfileProcess>, String> {
            let Some(before) = browser_stamp(&process)? else {
                return Ok(None);
            };
            let bytes = fs::read(process.join("cmdline")).map_err(|_| PROFILE_UNKNOWN)?;
            if bytes.last() != Some(&0) {
                return Err(PROFILE_UNKNOWN.into());
            }
            let args = bytes[..bytes.len() - 1]
                .split(|b| *b == 0)
                .map(|s| std::str::from_utf8(s).map(str::to_owned))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| PROFILE_UNKNOWN)?;
            if browser_stamp(&process)? != Some(before) {
                return Err(PROFILE_UNKNOWN.into());
            }
            let selected = browser_profile_argument(&args)?;
            // 辅助进程不持有父进程资料锁；普通外部浏览器的默认资料也不属于本应用。
            let helper = args
                .iter()
                .skip(1)
                .take_while(|s| s.as_str() != "--")
                .any(|s| {
                    s.strip_prefix("--type=")
                        .is_some_and(|value| !value.is_empty())
                });
            if helper {
                return Ok(None);
            }
            // 沙箱辅助进程的 exe 可能不可读；仅元数据稳定且参数明确为辅助模式才先排除。
            let executable = executable.map_err(|_| PROFILE_UNKNOWN)?;
            let Some(selected) = selected else {
                return if matches!(
                    executable
                        .parent()
                        .and_then(Path::file_name)
                        .and_then(|s| s.to_str()),
                    Some("LinuxBrowser" | "CCBaga-B")
                ) {
                    Err(PROFILE_UNKNOWN.into())
                } else {
                    Ok(None)
                };
            };
            let a = fs::metadata(selected).map_err(|_| PROFILE_UNKNOWN)?;
            let b = fs::metadata(profile).map_err(|_| PROFILE_UNKNOWN)?;
            if !a.is_dir() || !b.is_dir() {
                return Err(PROFILE_UNKNOWN.into());
            }
            if browser_stamp(&process)? != Some(before) {
                return Err(PROFILE_UNKNOWN.into());
            }
            Ok(
                (a.dev() == b.dev() && a.ino() == b.ino()).then_some(ProfileProcess {
                    pid,
                    executable,
                    started: before.1,
                }),
            )
        })();
        match read {
            Ok(Some(process)) => result.push(process),
            Ok(None) => {}
            Err(_)
                if fs::metadata(&process)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(result)
}
fn profile_processes(profile: &str) -> Result<Vec<ProfileProcess>, String> {
    profile_processes_at(Path::new("/proc"), Path::new(profile))
}
pub unsafe fn rf_browser_running(profile: *const c_char) -> *mut c_char {
    response(profile_processes(input(profile)).map(|ps| json!({"running": !ps.is_empty()})))
}
pub unsafe fn rf_browser_quit(profile: *const c_char) -> *mut c_char {
    response((|| {
        let profile = input(profile);
        for target in profile_processes(profile)? {
            if profile_processes(profile)?.iter().any(|p| p == &target) && kill(target.pid, 15) != 0
            {
                return Err(
                    "Could not close the companion browser; close its windows manually.".into(),
                );
            }
        }
        Ok(json!({"ok":true}))
    })())
}
fn zones() -> Vec<String> {
    let mut result = Vec::new();
    for table in [
        "/usr/share/zoneinfo/zone.tab",
        "/usr/share/zoneinfo/zone1970.tab",
    ] {
        if let Ok(text) = fs::read_to_string(table) {
            result.extend(
                text.lines()
                    .filter(|l| !l.starts_with('#'))
                    .filter_map(|l| l.split_whitespace().nth(2))
                    .map(str::to_owned),
            );
        }
    }
    result.extend(["UTC".into(), "Etc/UTC".into()]);
    result.sort();
    result.dedup();
    result
}
fn timezone() -> String {
    fs::read_link("/etc/localtime")
        .ok()
        .and_then(|p| {
            p.to_str()
                .and_then(|s| s.split("/zoneinfo/").nth(1))
                .map(str::to_owned)
        })
        .or_else(|| {
            fs::read_to_string("/etc/timezone")
                .ok()
                .map(|s| s.trim().into())
        })
        .unwrap_or_else(|| {
            output("timedatectl", &["show", "--property=Timezone", "--value"]).unwrap_or_default()
        })
}
fn raw_session_locale(category: &str) -> String {
    ["LC_ALL", category, "LANG"]
        .iter()
        .find_map(|key| std::env::var(key).ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "C".into())
}
fn session_locale(category: &str) -> String {
    let raw = raw_session_locale(category);
    let (head, modifier) = raw.split_once('@').unwrap_or((&raw, ""));
    let base = head.split('.').next().unwrap_or("C");
    if modifier.is_empty() {
        base.into()
    } else {
        format!("{base}@{modifier}")
    }
}
fn locale() -> String {
    session_locale("LC_TIME")
}
fn session_language() -> String {
    language(&session_locale("LC_MESSAGES"))
}
fn language_matches(current: &str, requested: &str) -> bool {
    !requested.is_empty()
        && (current == requested
            || current
                .strip_prefix(requested)
                .is_some_and(|s| s.starts_with('-')))
}
fn language(locale: &str) -> String {
    // 地区覆盖不改变语言/脚本；例如 zh_CN@rg=twzzzz 仍为简体中文。
    let locale = locale.split('@').next().unwrap_or(locale);
    match locale {
        "zh_CN" | "zh_SG" => "zh-Hans".into(),
        "zh_TW" | "zh_HK" => "zh-Hant".into(),
        s if s.starts_with("en_") || s == "pt_BR" => s.replace('_', "-"),
        s => s.split('_').next().unwrap_or(s).into(),
    }
}
fn location() -> Option<bool> {
    if !std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase()
        .contains("gnome")
    {
        return None;
    }
    output(
        "gsettings",
        &["get", "org.gnome.system.location", "enabled"],
    )
    .ok()
    .and_then(|s| match s.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    })
}
fn snapshot() -> Value {
    json!({"timezone":timezone(), "timezones":zones(), "automatic_timezone":null,
        "languages":[session_language()], "location_enabled":location(), "accessibility":false})
}
pub unsafe fn rf_browser_machine() -> *mut c_char {
    let mut state = snapshot();
    state["platform"] = json!("linux");
    state["version"] = json!(output("uname", &["-r"]).unwrap_or_default());
    state["cores"] = json!(std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1));
    // 浏览器保留原始区域；系统预检仅去除代码集，不能丢掉改变地区的 modifier。
    state["locale"] = json!(raw_session_locale("LC_TIME"));
    // TUN is the documented route. Do not silently report a desktop proxy as direct.
    let proxy = ["ALL_PROXY", "all_proxy", "HTTPS_PROXY", "https_proxy"]
        .iter()
        .find_map(|k| std::env::var(k).ok())
        .unwrap_or_default();
    state["system_proxy"] = json!(proxy);
    response(Ok(state))
}
pub unsafe fn rf_system_read() -> *mut c_char {
    response(Ok(snapshot()))
}
pub unsafe fn rf_system_open(pane: *const c_char) -> *mut c_char {
    response(open_settings(input(pane)))
}
pub unsafe fn rf_system_timezone(zone: *const c_char) -> *mut c_char {
    response((|| {
        let zone = input(zone);
        if !zones().iter().any(|s| s == zone) {
            return Err("Invalid time zone.".into());
        }
        output("timedatectl", &["set-timezone", zone])?;
        if timezone() != zone {
            return Err("Time zone change could not be verified.".into());
        }
        Ok(
            json!({"state":snapshot(), "message":"Time zone applied and verified. Automatic location-based time zone is controlled by your desktop."}),
        )
    })())
}
pub unsafe fn rf_system_language(requested: *const c_char) -> *mut c_char {
    let requested = input(requested);
    if language_matches(&session_language(), requested) {
        response(Ok(
            json!({"state":snapshot(), "message":"The current login-session language already matches."}),
        ))
    } else {
        response(Err("Change the login-session language in desktop settings, then sign out and back in. Linux has no portable per-user language setter.".into()))
    }
}
pub unsafe fn rf_system_region(requested: *const c_char, region: *const c_char) -> *mut c_char {
    let current = locale();
    let requested = input(requested);
    if language_matches(&session_language(), requested) && current == input(region) {
        response(Ok(
            json!({"state":snapshot(), "message":"Current session language and regional formats verified."}),
        ))
    } else {
        response(Err("Set language and regional formats in desktop settings, then sign out and back in before applying this region.".into()))
    }
}
pub unsafe fn rf_system_location() -> *mut c_char {
    response((|| {
        if location().is_none() {
            return Err("Location state is unknown on this desktop. Manage location in your desktop settings; no disabled state was verified.".into());
        }
        output(
            "gsettings",
            &["set", "org.gnome.system.location", "enabled", "false"],
        )?;
        if location() != Some(false) {
            return Err("Location disable could not be verified.".into());
        }
        Ok(
            json!({"state":snapshot(), "message":"GNOME location preference is disabled; other desktop/portal services are not independently verified."}),
        )
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_processes_require_exact_installed_executables() {
        for (path, expected) in [
            ("/usr/lib/claude-desktop/claude-desktop", true),
            ("/usr/lib/claude-desktop/claude-desktop (deleted)", true),
            ("/usr/lib/claude-desktop/chrome_crashpad_handler", true),
            (
                "/usr/lib/claude-desktop/resources/cowork-linux-helper",
                true,
            ),
            ("/tmp/claude-desktop", false),
            ("/usr/lib/claude-desktop-backup/claude-desktop", false),
            ("/usr/lib/claude-desktop/other-app", false),
            ("/usr/bin/node", false),
        ] {
            let p = Process {
                pid: 42,
                executable: path.into(),
                args: Vec::new(),
            };
            assert_eq!(desktop_process(&p), expected, "{path}");
        }
    }
    #[test]
    fn machine_read_is_non_mutating_and_unknown_location_is_null() {
        let state: Value = crate::native::decode(unsafe { rf_browser_machine() }).unwrap();
        assert_eq!(state["platform"], "linux");
        assert!(state["cores"].as_u64().unwrap() > 0);
        assert!(state["timezones"].as_array().unwrap().len() > 100);
    }
    #[test]
    fn process_arguments_require_exact_cli_entry() {
        let mut p = Process {
            pid: 2,
            executable: "/usr/bin/node".into(),
            args: vec![
                "node".into(),
                "/tmp/node_modules/@anthropic-ai/claude-code/cli.js".into(),
            ],
        };
        assert!(node_cli(&p));
        p.args[1].push_str(".bak");
        assert!(!node_cli(&p));
        p.args[1] = "/tmp/other.js".into();
        p.args
            .push("/tmp/node_modules/@anthropic-ai/claude-code/cli.js".into());
        assert!(!node_cli(&p));
    }
    #[test]
    fn browser_profiles_require_unique_arguments_and_verified_directory_identity() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!(
            "ccbaga-linux-proc-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let proc_root = root.join("proc");
        let profile = root.join("profile");
        let other = root.join("profile-other");
        for path in [&proc_root, &profile, &other] {
            fs::create_dir_all(path).unwrap();
        }
        let alias = root.join("alias");
        symlink(&profile, &alias).unwrap();
        let process = proc_root.join("42");
        fs::create_dir(&process).unwrap();
        symlink(root.join("LinuxBrowser/chrome"), process.join("exe")).unwrap();
        fs::write(process.join("comm"), "chrome\n").unwrap();
        let stat = format!("42 (chrome) S {} 100\n", vec!["0"; 18].join(" "));
        fs::write(process.join("stat"), &stat).unwrap();
        let command = |args: &[&str]| {
            let mut bytes = args.join("\0").into_bytes();
            bytes.push(0);
            fs::write(process.join("cmdline"), bytes).unwrap();
        };
        for selected in [&profile, &alias, &profile.join(".")] {
            command(&["chrome", "--user-data-dir", selected.to_str().unwrap()]);
            let found = profile_processes_at(&proc_root, &profile).unwrap();
            assert_eq!(found.len(), 1);
            assert_eq!((found[0].pid, found[0].started), (42, 100));
        }
        let expected = format!("--user-data-dir={}", profile.display());
        for invalid in [
            vec!["chrome"],
            vec!["chrome", "--", &expected],
            vec!["chrome", &expected, &expected],
            vec!["chrome", "--user-data-dir"],
            vec!["chrome", "--user-data-dir=relative"],
            vec!["chrome", "--user-data-dir=", &expected],
        ] {
            command(&invalid);
            assert!(
                profile_processes_at(&proc_root, &profile).is_err(),
                "{invalid:?}"
            );
        }
        command(&["chrome", "--type=renderer", &expected]);
        assert!(profile_processes_at(&proc_root, &profile)
            .unwrap()
            .is_empty());
        command(&["chrome", "--user-data-dir", other.to_str().unwrap()]);
        assert!(profile_processes_at(&proc_root, &profile)
            .unwrap()
            .is_empty());
        assert!(profile_processes_at(&proc_root, &root.join("not-created"))
            .unwrap()
            .is_empty());
        command(&["chrome", &expected]);
        fs::remove_file(process.join("stat")).unwrap();
        assert!(profile_processes_at(&proc_root, &profile).is_err());
        fs::write(process.join("stat"), &stat).unwrap();
        fs::remove_file(process.join("cmdline")).unwrap();
        assert!(profile_processes_at(&proc_root, &profile).is_err());
        fs::write(process.join("cmdline"), b"chrome\0\xff\0").unwrap();
        assert!(profile_processes_at(&proc_root, &profile).is_err());
        fs::remove_file(process.join("exe")).unwrap();
        assert!(profile_processes_at(&proc_root, &profile).is_err());
        command(&["chrome", "--type=renderer", &expected]);
        assert!(profile_processes_at(&proc_root, &profile)
            .unwrap()
            .is_empty());
        command(&["chrome", &expected]);
        assert!(profile_processes_at(&proc_root, &profile).is_err());
        // ----- 无关系统进程的读失败不阻断，明确外部浏览器默认资料也不混入 -----
        fs::write(process.join("comm"), "bash\n").unwrap();
        assert!(profile_processes_at(&proc_root, &profile)
            .unwrap()
            .is_empty());
        symlink(root.join("system/chrome"), process.join("exe")).unwrap();
        command(&["chrome"]);
        assert!(profile_processes_at(&proc_root, &profile)
            .unwrap()
            .is_empty());
        command(&["chrome", &expected]);
        fs::write(process.join("stat"), stat.replace(" S ", " Z ")).unwrap();
        assert!(profile_processes_at(&proc_root, &profile)
            .unwrap()
            .is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn session_language_mapping_and_matching_respect_language_boundaries() {
        assert_eq!(language("zh_TW"), "zh-Hant");
        assert_eq!(language("zh_CN"), "zh-Hans");
        assert_eq!(language("zh_CN@rg=twzzzz"), "zh-Hans");
        assert_eq!(language("en_US"), "en-US");
        assert_eq!(language("de_DE"), "de");
        assert!(language_matches("en-US", "en"));
        assert!(language_matches("en-US", "en-US"));
        for invalid in ["", "e", "en-U", "en-GB", "en-US;bad"] {
            assert!(!language_matches("en-US", invalid));
        }
    }
    #[test]
    fn locale_modifiers_do_not_pass_a_different_region_preflight() {
        const FIXTURE: &str = "CCBAGA_TEST_LINUX_LOCALE";
        if let Ok(expected) = std::env::var(FIXTURE) {
            assert_eq!(session_language(), "zh-Hans");
            let state = crate::browser::config::machine().unwrap();
            assert_eq!(
                state.locale_country,
                if expected == "allow" { "CN" } else { "TW" }
            );
            let result: Result<Value, _> = crate::native::decode(unsafe {
                rf_system_region(b"zh-Hans\0".as_ptr().cast(), b"zh_CN\0".as_ptr().cast())
            });
            assert_eq!(result.is_ok(), expected == "allow");
            return;
        }
        // ----- 独立子进程固定会话环境，不改其他回归或用户登录会话 -----
        for (region, expected) in [("zh_CN.UTF-8@rg=twzzzz", "deny"), ("zh_CN.UTF-8", "allow")] {
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "linux::tests::locale_modifiers_do_not_pass_a_different_region_preflight",
                    "--test-threads=1",
                ])
                .env_remove("LC_ALL")
                .env("LC_MESSAGES", "zh_CN.UTF-8")
                .env("LC_TIME", region)
                .env("LANG", "en_US.UTF-8")
                .env(FIXTURE, expected)
                .output()
                .unwrap();
            assert!(
                child.status.success(),
                "{region}: {}",
                String::from_utf8_lossy(&child.stderr)
            );
        }
    }
    #[test]
    fn unsupported_credentials_do_not_report_empty_success() {
        assert!(crate::native::credentials().is_err());
    }
}
