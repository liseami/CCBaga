// [INPUT]: 当前用户 procfs、zoneinfo、桌面偏好和本地会话工具。
// [OUTPUT]: Linux 平台 JSON 桥；未知状态保留 null，会话语言变化要求用户手动操作，机器国家保留 POSIX @rg 覆盖。
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
fn profile_processes(profile: &str) -> Result<Vec<Process>, String> {
    let expected = format!("--user-data-dir={profile}");
    Ok(processes()?
        .into_iter()
        .filter(|p| {
            // Only the browser parent, never unrelated processes mentioning this path.
            matches!(
                p.executable.file_name().and_then(|s| s.to_str()),
                Some("chrome" | "chromium" | "chromium-browser")
            ) && !p.args.iter().any(|a| a.starts_with("--type="))
                && p.args.iter().any(|a| a == &expected)
        })
        .collect())
}
pub unsafe fn rf_browser_running(profile: *const c_char) -> *mut c_char {
    response(profile_processes(input(profile)).map(|ps| json!({"running": !ps.is_empty()})))
}
pub unsafe fn rf_browser_quit(profile: *const c_char) -> *mut c_char {
    response((|| {
        let profile = input(profile);
        for target in profile_processes(profile)? {
            if profile_processes(profile)?
                .iter()
                .any(|p| p.pid == target.pid && p.executable == target.executable)
                && kill(target.pid, 15) != 0
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
    raw_session_locale(category)
        .split('.')
        .next()
        .unwrap_or("C")
        .into()
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
    state["locale"] = json!(locale());
    if let Some(country) = crate::regions::locale_country_from_posix(&raw_session_locale("LC_TIME"))
    {
        state["locale_country"] = json!(country);
    }
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
    fn session_language_mapping_and_matching_respect_language_boundaries() {
        assert_eq!(language("zh_TW"), "zh-Hant");
        assert_eq!(language("zh_CN"), "zh-Hans");
        assert_eq!(language("en_US"), "en-US");
        assert_eq!(language("de_DE"), "de");
        assert!(language_matches("en-US", "en"));
        assert!(language_matches("en-US", "en-US"));
        for invalid in ["", "e", "en-U", "en-GB", "en-US;bad"] {
            assert!(!language_matches("en-US", invalid));
        }
    }
    #[test]
    fn unsupported_credentials_do_not_report_empty_success() {
        assert!(crate::native::credentials().is_err());
    }
}
