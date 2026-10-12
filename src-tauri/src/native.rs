// [INPUT]: macOS Objective-C / Linux Rust / Windows Rust 的本地清理、授权与剪贴板接口。
// [OUTPUT]: 辅助功能权限、剪贴板复制、钥匙串解锁、脱敏凭据发现及用户确认的原生身份凭据暂存/恢复，不将正文交给 UI。
// [POS]: 清理与电脑设置的平台桥接边界；公共 JSON 解码供独立 system 桥复用。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde::Deserialize;
use std::ffi::{c_char, CStr, CString};

#[cfg(target_os = "linux")]
use crate::linux::*;
#[cfg(target_os = "windows")]
use crate::windows::*;
#[cfg(target_os = "macos")]
extern "C" {
    fn rf_permissions();
    fn rf_copy(text: *const c_char);
    fn rf_free(value: *mut c_char);
    fn rf_claude_pids() -> *mut c_char;
    fn rf_credentials_list() -> *mut c_char;
    fn rf_credentials_remove(reference: *const c_char) -> *mut c_char;
    fn rf_keychain_unlock() -> *mut c_char;
    fn rf_identity_snapshot() -> *mut c_char;
    fn rf_identity_remove(json: *const c_char) -> *mut c_char;
    fn rf_identity_restore(json: *const c_char) -> *mut c_char;
}

#[derive(Clone, Deserialize)]
pub struct Credential {
    pub service: String,
    pub reference: String,
}
pub fn credentials() -> Result<Vec<Credential>, String> {
    #[derive(Deserialize)]
    struct List {
        items: Vec<Credential>,
    }
    Ok(decode::<List>(unsafe { rf_credentials_list() })?.items)
}
pub fn unlock_keychain() -> Result<String, String> {
    #[derive(Deserialize)]
    struct ResultMessage {
        message: String,
    }
    Ok(decode::<ResultMessage>(unsafe { rf_keychain_unlock() })?.message)
}
pub fn remove_credential(reference: &str) -> Result<(), String> {
    let reference = CString::new(reference).map_err(|_| "凭据编号无效。")?;
    let _: serde_json::Value = decode(unsafe { rf_credentials_remove(reference.as_ptr()) })?;
    Ok(())
}

pub(crate) fn decode<T: serde::de::DeserializeOwned>(ptr: *mut c_char) -> Result<T, String> {
    if ptr.is_null() {
        return Err("系统操作失败。".into());
    }
    let text = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    unsafe {
        rf_free(ptr);
    }
    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "系统返回了无法识别的数据。")?;
    if let Some(error) = data["error"].as_str() {
        return Err(error.into());
    }
    serde_json::from_value(data).map_err(|_| "系统返回数据不完整。".into())
}

#[cfg(target_os = "macos")]
pub fn claude_pids() -> Result<Vec<i32>, String> {
    decode(unsafe { rf_claude_pids() })
}
pub fn permissions() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        crate::linux::open_settings("location").map(|_| ())
    }
    #[cfg(target_os = "windows")]
    {
        crate::windows::open_settings("location").map(|_| ())
    }
    #[cfg(target_os = "macos")]
    {
        unsafe {
            rf_permissions();
        }
        Ok(())
    }
}
pub fn copy(text: &str) -> Result<(), String> {
    let text = CString::new(text).map_err(|_| "文本包含无法复制的字符。")?;
    unsafe {
        #[cfg(target_os = "linux")]
        rf_copy(text.as_ptr())?;
        #[cfg(target_os = "windows")]
        rf_copy(text.as_ptr())?;
        #[cfg(target_os = "macos")]
        rf_copy(text.as_ptr());
    }
    Ok(())
}

// ----- 只供身份保险库使用；没有可直接调用的 WebView 凭据命令 -----
pub(crate) fn identity_snapshot() -> Result<Vec<serde_json::Value>, String> {
    #[cfg(target_os = "macos")]
    {
        #[derive(Deserialize)]
        struct Snapshot {
            items: Vec<serde_json::Value>,
        }
        return Ok(decode::<Snapshot>(unsafe { rf_identity_snapshot() })?.items);
    }
    #[cfg(not(target_os = "macos"))]
    Ok(Vec::new())
}
pub(crate) fn identity_keys(items: &[serde_json::Value], restore: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let bytes = serde_json::to_string(items).map_err(|_| "身份凭据无法处理。")?;
        let text = CString::new(bytes).map_err(|_| "身份凭据无法处理。")?;
        let _: serde_json::Value = decode(unsafe {
            if restore {
                rf_identity_restore(text.as_ptr())
            } else {
                rf_identity_remove(text.as_ptr())
            }
        })?;
    }
    let _ = (items, restore);
    Ok(())
}
