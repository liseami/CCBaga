// [INPUT]: VPS 地址与完整 pem/ppk/pub 密钥包、家宽 SOCKS5 参数、固定 Webshare/DMIT 入口编号与随应用编译的 setup-vps.sh。
// [OUTPUT]: 固定供应商链接的系统浏览器打开、SSH 连通检查、远程设置、自检通过后的订阅地址与脱敏阶段日志。
// [POS]: 远程执行边界；本机 OpenSSH 核验主机密钥，只允许 root:22 的显式私钥登录，禁用 Agent/密码；密钥核验与临时副本由 vps_keys 管理，家宽密码经加密标准输入传递。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use crate::progress::{Progress, Reporter};
use crate::vps_keys::{Keys, PrivateKey};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::{ipc::Channel, Manager};

#[derive(Default)]
pub struct Vps {
    active: AtomicBool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    host: String,
    keys: Keys,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HomeProxy {
    host: String,
    port: u16,
    username: String,
    password: String,
    expected_ip: String,
}
#[derive(Serialize)]
pub struct SetupResult {
    subscription: String,
    message: String,
}
const SCRIPT: &str = include_str!("../../scripts/setup-vps.sh");
fn provider_url(provider: &str) -> Result<&'static str, String> {
    match provider {
        "webshare" => Ok("https://www.webshare.io/?referral_code=qlm1mt9xibc9"),
        "dmit" => Ok("https://www.dmit.io/aff.php?aff=28800"),
        _ => Err("无效的供应商入口。".into()),
    }
}
#[tauri::command]
pub async fn vps_open_provider(provider: String) -> Result<(), String> {
    let url = provider_url(&provider)?;
    tauri::async_runtime::spawn_blocking(move || {
        let status = Command::new("/usr/bin/open")
            .arg(url)
            .status()
            .map_err(|_| "无法打开供应商网站。")?;
        if status.success() {
            Ok(())
        } else {
            Err("无法打开供应商网站。".into())
        }
    })
    .await
    .map_err(|_| "无法打开供应商网站。".to_string())?
}
fn host_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && !value.starts_with('-')
        && !value.contains(['\n', '\r'])
        && (value.parse::<std::net::IpAddr>().is_ok()
            || value.split('.').all(|part| {
                !part.is_empty()
                    && part.len() <= 63
                    && !part.starts_with('-')
                    && !part.ends_with('-')
                    && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            }))
}
impl Connection {
    fn validate(&self) -> Result<(), String> {
        if !host_valid(&self.host) {
            return Err("填写 VPS IP 或域名，不带 ssh:// 或命令参数。".into());
        }
        Ok(())
    }
}
#[tauri::command]
pub async fn vps_choose_keys(
    app: tauri::AppHandle,
    extension: String,
) -> Result<Vec<String>, String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = send.send(crate::native::choose_ssh_keys(&extension));
    })
    .map_err(|_| "无法打开密钥文件选择器。")?;
    receive.await.map_err(|_| "无法打开密钥文件选择器。")?
}
impl HomeProxy {
    fn validate(&self) -> Result<(), String> {
        if !host_valid(&self.host)
            || self.port == 0
            || self.username.is_empty()
            || self.password.is_empty()
            || [&self.username, &self.password]
                .iter()
                .any(|s| s.contains('\0') || s.len() > 4096)
            || (!self.expected_ip.is_empty()
                && self.expected_ip.parse::<std::net::IpAddr>().is_err())
        {
            return Err("填写家宽 SOCKS5 主机、端口、账号与密码；预期出口须是 IP 地址。".into());
        }
        if self.expected_ip.is_empty() && self.host.parse::<std::net::IpAddr>().is_err() {
            return Err("家宽地址是域名时，请填写供应商提供的预期出口 IP，供远端自检。".into());
        }
        Ok(())
    }
}
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
fn payload(connection: &Connection, home: &HomeProxy) -> String {
    let mut text = String::from("set -e\n");
    // ----- 秘密只在加密 SSH 标准输入内；不放进远端命令行 -----
    for (key, value) in [
        ("HOME_HOST", home.host.as_str()),
        ("HOME_PORT", &home.port.to_string()),
        ("HOME_USER", &home.username),
        ("HOME_PASS", &home.password),
        ("EXPECTED_EXIT_IP", &home.expected_ip),
        ("SERVER_ADDR", &connection.host),
        ("SSH_PORT", "22"),
    ] {
        text.push_str(&format!("export {key}={}\n", quote(value)));
    }
    text.push_str(SCRIPT);
    text
}
fn ssh(connection: &Connection) -> Result<(Command, PrivateKey), String> {
    connection.validate()?;
    let key = connection.keys.prepare()?;
    let mut command = Command::new("/usr/bin/ssh");
    command
        .args([
            "-F",
            "/dev/null",
            "-T",
            "-p",
            "22",
            "-l",
            "root",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "ConnectTimeout=15",
            "-o",
            "ConnectionAttempts=1",
            "-o",
            "ServerAliveInterval=15",
            "-o",
            "ServerAliveCountMax=3",
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "IdentityAgent=none",
            "-o",
            "PreferredAuthentications=publickey",
            "-o",
            "PasswordAuthentication=no",
            "-o",
            "KbdInteractiveAuthentication=no",
        ])
        .arg("-i")
        .arg(&key.0)
        .arg(&connection.host);
    Ok((command, key))
}
// 持续排空两个管道，仅保留最后 64 KiB；不将原始远端输出显示给用户。
fn tail(mut pipe: impl Read) -> Vec<u8> {
    let mut output = Vec::new();
    let mut chunk = [0; 4096];
    while let Ok(size) = pipe.read(&mut chunk) {
        if size == 0 {
            break;
        }
        output.extend_from_slice(&chunk[..size]);
        if output.len() > 65536 {
            output.drain(..output.len() - 65536);
        }
    }
    output
}
fn execute(
    mut command: Command,
    input: String,
    timeout: Duration,
    log: Arc<Reporter>,
) -> Result<String, String> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "无法启动本机 OpenSSH。")?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let out = std::thread::spawn(move || tail(stdout));
    let err = std::thread::spawn(move || tail(stderr));
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let began = Instant::now();
    let mut pulse = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if began.elapsed() < timeout => {
                if pulse.elapsed() > Duration::from_secs(20) {
                    log.log(
                        "vps",
                        "running",
                        "SSH 任务仍在执行；等待远端返回实际结果。",
                        0,
                        None,
                    );
                    pulse = Instant::now();
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(
                    "SSH 超时或连接中断。远端可能已部分修改；检查 VPS 后再重试。".to_string(),
                );
            }
        }
    };
    let written = writer.join().map_err(|_| "SSH 输入任务被中断。")?;
    let stdout =
        String::from_utf8_lossy(&out.join().map_err(|_| "SSH 输出任务被中断。")?).into_owned();
    let stderr =
        String::from_utf8_lossy(&err.join().map_err(|_| "SSH 输出任务被中断。")?).into_owned();
    let status = status?;
    if !status.success() {
        let message = if stderr.contains("REMOTE HOST IDENTIFICATION HAS CHANGED") {
            "VPS 主机密钥已改变，OpenSSH 拒绝连接。请核实服务器身份后处理 known_hosts。"
        } else if stderr.contains("Permission denied") {
            "SSH 登录被拒绝。请确认该密钥包已绑定到此 VPS，且允许 root 密钥登录。"
        } else if status.code() == Some(255) {
            "SSH 连接失败。检查 VPS 地址、端口、网络与本机 known_hosts 权限。"
        } else {
            "远端设置或自检失败，未生成可用订阅。请通过 SSH 检查服务和家宽出口；设置可能已部分完成。"
        };
        return Err(message.into());
    }
    written.map_err(|_| "SSH 数据传输未完成。")?;
    Ok(stdout)
}
async fn run<T: Send + 'static>(
    app: tauri::AppHandle,
    action: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let state = app.state::<Vps>();
    if state.active.swap(true, Ordering::SeqCst) {
        return Err("已有 VPS 任务执行中，请等它完成。".into());
    }
    let result = tauri::async_runtime::spawn_blocking(action)
        .await
        .map_err(|_| "VPS 任务被中断。".to_string())
        .and_then(|x| x);
    state.active.store(false, Ordering::SeqCst);
    result
}
#[tauri::command]
pub async fn vps_connect(
    app: tauri::AppHandle,
    connection: Connection,
    on_progress: Channel<Progress>,
) -> Result<String, String> {
    run(app, move || {
        let log = Arc::new(Reporter::channel(Some(on_progress)));
        let (mut command, _helper) = ssh(&connection)?;
        command.arg("sh -c 'test $(id -u) -eq 0'");
        log.log(
            "vps",
            "running",
            "检查 root SSH 密钥登录，不修改服务器。",
            0,
            None,
        );
        execute(command, String::new(), Duration::from_secs(30), log)?;
        Ok("SSH 密钥连接成功，root 权限已核验。".into())
    })
    .await
}
fn subscription(output: &str, host: &str) -> Result<String, String> {
    let link = output
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix("CC_SUB_URL="))
        .ok_or("远端未返回通过自检的订阅地址。")?;
    let url = reqwest::Url::parse(link).map_err(|_| "订阅地址格式无效。")?;
    let path: Vec<_> = url.path().split('/').collect();
    if url.scheme() != "http"
        || !url
            .host_str()
            .map(|s| {
                s.trim_matches(['[', ']']).eq_ignore_ascii_case(host)
                    || s.trim_matches(['[', ']'])
                        .parse::<std::net::IpAddr>()
                        .ok()
                        .zip(host.parse::<std::net::IpAddr>().ok())
                        .map(|(a, b)| a == b)
                        .unwrap_or(false)
            })
            .unwrap_or(false)
        || url.port() != Some(8443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || path.len() != 3
        || path[1].len() != 32
        || !path[1].bytes().all(|c| c.is_ascii_hexdigit())
        || path[2] != "clash.yaml"
    {
        return Err("远端返回的订阅地址与 VPS 不一致。".into());
    }
    Ok(link.into())
}
#[tauri::command]
pub async fn vps_setup(
    app: tauri::AppHandle,
    connection: Connection,
    home: HomeProxy,
    on_progress: Channel<Progress>,
) -> Result<SetupResult, String> {
    connection.validate()?;
    home.validate()?;
    run(app, move || {
        let log = Arc::new(Reporter::channel(Some(on_progress)));
        let (mut command, _helper) = ssh(&connection)?;
        let wrapper = "umask 077; tmp=$(mktemp); trap 'rm -f \"$tmp\"' EXIT; cat > \"$tmp\"; bash \"$tmp\"";
        command.arg(format!("bash -c {}", quote(wrapper)));
        log.log("vps", "running", "开始远端设置：更新系统、安装 Xray、配置 SSH 与防火墙，并核验家宽出口。可能需要 10–30 分钟。", 0, None);
        let output = execute(command, payload(&connection, &home), Duration::from_secs(1800), log.clone())?;
        let subscription = subscription(&output, &connection.host)?;
        log.log("vps", "ok", "远端自检已通过，订阅地址已返回。", 1, Some(1));
        Ok(SetupResult { subscription, message: "导入订阅后，只打开 TUN 模式，关闭系统代理；CC浏览器代理留空，跟随 TUN。".into() })
    }).await
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundaries_and_secret_transport() {
        assert_eq!(
            provider_url("webshare").unwrap(),
            "https://www.webshare.io/?referral_code=qlm1mt9xibc9"
        );
        assert_eq!(
            provider_url("dmit").unwrap(),
            "https://www.dmit.io/aff.php?aff=28800"
        );
        assert!(provider_url("https://example.com").is_err());
        assert!(host_valid("vps.example.com"));
        assert!(host_valid("2001:db8::1"));
        for host in [
            "-oProxyCommand=id",
            "host;id",
            "host\nwhoami",
            "user@host",
            "",
        ] {
            assert!(!host_valid(host));
        }
        let value = "p'$(touch /tmp/cc-injection)\nnext";
        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("printf %s {}", quote(value)))
            .output()
            .unwrap();
        assert_eq!(String::from_utf8(output.stdout).unwrap(), value);
        let link = "http://example.com:8443/0123456789abcdef0123456789abcdef/clash.yaml";
        assert_eq!(
            subscription(&format!("CC_SUB_URL={link}\n"), "example.com").unwrap(),
            link
        );
        assert!(subscription(&format!("CC_SUB_URL={link}"), "other.com").is_err());
        assert!(subscription("Setup complete - SOME SELF-TESTS FAILED", "example.com").is_err());
        let log = Arc::new(Reporter::channel(None));
        let mut command = Command::new("/bin/sh");
        command.arg("-s");
        assert_eq!(
            execute(
                command,
                "printf 'CC_TEST=ok\\n'\n".into(),
                Duration::from_secs(2),
                log
            )
            .unwrap(),
            "CC_TEST=ok\n"
        );
        assert_eq!(tail(&vec![b'x'; 100000][..]).len(), 65536);
        let mut timeout_command = Command::new("/bin/sh");
        timeout_command.arg("-s");
        assert!(execute(
            timeout_command,
            "exec /bin/sleep 2\n".into(),
            Duration::from_millis(50),
            Arc::new(Reporter::channel(None))
        )
        .is_err());
        assert!(serde_json::from_str::<Connection>(r#"{"host":"example.com","port":22,"username":"root","auth":"password","password":"secret","identity":""}"#).is_err());
        assert!(subscription(
            "CC_SUB_URL=http://[2001:db8::1]:8443/0123456789abcdef0123456789abcdef/clash.yaml",
            "2001:0db8:0:0:0:0:0:1"
        )
        .is_ok());
    }
}
