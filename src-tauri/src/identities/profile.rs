// [INPUT]: 默认 .claude.json 的 oauthAccount 显示字段、本地 .credentials.json 订阅/过期时间戳、活动会话 PID 元数据及白名单桌面工作区元数据。
// [OUTPUT]: 当前本地身份与已保存身份的脱敏摘要（用户名/邮箱/组织/套餐档位/OAuth有效状态）及有界活动会话列表；不输出令牌、不联网验证账号。
// [POS]: 身份卡片的只读展示边界；不读取钥匙串、Cookie 正文或任意项目文件。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, io::Read, path::Path};
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Account {
    display_name: Option<String>,
    email_address: Option<String>,
    organization_name: Option<String>,
    billing_type: Option<String>,
}
#[derive(Default, Deserialize)]
struct Config {
    #[serde(rename = "oauthAccount")]
    account: Option<Account>,
    #[serde(rename = "primaryApiKey")]
    api_key: Option<serde::de::IgnoredAny>,
}
#[derive(Clone, Default, Serialize)]
pub struct Current {
    pub present: bool,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub organization: Option<String>,
    pub unavailable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_status: Option<String>,
}
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_status: Option<String>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveSession {
    pub pid: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub entrypoint: String,
}
fn display(value: Option<String>, limit: usize) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && !v.chars().any(char::is_control))
        .map(|v| v.chars().take(limit).collect())
}
fn format_plan(sub: Option<&str>, tier: Option<&str>, billing: Option<&str>) -> Option<String> {
    let tier_lower = tier.unwrap_or("").to_ascii_lowercase();
    if tier_lower.contains("20x") {
        return Some("Max 20x".into());
    }
    if tier_lower.contains("5x") {
        return Some("Max 5x".into());
    }
    match sub.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("max") => Some("Max".into()),
        Some("pro") => Some("Pro".into()),
        Some("team") => Some("Team".into()),
        Some("enterprise") => Some("Enterprise".into()),
        Some("free") => Some("Free".into()),
        Some(other) if !other.is_empty() && !other.chars().any(char::is_control) => {
            display(Some(other.to_uppercase()), 24)
        }
        _ => match billing.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            Some("stripe_subscription") => Some("Pro / Max".into()),
            _ => None,
        },
    }
}
fn read_bounded_json(path: &Path, max_bytes: u64) -> Option<Value> {
    let meta = fs::symlink_metadata(path).ok()?;
    super::private::parents(path.parent()?).ok()?;
    super::private::safe(path).ok()?;
    if !meta.is_file() || meta.len() > max_bytes {
        return None;
    }
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    fs::File::open(path)
        .ok()?
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > max_bytes {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}
fn extract_credentials_meta(value: &Value) -> (Option<String>, Option<String>, Option<String>) {
    let oauth = value.get("claudeAiOauth").unwrap_or(value);
    let sub = oauth.get("subscriptionType").and_then(Value::as_str);
    let tier = oauth.get("rateLimitTier").and_then(Value::as_str);
    let has_access = oauth
        .get("accessToken")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty());
    let has_refresh = oauth
        .get("refreshToken")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty());
    let expires_at = oauth.get("expiresAt").and_then(Value::as_i64);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let status = if has_access {
        match expires_at {
            Some(exp) if exp > 0 && exp <= now_ms => {
                if has_refresh {
                    Some("refresh_needed".into())
                } else {
                    Some("expired".into())
                }
            }
            _ => Some("valid".into()),
        }
    } else {
        None
    };
    (
        sub.map(str::to_string),
        tier.map(str::to_string),
        status,
    )
}
fn extract_settings_api_auth(value: &Value) -> bool {
    [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "CLAUDE_CODE_OAUTH_TOKEN",
    ]
    .iter()
    .any(|k| {
        value
            .pointer(&format!("/env/{k}"))
            .or_else(|| value.get(&format!("env.{k}")))
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
    }) || value
        .get("apiKeyHelper")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty())
}
fn parse(bytes: &[u8]) -> Result<Current, serde_json::Error> {
    let config: Config = serde_json::from_slice(bytes)?;
    Ok(match config.account {
        Some(account) => Current {
            present: true,
            display_name: display(account.display_name, 80),
            email: display(account.email_address, 254),
            organization: display(account.organization_name, 120),
            unavailable: false,
            plan: format_plan(None, None, account.billing_type.as_deref()),
            auth_status: None,
        },
        None => Current {
            present: config.api_key.is_some(),
            ..Current::default()
        },
    })
}
pub fn read(home: &Path) -> Current {
    let path = home.join(".claude.json");
    let mut billing_type = None;
    let mut has_primary_api_key = false;
    let result = (|| {
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Current::default()),
            Err(_) => return Err(()),
        };
        super::private::parents(home).map_err(|_| ())?;
        super::private::safe(&path).map_err(|_| ())?;
        if !meta.is_file() || meta.len() > 8 * 1024 * 1024 {
            return Err(());
        }
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        fs::File::open(&path)
            .map_err(|_| ())?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ())?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(());
        }
        if let Ok(cfg) = serde_json::from_slice::<Config>(&bytes) {
            billing_type = cfg.account.and_then(|a| a.billing_type);
            has_primary_api_key = cfg.api_key.is_some();
        }
        parse(&bytes).map_err(|_| ())
    })();
    let mut current = result.unwrap_or_else(|_| Current {
        present: true,
        unavailable: true,
        ..Current::default()
    });
    if !current.unavailable {
        let mut sub = None;
        let mut tier = None;
        if let Some(cred) = read_bounded_json(&home.join(".claude/.credentials.json"), 1024 * 1024)
        {
            let (s, t, status) = extract_credentials_meta(&cred);
            sub = s;
            tier = t;
            if status.is_some() {
                current.auth_status = status;
                current.present = true;
            }
        }
        if let Some(resolved_plan) =
            format_plan(sub.as_deref(), tier.as_deref(), billing_type.as_deref())
        {
            current.plan = Some(resolved_plan);
        }
        let mut has_settings_api = false;
        for name in ["settings.json", "settings.local.json"] {
            if let Some(val) =
                read_bounded_json(&home.join(".claude").join(name), 2 * 1024 * 1024)
            {
                has_settings_api |= extract_settings_api_auth(&val);
            }
        }
        if current.auth_status.is_none() && (has_primary_api_key || has_settings_api) {
            current.auth_status = Some("api_key".into());
        }
    }
    current.present |= crate::cleaner::identity::desktop_present(home);
    current
}
pub(super) fn summarize_stage(stage: &super::archive::Stage) -> Summary {
    let mut summary = Summary::default();
    let mut billing_type = None;
    let mut sub = None;
    let mut tier = None;
    let mut has_api_key = false;
    let home = &stage.manifest.home;
    for (n, source) in stage.manifest.sources.iter().enumerate() {
        let data_path = stage.path.join(format!("data/{n}"));
        let Some(val) = read_bounded_json(&data_path, 8 * 1024 * 1024) else {
            continue;
        };
        if source.path == home.join(".claude.json") {
            if let Ok(cfg) = serde_json::from_value::<Config>(val) {
                has_api_key |= cfg.api_key.is_some();
                if let Some(acct) = cfg.account {
                    summary.display_name = display(acct.display_name, 80);
                    summary.email = display(acct.email_address, 254);
                    summary.organization = display(acct.organization_name, 120);
                    billing_type = acct.billing_type;
                }
            }
        } else if source.path == home.join(".claude/.credentials.json") {
            let (s, t, status) = extract_credentials_meta(&val);
            sub = s;
            tier = t;
            if status.is_some() {
                summary.auth_status = status;
            }
        } else if source.path == home.join(".claude/settings.json")
            || source.path == home.join(".claude/settings.local.json")
        {
            has_api_key |= extract_settings_api_auth(&val);
        }
    }
    for item in &stage.manifest.credentials {
        if item.get("service").and_then(Value::as_str) == Some("Claude Code-credentials") {
            if let Some(encoded) = item.get("data").and_then(Value::as_str) {
                if let Ok(bytes) =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
                {
                    if let Ok(val) = serde_json::from_slice::<Value>(&bytes) {
                        let (s, t, status) = extract_credentials_meta(&val);
                        if sub.is_none() {
                            sub = s;
                        }
                        if tier.is_none() {
                            tier = t;
                        }
                        if summary.auth_status.is_none() {
                            summary.auth_status = status;
                        }
                    }
                }
            }
        }
    }
    summary.plan = format_plan(sub.as_deref(), tier.as_deref(), billing_type.as_deref());
    if summary.auth_status.is_none() && has_api_key {
        summary.auth_status = Some("api_key".into());
    }
    summary
}
pub fn active_sessions(home: &Path) -> Vec<ActiveSession> {
    let running = crate::cleaner::identity::running_processes();
    if running.is_empty() {
        return Vec::new();
    }
    let sessions_dir = home.join(".claude/sessions");
    let mut out = Vec::with_capacity(running.len());
    for proc in running {
        let mut workspace = None;
        let mut entrypoint = proc.name.clone();
        let session_file = sessions_dir.join(format!("{}.json", proc.pid));
        if let Some(val) = read_bounded_json(&session_file, 64 * 1024) {
            if let Some(cwd) = val.get("cwd").and_then(Value::as_str) {
                let folder = Path::new(cwd)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_string)
                    .or_else(|| Some(cwd.to_string()));
                workspace = display(folder, 48);
            }
            if let Some(ep) = val.get("entrypoint").and_then(Value::as_str) {
                entrypoint = match ep {
                    "cli" => "CLI".into(),
                    "claude-vscode" => "VS Code".into(),
                    "claude-desktop" => "Claude Desktop".into(),
                    "sdk-cli" => "SDK".into(),
                    "mcp" => "MCP".into(),
                    other => display(Some(other.to_string()), 24).unwrap_or(entrypoint),
                };
            }
        }
        out.push(ActiveSession {
            pid: proc.pid,
            workspace,
            entrypoint,
        });
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicit_display_fields_reach_the_card() {
        let current=parse(br#"{"oauthAccount":{"displayName":" Person ","emailAddress":"fixture@example.invalid","organizationName":"Team","accessToken":"private"},"primaryApiKey":"private"}"#).unwrap();
        let output = serde_json::to_string(&current).unwrap();
        assert!(current.present);
        assert_eq!(current.display_name.as_deref(), Some("Person"));
        assert!(!output.contains("private"));
        assert!(!output.contains("Token"));
        assert!(parse(br#"{"primaryApiKey":"private"}"#).unwrap().present);
        assert!(!parse(b"{}").unwrap().present);
        assert!(!parse(br#"{"oauthAccount":null}"#).unwrap().present);
        assert!(display(Some("unsafe\nvalue".into()), 80).is_none());
        assert_eq!(
            format_plan(Some("max"), Some("default_claude_max_5x"), None).as_deref(),
            Some("Max 5x")
        );
    }
}
