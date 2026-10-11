// [INPUT]: macOS 电脑设置原生桥与 Tauri 后台任务。
// [OUTPUT]: 电脑状态查询、时区与语言应用、定位关闭和系统设置入口。
// [POS]: 独立于清理的命令边界；所有成功状态来自原生核验。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use crate::native::decode;
use std::ffi::{c_char, CString};
extern "C" {
    fn rf_system_region(language: *const c_char, locale: *const c_char) -> *mut c_char;
    fn rf_system_read() -> *mut c_char;
    fn rf_system_open(pane: *const c_char) -> *mut c_char;
    fn rf_system_language(language: *const c_char) -> *mut c_char;
    fn rf_system_timezone(timezone: *const c_char) -> *mut c_char;
    fn rf_system_location() -> *mut c_char;
}
async fn run(
    action: impl FnOnce() -> Result<serde_json::Value, String> + Send + 'static,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(action)
        .await
        .map_err(|_| "系统操作被中断，请刷新当前状态。".to_string())?
}
#[tauri::command]
pub async fn system_read() -> Result<serde_json::Value, String> {
    run(|| decode(unsafe { rf_system_read() })).await
}
#[tauri::command]
pub async fn system_open_settings(pane: String) -> Result<serde_json::Value, String> {
    run(move || {
        let pane = CString::new(pane).map_err(|_| "系统设置入口无效。")?;
        decode(unsafe { rf_system_open(pane.as_ptr()) })
    })
    .await
}
#[tauri::command]
pub async fn system_set_language(language: String) -> Result<serde_json::Value, String> {
    run(move || {
        let language = CString::new(language).map_err(|_| "语言无效。")?;
        decode(unsafe { rf_system_language(language.as_ptr()) })
    })
    .await
}
#[tauri::command]
pub async fn system_set_timezone(timezone: String) -> Result<serde_json::Value, String> {
    run(move || {
        let timezone = CString::new(timezone).map_err(|_| "时区无效。")?;
        decode(unsafe { rf_system_timezone(timezone.as_ptr()) })
    })
    .await
}
#[tauri::command]
pub async fn system_disable_location() -> Result<serde_json::Value, String> {
    run(|| decode(unsafe { rf_system_location() })).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_native_state_and_rejects_invalid_inputs_without_mutation() {
        let state: serde_json::Value = decode(unsafe { rf_system_read() }).unwrap();
        assert!(state["timezone"].as_str().is_some());
        assert!(state["timezones"].as_array().unwrap().len() > 100);
        assert!(state["languages"].is_array());
        assert!(state["location_enabled"].is_boolean());
        let invalid = CString::new("Invalid/Zone; touch /tmp/rf-injected").unwrap();
        assert!(
            decode::<serde_json::Value>(unsafe { rf_system_timezone(invalid.as_ptr()) }).is_err()
        );
        assert!(
            decode::<serde_json::Value>(unsafe { rf_system_language(invalid.as_ptr()) }).is_err()
        );
        assert!(decode::<serde_json::Value>(unsafe { rf_system_open(invalid.as_ptr()) }).is_err());
    }
}

#[tauri::command]
pub async fn system_set_region(region: String) -> Result<serde_json::Value, String> {
    let region = crate::regions::find(&region)?;
    run(move || {
        let zone = CString::new(region.timezone).unwrap();
        let language = CString::new(region.language).unwrap();
        let locale = CString::new(region.locale).unwrap();
        let _: serde_json::Value = decode(unsafe { rf_system_timezone(zone.as_ptr()) })?;
        let result = decode(unsafe { rf_system_region(language.as_ptr(), locale.as_ptr()) });
        result.map_err(|error| format!("时区已修改；{error}"))
    })
    .await
}
