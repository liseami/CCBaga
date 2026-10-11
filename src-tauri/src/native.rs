// [INPUT]: Objective-C 本地清理、授权、SSH 文件选择与剪贴板桥的 C 接口。
// [OUTPUT]: SSH 文件路径选择、辅助功能权限、剪贴板复制、钥匙串解锁、脱敏凭据发现与允许系统授权的直接删除。
// [POS]: 清理与电脑设置的 macOS 桥接边界；公共 JSON 解码供独立 system 桥复用。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde::Deserialize;
use std::ffi::{c_char, CStr, CString};

extern "C" {
    fn rf_choose_ssh_keys(extension: *const c_char) -> *mut c_char;
    fn rf_permissions();
    fn rf_copy(text: *const c_char);
    fn rf_free(value: *mut c_char);
    fn rf_claude_pids() -> *mut c_char;
    fn rf_credentials_list() -> *mut c_char;
    fn rf_credentials_remove(reference: *const c_char) -> *mut c_char;
    fn rf_keychain_unlock() -> *mut c_char;
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

pub fn choose_ssh_keys(extension: &str) -> Result<Vec<String>, String> {
    if !["all", "pem", "ppk", "pub"].contains(&extension) {
        return Err("无效的密钥文件类型。".into());
    }
    let extension = CString::new(extension).map_err(|_| "无效的密钥文件类型。")?;
    decode(unsafe { rf_choose_ssh_keys(extension.as_ptr()) })
}
pub fn claude_pids() -> Result<Vec<i32>, String> {
    decode(unsafe { rf_claude_pids() })
}
pub fn permissions() {
    unsafe {
        rf_permissions();
    }
}
pub fn copy(text: &str) -> Result<(), String> {
    let text = CString::new(text).map_err(|_| "文本包含无法复制的字符。")?;
    unsafe {
        rf_copy(text.as_ptr());
    }
    Ok(())
}
