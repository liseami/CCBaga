// [INPUT]: 原生机器快照、独立浏览器配置和实际引擎版本。
// [OUTPUT]: Default 资料的指纹设置与 Chromium 参数；默认跟随 TUN 网络，保持真实系统版本、CPU 与设备图形。
// [POS]: 浏览器配置真相源；路径来自宿主，输入不能变成额外命令参数。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use crate::native::decode;
use serde::{Deserialize, Serialize};
use std::{ffi::c_char, path::Path};
extern "C" {
    fn rf_browser_machine() -> *mut c_char;
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Machine {
    pub platform: String,
    pub version: String,
    pub cores: u32,
    pub timezone: String,
    pub languages: Vec<String>,
    pub locale: String,
    pub location_enabled: bool,
    pub timezones: Vec<String>,
    pub system_proxy: String,
}
pub fn machine() -> Result<Machine, String> {
    decode(unsafe { rf_browser_machine() })
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub seed: u32,
    pub proxy: String,
    pub follow_machine: bool,
    pub language: String,
    pub timezone: String,
    pub native_graphics: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            seed: 495574189,
            proxy: String::new(),
            follow_machine: true,
            language: "en-US".into(),
            timezone: "America/New_York".into(),
            native_graphics: true,
        }
    }
}
impl Config {
    pub fn language(&self, machine: &Machine) -> String {
        if self.follow_machine {
            let first = machine
                .languages
                .first()
                .map(String::as_str)
                .unwrap_or("en-US");
            if first == "en" {
                "en-US".into()
            } else {
                first.to_string()
            }
        } else {
            self.language.clone()
        }
    }
    pub fn timezone(&self, machine: &Machine) -> String {
        if self.follow_machine {
            machine.timezone.clone()
        } else {
            self.timezone.clone()
        }
    }
    pub fn validate(&self, machine: &Machine) -> Result<(), String> {
        if self.seed == 0 {
            return Err("指纹种子必须是 1–4294967295 的整数。".into());
        }
        proxy(&self.proxy)?;
        if ![
            "en-US", "en-CA", "en-GB", "en-SG", "en-AU", "en", "zh-Hans", "zh-Hant", "fr", "de",
            "es", "it", "ja", "ko",
        ]
        .contains(&self.language.as_str())
            || !machine.timezones.contains(&self.timezone)
        {
            return Err("语言或时区无效。".into());
        }
        Ok(())
    }
}
pub fn proxy(value: &str) -> Result<Option<reqwest::Url>, String> {
    if value.is_empty() {
        return Ok(None);
    }
    let url = reqwest::Url::parse(value).map_err(|_| "代理地址格式无效。")?;
    if !["http", "https", "socks5", "socks5h"].contains(&url.scheme())
        || url.host_str().is_none()
        || url.port().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !["", "/"].contains(&url.path())
    {
        return Err("填写 http、https 或 socks5 代理主机与端口；带密码代理请交给本机 Clash，不在本软件保存凭据。".into());
    }
    Ok(Some(url))
}
pub fn load(root: &Path) -> Result<Config, String> {
    match std::fs::read(root.join("config.json")) {
        Ok(data) => {
            serde_json::from_slice(&data).map_err(|_| "浏览器配置损坏，未使用默认值覆盖。".into())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(_) => Err("无法读取浏览器配置。".into()),
    }
}
pub fn save(root: &Path, config: &Config) -> Result<(), String> {
    super::profile::write_json(&root.join("config.json"), config)
}
pub fn arguments(
    config: &Config,
    machine: &Machine,
    profile: &Path,
    page: &str,
    version: &str,
) -> Vec<String> {
    let language = config.language(machine);
    let mut args = vec![
        format!("--user-data-dir={}", profile.display()),
        "--profile-directory=Default".into(),
        format!("--fingerprint={}", config.seed),
        "--fingerprint-platform=macos".into(),
        format!("--fingerprint-platform-version={}", machine.version),
        "--fingerprint-brand=Chrome".into(),
        format!("--fingerprint-brand-version={version}"),
        format!("--fingerprint-hardware-concurrency={}", machine.cores),
        format!("--lang={language}"),
        format!(
            "--accept-lang={language},{}",
            language.split('-').next().unwrap_or("en")
        ),
        format!("--timezone={}", config.timezone(machine)),
        "--force-webrtc-ip-handling-policy=disable_non_proxied_udp".into(),
        "--disable-non-proxied-udp".into(),
        "--disable-quic".into(),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        format!(
            "--disable-spoofing={}",
            if config.native_graphics {
                "font,audio,gpu,canvas,clientrects"
            } else {
                "font,audio,gpu"
            }
        ),
    ];
    if !config.proxy.is_empty() {
        args.push(format!(
            "--proxy-server={}",
            config.proxy.replace("socks5h://", "socks5://")
        ));
        if let Ok(Some(url)) = proxy(&config.proxy) {
            if url.scheme().starts_with("socks") {
                args.push(format!("--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE {}, EXCLUDE 127.0.0.1, EXCLUDE localhost",url.host_str().unwrap()));
            }
        }
    }
    args.push(page.into());
    args
}
