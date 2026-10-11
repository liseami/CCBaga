// [INPUT]: 本机指纹引擎（优先 /Applications/CC浏览器.app，回退 ~/Applications）、配置/资料边界、真实浏览器观测、宿主界面语言与共享操作日志。
// [OUTPUT]: 浏览器设置/启动/清理与环境核验；宿主重启重新连接现有浏览器，回报时间明确标识通讯状态；上下文提供当前界面语言；launch.args 共用资料与指纹参数。
// [POS]: 独立浏览器模块的唯一编排入口；改变资料前必须退出浏览器；不自行改系统默认浏览器，只发起系统确认。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
pub(crate) mod config;
mod network;
mod profile;
pub(crate) mod server;
#[cfg(test)]
mod tests;
use crate::{
    native::decode,
    progress::{Progress, Reporter},
};
use config::{Config, Machine};
use network::{Check, Exit, Report};
use serde::{Deserialize, Serialize};
use std::{
    ffi::{c_char, CString},
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
};
use tauri::{ipc::Channel, Manager};
extern "C" {
    fn rf_browser_register(path: *const c_char) -> *mut c_char;
    fn rf_browser_app_running(bundle: *const c_char) -> *mut c_char;
    fn rf_browser_default_read(bundle: *const c_char) -> *mut c_char;
    fn rf_browser_set_default(bundle: *const c_char) -> *mut c_char;
    fn rf_system_region(language: *const c_char, locale: *const c_char) -> *mut c_char;
    fn rf_system_timezone(zone: *const c_char) -> *mut c_char;
    fn rf_system_location() -> *mut c_char;
}
#[derive(Default)]
pub struct Browser {
    gate: Mutex<()>,
    session: Mutex<Option<Session>>,
    host: Mutex<Option<Exit>>,
}
struct Session {
    token: String,
    received_at: Option<u64>,
    report: Option<Report>,
}
#[derive(Clone, Serialize)]
pub struct Context {
    ui_language: String,
    exit_services: &'static [&'static str],
    region: Option<&'static crate::regions::Region>,
    config: Config,
    machine: Machine,
    engine_version: String,
}
#[derive(Serialize)]
pub struct Snapshot {
    ui_language: String,
    exit_services: &'static [&'static str],
    region: Option<&'static crate::regions::Region>,
    config: Config,
    machine: Machine,
    engine_version: String,
    engine_ready: bool,
    running: bool,
    profile: String,
    cache: Vec<profile::CacheItem>,
    cache_error: String,
    report: Option<Report>,
    report_received_at: Option<u64>,
    machine_exit: Option<Exit>,
    checks: Vec<Check>,
}
fn root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| "应用目录不可用。")?
        .join("browser"))
}
pub const BROWSER_ID: &str = "ai.local.requirement-french.browser";
const BROWSER_APP: &str = "CC浏览器.app";
const LSREGISTER: &str = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
/// install-apps.sh 安装的 CC浏览器：优先 /Applications，回退 ~/Applications；存在时作为唯一引擎，避免同一包标识出现两份注册。
fn installed() -> Option<PathBuf> {
    let mut places = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME") {
        places.push(PathBuf::from(home).join("Applications"));
    }
    places
        .into_iter()
        .map(|dir| dir.join(BROWSER_APP))
        .find(|app| app.join("Contents/MacOS/Chromium").is_file())
}
fn register(engine: &Path) -> Result<(), String> {
    let path = CString::new(engine.to_string_lossy().as_bytes()).map_err(|_| "引擎路径无效。")?;
    let _: serde_json::Value = decode(unsafe { rf_browser_register(path.as_ptr()) })?;
    Ok(())
}
fn bundle_id() -> CString {
    CString::new(BROWSER_ID).unwrap()
}
fn app_running() -> Result<bool, String> {
    let id = bundle_id();
    let state: serde_json::Value = decode(unsafe { rf_browser_app_running(id.as_ptr()) })?;
    Ok(state["running"].as_bool().unwrap_or(false))
}
/// CC浏览器 入口（launcher/CCBrowser.c）读取的参数：与实测启动相同，只去掉起始页。
fn write_launch_args(
    root: &Path,
    config: &Config,
    machine: &Machine,
    profile: &Path,
    version: &str,
) -> Result<(), String> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let mut args = config::arguments(config, machine, profile, "", version);
    args.pop();
    fs::create_dir_all(root).map_err(|_| "无法创建浏览器目录。")?;
    let target = root.join("launch.args");
    let temp = root.join("launch.args.tmp");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&temp)
        .map_err(|_| "无法写入浏览器启动参数。")?;
    file.write_all((args.join("\n") + "\n").as_bytes())
        .map_err(|_| "无法写入浏览器启动参数。")?;
    fs::rename(&temp, &target).map_err(|_| "无法写入浏览器启动参数。")?;
    Ok(())
}
fn refresh_launch_args(app: &tauri::AppHandle) -> Result<(), String> {
    let root = root(app)?;
    let profile = root.join("profile");
    profile::check_path(&root, &profile)?;
    let config = config::load(&root)?;
    let machine = config::machine()?;
    config.validate(&machine)?;
    let engine = engine(app)?;
    write_launch_args(
        &root,
        &config,
        &machine,
        &profile,
        &packaged_version(app, &engine),
    )
}
/// 新建起始页会话：随机令牌回环服务 + 写入 browser/start.url（0600），供 CCBrowser 入口在每次全新启动时作为首个标签打开。
fn open_session(
    app: &tauri::AppHandle,
    config: &Config,
    machine: &Machine,
    engine_version: String,
) -> Result<String, String> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let state = app.state::<Browser>();
    let listener = server::bind()?;
    let port = listener
        .local_addr()
        .map_err(|_| "本地端口不可用。")?
        .port();
    let token = server::nonce()?;
    let page = format!("http://127.0.0.1:{port}/{token}/browser.html");
    let context = Context {
        ui_language: crate::ui_language::ui_language_read(app.clone()),
        exit_services: &network::SERVICES,
        region: crate::regions::matching(&config.language(machine), &config.timezone(machine)),
        config: config.clone(),
        machine: machine.clone(),
        engine_version,
    };
    let web = web(app)?;
    *state.session.lock().unwrap() = Some(Session {
        token: token.clone(),
        report: None,
        received_at: None,
    });
    server::start(app.clone(), listener, token, web, context);
    let root = root(app)?;
    fs::create_dir_all(&root).map_err(|_| "无法创建浏览器目录。")?;
    let temp = root.join("start.url.tmp");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&temp)
        .map_err(|_| "无法写入起始页地址。")?;
    file.write_all(format!("{page}\n").as_bytes())
        .map_err(|_| "无法写入起始页地址。")?;
    fs::rename(&temp, root.join("start.url")).map_err(|_| "无法写入起始页地址。")?;
    Ok(page)
}
/// 资料或配置变化使旧会话失效后，立即换新会话，保证 start.url 始终指向存活的起始页服务。
fn renew_session(app: &tauri::AppHandle) {
    let result = (|| -> Result<(), String> {
        let root = root(app)?;
        let config = config::load(&root)?;
        let machine = config::machine()?;
        config.validate(&machine)?;
        let engine = engine(app)?;
        open_session(app, &config, &machine, packaged_version(app, &engine))?;
        Ok(())
    })();
    if let Err(error) = result {
        *app.state::<Browser>().session.lock().unwrap() = None;
        forget_start_url(app);
        eprintln!("起始页会话未更新：{error}");
    }
}
/// 宿主退出时删除 start.url，避免 CCBrowser 入口读到失效令牌。
pub fn forget_start_url(app: &tauri::AppHandle) {
    if let Ok(root) = root(app) {
        let _ = fs::remove_file(root.join("start.url"));
    }
}
/// 宿主启动时建立起始页会话并带起 CC浏览器（已在运行则不动），不抢焦点。
pub fn companion(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        if let Err(error) = start_companion(&app) {
            eprintln!("CC浏览器未随宿主启动：{error}");
        }
    });
}
fn start_companion(app: &tauri::AppHandle) -> Result<(), String> {
    let browser = installed().ok_or("未安装 /Applications/CC浏览器.app。")?;
    let state = app.state::<Browser>();
    let _guard = state.gate.lock().unwrap();
    let root = root(app)?;
    let profile = root.join("profile");
    profile::check_path(&root, &profile)?;
    let config = config::load(&root)?;
    let machine = config::machine()?;
    config.validate(&machine)?;
    // 先刷新参数：浏览器已由 CC浏览器 入口先行启动时，下一次启动也能沿用指纹参数。
    write_launch_args(&root, &config, &machine, &profile, &version(&browser))?;
    let page = open_session(app, &config, &machine, version(&browser))?;
    if app_running()? || profile::running(&profile)? {
        // 宿主重装/重启后端口与令牌变化：给运行中的浏览器打开新的实测页，避免旧标签失联。
        let mut child = Command::new(browser.join("Contents/MacOS/Chromium"))
            .arg(format!("--user-data-dir={}", profile.display()))
            .args(["--profile-directory=Default", &page])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "无法重新连接运行中的 CC浏览器。")?;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        return Ok(());
    }
    profile::preferences(&profile, &config, &machine)?;
    register(&browser)?;
    let status = Command::new("/usr/bin/open")
        .args(["-g", "-a"])
        .arg(&browser)
        .status()
        .map_err(|_| "无法打开 CC浏览器。")?;
    if !status.success() {
        return Err("LaunchServices 拒绝打开 CC浏览器。".into());
    }
    Ok(())
}
#[derive(Serialize, Deserialize)]
pub struct DefaultBrowser {
    is_default: bool,
    http: String,
    https: String,
    #[serde(default)]
    installed: bool,
    #[serde(default)]
    app: String,
}
fn default_read() -> Result<DefaultBrowser, String> {
    let id = bundle_id();
    let mut value: DefaultBrowser = decode(unsafe { rf_browser_default_read(id.as_ptr()) })?;
    let browser = installed();
    value.installed = browser.is_some();
    value.app = browser
        .map(|x| x.to_string_lossy().into())
        .unwrap_or_default();
    Ok(value)
}
#[tauri::command]
pub async fn browser_default_read() -> Result<DefaultBrowser, String> {
    run(default_read).await
}
/// 一键设为默认浏览器：重新注册已安装的 CC浏览器，注销应用数据中的旧引擎副本，再请求系统切换；macOS 会弹出确认框。
#[tauri::command]
pub async fn browser_set_default(app: tauri::AppHandle) -> Result<DefaultBrowser, String> {
    run(move || {
        let browser = installed()
            .ok_or("未找到 /Applications/CC浏览器.app，请先运行 bun run install:apps。")?;
        let copy = root(&app)?.join("engine/Browser.app");
        if copy.exists() {
            let _ = Command::new(LSREGISTER).arg("-u").arg(&copy).status();
        }
        let _ = Command::new(LSREGISTER).arg("-f").arg(&browser).status();
        register(&browser)?;
        let id = bundle_id();
        let _: serde_json::Value = decode(unsafe { rf_browser_set_default(id.as_ptr()) })?;
        default_read()
    })
    .await
}
fn engine(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Some(browser) = installed() {
        return Ok(browser);
    }
    let installed = root(app)?.join("engine/Browser.app");
    if installed.exists() {
        return Ok(installed);
    }
    if cfg!(debug_assertions) {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../browser-runtime/Browser.app"));
    }
    Ok(installed)
}
fn prepare_engine(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let engine = engine(app)?;
    if engine.join("Contents/MacOS/Chromium").is_file() {
        return Ok(engine);
    }
    let parent = engine.parent().ok_or("引擎目录无效。")?;
    profile::check_path(&root(app)?, parent)?;
    fs::create_dir_all(parent).map_err(|_| "无法安装内置引擎。")?;
    let archive = app
        .path()
        .resource_dir()
        .map_err(|_| "资源目录不可用。")?
        .join("browser-runtime/Browser.zip");
    let status = Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(archive)
        .arg(parent)
        .status()
        .map_err(|_| "无法解压内置引擎。")?;
    if !status.success() || !engine.join("Contents/MacOS/Chromium").is_file() {
        return Err("内置引擎安装失败。".into());
    }
    Ok(engine)
}
fn packaged_version(app: &tauri::AppHandle, engine: &Path) -> String {
    if engine.exists() {
        return version(engine);
    }
    app.path()
        .resource_dir()
        .ok()
        .and_then(|p| fs::read_to_string(p.join("browser-runtime/version.txt")).ok())
        .unwrap_or_default()
        .trim()
        .into()
}
fn web(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let packaged = app
        .path()
        .resource_dir()
        .map_err(|_| "资源目录不可用。")?
        .join("browser-ui");
    if packaged.join("browser.html").exists() {
        return Ok(packaged);
    }
    if cfg!(debug_assertions) {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist"));
    }
    Err("浏览器起始页缺失。".into())
}
fn version(engine: &Path) -> String {
    let out = Command::new("/usr/bin/plutil")
        .args(["-extract", "CFBundleShortVersionString", "raw", "-o", "-"])
        .arg(engine.join("Contents/Info.plist"))
        .output();
    out.ok()
        .filter(|x| x.status.success())
        .map(|x| String::from_utf8_lossy(&x.stdout).trim().to_string())
        .unwrap_or_default()
}
fn snapshot(app: &tauri::AppHandle) -> Result<Snapshot, String> {
    let root = root(app)?;
    let profile = root.join("profile");
    profile::check_path(&root, &profile)?;
    let machine = config::machine()?;
    let config = config::load(&root)?;
    config.validate(&machine)?;
    let engine = engine(app)?;
    let engine_version = packaged_version(app, &engine);
    let running = profile::running(&profile)?;
    let (report, report_received_at) = app
        .state::<Browser>()
        .session
        .lock()
        .unwrap()
        .as_ref()
        .filter(|_| running)
        .map(|x| (x.report.clone(), x.received_at))
        .unwrap_or((None, None));
    let host = app.state::<Browser>().host.lock().unwrap().clone();
    let checks = network::checks(
        &machine,
        &config,
        report.as_ref(),
        host.as_ref(),
        &engine_version,
    );
    let (cache, cache_error) = match profile::scan(&profile) {
        Ok(items) => (items, String::new()),
        Err(error) => (Vec::new(), error),
    };
    Ok(Snapshot {
        ui_language: crate::ui_language::ui_language_read(app.clone()),
        exit_services: &network::SERVICES,
        region: crate::regions::matching(&config.language(&machine), &config.timezone(&machine)),
        config,
        machine,
        engine_ready: !engine_version.is_empty(),
        engine_version,
        running,
        profile: profile.to_string_lossy().into(),
        cache,
        cache_error,
        report,
        report_received_at,
        machine_exit: host,
        checks,
    })
}
async fn run<T: Send + 'static>(
    action: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(action)
        .await
        .map_err(|_| "浏览器操作被中断。".to_string())?
}
#[tauri::command]
pub async fn browser_read(app: tauri::AppHandle) -> Result<Snapshot, String> {
    run(move || snapshot(&app)).await
}
#[tauri::command]
pub async fn browser_save(app: tauri::AppHandle, config: Config) -> Result<Snapshot, String> {
    run(move || {
        let state = app.state::<Browser>();
        let _guard = state.gate.lock().unwrap();
        let root = root(&app)?;
        if profile::running(&root.join("profile"))? {
            return Err("先退出浏览器，再保存指纹设置。".into());
        }
        config.validate(&config::machine()?)?;
        config::save(&root, &config)?;
        refresh_launch_args(&app)?;
        renew_session(&app);
        snapshot(&app)
    })
    .await
}
#[tauri::command]
pub async fn browser_launch(
    app: tauri::AppHandle,
    on_progress: Channel<Progress>,
) -> Result<Snapshot, String> {
    run(move || {
        let state = app.state::<Browser>();
        let _guard = state.gate.lock().unwrap();
        let log = Reporter::channel(Some(on_progress));
        log.log(
            "browser",
            "running",
            "检查独立浏览器配置与资料占用。",
            0,
            None,
        );
        let root = root(&app)?;
        let profile = root.join("profile");
        profile::check_path(&root, &profile)?;
        let running = profile::running(&profile)?;
        let config = config::load(&root)?;
        let machine = config::machine()?;
        config.validate(&machine)?;
        let engine = prepare_engine(&app)?;
        register(&engine)?;
        let binary = engine.join("Contents/MacOS/Chromium");
        if !binary.is_file() {
            return Err("内置浏览器引擎不可用。".into());
        }
        write_launch_args(&root, &config, &machine, &profile, &version(&engine))?;
        if !running {
            profile::preferences(&profile, &config, &machine)?;
        } else {
            log.log(
                "browser",
                "running",
                "浏览器正在运行：打开 Default 实测页，沿用当前进程参数，不改写正在使用的资料。",
                0,
                None,
            );
        }
        *state.host.lock().unwrap() = None;
        let page = open_session(&app, &config, &machine, version(&engine))?;
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("engine.log"))
            .map_err(|_| "浏览器诊断日志不可用。")?;
        let mut child = Command::new(binary)
            .args(config::arguments(
                &config,
                &machine,
                &profile,
                &page,
                &version(&engine),
            ))
            .stdin(Stdio::null())
            .stdout(file.try_clone().map_err(|_| "浏览器日志不可用。")?)
            .stderr(file)
            .spawn()
            .map_err(|_| "浏览器启动失败。")?;
        for _ in 0..100 {
            if profile::running(&profile)? {
                log.log(
                    "browser",
                    "ok",
                    "浏览器已启动，等待起始页上报真实指纹与出口。",
                    0,
                    None,
                );
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                return snapshot(&app);
            }
            if child
                .try_wait()
                .map_err(|_| "无法读取浏览器状态。")?
                .is_some()
            {
                *state.session.lock().unwrap() = None;
                return Err("浏览器启动后立即退出，请检查引擎完整性。".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Err("浏览器仍在启动，请刷新状态。".into())
    })
    .await
}
#[tauri::command]
pub async fn browser_quit(app: tauri::AppHandle) -> Result<Snapshot, String> {
    run(move || {
        let state = app.state::<Browser>();
        let _guard = state.gate.lock().unwrap();
        profile::quit(&root(&app)?.join("profile"))?;
        renew_session(&app);
        snapshot(&app)
    })
    .await
}
#[tauri::command]
pub async fn browser_clear(
    app: tauri::AppHandle,
    include_site_data: bool,
    on_progress: Channel<Progress>,
) -> Result<Snapshot, String> {
    run(move || {
        let state = app.state::<Browser>();
        let _guard = state.gate.lock().unwrap();
        let profile = root(&app)?.join("profile");
        let log = Reporter::channel(Some(on_progress));
        log.log(
            "browser",
            "running",
            "核验浏览器已退出；只处理独立资料内的固定项目。",
            0,
            None,
        );
        if profile::running(&profile)? {
            return Err("浏览器正在使用资料，退出后再清理。".into());
        }
        let mut count = 0;
        let total = profile::scan(&profile)?
            .iter()
            .filter(|x| include_site_data || !x.login)
            .count();
        let result = profile::clear(&profile, include_site_data, |name, done| {
            if done {
                count += 1;
            }
            log.log(
                "browser",
                if done { "ok" } else { "running" },
                &format!("{}：{name}", if done { "已删除" } else { "正在删除" }),
                count,
                Some(total),
            );
        });
        if let Err(error) = result {
            log.fail(&error);
            return Err(error);
        }
        renew_session(&app);
        log.log(
            "browser",
            "ok",
            &format!("直接清理 {count} 个项目，不保留副本。"),
            count,
            Some(total),
        );
        snapshot(&app)
    })
    .await
}
#[tauri::command]
pub async fn browser_check(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let machine = config::machine()?;
    let host = network::probe(&machine.system_proxy).await;
    let state = app.state::<Browser>();
    *state.host.lock().unwrap() = host.as_ref().ok().cloned();
    let value = run(move || snapshot(&app)).await?;
    match host {
        Ok(_) => Ok(value),
        Err(error) => Err(error),
    }
}
#[tauri::command]
pub async fn browser_align(
    app: tauri::AppHandle,
    on_progress: Channel<Progress>,
) -> Result<Snapshot, String> {
    let config = config::load(&root(&app)?)?;
    let exit = network::probe(&config.proxy).await?;
    if exit.country != "US" {
        return Err("实际代理出口不是美国，未修改机器。".into());
    }
    run(move || {
        let state = app.state::<Browser>();
        let _guard = state.gate.lock().unwrap();
        let log = Reporter::channel(Some(on_progress));
        if profile::running(&root(&app)?.join("profile"))? {
            return Err("先退出浏览器，再对齐机器。".into());
        }
        let machine = config::machine()?;
        if !machine.timezones.contains(&exit.timezone) {
            return Err("出口未返回可用时区，未修改机器。".into());
        }
        log.log(
            "browser",
            "running",
            "设置实际美国出口对应的时区，请完成系统管理员授权。",
            0,
            None,
        );
        let zone = std::ffi::CString::new(exit.timezone.clone()).map_err(|_| "出口时区无效。")?;
        let _: serde_json::Value = decode(unsafe { rf_system_timezone(zone.as_ptr()) })?;
        log.log("browser", "ok", "系统时区已核验。", 1, Some(3));
        let _: serde_json::Value = decode(unsafe {
            rf_system_region(b"en-US\0".as_ptr().cast(), b"en_US\0".as_ptr().cast())
        })?;
        log.log("browser", "ok", "美国英语与 en_US 区域已保存。", 2, Some(3));
        if config::machine()?.location_enabled {
            let _: serde_json::Value = decode(unsafe { rf_system_location() })?;
        }
        let fresh = config::machine()?;
        if fresh.location_enabled
            || fresh.timezone != exit.timezone
            || !fresh.locale.starts_with("en_US")
            || fresh.languages.first().map(String::as_str) != Some("en-US")
        {
            return Err("部分系统设置未能核验，请查看电脑设置。".into());
        }
        let mut config = config;
        config.follow_machine = true;
        config.language = "en-US".into();
        config.timezone = exit.timezone;
        config::save(&root(&app)?, &config)?;
        refresh_launch_args(&app)?;
        renew_session(&app);
        log.log(
            "browser",
            "ok",
            "机器与实际美国出口对齐完成；重新打开浏览器核验。",
            3,
            Some(3),
        );
        snapshot(&app)
    })
    .await
}

/// 快捷换指纹：正常退出后更新种子与选定地区，网络跟随 TUN；旧观测会话失效。
#[tauri::command]
pub async fn browser_rotate(app: tauri::AppHandle, region: String) -> Result<Snapshot, String> {
    let region = crate::regions::find(&region)?;
    run(move || {
        let state = app.state::<Browser>();
        let _guard = state.gate.lock().unwrap();
        let root = root(&app)?;
        let profile = root.join("profile");
        profile::check_path(&root, &profile)?;
        profile::quit(&profile)?;
        let mut config = config::load(&root)?;
        let old = config.seed;
        while config.seed == old || config.seed == 0 {
            let nonce = server::nonce()?;
            config.seed = u32::from_str_radix(&nonce[..8], 16).map_err(|_| "无法生成指纹种子。")?;
        }
        config.follow_machine = false;
        config.language = region.language.into();
        config.timezone = region.timezone.into();
        config.proxy.clear();
        // ----- 种子必须作用于 Canvas/布局，真实设备参数继续沿用 -----
        config.native_graphics = false;
        config.validate(&config::machine()?)?;
        config::save(&root, &config)?;
        refresh_launch_args(&app)?;
        *state.host.lock().unwrap() = None;
        renew_session(&app);
        snapshot(&app)
    })
    .await
}
