use std::sync::Arc;

use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    lock::Mutex,
};
use protocol::{
    RuntimeCommand, RuntimeEvent, Session, SessionId, TabId, TerminalCommand, TransferId,
    ssh::{SshConnection, TerminalChannel},
};
use utils::collections::HashMap;

use crate::{monitor_manager::MonitorRuntime, transfer_manager::TransferRuntime};

pub struct RuntimeManager {
    runtimes: HashMap<SessionId, SessionRuntimeHandle>,
}

#[derive(Clone)]
pub struct SessionRuntimeHandle {
    tx: UnboundedSender<RuntimeCommand>,
}
impl SessionRuntimeHandle {
    pub fn new(tx: UnboundedSender<RuntimeCommand>) -> Self {
        Self { tx }
    }
    pub fn terminal_input(&self, tab_id: TabId, data: Vec<u8>) {
        let _ = self
            .tx
            .unbounded_send(RuntimeCommand::Terminal(TerminalCommand::Input {
                tab_id,
                data,
            }));
    }

    pub fn terminal_resize(&self, tab_id: TabId, cols: u16, rows: u16) {
        let _ = self
            .tx
            .unbounded_send(RuntimeCommand::Terminal(TerminalCommand::Resize {
                tab_id,
                cols,
                rows,
            }));
    }

    pub fn start_monitor(&self) {
        let _ = self
            .tx
            .unbounded_send(RuntimeCommand::Monitor(protocol::MonitorCommand::Start));
    }

    pub fn stop_monitor(&self) {
        let _ = self
            .tx
            .unbounded_send(RuntimeCommand::Monitor(protocol::MonitorCommand::Stop));
    }

    pub fn list_directory(&self, path: impl Into<String>) {
        let _ = self
            .tx
            .unbounded_send(RuntimeCommand::Files(protocol::FileCommand::List {
                path: path.into(),
            }));
    }

    pub fn disconnect(&self) {
        let _ = self.tx.unbounded_send(RuntimeCommand::Disconnect);
    }
}
pub struct SessionRuntime {
    session: Session,

    command_rx: UnboundedReceiver<RuntimeCommand>,

    event_tx: UnboundedSender<RuntimeEvent>,

    connection: Option<SshConnection>,

    terminals: HashMap<TabId, TerminalChannel>,

    monitor: Option<MonitorRuntime>,

    transfers: HashMap<TransferId, TransferRuntime>,
}
impl SessionRuntime {
    pub fn new(
        session: Session,
        connection: SshConnection,
    ) -> (
        Self,
        // UnboundedSender<RuntimeCommand>,
        SessionRuntimeHandle,
        UnboundedReceiver<RuntimeEvent>,
    ) {
        let (command_tx, command_rx) = futures::channel::mpsc::unbounded();
        let (event_tx, event_rx) = futures::channel::mpsc::unbounded();
        let runtime = Self {
            session,
            command_rx,
            event_tx,
            connection: Some(connection),
            terminals: HashMap::default(),
            monitor: None,
            transfers: HashMap::default(),
        };
        let handle = SessionRuntimeHandle::new(command_tx);
        (runtime, handle, event_rx)
    }
}
