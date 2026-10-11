// [INPUT]: 显式代理地址、ipwho.is/ipapi.co HTTPS 出口信息与真实浏览器上报。
// [OUTPUT]: 共用检测服务目录、出口解析与按同一路由切换服务的检测；无上报或失败保持未核验。
// [POS]: 环境核验边界；只把出口观测当证据，配置值不能冒充实际结果。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
pub const SERVICES: [&str; 2] = ["https://ipwho.is/", "https://ipapi.co/json/"];
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Exit {
    pub ip: String,
    pub country: String,
    pub city: String,
    pub timezone: String,
    pub isp: String,
}
pub fn parse(data: &Value) -> Result<Exit, String> {
    if data["success"].as_bool() == Some(false) || data["error"].as_bool() == Some(true) {
        return Err("出口服务未返回有效结果。".into());
    }
    let ip = data["ip"].as_str().ok_or("出口 IP 缺失。")?;
    ip.parse::<std::net::IpAddr>()
        .map_err(|_| "出口 IP 无效。")?;
    let country = data["country_code"]
        .as_str()
        .filter(|x| x.len() == 2 && x.bytes().all(|c| c.is_ascii_uppercase()))
        .ok_or("出口国家无效。")?;
    Ok(Exit {
        ip: ip.into(),
        country: country.into(),
        city: data["city"].as_str().unwrap_or("").into(),
        timezone: data["timezone"]
            .as_str()
            .or_else(|| data["timezone"]["id"].as_str())
            .unwrap_or("")
            .into(),
        isp: data["org"]
            .as_str()
            .or_else(|| data["connection"]["isp"].as_str())
            .unwrap_or("")
            .into(),
    })
}
pub async fn probe(proxy: &str) -> Result<Exit, String> {
    probe_urls(proxy, &SERVICES).await
}
pub(super) async fn probe_urls(proxy: &str, services: &[&str]) -> Result<Exit, String> {
    let mut client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(8));
    if !proxy.is_empty() {
        client = client.proxy(reqwest::Proxy::all(proxy).map_err(|_| "代理格式无效。")?);
    }
    let client = client.build().map_err(|_| "无法建立出口检查。")?;
    let mut errors = Vec::new();
    for address in services {
        let name = reqwest::Url::parse(address)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .unwrap_or_default();
        let result = async {
            let mut response = client.get(*address).send().await.map_err(|e| {
                if e.is_timeout() {
                    "请求超时"
                } else {
                    "网络连接失败"
                }
                .to_string()
            })?;
            if !response.status().is_success() {
                return Err(format!("HTTP {}", response.status().as_u16()));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| "出口响应读取失败".to_string())?
            {
                if bytes.len() + chunk.len() > 32_768 {
                    return Err("出口响应过大".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            parse(&serde_json::from_slice(&bytes).map_err(|_| "出口响应格式错误".to_string())?)
        }
        .await;
        match result {
            Ok(exit) => return Ok(exit),
            Err(error) => errors.push(format!("{name}：{error}")),
        }
    }
    Err(format!(
        "出口检测未完成（{}）。请检查 TUN/代理连接后重试。",
        errors.join("；")
    ))
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub language: String,
    pub languages: Vec<String>,
    pub timezone: String,
    pub platform: String,
    pub cores: u32,
    pub user_agent: String,
    pub platform_version: String,
    pub browser_version: String,
    pub geolocation: String,
    pub exit: Option<Exit>,
    pub error: String,
}
impl Report {
    pub fn valid(&self) -> bool {
        self.language.len() < 80
            && self.languages.len() < 20
            && self.languages.iter().all(|x| x.len() < 80)
            && self.timezone.len() < 100
            && self.platform.len() < 60
            && self.user_agent.len() < 2048
            && self.platform_version.len() < 60
            && self.browser_version.len() < 60
            && self.geolocation.len() < 30
            && self.error.len() < 1000
            && self.cores > 0
            && self.cores <= 1024
            && self
                .exit
                .as_ref()
                .map(|x| {
                    x.ip.parse::<std::net::IpAddr>().is_ok()
                        && [&x.country, &x.city, &x.timezone, &x.isp]
                            .iter()
                            .all(|x| x.len() < 300)
                })
                .unwrap_or(true)
    }
}
#[derive(Serialize)]
pub struct Check {
    pub title: &'static str,
    pub status: &'static str,
    pub detail: String,
}
pub fn checks(
    machine: &super::config::Machine,
    config: &super::config::Config,
    report: Option<&Report>,
    host: Option<&Exit>,
    version: &str,
) -> Vec<Check> {
    let check = |title, ok: Option<bool>, detail: String| Check {
        title,
        status: match ok {
            Some(true) => "ok",
            Some(false) => "mismatch",
            None => "pending",
        },
        detail,
    };
    let target = crate::regions::matching(&config.language(machine), &config.timezone(machine));
    vec![
        check(
            "目标地区网络出口",
            target
                .zip(report.and_then(|r| r.exit.as_ref()))
                .map(|(r, x)| x.country == r.country),
            report
                .and_then(|r| r.exit.as_ref())
                .map(|x| {
                    format!(
                        "目标：{}；实际：{} · {} · {} · {}",
                        target.map(|r| r.label).unwrap_or("自定义组合，未确定国家"),
                        x.country,
                        x.ip,
                        x.city,
                        x.isp
                    )
                })
                .unwrap_or("需要打开浏览器实测出口。".into()),
        ),
        check(
            "浏览器与机器出口",
            host.zip(report.and_then(|r| r.exit.as_ref()))
                .map(|(a, b)| a.ip == b.ip),
            host.map(|x| format!("上次检测的机器出口：{}", x.ip))
                .unwrap_or("点击检测机器出口；配置代理不等于已核验。".into()),
        ),
        check(
            "时区一致",
            report
                .zip(report.and_then(|r| r.exit.as_ref()))
                .map(|(r, e)| {
                    r.timezone == config.timezone(machine)
                        && r.timezone == machine.timezone
                        && r.timezone == e.timezone
                }),
            format!(
                "本机：{}；浏览器：{}；出口：{}",
                machine.timezone,
                report.map(|x| x.timezone.as_str()).unwrap_or("待测"),
                report
                    .and_then(|x| x.exit.as_ref())
                    .map(|x| x.timezone.as_str())
                    .unwrap_or("待测")
            ),
        ),
        check(
            "目标语言与系统区域",
            target.zip(report).map(|(target, r)| {
                r.language == target.language
                    && machine
                        .languages
                        .first()
                        .map(|x| {
                            x == target.language || x.starts_with(&format!("{}-", target.language))
                        })
                        .unwrap_or(false)
                    && machine.locale.starts_with(target.locale)
            }),
            format!(
                "机器：{} / {}；浏览器：{}",
                machine.languages.join(" → "),
                machine.locale,
                report.map(|x| x.language.as_str()).unwrap_or("待测")
            ),
        ),
        check(
            "真实设备与 CPU",
            report.map(|r| {
                r.platform.to_lowercase().contains("mac")
                    && r.cores == machine.cores
                    && r.platform_version == machine.version
            }),
            format!(
                "macOS {} · {} 核；浏览器版本 {} · {} 核",
                machine.version,
                machine.cores,
                report
                    .map(|x| x.platform_version.as_str())
                    .unwrap_or("待测"),
                report.map(|x| x.cores.to_string()).unwrap_or("待测".into())
            ),
        ),
        check(
            "浏览器版本",
            report.map(|r| r.browser_version == version),
            format!(
                "引擎：{version}；实测：{}",
                report.map(|x| x.browser_version.as_str()).unwrap_or("待测")
            ),
        ),
        check(
            "地理位置关闭",
            report.map(|r| !machine.location_enabled && r.geolocation == "denied"),
            format!(
                "机器定位：{}；网页权限：{}",
                if machine.location_enabled {
                    "开启"
                } else {
                    "关闭"
                },
                report.map(|x| x.geolocation.as_str()).unwrap_or("待测")
            ),
        ),
        check(
            "真实图形与字体",
            None,
            if config.native_graphics {
                "已配置使用真实字体、GPU、音频、Canvas 与布局；读数尚未独立核验。".into()
            } else {
                "Canvas 与布局矩形使用稳定种子；不作为机器完全一致。".into()
            },
        ),
        check(
            "DNS / WebRTC 泄漏",
            None,
            "已设置代理 DNS、限制未代理 UDP、关闭 QUIC；尚未完成独立泄漏测试。".into(),
        ),
    ]
}
