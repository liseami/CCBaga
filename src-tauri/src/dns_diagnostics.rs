// [INPUT]: 用户触发的系统解析器 TXT 查询与 dnsleak.dev 固定 HTTPS 权威观测接口。
// [OUTPUT]: 本次公网出口、实际递归解析服务器与观测完整性；不读取或生成泄漏结论。
// [POS]: 独立系统 DNS 诊断，不修改 DNS/代理，不测浏览器 DoH、WebRTC 或抓包路径。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::BTreeSet, net::IpAddr, sync::Arc, time::Duration};
use tokio::sync::Semaphore;

const SERVICE: &str = "https://dnsleak.dev";
const BODY_LIMIT: usize = 128 * 1024;
const LOOKUPS: usize = 12;
static GATE: std::sync::OnceLock<Arc<Semaphore>> = std::sync::OnceLock::new();

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    test_id: String,
    hostnames: Vec<String>,
}

impl Session {
    fn validate(&self) -> Result<(), String> {
        if !(8..=64).contains(&self.test_id.len())
            || !self
                .test_id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        {
            return Err("DNS 检测服务返回了无效结果。".into());
        }
        // 服务不能指示宿主查询任意域名；只接受本次会话的完整、唯一的固定测试域名。
        let expected: BTreeSet<_> = (1..=6)
            .flat_map(|n| {
                ['a', 'q']
                    .map(move |family| format!("{}-{family}{n:02}.t.dnsleak.dev", self.test_id))
            })
            .collect();
        if self.hostnames.len() != LOOKUPS
            || self.hostnames.iter().cloned().collect::<BTreeSet<_>>() != expected
        {
            return Err("DNS 检测服务返回了无效结果。".into());
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DnsClient {
    ip: IpAddr,
    country: Option<String>,
    org: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DnsResolver {
    ip: IpAddr,
    operator: Option<String>,
    org: Option<String>,
    country: Option<String>,
    asn: Option<u32>,
    role: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Observation {
    test_id: String,
    client: DnsClient,
    resolvers: Vec<DnsResolver>,
    lookups: usize,
    truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct DnsDiagnosis {
    complete: bool,
    client: DnsClient,
    resolvers: Vec<DnsResolver>,
    lookups: usize,
    expected_lookups: usize,
}

impl Observation {
    fn finish(self, session: &Session) -> Result<DnsDiagnosis, String> {
        if self.test_id != session.test_id || self.lookups > LOOKUPS || self.resolvers.len() > 128 {
            return Err("DNS 检测服务返回了无效结果。".into());
        }
        // 只展示实际递归解析器；权威端的重复观测者不是本机所用的解析器。
        let resolvers: Vec<_> = self
            .resolvers
            .into_iter()
            .filter(|r| r.role.as_deref() == Some("resolver"))
            .collect();
        let complete = self.lookups == LOOKUPS && !self.truncated && !resolvers.is_empty();
        Ok(DnsDiagnosis {
            complete,
            client: self.client,
            resolvers,
            lookups: self.lookups,
            expected_lookups: LOOKUPS,
        })
    }
}

async fn fetch_json<T: DeserializeOwned>(client: &reqwest::Client, url: &str) -> Result<T, String> {
    let mut response = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| "DNS 检测服务连接失败。")?;
    if response.status() != reqwest::StatusCode::OK {
        return Err("DNS 检测服务暂不可用，请稍后重试。".into());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "DNS 检测服务响应读取失败。")?
    {
        if body.len() + chunk.len() > BODY_LIMIT {
            return Err("DNS 检测服务返回了无效结果。".into());
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| "DNS 检测服务返回了无效结果。".into())
}

async fn diagnose(
    base: &str,
    probe: fn(&str),
    permit: tokio::sync::OwnedSemaphorePermit,
) -> Result<DnsDiagnosis, String> {
    // 不继承环境中的 HTTP 代理或凭据：控制请求与系统查询均走本机路由/TUN。
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "DNS 检测服务连接失败。")?;
    let session: Session = fetch_json(&client, &format!("{base}/api/new")).await?;
    session.validate()?;
    let hosts = session.hostnames.clone();
    let worker = tauri::async_runtime::spawn_blocking(move || {
        // 系统 API 不能可靠取消；超时后仍持有许可，避免重测堆积阻塞线程。
        let _permit = permit;
        let started = std::time::Instant::now();
        for host in hosts {
            if started.elapsed() >= Duration::from_secs(20) {
                break;
            }
            probe(&host);
        }
    });
    tokio::time::timeout(Duration::from_secs(25), worker)
        .await
        .map_err(|_| "系统 DNS 查询超时，检测未完成。")?
        .map_err(|_| "系统 DNS 查询失败，检测未完成。")?;
    // TXT 的 NODATA 是正常结果：关注权威端是否看到查询，不依赖本地返回地址。
    let url = format!("{base}/api/result/{}", session.test_id);
    let mut observation: Observation = fetch_json(&client, &url).await?;
    if observation.lookups < LOOKUPS {
        tokio::time::sleep(Duration::from_secs(1)).await;
        observation = fetch_json(&client, &url).await?;
    }
    observation.finish(&session)
}

#[tauri::command]
pub async fn network_diagnose_dns() -> Result<DnsDiagnosis, String> {
    let permit = GATE
        .get_or_init(|| Arc::new(Semaphore::new(1)))
        .clone()
        .try_acquire_owned()
        .map_err(|_| "系统 DNS 检测仍在运行，请稍后重试。")?;
    diagnose(SERVICE, system_query, permit).await
}

// TXT 避免 TUN 的 Fake-IP 直接合成 A 记录，以及禁用 AAAA 时造成的假完成。
#[cfg(target_os = "linux")]
fn system_query(host: &str) {
    use std::ffi::{c_char, c_int, CString};
    #[link(name = "resolv")]
    extern "C" {
        fn res_query(
            name: *const c_char,
            class: c_int,
            kind: c_int,
            answer: *mut u8,
            length: c_int,
        ) -> c_int;
    }
    let host = CString::new(format!("{host}.")).expect("validated DNS hostname");
    let mut answer = [0u8; 4096];
    // SAFETY: NUL 结尾的已校验域名与独占的有效缓冲；忽略内容，只触发系统解析。
    unsafe {
        res_query(
            host.as_ptr(),
            1,
            16,
            answer.as_mut_ptr(),
            answer.len() as c_int,
        );
    }
}

#[cfg(target_os = "macos")]
fn system_query(host: &str) {
    use std::ffi::{c_char, c_void, CString};
    type Service = *mut c_void;
    type Callback = unsafe extern "C" fn(
        Service,
        u32,
        u32,
        i32,
        *const c_char,
        u16,
        u16,
        u16,
        *const c_void,
        u32,
        *mut c_void,
    );
    #[link(name = "dns_sd")]
    extern "C" {
        fn DNSServiceQueryRecord(
            service: *mut Service,
            flags: u32,
            interface: u32,
            name: *const c_char,
            kind: u16,
            class: u16,
            callback: Callback,
            context: *mut c_void,
        ) -> i32;
        fn DNSServiceProcessResult(service: Service) -> i32;
        fn DNSServiceRefDeallocate(service: Service);
    }
    unsafe extern "C" fn received(
        _: Service,
        _: u32,
        _: u32,
        _: i32,
        _: *const c_char,
        _: u16,
        _: u16,
        _: u16,
        _: *const c_void,
        _: u32,
        context: *mut c_void,
    ) {
        // SAFETY: ProcessResult 同步回调；context 在 service 释放前一直存活。
        unsafe {
            *context.cast::<bool>() = true;
        }
    }
    let host = CString::new(format!("{host}.")).expect("validated DNS hostname");
    let mut service = std::ptr::null_mut();
    let mut done = false;
    // 使用 mDNSResponder 的系统分域解析器，而不是 macOS 不使用的 resolv.conf。
    // ReturnIntermediates (0x1000) 回传否定答案；Timeout (0x10000) 限制等待。
    unsafe {
        if DNSServiceQueryRecord(
            &mut service,
            0x11000,
            0,
            host.as_ptr(),
            16,
            1,
            received,
            (&mut done as *mut bool).cast(),
        ) == 0
        {
            while !done && DNSServiceProcessResult(service) == 0 {}
            DNSServiceRefDeallocate(service);
        }
    }
}

#[cfg(windows)]
fn system_query(host: &str) {
    use windows_sys::Win32::NetworkManagement::Dns::{
        DnsFree, DnsFreeRecordList, DnsQuery_W, DNS_QUERY_STANDARD, DNS_TYPE_TEXT,
    };
    let host: Vec<u16> = format!("{host}.").encode_utf16().chain(Some(0)).collect();
    let mut records = std::ptr::null_mut();
    // SAFETY: 有效的 UTF-16 域名；仅调用系统解析器，并释放其返回的记录链。
    unsafe {
        DnsQuery_W(
            host.as_ptr(),
            DNS_TYPE_TEXT,
            DNS_QUERY_STANDARD,
            std::ptr::null_mut(),
            &mut records,
            std::ptr::null_mut(),
        );
        if !records.is_null() {
            DnsFree(records.cast(), DnsFreeRecordList);
        }
    }
}

#[cfg(test)]
#[path = "dns_diagnostics_tests.rs"]
mod tests;
