// [INPUT]: 原生默认凭据快照、用户 MCP/插件保留策略与 base64/JSON。
// [OUTPUT]: 仅身份字段变换的钥匙串写入协议；保留当前共享 MCP/插件秘密。
// [POS]: 保险库和 macOS Security 桥之间的秘密边界，不进入 GUI 或日志。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::Policy;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
fn object(item: &Value) -> Result<Value, String> {
    let bytes = STANDARD
        .decode(item["data"].as_str().ok_or("身份凭据无法处理。")?)
        .map_err(|_| "身份凭据无法处理。")?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| "身份凭据格式不受支持，未修改凭据。")?;
    if !value.is_object() {
        return Err("身份凭据格式不受支持，未修改凭据。".into());
    }
    Ok(value)
}
fn encoded(value: &Value) -> String {
    STANDARD.encode(serde_json::to_vec(value).unwrap())
}
pub fn clear(items: &[Value], policy: &Policy) -> Result<Vec<Value>, String> {
    items
        .iter()
        .map(|item| {
            let mut result = item.clone();
            if item["service"] == "Claude Code-credentials" {
                let mut current = object(item)?;
                for key in crate::cleaner::identity::credential_keys(policy) {
                    current.as_object_mut().unwrap().remove(&key);
                }
                if !current.as_object().unwrap().is_empty() {
                    result["remaining"] = Value::String(encoded(&current));
                }
            }
            Ok(result)
        })
        .collect()
}
pub fn restore(items: &[Value], live: &[Value], policy: &Policy) -> Result<Vec<Value>, String> {
    items
        .iter()
        .map(|item| {
            let mut result = item.clone();
            let current = live
                .iter()
                .find(|row| row["service"] == item["service"] && row["account"] == item["account"]);
            if let Some(row) = current {
                result["expected"] = row["data"].clone();
            }
            if item["service"] == "Claude Code-credentials" {
                let saved = object(item)?;
                let mut value = if let Some(current) = current {
                    object(current)?
                } else {
                    serde_json::json!({})
                };
                for key in crate::cleaner::identity::credential_keys(policy) {
                    value.as_object_mut().unwrap().remove(&key);
                    if let Some(v) = saved.get(&key) {
                        value.as_object_mut().unwrap().insert(key, v.clone());
                    }
                }
                result["data"] = Value::String(encoded(&value));
            }
            Ok(result)
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changing_identity_preserves_current_mcp_and_plugin_secrets() {
        let row = |oauth: &str, mcp: &str| serde_json::json!({"service":"Claude Code-credentials","account":"fixture","data":encoded(&serde_json::json!({"claudeAiOauth":{"accessToken":oauth},"mcpOAuth":mcp,"pluginSecrets":"keep-plugin"}))});
        let current = row("current", "latest");
        let target = row("target", "old");
        let policy = Policy::default();
        let cleared = clear(&[current.clone()], &policy).unwrap();
        let remaining: Value = serde_json::from_slice(
            &STANDARD
                .decode(cleared[0]["remaining"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
        assert!(remaining.get("claudeAiOauth").is_none());
        assert_eq!(remaining["mcpOAuth"], "latest");
        let next = restore(&[target], &[current], &policy).unwrap();
        let value = object(&next[0]).unwrap();
        assert_eq!(value["claudeAiOauth"]["accessToken"], "target");
        assert_eq!(value["mcpOAuth"], "latest");
        assert_eq!(value["pluginSecrets"], "keep-plugin");
    }
}
