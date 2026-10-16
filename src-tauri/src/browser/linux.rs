// [INPUT]: libexec 隔离的 Linux Chromium、兼容源码/Arch 资源目录、XDG 桌面入口与共用资料/随机令牌会话。
// [OUTPUT]: 本地引擎发现与启动、用户主动设置默认浏览器。
// [POS]: browser 编排的 Linux 平台实现；只接收 URL/文件入口，不执行用户提供的 shell。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
fn register(_: &Path) -> Result<(), String> {
    Ok(())
}
fn binary(engine: &Path) -> PathBuf {
    engine.join("chrome")
}
fn packaged_engine(resources: &Path) -> PathBuf {
    let legacy = resources.join("browser-runtime/LinuxBrowser");
    if binary(&legacy).is_file() {
        return legacy;
    }
    resources.join("../../libexec/CCBaga-B")
}
fn engine(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let resources = app.path().resource_dir().map_err(|_| "资源目录不可用。")?;
    let packaged = packaged_engine(&resources);
    if binary(&packaged).is_file() || !cfg!(debug_assertions) {
        return Ok(packaged);
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../browser-runtime/LinuxBrowser"))
}
#[test]
fn packaged_browser_paths_preserve_both_layouts() {
    let root = tests::sample();
    let resources = root.join("usr/lib/CCBaga-A");
    fs::create_dir_all(&resources).unwrap();
    let isolated = root.join("usr/libexec/CCBaga-B");
    fs::create_dir_all(&isolated).unwrap();
    fs::write(binary(&isolated), "isolated engine").unwrap();
    assert_eq!(
        packaged_engine(&resources).canonicalize().unwrap(),
        isolated
    );
    let legacy = resources.join("browser-runtime/LinuxBrowser");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(binary(&legacy), "source/Arch engine").unwrap();
    assert_eq!(packaged_engine(&resources), legacy);
    fs::remove_dir_all(root).unwrap();
}
fn prepare_engine(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let engine = engine(app)?;
    if !binary(&engine).is_file() {
        return Err(
            "Linux browser engine missing. Run python3 scripts/fetch-browser.py before building."
                .into(),
        );
    }
    Ok(engine)
}
fn version(engine: &Path) -> String {
    fs::read_to_string(engine.join("version.txt"))
        .unwrap_or_default()
        .trim()
        .into()
}
pub fn open_urls(app: tauri::AppHandle, args: Vec<String>) {
    let urls = args
        .into_iter()
        .skip(1)
        .filter(|s| {
            s.starts_with("https://")
                || s.starts_with("http://")
                || s.starts_with("file://")
                || Path::new(s).is_absolute()
        })
        .collect::<Vec<_>>();
    std::thread::spawn(move || {
        if let Err(error) = start_linux_companion(&app, &urls) {
            eprintln!("CCBaga-B: {error}");
        }
    });
}
fn start_linux_companion(app: &tauri::AppHandle, urls: &[String]) -> Result<(), String> {
    let state = app.state::<Browser>();
    let _guard = state.gate.lock().unwrap();
    let engine = prepare_engine(app)?;
    let root = root(app)?;
    let settings = workspaces::ensure(&root)?;
    let profile = settings.join("profile");
    profile::check_path(&root, &profile)?;
    let config = config::load(&settings)?;
    let machine = config::machine()?;
    config.validate(&machine)?;
    let running = profile::running(&profile)?;
    if !running {
        profile::preferences(&profile, &config, &machine)?;
    }
    write_launch_args(&root, &config, &machine, &profile, &version(&engine))?;
    workspaces::unblock(&root)?;
    let reuse_running = !urls.is_empty() && running && state.session.lock().unwrap().is_some();
    let mut cmd_args = if reuse_running {
        let mut a = config::arguments(&config, &machine, &profile, "", &version(&engine));
        a.pop();
        a
    } else {
        let page = open_session(app, &config, &machine, version(&engine))?;
        config::arguments(&config, &machine, &profile, &page, &version(&engine))
    };
    cmd_args.extend_from_slice(urls);
    let mut child = Command::new(binary(&engine))
        .args(cmd_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            "Could not launch Linux Chromium. Check engine permissions and runtime libraries."
                .to_string()
        })?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
const LINUX_DESKTOP_ID: &str = "ccbaga-browser.desktop";
fn default_read() -> Result<DefaultBrowser, String> {
    let query = |scheme| -> Result<String, String> {
        let out = Command::new("xdg-mime")
            .args(["query", "default", scheme])
            .output()
            .map_err(|_| "Install xdg-utils to manage the default browser.")?;
        if !out.status.success() {
            return Err("Cannot query the desktop's default browser.".into());
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().into())
    };
    let http = query("x-scheme-handler/http")?;
    let https = query("x-scheme-handler/https")?;
    Ok(DefaultBrowser {
        is_default: http == LINUX_DESKTOP_ID && https == LINUX_DESKTOP_ID,
        http,
        https,
        installed: true,
        app: std::env::current_exe()
            .map_err(|_| "Cannot locate CCBaga-A.")?
            .display()
            .to_string(),
    })
}
// Desktop Entry Exec has two quoting layers (string-value escapes around the Exec double-quote
// rules), so each reserved character is prefixed by an escaped backslash: `"` becomes two backslashes plus the quote.
fn desktop_exec_escape(path: &str) -> Result<String, String> {
    if path.contains(['\n', '\r']) {
        return Err("Invalid application path.".into());
    }
    Ok(path
        .replace('\\', "\\\\\\\\")
        .replace('"', "\\\\\"")
        .replace('`', "\\\\`")
        .replace('$', "\\\\$")
        .replace('%', "%%"))
}
#[test]
fn desktop_exec_escapes_quotes_with_one_exec_layer_escape() {
    assert_eq!(desktop_exec_escape(r#"/opt/say "hi"/app"#).unwrap(), r#"/opt/say \\"hi\\"/app"#);
    assert_eq!(desktop_exec_escape(r"/opt/a\b/app").unwrap(), r"/opt/a\\\\b/app");
    assert_eq!(desktop_exec_escape("/opt/`x`/$HOME/100%").unwrap(), r"/opt/\\`x\\`/\\$HOME/100%%");
    assert!(desktop_exec_escape("/opt/a\nb").is_err());
}
#[tauri::command]
pub async fn browser_set_default(app: tauri::AppHandle) -> Result<DefaultBrowser, String> {
    run(move || {
        prepare_engine(&app)?;
        let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share"))).ok_or("Cannot locate XDG data directory.")?;
        let desktop = data.join("applications").join(LINUX_DESKTOP_ID);
        profile::check_path(&data, &desktop)?;
        fs::create_dir_all(desktop.parent().unwrap()).map_err(|_| "Cannot create applications directory.")?;
        // AppImage extraction paths are temporary: register the persistent AppImage when available.
        let executable = std::env::var_os("APPIMAGE").map(PathBuf::from).unwrap_or(std::env::current_exe().map_err(|_| "Cannot locate CCBaga-A.")?);
        let escaped = desktop_exec_escape(&executable.to_string_lossy())?;
        fs::write(&desktop, format!("[Desktop Entry]\nType=Application\nName=CCBaga-B\nExec=\"{escaped}\" --browser %U\nIcon=ccbaga-browser\nTerminal=false\nCategories=Network;WebBrowser;\nMimeType=x-scheme-handler/http;x-scheme-handler/https;text/html;\n")).map_err(|_| "Cannot register CCBaga-B desktop entry.")?;
        for scheme in ["x-scheme-handler/http", "x-scheme-handler/https"] {
            let status = Command::new("xdg-mime").args(["default", LINUX_DESKTOP_ID, scheme]).status().map_err(|_| "Install xdg-utils to manage default applications.")?;
            if !status.success() { return Err("Default browser update failed; check desktop settings.".into()); }
        }
        let state = default_read()?;
        if !state.is_default { return Err("Desktop did not confirm CCBaga-B as the default browser.".into()); }
        Ok(state)
    }).await
}
