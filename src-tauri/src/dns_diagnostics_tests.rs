// [INPUT]: 虚构权威观测、注入的无网络解析器与临时回环 HTTP 服务。
// [OUTPUT]: 完整/混合/不完整/未知结论、域名白名单与会话绑定、HTTP 大小/重定向/凭据边界的离线回归。
// [POS]: DNS 检测的确定性测试；不查询外网、不读取凭据或修改系统解析器。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};

fn session() -> Session {
    let test_id = "test123456".to_string();
    let hostnames = (1..=6)
        .flat_map(|n| ['a', 'q'].map(move |f| format!("test123456-{f}{n:02}.t.dnsleak.dev")))
        .collect();
    Session { test_id, hostnames }
}

fn observation() -> Value {
    json!({"testId":"test123456","client":{"ip":"203.0.113.5","country":"United States","org":"Fixture"},
        "lookups":12,"truncated":false,"verdict":{"status":"no_leak"},
        "resolvers":[{"ip":"1.1.1.1","operator":"Cloudflare","org":"Cloudflare","country":"United States","asn":13335,"role":"resolver"},
            {"ip":"2606:4700:4700::1111","operator":"Cloudflare","org":"Cloudflare","country":"United States","asn":13335,"role":"resolver"}]})
}

fn finish(value: Value) -> Result<DnsDiagnosis, String> {
    serde_json::from_value::<Observation>(value)
        .unwrap()
        .finish(&session())
}

#[test]
fn only_complete_unique_session_names_are_queried() {
    session().validate().unwrap();
    for host in [
        "example.com",
        "test123456-a01.t.dnsleak.dev.evil.invalid",
        "test123456-a01.t.dnsleak.dev\0",
    ] {
        let mut value = session();
        value.hostnames[0] = host.into();
        assert!(value.validate().is_err());
    }
    let mut value = session();
    value.hostnames[0] = value.hostnames[1].clone();
    assert!(value.validate().is_err());
    let mut value = session();
    value.hostnames.pop();
    assert!(value.validate().is_err());
    for id in ["../result", "TEST123456", "x", "test%12345"] {
        let mut value = session();
        value.test_id = id.into();
        assert!(value.validate().is_err());
    }
}

#[test]
fn complete_observations_preserve_actual_ipv4_ipv6_resolvers() {
    let report = finish(observation()).unwrap();
    assert_eq!(report.status, DnsStatus::Observed);
    assert_eq!(report.resolvers.len(), 2);
    assert!(report.resolvers[1].ip.is_ipv6());
    assert_eq!(report.expected_lookups, LOOKUPS);
    let mut value = observation();
    value["verdict"]["status"] = json!("split");
    assert_eq!(finish(value).unwrap().status, DnsStatus::Mixed);
}

#[test]
fn missing_partial_truncated_and_unknown_results_never_pass() {
    for count in [0, 11] {
        let mut value = observation();
        value["lookups"] = json!(count);
        assert_eq!(finish(value).unwrap().status, DnsStatus::Inconclusive);
    }
    let mut value = observation();
    value["truncated"] = json!(true);
    assert_eq!(finish(value).unwrap().status, DnsStatus::Inconclusive);
    for status in ["inconclusive", "new_future_status", "leak"] {
        let mut value = observation();
        value["verdict"] = json!({"status":status,"leaking":false});
        assert_eq!(finish(value).unwrap().status, DnsStatus::Inconclusive);
    }
    let mut value = observation();
    value["resolvers"] = json!([]);
    assert_eq!(finish(value).unwrap().status, DnsStatus::Inconclusive);
}

#[test]
fn observers_are_not_resolvers_and_results_must_match_the_session() {
    let mut value = observation();
    value["resolvers"][0]["role"] = json!("observer");
    assert_eq!(finish(value.clone()).unwrap().resolvers.len(), 1);
    value["resolvers"][1]["role"] = json!("observer");
    assert_eq!(finish(value).unwrap().status, DnsStatus::Inconclusive);
    let mut value = observation();
    value["testId"] = json!("another123");
    assert!(finish(value).is_err());
    let mut value = observation();
    value["lookups"] = json!(13);
    assert!(finish(value).is_err());
    let mut value = observation();
    value["client"]["ip"] = json!("not an IP");
    assert!(serde_json::from_value::<Observation>(value).is_err());
}

fn http(responses: Vec<Vec<u8>>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let mut paths = Vec::new();
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(&stream);
            let mut request = String::new();
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).unwrap() > 0);
                request.push_str(&line);
                if line == "\r\n" {
                    break;
                }
            }
            let lower = request.to_lowercase();
            assert!(!lower.contains("authorization:") && !lower.contains("cookie:"));
            paths.push(request.lines().next().unwrap().to_string());
            let _ = stream.write_all(&response);
        }
        paths
    });
    (format!("http://{address}"), server)
}

fn response(status: &str, body: &[u8]) -> Vec<u8> {
    [
        format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes(),
        body.to_vec(),
    ]
    .concat()
}

fn json_response(value: Value) -> Vec<u8> {
    response("200 OK", &serde_json::to_vec(&value).unwrap())
}

fn session_response() -> Vec<u8> {
    json_response(json!({"testId":session().test_id,"hostnames":session().hostnames}))
}

#[test]
fn real_http_flow_rechecks_incomplete_results_and_retains_provider_evidence() {
    let mut partial = observation();
    partial["lookups"] = json!(11);
    let (base, server) = http(vec![
        session_response(),
        json_response(partial),
        json_response(observation()),
    ]);
    let gate = Arc::new(Semaphore::new(1));
    let report = tauri::async_runtime::block_on(diagnose(
        &base,
        |host| {
            assert!(session().hostnames.contains(&host.to_string()));
        },
        gate.clone().try_acquire_owned().unwrap(),
    ))
    .unwrap();
    assert_eq!(report.status, DnsStatus::Observed);
    assert_eq!(report.resolvers[0].operator.as_deref(), Some("Cloudflare"));
    assert_eq!(gate.available_permits(), 1);
    assert_eq!(
        server.join().unwrap(),
        [
            "GET /api/new HTTP/1.1",
            "GET /api/result/test123456 HTTP/1.1",
            "GET /api/result/test123456 HTTP/1.1"
        ]
    );
}

#[test]
fn invalid_session_stops_before_system_queries() {
    let (base, server) = http(vec![json_response(
        json!({"testId":"test123456","hostnames":["evil.invalid"]}),
    )]);
    let gate = Arc::new(Semaphore::new(1));
    assert!(tauri::async_runtime::block_on(diagnose(
        &base,
        |_| panic!("must not query"),
        gate.clone().try_acquire_owned().unwrap()
    ))
    .is_err());
    assert_eq!(gate.available_permits(), 1);
    assert_eq!(server.join().unwrap().len(), 1);
}

#[test]
fn redirects_oversized_and_malformed_responses_are_rejected() {
    for bytes in [
        response("302 Found\r\nLocation: http://127.0.0.1:1/never", b""),
        response("429 Too Many Requests", b""),
        response("206 Partial Content", b"{}"),
        response("200 OK", &vec![b' '; BODY_LIMIT + 1]),
        response("200 OK", b"not json"),
        b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\n{}".to_vec(),
    ] {
        let (base, server) = http(vec![bytes]);
        let gate = Arc::new(Semaphore::new(1));
        assert!(tauri::async_runtime::block_on(diagnose(
            &base,
            |_| panic!("must not query"),
            gate.try_acquire_owned().unwrap()
        ))
        .is_err());
        assert_eq!(server.join().unwrap().len(), 1);
    }
}

#[test]
fn timed_out_blocking_queries_keep_the_single_worker_permit() {
    tauri::async_runtime::block_on(async {
        let gate = Arc::new(Semaphore::new(1));
        let permit = gate.clone().try_acquire_owned().unwrap();
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let (finished, done) = tokio::sync::oneshot::channel();
        let worker = tauri::async_runtime::spawn_blocking(move || {
            let _ = entered.send(());
            wait.recv().unwrap();
            drop(permit);
            let _ = finished.send(());
        });
        ready.await.unwrap();
        assert!(tokio::time::timeout(Duration::from_millis(1), worker)
            .await
            .is_err());
        assert!(gate.clone().try_acquire_owned().is_err());
        release.send(()).unwrap();
        done.await.unwrap();
        assert_eq!(gate.available_permits(), 1);
    });
}

#[test]
#[ignore = "Explicit live system DNS test: contacts dnsleak.dev; does not change settings"]
fn live_system_dns_observation() {
    let report = tauri::async_runtime::block_on(network_diagnose_dns()).unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    assert_eq!(report.lookups, LOOKUPS);
    assert!(!report.resolvers.is_empty());
}
