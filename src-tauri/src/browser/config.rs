// [INPUT]: 原生机器快照、独立浏览器配置和实际引擎版本。
// [OUTPUT]: 含有效国家的机器快照、独立目标地区与 Default 资料参数；仅显式代理时增加回环正向豁免，已写明的方案默认端口仍算指定端口，保留真实设备与 TUN 默认路由。
// [POS]: 浏览器配置真相源；路径来自宿主，输入不能变成额外命令参数。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
#[cfg(target_os = "linux")]
use crate::linux::*;
use crate::native::decode;
#[cfg(target_os = "windows")]
use crate::windows::*;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::ffi::c_char;
use std::path::Path;
#[cfg(target_os = "macos")]
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
    #[serde(default)]
    pub locale_country: String,
    pub location_enabled: Option<bool>,
    pub timezones: Vec<String>,
    #[serde(skip_serializing)]
    pub system_proxy: String,
}
pub fn machine() -> Result<Machine, String> {
    let mut machine: Machine = decode(unsafe { rf_browser_machine() })?;
    if machine.locale_country.is_empty() {
        machine.locale_country =
            crate::regions::locale_country(&machine.locale).unwrap_or_default();
    }
    Ok(machine)
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
    pub fn region(&self, machine: &Machine) -> Option<&'static crate::regions::Region> {
        if self.follow_machine {
            crate::regions::matching_country(&machine.locale_country, &machine.timezone)
        } else {
            crate::regions::matching(&self.language, &self.timezone)
        }
    }
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
        || !specifies_port(value)
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
/// `Url::port()` hides a scheme default, so `http://host:80` and `https://host:443` look portless.
/// The field must contain an explicit port, including those defaults and a bracketed IPv6 host.
fn specifies_port(value: &str) -> bool {
    let Some((_, rest)) = value.split_once("://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let hostport = authority
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(authority);
    let after_host = if let Some(rest) = hostport.strip_prefix('[') {
        match rest.split_once(']') {
            Some((_, after)) => after,
            None => return false,
        }
    } else {
        match hostport.rfind(':') {
            Some(index) => &hostport[index..],
            None => return false,
        }
    };
    let Some(port) = after_host.strip_prefix(':') else {
        return false;
    };
    !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit())
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
    super::profile::check_path(root, &root.join("config.json"))?;
    crate::identities::private::directory(root)?;
    crate::identities::private::write(
        &root.join("config.json"),
        &serde_json::to_vec_pretty(config).map_err(|_| "配置编码失败。")?,
    )
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
        format!("--fingerprint-platform={}", machine.platform),
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
        // 不使用 <-loopback>：它会减去隐式回环豁免，不能把带令牌的本地页面交给代理。
        args.push("--proxy-bypass-list=127.0.0.1;localhost;[::1]".into());
        if let Ok(Some(url)) = proxy(&config.proxy) {
            if url.scheme().starts_with("socks") {
                let host = url.host_str().unwrap().trim_matches(['[', ']']);
                args.push(format!(
                    "--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE {host}, EXCLUDE 127.0.0.1, EXCLUDE localhost, EXCLUDE ::1"
                ));
            }
        }
    }
    args.push(page.into());
    args
}
