// [INPUT]: 临时样本、指纹参数、固定清理边界与本机 HTTP 限流样本。
// [OUTPUT]: 缓存/旧站点清理保留资产，显式完整重置同时清除密码与密钥、全部子 Profile 并隔离兄弟资料；链接/进程重启阻断与真实删除计数；含方案默认端口的代理与地区/网络回归。
// [POS]: 浏览器模块行为验证；不访问用户实际资料。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
pub(super) fn machine() -> Machine {
    Machine {
        platform: "macos".into(),
        version: "27.2.0".into(),
        cores: 10,
        timezone: "America/New_York".into(),
        languages: vec!["en-US".into()],
        locale: "en_US".into(),
        locale_country: "US".into(),
        location_enabled: Some(false),
        timezones: vec!["America/New_York".into()],
        system_proxy: String::new(),
    }
}
pub(super) fn sample() -> PathBuf {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("rf-browser-{}", server::nonce().unwrap()));
    fs::create_dir(&root).unwrap();
    root
}
#[test]
fn proxy_and_real_hardware() {
    let mut c = Config::default();
    for bad in [
        "http://a:8080/--disable-web-security",
        "http://user:pass@a:8080",
        "file:///tmp/a",
        "http://a:8080?q=x",
    ] {
        assert!(config::proxy(bad).is_err());
    }
    c.proxy = "socks5h://127.0.0.1:7898".into();
    let a = config::arguments(
        &c,
        &machine(),
        Path::new("/profile"),
        "http://127.0.0.1/page",
        "148.0.7778.215",
    );
    assert!(a.contains(&"--fingerprint-brand-version=148.0.7778.215".into()));
    assert!(a.contains(&"--profile-directory=Default".into()));
    assert!(a.contains(&"--fingerprint-platform-version=27.2.0".into()));
    assert!(a.contains(&"--fingerprint-hardware-concurrency=10".into()));
    assert!(a.contains(&"--proxy-bypass-list=127.0.0.1;localhost;[::1]".into()));
    assert!(!a.iter().any(|argument| argument.contains("<-loopback>")));
    assert!(a.iter().any(|x| x.starts_with("--host-resolver-rules=")));
    c.proxy = "socks5://[::1]:7898".into();
    let ipv6_args = config::arguments(
        &c,
        &machine(),
        Path::new("/profile"),
        "http://127.0.0.1/page",
        "148.0.7778.215",
    );
    assert!(ipv6_args
        .iter()
        .any(|x| x.contains("EXCLUDE ::1") && !x.contains("EXCLUDE [::1]")));
    assert!(!a
        .iter()
        .any(|x| x.contains("load-extension") || x.contains("disable-web-security")));
}
#[test]
fn explicit_scheme_default_proxy_port_counts_as_specified() {
    for value in [
        "http://127.0.0.1:80",
        "https://proxy.example:443",
        "http://[::1]:80",
        "https://[::1]:443/",
    ] {
        let url = config::proxy(value)
            .unwrap_or_else(|error| panic!("{value}: {error}"))
            .expect(value);
        assert_eq!(url.port_or_known_default().is_some(), true, "{value}");
        assert!(url.port().is_none(), "{value}");
    }
    for value in [
        "http://127.0.0.1",
        "https://proxy.example",
        "http://[::1]",
        "https://[::1]",
        "socks5://127.0.0.1",
        "socks5h://127.0.0.1",
    ] {
        assert!(config::proxy(value).is_err(), "{value}");
    }
    let mut config = Config::default();
    config.proxy = "http://127.0.0.1:80".into();
    let args = config::arguments(
        &config,
        &machine(),
        Path::new("/profile"),
        "http://127.0.0.1/page",
        "148.0.7778.215",
    );
    assert!(args.contains(&"--proxy-server=http://127.0.0.1:80".into()));
    assert!(!args
        .iter()
        .any(|argument| argument.starts_with("--host-resolver-rules=")));
}
#[test]
fn cache_isolation_and_direct_delete() {
    let root = sample();
    for name in [
        "Default/Cache/data",
        "Default/Network/Cookies",
        "Default/Bookmarks",
        "Default/Login Data",
        "Local State",
        "Default/Preferences",
        "Default/Web Data",
        "Default/History",
    ] {
        let p = root.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b"fixture").unwrap();
    }
    let mut log = Vec::new();
    assert_eq!(
        profile::clear(&root, false, |n, d| log.push((n.to_string(), d))).unwrap(),
        1
    );
    assert_eq!(log.len(), 2);
    assert!(!root.join("Default/Cache").exists());
    assert!(root.join("Default/Network/Cookies").exists());
    profile::clear(&root, true, |_, _| {}).unwrap();
    assert!(!root.join("Default/Network/Cookies").exists());
    assert!(root.join("Default/Bookmarks").exists());
    assert!(root.join("Default/Login Data").exists());
    for name in [
        "Local State",
        "Default/Preferences",
        "Default/Web Data",
        "Default/History",
    ] {
        assert_eq!(
            fs::read(root.join(name)).unwrap(),
            b"fixture",
            "preserve {name}"
        );
    }
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn explicit_profile_reset_clears_all_profiles_keys_and_assets_but_nothing_outside() {
    let root = sample();
    let active = root.join("browser/contexts/a/profile");
    let files = [
        "Local State",
        "Default/Login Data",
        "Default/Login Data For Account",
        "Default/Bookmarks",
        "Default/History",
        "Default/Preferences",
        "Default/Web Data",
        "Default/Network/Cookies",
        "Default/Extensions/example/data",
        "Profile 1/Login Data",
        "Profile 1/Bookmarks",
        "Guest Profile/Local Storage/value",
        "unknown-provider/credential",
        "unknown-root-file",
    ];
    for file in files {
        let path = active.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"synthetic profile value").unwrap();
    }
    let outside = [
        "browser/contexts/b/profile/Default/Login Data",
        "browser/contexts/a/config.json",
        "browser/contexts/a/seed.owner",
        "identity-vault/key.bin",
        "browser-runtime/chrome",
        "login.keychain-db",
    ];
    for file in outside {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"keep synthetic external value").unwrap();
    }
    let mut events = Vec::new();
    let count = profile::reset(
        &active,
        || Ok(()),
        |name, done, current, total| events.push((name.to_owned(), done, current, total)),
    )
    .unwrap();
    assert_eq!(fs::read_dir(&active).unwrap().count(), 0);
    assert_eq!(events.len(), count * 2);
    for (index, pair) in events.chunks_exact(2).enumerate() {
        assert_eq!((pair[0].1, pair[0].2, pair[0].3), (false, index, count));
        assert_eq!((pair[1].1, pair[1].2, pair[1].3), (true, index + 1, count));
        assert_eq!(pair[0].0, pair[1].0);
    }
    for file in outside {
        assert_eq!(
            fs::read(root.join(file)).unwrap(),
            b"keep synthetic external value"
        );
    }
    // 下一次启动的既有偏好层可重建空资料；没有恢复旧密码或书签。
    profile::preferences(&active, &Config::default(), &machine()).unwrap();
    assert!(active.join("Default/Preferences").is_file());
    assert!(!active.join("Default/Login Data").exists());
    assert!(!active.join("Default/Bookmarks").exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn profile_reset_preflights_every_link_before_deleting_any_file() {
    let root = sample();
    let active = root.join("profile");
    let outside = root.join("external");
    fs::create_dir_all(active.join("Default/Cache")).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(
        active.join("Default/Cache/data"),
        b"keep before failed preflight",
    )
    .unwrap();
    fs::write(outside.join("keychain"), b"synthetic external keychain").unwrap();
    let link = active.join("linked-profile");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    #[cfg(windows)]
    assert!(Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(link.to_string_lossy().replace('/', "\\"))
        .arg(outside.to_string_lossy().replace('/', "\\"))
        .output()
        .unwrap()
        .status
        .success());
    let mut events = Vec::new();
    assert!(profile::reset(
        &active,
        || Ok(()),
        |name, _, _, _| events.push(name.to_owned())
    )
    .is_err());
    assert!(events.is_empty());
    assert!(active.join("Default/Cache/data").exists());
    assert_eq!(
        fs::read(outside.join("keychain")).unwrap(),
        b"synthetic external keychain"
    );
    #[cfg(unix)]
    fs::remove_file(link).unwrap();
    #[cfg(windows)]
    fs::remove_dir(link).unwrap();
    fs::remove_dir_all(root).unwrap();
}
#[cfg(unix)]
#[test]
fn profile_reset_unlinks_only_owned_root_chromium_crash_locks() {
    let root = sample();
    let active = root.join("profile");
    let outside = root.join("outside");
    fs::create_dir(&active).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), b"external fixture").unwrap();
    for name in ["SingletonLock", "SingletonCookie", "SingletonSocket"] {
        std::os::unix::fs::symlink(&outside, active.join(name)).unwrap();
    }
    assert_eq!(
        profile::reset(&active, || Ok(()), |_, _, _, _| {}).unwrap(),
        3
    );
    assert_eq!(fs::read_dir(&active).unwrap().count(), 0);
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"external fixture");
    // 相同名称放在子目录中也不能获得根锁例外。
    fs::create_dir(active.join("Default")).unwrap();
    std::os::unix::fs::symlink(&outside, active.join("Default/SingletonLock")).unwrap();
    assert!(profile::reset(&active, || Ok(()), |_, _, _, _| {}).is_err());
    fs::remove_file(active.join("Default/SingletonLock")).unwrap();
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn profile_reset_reports_partial_deletion_and_stops_when_browser_restarts() {
    let root = sample();
    fs::write(root.join("a"), b"first").unwrap();
    fs::write(root.join("b"), b"second").unwrap();
    assert!(profile::reset(
        &root,
        || Err("fixture process running".into()),
        |_, _, _, _| panic!("no deletion before process check")
    )
    .is_err());
    assert!(root.join("a").exists());
    let mut checks = 0;
    let mut completed = Vec::new();
    let error = profile::reset(
        &root,
        || {
            checks += 1;
            if checks >= 3 {
                Err("fixture browser restarted".into())
            } else {
                Ok(())
            }
        },
        |_, done, count, total| {
            if done {
                completed.push((count, total));
                std::thread::sleep(std::time::Duration::from_millis(260));
            }
        },
    )
    .unwrap_err();
    assert!(error.contains("1/2"));
    assert_eq!(completed, [(1, 2)]);
    assert!(!root.join("a").exists());
    assert!(root.join("b").exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn reject_links_and_malformed_preferences() {
    let root = sample();
    let outside = sample();
    fs::write(outside.join("keep"), b"keep").unwrap();
    fs::create_dir(root.join("Default")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, root.join("Default/Cache")).unwrap();
    #[cfg(windows)]
    assert!(Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(
            root.join("Default/Cache")
                .to_string_lossy()
                .replace('/', "\\")
        )
        .arg(outside.to_string_lossy().replace('/', "\\"))
        .output()
        .unwrap()
        .status
        .success());
    assert!(profile::clear(&root, false, |_, _| {}).is_err());
    assert!(outside.join("keep").exists());
    let prefs = root.join("Default/Preferences");
    fs::write(&prefs, b"{\"intl\":\"wrong\"}").unwrap();
    assert!(profile::preferences(&root, &Config::default(), &machine()).is_err());
    assert_eq!(fs::read(&prefs).unwrap(), b"{\"intl\":\"wrong\"}");
    assert!(server::resource(&root, "../keep").is_none());
    assert!(server::resource(&root, "Default/Cache/keep").is_none());
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}
#[test]
fn preferences_preserve_assets_and_block_both_geolocation_modes() {
    let root = sample();
    fs::create_dir(root.join("Default")).unwrap();
    fs::write(root.join("Default/Preferences"),br#"{"bookmarks":{"keep":true},"profile":{"content_settings":{"exceptions":{"geolocation_with_options":{"site":{"setting":1}}}}}}"#).unwrap();
    profile::preferences(&root, &Config::default(), &machine()).unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("Default/Preferences")).unwrap()).unwrap();
    assert_eq!(value["bookmarks"]["keep"], true);
    assert_eq!(
        value["profile"]["default_content_setting_values"]["geolocation_with_options"],
        serde_json::json!({"approximate":2,"precise":2})
    );
    assert_eq!(
        value["profile"]["content_settings"]["exceptions"]["geolocation_with_options"],
        serde_json::json!({})
    );
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn observations_required() {
    let checks = network::checks(&machine(), &Config::default(), None, None, "148");
    assert_eq!(checks.iter().filter(|x| x.status == "pending").count(), 9);
    assert!(
        network::parse(&serde_json::json!({"success":true,"ip":"fake","country_code":"US"}))
            .is_err()
    );
}
#[test]
fn country_checks_follow_region_instead_of_assuming_america() {
    let mut machine = machine();
    machine.timezone = "Asia/Tokyo".into();
    machine.languages = vec!["ja".into()];
    machine.locale = "ja_JP".into();
    machine.locale_country = "JP".into();
    let mut config = Config::default();
    config.follow_machine = false;
    config.language = "ja".into();
    config.timezone = "Asia/Tokyo".into();
    let mut report = network::Report {
        language: "ja".into(),
        languages: vec!["ja".into()],
        timezone: "Asia/Tokyo".into(),
        platform: "MacIntel".into(),
        cores: 10,
        user_agent: "fixture".into(),
        platform_version: machine.version.clone(),
        browser_version: "148".into(),
        geolocation: "denied".into(),
        error: String::new(),
        exit: Some(network::Exit {
            ip: "203.0.113.1".into(),
            country: "JP".into(),
            city: "Tokyo".into(),
            timezone: "Asia/Tokyo".into(),
            isp: "fixture".into(),
        }),
    };
    let checks = network::checks(&machine, &config, Some(&report), None, "148");
    assert_eq!(checks[0].status, "ok");
    assert_eq!(checks[3].status, "ok");
    report.exit.as_mut().unwrap().country = "US".into();
    assert_eq!(
        network::checks(&machine, &config, Some(&report), None, "148")[0].status,
        "mismatch"
    );
    assert_eq!(
        crate::regions::matching("ja", "Europe/Paris").map(|r| r.country),
        None
    );
}

#[test]
fn rate_limited_service_falls_back_without_changing_route() {
    use std::io::{BufRead, BufReader, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        for status in ["429 Too Many Requests", "200 OK"] {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
            }
            let body = r#"{"ip":"203.0.113.2","country_code":"JP","timezone":"Asia/Tokyo","org":"fixture"}"#;
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    let urls = [
        format!("http://{address}/primary"),
        format!("http://{address}/backup"),
    ];
    let exit =
        tauri::async_runtime::block_on(network::probe_urls("", &[&urls[0], &urls[1]])).unwrap();
    assert_eq!(exit.ip, "203.0.113.2");
    assert_eq!(exit.timezone, "Asia/Tokyo");
    assert!(network::parse(
        &serde_json::json!({"error":true,"ip":"203.0.113.2","country_code":"US"})
    )
    .is_err());
    server.join().unwrap();
}

#[test]
fn linux_platform_and_unknown_location_cannot_be_misreported_as_mac_or_disabled() {
    let mut m = machine();
    m.platform = "linux".into();
    m.version = "6.18.0".into();
    m.location_enabled = None;
    let args = config::arguments(
        &Config::default(),
        &m,
        Path::new("/profile"),
        "http://localhost",
        "148",
    );
    assert!(args.contains(&"--fingerprint-platform=linux".into()));
    let report = network::Report {
        language: "en-US".into(),
        languages: vec!["en-US".into()],
        timezone: m.timezone.clone(),
        platform: "Linux x86_64".into(),
        cores: m.cores,
        user_agent: "fixture".into(),
        platform_version: String::new(),
        browser_version: "148".into(),
        geolocation: "denied".into(),
        error: String::new(),
        exit: None,
    };
    let checks = network::checks(&m, &Config::default(), Some(&report), None, "148");
    assert_eq!(
        checks
            .iter()
            .find(|c| c.title == "真实设备与 CPU")
            .unwrap()
            .status,
        "ok"
    );
    assert_ne!(
        checks
            .iter()
            .find(|c| c.title == "地理位置关闭")
            .unwrap()
            .status,
        "ok"
    );
}

#[test]
fn invalid_or_unreachable_proxy_never_falls_back_to_direct() {
    let origin = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    origin.set_nonblocking(true).unwrap();
    let url = format!("http://{}/", origin.local_addr().unwrap());
    let unused = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let unavailable_proxy = format!("http://{}", unused.local_addr().unwrap());
    drop(unused);
    for proxy in ["http://[invalid", unavailable_proxy.as_str()] {
        assert!(tauri::async_runtime::block_on(network::probe_urls(proxy, &[&url])).is_err());
        assert_eq!(
            origin.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[test]
fn distinct_iana_timezones_remain_conflicts_on_every_platform() {
    let mut machine = machine();
    machine.timezone = "Europe/Berlin".into();
    machine.languages = vec!["de".into()];
    machine.locale_country = "DE".into();
    let report = network::Report {
        language: "de".into(),
        languages: machine.languages.clone(),
        timezone: "Europe/Paris".into(),
        platform: "MacIntel".into(),
        cores: machine.cores,
        user_agent: "fixture".into(),
        platform_version: machine.version.clone(),
        browser_version: "148".into(),
        geolocation: "denied".into(),
        error: String::new(),
        exit: Some(network::Exit {
            ip: "203.0.113.2".into(),
            country: "DE".into(),
            city: String::new(),
            timezone: "Europe/Berlin".into(),
            isp: String::new(),
        }),
    };
    let checks = network::checks(&machine, &Config::default(), Some(&report), None, "148");
    assert_eq!(
        checks
            .iter()
            .find(|check| check.title == "时区一致")
            .unwrap()
            .status,
        "mismatch"
    );
}

#[test]
fn machine_proxy_credentials_never_leave_native_code() {
    let mut m = machine();
    m.system_proxy = "http://private-user:private-password@localhost:8080".into();
    let value = serde_json::to_string(&m).unwrap();
    assert!(!value.contains("private-password"));
    assert!(!value.contains("system_proxy"));
}

#[test]
fn windows_report_requires_real_platform_cpu_and_version() {
    let mut m = machine();
    m.platform = "windows".into();
    m.version = "10.0.26100".into();
    let mut report = network::Report {
        language: "en-US".into(),
        languages: vec!["en-US".into()],
        timezone: m.timezone.clone(),
        platform: "Win32".into(),
        cores: m.cores,
        user_agent: "fixture".into(),
        platform_version: m.version.clone(),
        browser_version: "148".into(),
        geolocation: "denied".into(),
        error: String::new(),
        exit: None,
    };
    let result = |r: &network::Report| {
        network::checks(&m, &Config::default(), Some(r), None, "148")
            .into_iter()
            .find(|c| c.title == "真实设备与 CPU")
            .unwrap()
            .status
    };
    assert_eq!(result(&report), "ok");
    report.platform = "MacIntel".into();
    assert_eq!(result(&report), "mismatch");
    report.platform = "Win32".into();
    report.platform_version = "fake".into();
    assert_eq!(result(&report), "mismatch");
}

#[test]
fn follow_machine_taiwan_preserves_language_script_and_checks_actual_country() {
    let mut machine = machine();
    machine.timezone = "Asia/Taipei".into();
    machine.languages = vec!["zh-Hans-TW".into(), "zh-Hant-TW".into()];
    machine.locale = "zh_CN@rg=twzzzz".into();
    machine.locale_country = "TW".into();
    let config = Config::default();
    assert_eq!(config.region(&machine).map(|r| r.id), Some("tw"));
    assert_eq!(config.language(&machine), "zh-Hans-TW");
    let mut report = network::Report {
        language: "zh-Hans-TW".into(),
        languages: machine.languages.clone(),
        timezone: machine.timezone.clone(),
        platform: "MacIntel".into(),
        cores: machine.cores,
        user_agent: "fixture".into(),
        platform_version: machine.version.clone(),
        browser_version: "148".into(),
        geolocation: "denied".into(),
        error: String::new(),
        exit: Some(network::Exit {
            ip: "203.0.113.1".into(),
            country: "TW".into(),
            city: "Taipei".into(),
            timezone: machine.timezone.clone(),
            isp: "fixture".into(),
        }),
    };
    let checks = network::checks(&machine, &config, Some(&report), None, "148");
    assert_eq!(checks[0].status, "ok");
    assert_eq!(checks[3].status, "ok");
    report.language = "zh-Hant-TW".into();
    assert_eq!(
        network::checks(&machine, &config, Some(&report), None, "148")[3].status,
        "mismatch"
    );
    report.exit.as_mut().unwrap().country = "CN".into();
    assert_eq!(
        network::checks(&machine, &config, Some(&report), None, "148")[0].status,
        "mismatch"
    );
    machine.locale_country = "CN".into();
    assert!(config.region(&machine).is_none());
    assert_eq!(
        network::checks(&machine, &config, Some(&report), None, "148")[0].status,
        "pending"
    );
    machine.locale_country.clear();
    assert!(config.region(&machine).is_none());
    // A matching timezone and Chinese language alone never establish Taiwan.
    machine.locale_country = "TW".into();
    machine.languages = vec!["en-GB".into()];
    assert_eq!(config.region(&machine).map(|r| r.id), Some("tw"));
    assert_eq!(config.language(&machine), "en-GB");
}

#[test]
fn refreshed_context_recomputes_target_from_effective_machine_country() {
    let mut context = Context {
        identity: crate::identities::browser_link::BrowserIdentity {
            linked: false,
            status: "unlinked",
            name: None,
            email: None,
            organization: None,
        },
        ui_language: "zh".into(),
        exit_services: &network::SERVICES,
        config: Config::default(),
        region: None,
        machine: machine(),
        engine_version: "148".into(),
    };
    let mut taiwan = machine();
    taiwan.timezone = "Asia/Taipei".into();
    taiwan.languages = vec!["zh-Hans-TW".into()];
    taiwan.locale = "zh_CN@rg=twzzzz".into();
    taiwan.locale_country = "TW".into();
    context.update_machine(taiwan.clone());
    assert_eq!(context.region.map(|r| r.country), Some("TW"));
    assert_eq!(context.config.language(&context.machine), "zh-Hans-TW");
    taiwan.locale_country = "CN".into();
    context.update_machine(taiwan);
    assert!(context.region.is_none());
}
