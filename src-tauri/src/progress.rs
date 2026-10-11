// [INPUT]: 原生操作的真实阶段、完成数量与 Tauri 单次调用通道。
// [OUTPUT]: 有序脱敏进度和内存中的有界活动记录。
// [POS]: 只观察事务，供 Claude 清理与浏览器操作共享，不改变执行；不读取文件正文、凭据或配置值。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde::Serialize;
use std::sync::Mutex;
use std::time::Instant;
use tauri::ipc::Channel;
#[derive(Clone, Serialize)]
pub struct Progress {
    stage: String,
    status: String,
    message: String,
    current: usize,
    total: Option<usize>,
    elapsed_ms: u64,
}
pub(crate) struct Reporter {
    started: Instant,
    channel: Option<Channel<Progress>>,
    events: Mutex<Vec<Progress>>,
}
impl Reporter {
    pub(crate) fn channel(channel: Option<Channel<Progress>>) -> Self {
        Self {
            started: Instant::now(),
            channel,
            events: Mutex::new(Vec::new()),
        }
    }
    pub(crate) fn log(
        &self,
        stage: &str,
        status: &str,
        message: &str,
        current: usize,
        total: Option<usize>,
    ) {
        let event = Progress {
            stage: stage.into(),
            status: status.into(),
            message: message.into(),
            current,
            total,
            elapsed_ms: self.started.elapsed().as_millis() as u64,
        };
        let mut events = self.events.lock().unwrap();
        if events.len() == 2000 {
            events.remove(0);
        }
        events.push(event.clone());
        if let Some(channel) = &self.channel {
            let _ = channel.send(event);
        }
    }
    pub(crate) fn fail(&self, message: &str) {
        let stage = self
            .events
            .lock()
            .unwrap()
            .last()
            .map(|event| event.stage.clone())
            .unwrap_or_else(|| "validate".into());
        self.log(&stage, "error", message, 0, None);
    }
    #[cfg(test)]
    pub(crate) fn events(&self) -> Vec<Progress> {
        self.events.lock().unwrap().clone()
    }
}
#[cfg(test)]
impl Progress {
    pub(crate) fn stage(&self) -> &str {
        &self.stage
    }
    pub(crate) fn status(&self) -> &str {
        &self.status
    }
    pub(crate) fn current(&self) -> usize {
        self.current
    }
    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}
