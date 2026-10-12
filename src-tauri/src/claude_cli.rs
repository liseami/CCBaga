// [INPUT]: 平台内嵌 claude-cli.sh/ps1、当前用户平台环境、清理事务互斥锁及真实进度通道。
// [OUTPUT]: Claude Code CLI 只读安装状态、一键卸载/重装与版本核验，配置和个人资产不在删除范围。
// [POS]: 本机程序安装边界；白名单动作由原生层限定，进程未退出时拒绝修改，输出不暴露包管理器原始诊断。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use crate::{
    cleaner::Cleaner,
    progress::{Progress, Reporter},
};
use serde::Serialize;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::io::Write;
use std::{
    io::{BufRead, BufReader, Read},
    path::PathBuf,
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::{ipc::Channel, Manager};
#[cfg(not(windows))]
const SCRIPT: &str = include_str!("../../scripts/claude-cli.sh");
#[cfg(windows)]
const SCRIPT: &str = include_str!("../../scripts/claude-cli.ps1");
#[derive(Debug, Default, Serialize)]
pub struct CliStatus {
    method: String,
    path: String,
    version: String,
}
fn execute(mut command: Command, log: Arc<Reporter>, seconds: u64) -> Result<CliStatus, String> {
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "无法启动 Claude Code 安装任务。")?;
    #[cfg(windows)]
    let job = match crate::windows::ChildJob::attach(&child) { Ok(job) => job, Err(error) => { let _ = child.kill(); let _ = child.wait(); return Err(error); } };
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(SCRIPT.trim_start_matches('\u{feff}').as_bytes()));
    let output = std::thread::spawn(move || {
        let mut result = CliStatus::default();
        let mut completed = false;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(message) = line.strip_prefix("CC_CLI_STEP=") {
                log.log("cli", "running", message, 0, None);
            }
            if let Some(value) = line.strip_prefix("CC_CLI_METHOD=") {
                result.method = value.into();
            }
            if let Some(value) = line.strip_prefix("CC_CLI_PATH=") {
                result.path = value.into();
            }
            if let Some(value) = line.strip_prefix("CC_CLI_VERSION=") {
                result.version = value.into();
            }
            if line == "CC_CLI_DONE=1" {
                completed = true;
            }
        }
        (result, completed)
    });
    let errors = std::thread::spawn(move || {
        let mut pipe = stderr;
        let mut message = Vec::new();
        let mut bytes = [0; 4096];
        while let Ok(n) = pipe.read(&mut bytes) {
            if n == 0 {
                break;
            }
            message.extend_from_slice(&bytes[..n]);
            if message.len() > 65536 {
                message.drain(..message.len() - 65536);
            }
        }
        String::from_utf8_lossy(&message).into_owned()
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if started.elapsed() < Duration::from_secs(seconds) => {
                std::thread::sleep(Duration::from_millis(100))
            }
            _ => {
                // 安装器可能带起 curl/npm；超时结束整个本次子进程组，防止后台继续修改。
                #[cfg(windows)]
                job.stop();
                #[cfg(unix)]
                unsafe {
                    kill(-(child.id() as i32), 9);
                }
                let _ = child.wait();
                break Err("Claude Code 安装任务超时，操作可能已部分完成，请重试。".to_string());
            }
        }
    };
    #[cfg(windows)]
    job.stop();
    let _ = writer.join();
    let result = output.join().map_err(|_| "Claude Code 安装任务被中断。")?;
    let errors = errors.join().map_err(|_| "Claude Code 安装任务被中断。")?;
    if !status?.success() {
        // 只显示脚本自有的固定错误文案，不泄漏 shell/npm/Homebrew 输出中的配置或凭据。
        let message = errors.lines().find(|line| {
            line.starts_with("Claude Code ")
                || line.starts_with("npm ")
                || line.starts_with("Homebrew ")
                || line.starts_with("无法读取 ")
                || line.starts_with("发现未识别的 ")
                || line.starts_with("仍有其他 ")
        });
        return Err(message
            .unwrap_or("Claude Code 安装任务失败，请检查终端环境和安装目录权限。")
            .into());
    }
    if !result.1 {
        return Err("Claude Code 安装状态回报不完整，请检查终端环境。".into());
    }
    Ok(result.0)
}
#[cfg(unix)]
extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}
fn command(action: &str) -> Result<Command, String> {
    #[cfg(windows)]
    {
        let user_dir = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .ok_or("无法定位用户目录。")?;
        let root = dunce::canonicalize(PathBuf::from(user_dir))
            .map_err(|_| "无法读取用户目录。")?;
        use std::os::windows::process::CommandExt;
        let shell = PathBuf::from(std::env::var_os("SystemRoot").ok_or("无法定位 Windows 系统目录。")?)
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut command = Command::new(shell);
        command.creation_flags(0x08000000)
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command"])
            .arg(format!("[Console]::InputEncoding = New-Object System.Text.UTF8Encoding; & ([scriptblock]::Create([Console]::In.ReadToEnd())) '{action}'"))
            .env("CC_CLI_USER_DIR", root);
        Ok(command)
    }
    #[cfg(not(windows))]
    {
    let root = std::fs::canonicalize(PathBuf::from(
        std::env::var_os("HOME").ok_or("无法定位用户目录。")?,
    ))
    .map_err(|_| "无法读取用户目录。")?;
    let mut command = Command::new("/bin/zsh");
    command
        .args(["-ilc", "source /dev/stdin", "cctool-cli", action])
        .env("CC_CLI_USER_DIR", root)
        .current_dir("/");
    Ok(command)
    }
}
#[tauri::command]
pub async fn claude_cli_read() -> Result<CliStatus, String> {
    tauri::async_runtime::spawn_blocking(|| {
        execute(command("read")?, Arc::new(Reporter::channel(None)), 60)
    })
    .await
    .map_err(|_| "Claude Code 安装任务被中断。".to_string())?
}
#[tauri::command]
pub async fn claude_cli_manage(
    app: tauri::AppHandle,
    action: String,
    on_progress: Channel<Progress>,
) -> Result<CliStatus, String> {
    if !matches!(action.as_str(), "uninstall" | "reinstall") {
        return Err("无效的 Claude Code 安装操作。".into());
    }
    let state = app.state::<Cleaner>();
    state.enter()?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let log = Arc::new(Reporter::channel(Some(on_progress)));
        let result = (|| {
            if crate::cleaner::cli_running()? {
                return Err("Claude Code 仍在运行，请先结束进程。".into());
            }
            execute(command(&action)?, log.clone(), 600)
        })();
        if let Err(error) = &result {
            log.fail(error);
        }
        result
    })
    .await
    .map_err(|_| "Claude Code 安装任务被中断。".to_string())
    .and_then(|result| result);
    state.leave();
    result
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{symlink, PermissionsExt},
    };
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "cctool-cli-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(path.join("bin")).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }
        fn file(&self, name: &str, text: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, text).unwrap();
            path
        }
        fn executable(&self, name: &str, text: &str) -> PathBuf {
            let path = self.file(name, text);
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            path
        }
        fn run(&self, action: &str) -> Result<CliStatus, String> {
            let mut command = Command::new("/bin/zsh");
            command
                .env_clear()
                .args(["-fc", "PATH=\"$CC_TEST_BIN\"; source /dev/stdin", "fixture", action])
                .env("HOME", &self.0)
                .env("ZDOTDIR", &self.0)
                .env("LANG", "en_US.UTF-8")
                .env("CC_CLI_USER_DIR", &self.0)
                .env("PATH", self.0.join("bin"))
                .env("CC_TEST_ROOT", self.0.join("lib/node_modules"))
                .env("CC_TEST_BINARY", self.0.join("fake-cli"))
                .env("CC_TEST_BIN", self.0.join("bin"));
            execute(command, Arc::new(Reporter::channel(None)), 10)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn native_uninstall_preserves_assets_and_rejects_custom_launchers() {
        let fixture = Fixture::new();
        fixture.file(".claude/settings.json", "personal-settings");
        fixture.file(".claude/skills/custom/SKILL.md", "personal-skill");
        let binary = fixture.executable(
            ".local/share/claude/versions/2.0",
            "#!/bin/sh\nprintf '2.0 (Claude Code)\\n'\n",
        );
        fs::create_dir_all(fixture.0.join(".local/bin")).unwrap();
        symlink(&binary, fixture.0.join(".local/bin/claude")).unwrap();
        assert_eq!(fixture.run("read").unwrap().method, "native");
        assert!(fixture.run("uninstall").unwrap().method.is_empty());
        assert!(!binary.exists());
        assert_eq!(
            fs::read_to_string(fixture.0.join(".claude/settings.json")).unwrap(),
            "personal-settings"
        );
        assert!(fixture.0.join(".claude/skills/custom/SKILL.md").exists());
        let custom = fixture.executable(".local/bin/claude", "#!/bin/sh\nexit 0\n");
        assert!(fixture.run("uninstall").unwrap_err().contains("自定义"));
        assert!(custom.exists());
    }
    #[test]
    fn npm_reinstall_verifies_version_and_uninstall_is_real() {
        let fixture = Fixture::new();
        fixture.file(
            "lib/node_modules/@anthropic-ai/claude-code/package.json",
            "{\"name\":\"@anthropic-ai/claude-code\"}",
        );
        fixture.executable("fake-cli", "#!/bin/sh\nprintf '3.0 (Claude Code)\\n'\n");
        fixture.executable("bin/claude", "#!/bin/sh\nprintf '2.0 (Claude Code)\\n'\n");
        fixture.executable("bin/npm",r#"#!/bin/sh
case "$1" in
root) printf '%s\n' "$CC_TEST_ROOT" ;;
uninstall) /bin/rm -rf "$CC_TEST_ROOT/@anthropic-ai/claude-code"; /bin/rm -f "$CC_TEST_BIN/claude" ;;
install) /bin/mkdir -p "$CC_TEST_ROOT/@anthropic-ai/claude-code"; printf '{}' > "$CC_TEST_ROOT/@anthropic-ai/claude-code/package.json"; /bin/cp "$CC_TEST_BINARY" "$CC_TEST_BIN/claude" ;;
*) exit 1 ;;
esac
"#);
        assert_eq!(fixture.run("read").unwrap().version, "2.0 (Claude Code)");
        let result = fixture.run("reinstall").unwrap();
        assert_eq!(result.method, "npm");
        assert_eq!(result.version, "3.0 (Claude Code)");
        fixture.run("uninstall").unwrap();
        assert!(!fixture.0.join("bin/claude").exists());
        assert!(fixture.run("read").unwrap().method.is_empty());
    }
    #[test]
    fn failed_install_and_timeout_cannot_report_success() {
        let fixture = Fixture::new();
        fixture.file(
            "lib/node_modules/@anthropic-ai/claude-code/package.json",
            "{}",
        );
        fixture.executable(
            "bin/npm",
            "#!/bin/sh\ncase \"$1\" in root) printf '%s\\n' \"$CC_TEST_ROOT\";; *) exit 1;; esac\n",
        );
        assert!(fixture.run("reinstall").is_err());
        let mut command = Command::new("/bin/zsh");
        command.args(["-fc", "/bin/sleep 10"]);
        let began = Instant::now();
        assert!(execute(command, Arc::new(Reporter::channel(None)), 0)
            .unwrap_err()
            .contains("超时"));
        assert!(began.elapsed() < Duration::from_secs(3));
    }
}
