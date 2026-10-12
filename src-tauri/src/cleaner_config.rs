// [INPUT]: 有大小限制的配置原文及用户选择的清理类型。
// [OUTPUT]: 脱敏键名预览与只修改指定配置的替换文本。
// [POS]: 清理的配置变换边界；不执行 shell，不返回密钥或代理地址。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde_json::Value;
#[derive(Clone, Copy)]
pub(super) enum EditKind {
    Account,
    Settings,
    Shell,
    Npm,
    Git,
}
fn sensitive_key(key: &str) -> bool {
    matches!(
        key.to_ascii_uppercase().as_str(),
        "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY" | "NO_PROXY" | "CLAUDE_CODE_OAUTH_TOKEN"
    ) || key.starts_with("ANTHROPIC_")
}
pub(super) const ACCOUNT_KEYS: &[&str] = &[
    "userID",
    "machineID",
    "anonymousId",
    "oauthAccount",
    "primaryApiKey",
    "additionalModelCostsCache",
    "additionalModelOptionsCache",
    "autoCompactWindowsCache",
    "cachedChromeExtensionInstalled",
    "cachedDynamicConfigs",
    "cachedExperimentData",
    "cachedExperimentFeatures",
    "cachedExtraUsageDisabledReason",
    "cachedGrowthBookFeatures",
    "cachedGrowthBookFeaturesAt",
    "cachedStatsigGates",
    "clientDataCache",
    "clientDataCacheSlots",
    "feedbackSurveyState",
    "groveConfigCache",
    "metricsStatusCache",
    "modelAccessCache",
    "orgModelDefaultCache",
    "passesEligibilityCache",
    "s1mAccessCache",
];
pub(super) fn transform(text: &str, kind: EditKind) -> Result<(String, Vec<String>), String> {
    let mut names = Vec::new();
    if matches!(kind, EditKind::Account | EditKind::Settings) {
        let mut data: Value =
            serde_json::from_str(text).map_err(|_| "配置不是有效 JSON，拒绝修改。")?;
        let object = data.as_object_mut().ok_or("配置顶层必须是 JSON 对象。")?;
        if matches!(kind, EditKind::Account) {
            for key in ACCOUNT_KEYS {
                if object.remove(*key).is_some() {
                    names.push((*key).into());
                }
            }
        } else {
            if let Some(env) = object.get_mut("env") {
                let env = env
                    .as_object_mut()
                    .ok_or("env 必须是 JSON 对象，拒绝修改。")?;
                let keys: Vec<_> = env.keys().filter(|k| sensitive_key(k)).cloned().collect();
                for key in keys {
                    env.remove(&key);
                    names.push(format!("env.{key}"));
                }
            }
            if object.remove("apiKeyHelper").is_some() {
                names.push("apiKeyHelper".into());
            }
        }
        let updated = if names.is_empty() {
            text.into()
        } else {
            format!(
                "{}\n",
                serde_json::to_string_pretty(&data).map_err(|_| "无法生成配置。")?
            )
        };
        return Ok((updated, names));
    }
    let mut output = String::new();
    let mut git_section = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.is_empty() {
            output.push_str(line);
            continue;
        }
        if matches!(kind, EditKind::Git) && trimmed.starts_with('[') {
            let section = trimmed
                .trim_start_matches('[')
                .split([' ', '"', ']'])
                .next()
                .unwrap_or("");
            git_section =
                section.eq_ignore_ascii_case("http") || section.eq_ignore_ascii_case("https");
        }
        let candidate = if matches!(kind, EditKind::Shell) {
            trimmed
                .strip_prefix("export ")
                .unwrap_or(trimmed)
                .trim_start()
        } else {
            trimmed
        };
        let remove = candidate.split_once('=').and_then(|(key, value)| {
            let key = key.trim();
            let selected = match kind {
                EditKind::Shell => {
                    sensitive_key(key)
                        && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                        && simple_shell_value(value)
                }
                EditKind::Npm => {
                    matches!(key.to_ascii_lowercase().as_str(), "proxy" | "https-proxy")
                }
                EditKind::Git => git_section && key.eq_ignore_ascii_case("proxy"),
                _ => false,
            };
            selected.then(|| key.to_string())
        });
        if let Some(key) = remove {
            names.push(key);
        } else {
            output.push_str(line);
        }
    }
    names.sort();
    names.dedup();
    Ok((output, names))
}
fn simple_shell_value(value: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;
    for ch in value.trim().chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if matches!(ch, '\'' | '"') {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            }
        } else if quote.is_none()
            && (ch.is_whitespace() || matches!(ch, ';' | '&' | '|' | '(' | ')' | '`' | '<' | '>'))
        {
            return false;
        }
    }
    quote.is_none() && !escaped
}

pub(super) fn needs_manual_review(text: &str, kind: EditKind) -> bool {
    match kind {
        EditKind::Shell => text
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .any(|line| {
                let line = line.trim().strip_prefix("export ").unwrap_or(line.trim());
                line.split_once('=').is_some_and(|(key, value)| {
                    sensitive_key(key.trim()) && !simple_shell_value(value)
                })
            }),
        EditKind::Git => text
            .lines()
            .any(|line| line.trim_start().starts_with("[include")),
        _ => false,
    }
}
