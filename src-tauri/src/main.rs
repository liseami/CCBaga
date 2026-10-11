// [INPUT]: Tauri 生命周期、发布构建 custom-protocol 与清理、CLI 安装管理、浏览器、地区、系统、VPS 三文件密钥选择与独立界面语言命令。
// [OUTPUT]: CC清理器窗口、托盘、本地清理和 CLI 卸载/重装、SSH 远程设置入口与随界面语言切换的托盘菜单。
// [POS]: 应用编排层；不注册翻译快捷键，不读取翻译设置或 API 密钥。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
// ----- 发布包必须内嵌前端资源，禁止依赖开发服务 -----
#[cfg(all(not(debug_assertions), not(feature = "custom-protocol")))]
compile_error!("发布构建需要 custom-protocol；使用 tauri build 或 cargo build --release --features custom-protocol。");
mod browser;
mod claude_cli;
mod cleaner;
mod native;
mod progress;
mod regions;
mod system;
mod ui_language;
mod vps;
mod vps_keys;
use tauri::{tray::TrayIconBuilder, Manager};
fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
#[tauri::command]
fn open_permissions(app: tauri::AppHandle) -> Result<(), String> {
    app.run_on_main_thread(native::permissions)
        .map_err(|_| "无法打开系统权限设置。".into())
}
#[tauri::command]
async fn unlock_keychain() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(native::unlock_keychain)
        .await
        .map_err(|_| "钥匙串授权被中断。".to_string())?
}
#[tauri::command]
async fn copy_result(app: tauri::AppHandle, text: String) -> Result<(), String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = send.send(native::copy(&text));
    })
    .map_err(|_| "复制失败。")?;
    receive.await.map_err(|_| "复制失败。")?
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .setup(|app| {
            app.manage(ui_language::UiLanguage::load(app.handle()));
            app.manage(cleaner::Cleaner::default());
            app.manage(browser::Browser::default());
            app.manage(vps::Vps::default());
            browser::companion(app.handle().clone());
            let menu = ui_language::menu(
                app.handle(),
                &ui_language::ui_language_read(app.handle().clone()),
            )?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("CC")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => show(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            ui_language::ui_language_read,
            ui_language::ui_language_write,
            open_permissions,
            unlock_keychain,
            copy_result,
            cleaner::cleaner_scan,
            cleaner::cleaner_stop,
            cleaner::cleaner_processes,
            cleaner::cleaner_clean,
            claude_cli::claude_cli_read,
            claude_cli::claude_cli_manage,
            regions::get_regions,
            system::system_read,
            system::system_set_timezone,
            system::system_set_language,
            system::system_disable_location,
            system::system_open_settings,
            system::system_set_region,
            browser::browser_read,
            browser::browser_save,
            browser::browser_launch,
            browser::browser_quit,
            browser::browser_clear,
            browser::browser_check,
            browser::browser_align,
            browser::browser_default_read,
            browser::browser_set_default,
            browser::browser_rotate,
            vps::vps_connect,
            vps::vps_setup,
            vps::vps_choose_keys,
            vps::vps_open_provider
        ])
        .build(tauri::generate_context!())
        .expect("无法启动 CC清理器")
        .run(|app, event| match event {
            tauri::RunEvent::Reopen { .. } => show(app),
            tauri::RunEvent::Exit => browser::forget_start_url(app),
            _ => {}
        });
}
