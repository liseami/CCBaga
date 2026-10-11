// [INPUT]: 打包的起始页、一次启动的随机令牌、机器上下文、共享界面语言、共用出口服务目录和浏览器实测。
// [OUTPUT]: 仅 127.0.0.1 的起始页、实时机器/界面语言上下文与带接收时间的观测接口；资源限定在打包目录；服务存活到会话令牌被替换或清空。
// [POS]: Chromium 与工具箱之间的窄桥；不提供文件写入、脚本执行或 Tauri 命令。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::{network::Report, Browser, Context};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Component, Path},
    time::Duration,
};
use tauri::{Emitter, Manager};
fn answer(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) {
    let header=format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; connect-src 'self' https://ipwho.is https://ipapi.co; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; script-src 'self'\r\n\r\n",body.len());
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
}
pub fn nonce() -> Result<String, String> {
    let mut bytes = [0u8; 24];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|_| "无法创建本地会话令牌。")?;
    Ok(bytes.iter().map(|x| format!("{x:02x}")).collect())
}
pub fn bind() -> Result<TcpListener, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|_| "起始页服务启动失败。")?;
    listener
        .set_nonblocking(true)
        .map_err(|_| "起始页服务配置失败。")?;
    Ok(listener)
}
pub fn resource(root: &Path, relative: &str) -> Option<std::path::PathBuf> {
    let path = Path::new(relative);
    if path
        .components()
        .any(|x| !matches!(x, Component::Normal(_)))
    {
        return None;
    }
    let target = root.join(path).canonicalize().ok()?;
    let root = root.canonicalize().ok()?;
    if !target.starts_with(root) || !target.is_file() {
        return None;
    }
    Some(target)
}
pub fn start(
    app: tauri::AppHandle,
    listener: TcpListener,
    token: String,
    root: std::path::PathBuf,
    context: Context,
) {
    std::thread::spawn(move || {
        // 生命周期跟随会话令牌：令牌被替换或清空即退出；宿主运行期间保持可用，供 CC浏览器 每次全新启动打开。
        loop {
            if app
                .state::<Browser>()
                .session
                .lock()
                .unwrap()
                .as_ref()
                .map(|x| x.token.as_str())
                != Some(token.as_str())
            {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                    handle(&app, &mut stream, &token, &root, &context);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(40))
                }
                Err(_) => break,
            }
        }
    });
}
fn handle(
    app: &tauri::AppHandle,
    stream: &mut TcpStream,
    token: &str,
    root: &Path,
    context: &Context,
) {
    let mut data = Vec::new();
    let mut byte = [0u8; 1];
    while data.len() < 8192 && !data.ends_with(b"\r\n\r\n") {
        if stream.read_exact(&mut byte).is_err() {
            return;
        }
        data.push(byte[0]);
    }
    if !data.ends_with(b"\r\n\r\n") {
        return;
    }
    let header = String::from_utf8_lossy(&data);
    let mut words = header.lines().next().unwrap_or("").split_whitespace();
    let method = words.next().unwrap_or("");
    let url = words.next().unwrap_or("");
    let prefix = format!("/{token}/");
    let Some(path)=url.strip_prefix(&prefix) else{answer(stream,"404 Not Found","text/plain",b"Not found");return;};
    if method == "GET" && path == "context" {
        let mut current = context.clone();
        current.ui_language = crate::ui_language::ui_language_read(app.clone());
        match super::config::machine() {
            Ok(machine) => current.machine = machine,
            Err(_) => {
                answer(
                    stream,
                    "503 Service Unavailable",
                    "text/plain",
                    b"Machine state unavailable",
                );
                return;
            }
        }
        current.region = crate::regions::matching(
            &current.config.language(&current.machine),
            &current.config.timezone(&current.machine),
        );
        answer(
            stream,
            "200 OK",
            "application/json",
            &serde_json::to_vec(&current).unwrap(),
        );
        return;
    }
    if method == "POST" && path == "report" {
        let length = header
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, v)| v.trim().parse::<usize>().ok())
            .filter(|n| *n > 0 && *n <= 16_384);
        let Some(length)=length else{answer(stream,"400 Bad Request","text/plain",b"Invalid length");return;};
        let mut body = vec![0u8; length];
        if stream.read_exact(&mut body).is_err() {
            return;
        }
        let report = serde_json::from_slice::<Report>(&body)
            .ok()
            .filter(Report::valid);
        let Some(report)=report else{answer(stream,"400 Bad Request","text/plain",b"Invalid report");return;};
        let state = app.state::<Browser>();
        let mut session = state.session.lock().unwrap();
        if let Some(state) = session.as_mut().filter(|x| x.token == token) {
            state.report = Some(report);
            state.received_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            );
            let _ = app.emit("browser-report", ());
        }
        answer(stream, "200 OK", "application/json", b"{\"ok\":true}");
        return;
    }
    if method != "GET" {
        answer(
            stream,
            "405 Method Not Allowed",
            "text/plain",
            b"Method not allowed",
        );
        return;
    }
    let Some(path)=resource(root,path) else{answer(stream,"404 Not Found","text/plain",b"Not found");return;};
    let kind = match path.extension().and_then(|x| x.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript",
        "css" => "text/css",
        "woff2" => "font/woff2",
        _ => {
            answer(stream, "404 Not Found", "text/plain", b"Not found");
            return;
        }
    };
    if let Ok(body) = std::fs::read(path) {
        answer(stream, "200 OK", kind, &body);
    }
}
