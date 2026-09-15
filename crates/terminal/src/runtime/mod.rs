use std::collections::HashMap;

use futures::channel::mpsc::UnboundedSender;
use gpui::Task;
use log::info;

use crate::{
    SessionRuntimeHandle,
    id::{SessionId, TabId, TransferId},
    runtime::event::{RuntimeEvent, TerminalCommand},
    session::{SessionRuntime, session::Session},
};

pub mod event;
// ============================================================
// RuntimeManager
// ============================================================

pub struct RuntimeManager {
    runtimes: HashMap<SessionId, SessionRuntimeHandle>,
    event_tx: UnboundedSender<RuntimeEvent>,
    event_loop_task: Task<Result<(), anyhow::Error>>,
    tokio_runtime: tokio::runtime::Runtime,
}

impl RuntimeManager {
    pub fn new(event_tx: UnboundedSender<RuntimeEvent>) -> Self {
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        Self {
            runtimes: HashMap::default(),
            event_tx,
            event_loop_task: Task::ready(Ok(())),
            tokio_runtime,
        }
    }

    pub fn open_session(
        &mut self,
        session: Session,
    ) -> anyhow::Result<(TabId, SessionRuntimeHandle)> {
        let runtime = self.get_or_create(session.clone())?;

        let tab_id = TabId::new();

        runtime.open_terminal(tab_id.clone());

        Ok((tab_id, runtime))
    }
    pub fn get_or_create(&mut self, session: Session) -> anyhow::Result<SessionRuntimeHandle> {
        info!("get or create");
        let session_id = session.id.clone();
        if let None = self.runtimes.get(&session_id) {
            let (runtime, handle) = SessionRuntime::new(session.clone(), self.event_tx.clone());
            self.runtimes.insert(session_id, handle);

            // 启动 Runtime 主循环
            self.tokio_runtime.spawn(async move {
                runtime.run().await;
            });
        }
        if let Some(handle) = self.runtimes.get(&session_id) {
            return Ok(handle.clone());
        }
        anyhow::bail!("Session not found")
    }

    pub fn get(&self, session_id: &SessionId) -> Option<SessionRuntimeHandle> {
        self.runtimes.get(session_id).cloned()
    }

    pub fn remove(&mut self, session_id: &SessionId) {
        if let Some(handle) = self.runtimes.remove(session_id) {
            handle.disconnect();
        }
    }
}
// ============================================================
// TerminalHandle
//
// Runtime / UI 只通过这个 Handle 控制 Terminal。
// 它不拥有 SSH Channel。
// ============================================================

#[derive(Clone)]
pub struct TerminalHandle {
    cmd_tx: UnboundedSender<TerminalCommand>,
}

impl TerminalHandle {
    pub fn new(cmd_tx: UnboundedSender<TerminalCommand>) -> Self {
        Self { cmd_tx }
    }

    pub fn input(&self, data: Vec<u8>) {
        let _ = self.cmd_tx.unbounded_send(TerminalCommand::Input { data });
    }
    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self
            .cmd_tx
            .unbounded_send(TerminalCommand::Resize { cols, rows });
    }

    pub fn close(&self) {
        let _ = self.cmd_tx.unbounded_send(TerminalCommand::Close);
    }
}
pub struct MonitorRuntimeHandle {
    running: bool,
}
pub struct TransferRuntimeHandle {
    pub id: TransferId,

    pub session_id: SessionId,

    pub direction: TransferDirection,

    pub source: String,

    pub destination: String,

    pub transferred: u64,

    pub total: Option<u64>,
}
pub enum TransferDirection {
    Upload,
    Download,
}
pub struct SftpRuntimeHandle {}
