// [INPUT]: Cleaner 白名单、完整目录快照、用户保留策略与 JSON 配置。
// [OUTPUT]: 受限身份目标与配置身份键的提取/清空/恢复，保留工作区未选择的个人资产及桌面身份路径元数据检测。
// [POS]: 身份保险库复用原有路径边界；不扫描项目源码、Shell 或系统代理配置。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
use crate::identities::Policy;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Source {
    pub path: PathBuf,
    pub category: String,
    pub keys: Vec<String>,
}
pub(crate) fn credential_keys(policy: &Policy) -> Vec<String> {
    let mut keys = vec!["claudeAiOauth".into(), "trustedDeviceToken".into()];
    if !policy.mcp {
        keys.extend(
            [
                "mcpOAuth",
                "mcpOAuthClientConfig",
                "mcpXaaIdp",
                "mcpXaaIdpConfig",
            ]
            .map(str::to_string),
        );
    }
    if !policy.plugins {
        keys.push("pluginSecrets".into());
    }
    keys
}
fn definitions(root: &Path, policy: &Policy) -> Vec<Source> {
    let mut result: Vec<_> = targets::definitions(root)
        .into_iter()
        .filter(|t| {
            (matches!(t.group, "cache" | "history" | "account" | "desktop")
                || (t.group == "backups"
                    && (t.path.starts_with(root.join(".claude"))
                        || t.path.file_name().is_some_and(|n| {
                            n.to_string_lossy().starts_with(".claude.json.backup")
                        }))))
                && !(t.group == "history" && policy.history)
                && !(t.group == "cache" && policy.cache)
        })
        .map(|t| Source {
            path: t.path,
            category: t.group.into(),
            keys: if matches!(t.operation, Operation::Edit(config::EditKind::Account)) {
                config::ACCOUNT_KEYS.iter().map(|s| s.to_string()).collect()
            } else {
                Vec::new()
            },
        })
        .collect();
    if let Some(source) = result
        .iter_mut()
        .find(|s| s.path == root.join(".claude.json"))
    {
        if !policy.mcp {
            source.keys.push("mcpServers".into());
        }
        if !policy.projects {
            source.keys.push("projects".into());
        }
    }
    if let Some(source) = result
        .iter_mut()
        .find(|s| s.path == root.join(".claude/.credentials.json"))
    {
        source.keys = credential_keys(policy);
    }
    if !policy.mcp {
        let roots: std::collections::BTreeSet<_> = result
            .iter()
            .filter(|s| {
                s.category == "desktop" && s.path.file_name().is_some_and(|n| n == "Local Storage")
            })
            .filter_map(|s| s.path.parent().map(Path::to_path_buf))
            .collect();
        for desktop in roots {
            result.push(Source {
                path: desktop.join("claude_desktop_config.json"),
                category: "mcp".into(),
                keys: vec!["mcpServers".into()],
            });
        }
    }
    for name in ["settings.json", "settings.local.json"] {
        result.push(Source {
            path: root.join(".claude").join(name),
            category: "settings".into(),
            keys: if policy.settings {
                vec![
                    "env.CLAUDE_CODE_OAUTH_TOKEN".into(),
                    "env.ANTHROPIC_API_KEY".into(),
                    "env.ANTHROPIC_AUTH_TOKEN".into(),
                    "apiKeyHelper".into(),
                ]
            } else {
                Vec::new()
            },
        });
    }
    for (category, keep, names) in [
        (
            "skills",
            policy.skills,
            vec!["skills", "commands", "agents", "hooks"],
        ),
        ("plugins", policy.plugins, vec!["plugins"]),
        ("mcp", policy.mcp, vec![".mcp.json"]),
    ] {
        if !keep {
            for name in names {
                result.push(Source {
                    path: root.join(".claude").join(name),
                    category: category.into(),
                    keys: Vec::new(),
                });
            }
        }
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    result.dedup_by(|a, b| a.path == b.path);
    result
}
pub(crate) fn sources(root: &Path, policy: &Policy) -> Result<Vec<Source>, String> {
    let mut result = Vec::new();
    for mut source in definitions(root, policy) {
        let meta = match fs::symlink_metadata(&source.path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err("身份数据无法完整读取；未修改任何数据。".into()),
        };
        #[cfg(test)]
        if !linked(&meta) {
            assert_eq!(canonicalize(&source.path).ok().as_deref(), Some(source.path.as_path()), "fixture source must be canonical");
        }
        if linked(&meta)
            || canonicalize(&source.path).ok().as_deref() != Some(source.path.as_path())
        {
            return Err("身份路径包含链接或特殊文件。".into());
        }
        crate::identities::private::safe(&source.path)?;
        targets::measure(&source.path)?;
        if source.path == root.join(".claude.json") && !policy.settings {
            let value: serde_json::Value =
                serde_json::from_slice(&fs::read(&source.path).map_err(|_| "身份配置无法读取。")?)
                    .map_err(|_| "身份配置不是有效 JSON。")?;
            let object = value.as_object().ok_or("身份配置不是对象。")?;
            source.keys = object
                .keys()
                .filter(|k| {
                    !(policy.mcp && *k == "mcpServers") && !(policy.projects && *k == "projects")
                })
                .cloned()
                .collect();
            if source.keys.is_empty() {
                continue;
            }
        }
        if !source.keys.is_empty() {
            let text = fs::read_to_string(&source.path).map_err(|_| "身份配置无法读取。")?;
            if extract(&source, &text)?
                .as_object()
                .is_some_and(|o| o.is_empty())
            {
                continue;
            }
        }
        result.push(source);
    }
    let all = result.clone();
    result.retain(|s| {
        !all.iter()
            .any(|p| p.path != s.path && p.keys.is_empty() && s.path.starts_with(&p.path))
    });
    Ok(result)
}
pub(crate) fn allowed(root: &Path, source: &Source) -> bool {
    let all = definitions(root, &Policy::all());
    let known = all.iter().any(|t| {
        t.path == source.path
            && t.category == source.category
            && ((source.path == root.join(".claude.json")
                && source.category == "account"
                && !source.keys.is_empty()
                && source
                    .keys
                    .iter()
                    .all(|k| !k.contains('.') && !k.contains('/')))
                || source.keys.iter().all(|k| t.keys.contains(k))
                || (source.category == "settings"
                    && source.keys.iter().all(|k| {
                        [
                            "env.CLAUDE_CODE_OAUTH_TOKEN",
                            "env.ANTHROPIC_API_KEY",
                            "env.ANTHROPIC_AUTH_TOKEN",
                            "apiKeyHelper",
                        ]
                        .contains(&k.as_str())
                    })))
    });
    let special = source.keys.is_empty()
        && [
            (
                "Library/Preferences/ByHost",
                "com.anthropic.claudefordesktop",
            ),
            ("Library/Caches", "com.anthropic.claudefordesktop"),
            ("Library/Logs/DiagnosticReports", "Claude"),
            ("Library/Application Support/CrashReporter", "Claude"),
        ]
        .iter()
        .any(|(parent, prefix)| {
            source.path.parent() == Some(root.join(parent).as_path())
                && source
                    .path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with(prefix))
        });
    known || special
}
pub(crate) fn fingerprint(path: &Path) -> Result<Vec<u8>, String> {
    Ok(targets::measure(path)?.2)
}
fn pointer(key: &str) -> String {
    format!("/{}", key.replace('.', "/"))
}
pub(crate) fn extract(source: &Source, text: &str) -> Result<serde_json::Value, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| "身份配置不是有效 JSON。")?;
    if !value.is_object() {
        return Err("身份配置不是对象。".into());
    }
    Ok(serde_json::Value::Object(
        source
            .keys
            .iter()
            .filter_map(|k| value.pointer(&pointer(k)).cloned().map(|v| (k.clone(), v)))
            .collect(),
    ))
}
pub(crate) fn apply_config(source: &Source, saved: &serde_json::Value) -> Result<(), String> {
    if source.path.exists() {
        crate::identities::private::protect(&source.path)?;
    }
    let text = if source.path.exists() {
        fs::read_to_string(&source.path).map_err(|_| "身份配置无法读取。")?
    } else {
        "{}".into()
    };
    let mut current: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "身份配置不是有效 JSON。")?;
    if !current.is_object() {
        return Err("身份配置不是对象。".into());
    }
    for key in &source.keys {
        let mut parts = key.split('.');
        let first = parts.next().unwrap();
        let obj = if let Some(second) = parts.next() {
            let parent = current
                .as_object_mut()
                .unwrap()
                .entry(first)
                .or_insert_with(|| serde_json::json!({}));
            let object = parent.as_object_mut().ok_or("身份配置不是对象。")?;
            if let Some(v) = saved.get(key) {
                object.insert(second.into(), v.clone());
            } else {
                object.remove(second);
            }
            continue;
        } else {
            current.as_object_mut().unwrap()
        };
        if let Some(v) = saved.get(key) {
            obj.insert(first.into(), v.clone());
        } else {
            obj.remove(first);
        }
    }
    let bytes = serde_json::to_vec_pretty(&current).map_err(|_| "身份配置无法保存。")?;
    if source.path.exists() {
        file_io::atomic_write(&source.path, &bytes, 0o600)
    } else {
        crate::identities::private::write(&source.path, &bytes)
    }
}
pub(crate) fn root() -> Result<PathBuf, String> {
    home()
}
pub(crate) fn ensure_stopped() -> Result<(), String> {
    if running()? {
        Err("请先结束 Claude 进程，再切换身份。".into())
    } else {
        Ok(())
    }
}

pub(crate) fn processes_running() -> Result<bool, String> {
    running()
}

pub(crate) fn account_key(key: &str) -> bool {
    config::ACCOUNT_KEYS.contains(&key)
}

// ----- 只检查桌面身份路径元数据，不读取 Cookie 或数据库正文 -----
pub(crate) fn desktop_present(root: &Path) -> bool {
    definitions(root, &Policy::default())
        .into_iter()
        .filter(|s| s.category == "desktop")
        .any(|s| {
            crate::identities::private::parents(s.path.parent().unwrap()).is_ok()
                && crate::identities::private::safe(&s.path).is_ok()
        })
}
