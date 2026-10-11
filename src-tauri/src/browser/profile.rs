// [INPUT]: 宿主独立资料目录、原生进程核验与已验证配置。
// [OUTPUT]: 浏览资料占用保护、窄范围 Preferences 更新及缓存/登录数据清理。
// [POS]: 文件边界；清理只遍历固定位置，拒绝链接，不生成备份。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use crate::native::decode;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    ffi::{c_char, CString},
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
extern "C" {
    fn rf_browser_running(profile: *const c_char) -> *mut c_char;
    fn rf_browser_quit(profile: *const c_char) -> *mut c_char;
}
pub fn running(profile: &Path) -> Result<bool, String> {
    let path = CString::new(profile.to_string_lossy().as_bytes()).map_err(|_| "资料路径无效。")?;
    let state: Value = decode(unsafe { rf_browser_running(path.as_ptr()) })?;
    Ok(state["running"].as_bool().unwrap_or(false))
}
pub fn quit(profile: &Path) -> Result<(), String> {
    let path = CString::new(profile.to_string_lossy().as_bytes()).map_err(|_| "资料路径无效。")?;
    let _: Value = decode(unsafe { rf_browser_quit(path.as_ptr()) })?;
    for _ in 0..100 {
        if !running(profile)? {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err("浏览器仍在运行，请保存任务并关闭浏览器窗口。".into())
}
pub fn check_path(root: &Path, path: &Path) -> Result<(), String> {
    if !path.starts_with(root) {
        return Err("路径不在独立浏览资料内。".into());
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("检测到链接，未修改资料。".into())
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err("无法检查浏览资料路径。".into())
            }
            _ => {}
        }
    }
    Ok(())
}
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let root = path.parent().ok_or("配置目录无效。")?;
    check_path(root, path)?;
    fs::create_dir_all(root).map_err(|_| "无法创建浏览器目录。")?;
    let temp = path.with_extension("pending");
    check_path(root, &temp)?;
    let data = serde_json::to_vec_pretty(value).map_err(|_| "配置编码失败。")?;
    fs::write(&temp, data).map_err(|_| "配置写入失败。")?;
    fs::set_permissions(&temp, fs::Permissions::from_mode(0o600))
        .map_err(|_| "配置权限设置失败。")?;
    if fs::rename(&temp, path).is_err() {
        let _ = fs::remove_file(&temp);
        return Err("配置保存失败。".into());
    }
    Ok(())
}
pub fn preferences(
    profile: &Path,
    config: &super::config::Config,
    machine: &super::config::Machine,
) -> Result<(), String> {
    let path = profile.join("Default/Preferences");
    check_path(profile, &path)?;
    let mut data: Value = match fs::read(&path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|_| "Preferences 损坏，未覆盖原文件。")?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(_) => return Err("无法读取 Preferences。".into()),
    };
    if !data.is_object() {
        return Err("Preferences 格式错误。".into());
    }
    for pointer in [
        "/intl",
        "/webrtc",
        "/profile",
        "/profile/default_content_setting_values",
        "/profile/content_settings",
        "/profile/content_settings/exceptions",
    ] {
        if data
            .pointer(pointer)
            .map(|v| !v.is_object() && !v.is_null())
            .unwrap_or(false)
        {
            return Err("Preferences 字段格式错误，未修改原文件。".into());
        }
    }
    let lang = config.language(machine);
    data["intl"]["accept_languages"] =
        json!(format!("{lang},{}", lang.split('-').next().unwrap_or("en")));
    data["intl"]["selected_languages"] = data["intl"]["accept_languages"].clone();
    data["webrtc"]["ip_handling_policy"] = json!("disable_non_proxied_udp");
    data["profile"]["default_content_setting_values"]["geolocation"] = json!(2);
    // Chromium 148 的定位拆分近似/精确权限；同时关闭两项，旧键兼容旧资料。
    data["profile"]["default_content_setting_values"]["geolocation_with_options"] =
        json!({"approximate":2,"precise":2});
    data["profile"]["content_settings"]["exceptions"]["geolocation"] = json!({});
    data["profile"]["content_settings"]["exceptions"]["geolocation_with_options"] = json!({});
    write_json(&path, &data)
}
pub const CACHE: &[&str] = &[
    "Default/Cache",
    "Default/Code Cache",
    "Default/GPUCache",
    "Default/DawnCache",
    "Default/Service Worker/CacheStorage",
    "Default/Service Worker/ScriptCache",
    "ShaderCache",
    "GrShaderCache",
    "GraphiteDawnCache",
    "Crashpad/reports",
];
pub const SITE_DATA: &[&str] = &[
    "Default/Network/Cookies",
    "Default/Network/Cookies-journal",
    "Default/Cookies",
    "Default/Cookies-journal",
    "Default/Local Storage",
    "Default/Session Storage",
    "Default/IndexedDB",
    "Default/Service Worker",
    "Default/WebStorage",
    "Default/Storage",
    "Default/Sessions",
];
fn size(path: &Path) -> Result<u64, String> {
    let meta = fs::symlink_metadata(path).map_err(|_| "无法读取缓存项目。")?;
    if meta.file_type().is_symlink() {
        return Err("缓存中包含链接，已停止。".into());
    }
    if meta.is_file() {
        return Ok(meta.len());
    }
    let mut bytes = 0;
    for entry in fs::read_dir(path).map_err(|_| "无法枚举缓存。")? {
        bytes += size(&entry.map_err(|_| "无法读取缓存条目。")?.path())?;
    }
    Ok(bytes)
}
#[derive(Serialize)]
pub struct CacheItem {
    pub path: String,
    pub bytes: u64,
    pub login: bool,
}
pub fn scan(profile: &Path) -> Result<Vec<CacheItem>, String> {
    let mut items = Vec::new();
    for (paths, login) in [(CACHE, false), (SITE_DATA, true)] {
        for name in paths {
            let path = profile.join(name);
            check_path(profile, &path)?;
            if path.exists() {
                items.push(CacheItem {
                    path: name.to_string(),
                    bytes: size(&path)?,
                    login,
                });
            }
        }
    }
    Ok(items)
}
pub fn clear(
    profile: &Path,
    login: bool,
    mut log: impl FnMut(&str, bool),
) -> Result<usize, String> {
    let paths = if login {
        CACHE.iter().chain(SITE_DATA).copied().collect::<Vec<_>>()
    } else {
        CACHE.to_vec()
    };
    let mut count = 0;
    for name in paths {
        let path = profile.join(name);
        check_path(profile, &path)?;
        if !path.exists() {
            continue;
        }
        size(&path)?;
        log(name, false);
        let result = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        if result.is_err() {
            return Err(format!("{name} 删除失败；已处理 {count} 项，后续未执行。"));
        }
        count += 1;
        log(name, true);
    }
    Ok(count)
}
