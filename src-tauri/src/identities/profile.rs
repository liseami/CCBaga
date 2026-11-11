// [INPUT]: 默认 .claude.json 的 oauthAccount 显示字段、本地凭据的订阅/过期元数据、已认证暂存归档和精确进程查询结果。
// [OUTPUT]: 当前身份与已保存摘要的显示白名单、按读取时间计算的本地凭据状态，以及至多 32 个进程预览；查询失败明确未知，Desktop 读取失败保守标记 present/unavailable。
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct AuthMetadata {
    access: bool,
    refresh: bool,
    api_key: bool,
    expires_at: Option<i64>,
}
impl AuthMetadata {
    fn status_at(&self, now_ms: i64) -> &'static str {
        if self.access {
            match self.expires_at.filter(|exp| *exp > 0) {
                Some(exp) if exp > now_ms => "local_unexpired",
                Some(_) if self.refresh => "refresh_available",
                Some(_) => "expired",
                None => "unknown",
            }
        } else if self.api_key {
            "api_key"
        } else {
            "unknown"
        }
    }
    fn expiry(&self) -> Option<i64> {
        self.expires_at.filter(|exp| self.access && *exp > 0)
    }
}
#[derive(Default, Serialize, Deserialize)]
pub(super) struct SavedSummary {
    #[serde(flatten)]
    summary: Summary,
    #[serde(default)]
    auth: Option<AuthMetadata>,
}
impl SavedSummary {
    pub(super) fn public_at(&self, now_ms: i64) -> Summary {
        let mut summary = self.summary.clone();
        summary.auth_status = Some(
            self.auth
                .as_ref()
                .map_or("unknown", |auth| auth.status_at(now_ms))
                .into(),
        );
        summary.expires_at = self.auth.as_ref().and_then(AuthMetadata::expiry);
        summary
    }
}
pub(super) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessPreview {
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
fn extract_credentials_meta(value: &Value) -> (Option<String>, Option<String>, AuthMetadata) {
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
    (
        sub.map(str::to_string),
        tier.map(str::to_string),
        AuthMetadata {
            access: has_access,
            refresh: has_refresh,
            api_key: false,
            expires_at,
        },
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
            ..Current::default()
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
        let mut auth = AuthMetadata::default();
        if let Some(cred) = read_bounded_json(&home.join(".claude/.credentials.json"), 1024 * 1024)
        {
            let (s, t, metadata) = extract_credentials_meta(&cred);
            sub = s;
            tier = t;
            current.present |= metadata.access || metadata.refresh;
            auth = metadata;
        }
        if let Some(resolved_plan) =
            format_plan(sub.as_deref(), tier.as_deref(), billing_type.as_deref())
        {
            current.plan = Some(resolved_plan);
        }
        let mut has_settings_api = false;
        for name in ["settings.json", "settings.local.json"] {
            if let Some(val) = read_bounded_json(&home.join(".claude").join(name), 2 * 1024 * 1024)
            {
                has_settings_api |= extract_settings_api_auth(&val);
            }
        }
        auth.api_key = has_primary_api_key || has_settings_api;
        current.present |= auth.api_key;
        current.auth_status = Some(auth.status_at(now_ms()).into());
        current.expires_at = auth.expiry();
    }
    match crate::cleaner::identity::desktop_present(home) {
        Ok(present) => current.present |= present,
        Err(_) => {
            current.present = true;
            current.unavailable = true;
        }
    }
    current
}
pub(super) fn summarize_stage(stage: &super::archive::Stage) -> SavedSummary {
    let mut summary = Summary::default();
    let mut auth = AuthMetadata::default();
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
            let (s, t, metadata) = extract_credentials_meta(&val);
            sub = s;
            tier = t;
            auth = metadata;
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
                    let bytes = zeroize::Zeroizing::new(bytes);
                    if let Ok(val) = serde_json::from_slice::<Value>(&bytes) {
                        let (s, t, metadata) = extract_credentials_meta(&val);
                        if sub.is_none() {
                            sub = s;
                        }
                        if tier.is_none() {
                            tier = t;
                        }
                        if !auth.access {
                            auth = metadata;
                        }
                    }
                }
            }
        }
    }
    summary.plan = format_plan(sub.as_deref(), tier.as_deref(), billing_type.as_deref());
    auth.api_key = has_api_key;
    SavedSummary {
        summary,
        auth: Some(auth),
    }
}
pub fn process_previews(
    result: Result<Vec<crate::cleaner::ClaudeProcess>, String>,
) -> (Option<Vec<ProcessPreview>>, bool) {
    let Ok(running) = result else {
        return (None, false);
    };
    let truncated = running.len() > 32;
    let out = running
        .into_iter()
        .take(32)
        .map(|proc| ProcessPreview {
            pid: proc.pid,
            // PID 不能证明会话索引属于这次进程；不读取旧 sessions 文件猜测工作区。
            workspace: None,
            entrypoint: display(Some(proc.name), 48).unwrap_or_else(|| "Claude".into()),
        })
        .collect();
    (Some(out), truncated)
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
    #[test]
    fn auth_metadata_expires_on_read_and_legacy_or_incomplete_values_are_unknown() {
        let (_, _, mut auth) = extract_credentials_meta(&serde_json::json!({
            "claudeAiOauth": {"accessToken":"private-access", "refreshToken":"private-refresh", "expiresAt":2000}
        }));
        let saved = SavedSummary {
            auth: Some(auth.clone()),
            ..SavedSummary::default()
        };
        assert_eq!(
            saved.public_at(1000).auth_status.as_deref(),
            Some("local_unexpired")
        );
        assert_eq!(
            saved.public_at(2000).auth_status.as_deref(),
            Some("refresh_available")
        );
        auth.refresh = false;
        assert_eq!(auth.status_at(2000), "expired");
        for expiry in [None, Some(0), Some(-1)] {
            auth.expires_at = expiry;
            assert_eq!(auth.status_at(1000), "unknown");
        }
        auth.access = false;
        auth.refresh = true;
        auth.expires_at = Some(2000);
        assert_eq!(auth.status_at(1000), "unknown");
        let old: SavedSummary = serde_json::from_value(serde_json::json!({
            "email":"fixture@example.invalid", "auth_status":"valid", "expires_at":9999
        }))
        .unwrap();
        assert_eq!(old.public_at(1000).auth_status.as_deref(), Some("unknown"));
        assert_eq!(old.public_at(1000).expires_at, None);
        let raw = serde_json::to_string(&saved).unwrap();
        assert!(!raw.contains("private-access") && !raw.contains("private-refresh"));
        let public = serde_json::to_string(&saved.public_at(1000)).unwrap();
        assert!(!public.contains("\"auth\"") && !public.contains("refreshToken"));
    }
    #[test]
    fn settings_only_auth_is_present_without_reading_keychain() {
        let f = crate::identities::optimization_tests::Fixture::new();
        f.write(
            ".claude/settings.json",
            br#"{"theme":"dark","env":{"ANTHROPIC_API_KEY":"private-api-key"}}"#,
        );
        let current = read(&f.home);
        assert!(current.present);
        assert_eq!(current.auth_status.as_deref(), Some("api_key"));
        assert!(!serde_json::to_string(&current)
            .unwrap()
            .contains("private-api-key"));
        let s = super::super::store::Store::open(&f.vault, &f.home).unwrap();
        let (record, stage) = s
            .save(
                "API",
                &super::super::Policy::default(),
                &crate::progress::Reporter::channel(None),
            )
            .unwrap();
        assert!(!record.empty);
        assert_eq!(
            summarize_stage(&stage)
                .public_at(1000)
                .auth_status
                .as_deref(),
            Some("api_key")
        );
    }
    #[test]
    fn process_preview_is_bounded_read_only_and_distinguishes_unknown() {
        let rows = (0..40)
            .map(|n| crate::cleaner::ClaudeProcess {
                pid: n + 10,
                name: "Claude Desktop".into(),
                executable: "private-command-token".into(),
                #[cfg(windows)]
                created: 1,
            })
            .collect();
        let (previews, truncated) = process_previews(Ok(rows));
        let previews = previews.unwrap();
        assert_eq!(previews.len(), 32);
        assert!(truncated && previews.iter().all(|p| p.workspace.is_none()));
        assert!(!serde_json::to_string(&previews)
            .unwrap()
            .contains("private-command-token"));
        assert_eq!(process_previews(Ok(Vec::new())), (Some(Vec::new()), false));
        assert_eq!(process_previews(Err("query failed".into())), (None, false));
    }
}
