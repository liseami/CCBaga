// [INPUT]: 应用数据目录和显式的 zh/en 界面偏好。
// [OUTPUT]: 语言读写命令及回环服务共享的当前语言；原子保存后再发布内存状态。
// [POS]: 界面语言边界，独立于系统语言、浏览器指纹和地区设置。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use std::{fs, sync::Mutex};
use tauri::Manager;
pub struct UiLanguage(pub Mutex<String>);
pub fn valid(language: &str) -> bool {
    matches!(language, "zh" | "en")
}
impl UiLanguage {
    pub fn load(app: &tauri::AppHandle) -> Self {
        let stored = app
            .path()
            .app_data_dir()
            .ok()
            .and_then(|dir| fs::read_to_string(dir.join("ui-language")).ok())
            .filter(|language| valid(language));
        Self(Mutex::new(stored.unwrap_or_else(|| "zh".into())))
    }
}
#[tauri::command]
pub fn ui_language_read(app: tauri::AppHandle) -> String {
    app.state::<UiLanguage>().0.lock().unwrap().clone()
}
#[tauri::command]
pub fn ui_language_write(app: tauri::AppHandle, language: String) -> Result<(), String> {
    if !valid(&language) {
        return Err("不支持的界面语言。".into());
    }
    let state = app.state::<UiLanguage>();
    let mut current = state.0.lock().unwrap();
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "无法保存界面语言。")?;
    let result = (|| -> std::io::Result<()> {
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("ui-language.tmp"), &language)?;
        fs::rename(dir.join("ui-language.tmp"), dir.join("ui-language"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(dir.join("ui-language.tmp"));
        return Err("无法保存界面语言。".into());
    }
    *current = language.clone();
    drop(current);
    if let (Some(tray), Ok(menu)) = (app.tray_by_id("main"), menu(&app, &language)) {
        let _ = tray.set_menu(Some(menu));
    }
    Ok(())
}
pub fn menu(
    app: &tauri::AppHandle,
    language: &str,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem};
    let open = MenuItem::with_id(
        app,
        "open",
        if language == "en" {
            "Open CC Cleaner"
        } else {
            "打开 CC清理器"
        },
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        if language == "en" { "Quit" } else { "退出" },
        true,
        None::<&str>,
    )?;
    Menu::with_items(app, &[&open, &quit])
}
#[cfg(test)]
mod tests {
    #[test]
    fn locale_is_independent_and_restricted() {
        assert!(super::valid("zh"));
        assert!(super::valid("en"));
        for value in ["", "en-US", "zh-Hant", "../en", "fr"] {
            assert!(!super::valid(value));
        }
    }
}
