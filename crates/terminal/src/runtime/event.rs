use protocol::{file::RemoteFile, monitor::MetricSnapshot};

use crate::{
    error::{RuntimeError, TerminalError},
    id::{SessionId, TabId, TransferId},
    session::session::SessionStatus,
};
#[derive(Debug)]
pub enum RuntimeCommand {
    Terminal {
        tab_id: TabId,
        command: TerminalCommand,
    },
    Monitor(MonitorCommand),
    Files(FileCommand),
    Disconnect,
}
#[derive(Debug)]
pub enum RuntimeEvent {
    // =========================
    // Runtime
    // =========================
    Connected {
        session_id: SessionId,
    },

    Disconnected,

    Error {
        error: RuntimeError,
    },

    // =========================
    // Terminal
    // =========================
    TerminalOutput {
        tab_id: TabId,
        bytes: Vec<u8>,
    },

    TerminalExit {
        tab_id: TabId,
    },

    // =========================
    // Monitor
    // =========================
    MetricsUpdated {
        metrics: MetricSnapshot,
    },

    // =========================
    // File
    // =========================
    DirectoryListed {
        path: String,
        entries: Vec<RemoteFile>,
    },

    // =========================
    // Transfer
    // =========================
    TransferStarted {
        transfer_id: TransferId,
    },

    TransferProgress {
        transfer_id: TransferId,
        transferred: u64,
        total: Option<u64>,
    },

    TransferCompleted {
        transfer_id: TransferId,
    },

    TransferFailed {
        transfer_id: TransferId,
        message: String,
    },
}

#[derive(Debug)]
pub enum TerminalCommand {
    Open,

    Input { data: Vec<u8> },

    Resize { cols: u16, rows: u16 },

    Close,
}

#[derive(Debug)]
pub enum MonitorCommand {
    Start,
    Stop,
    Refresh,
}

#[derive(Debug)]
pub enum FileCommand {
    List { path: String },

    Mkdir { path: String },

    Delete { path: String },

    Rename { from: String, to: String },
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

// ============================================================
// Terminal
// ============================================================

#[derive(Debug, Clone)]
pub enum TerminalStatus {
    /// 正在创建 SSH Channel / PTY / Shell
    Creating,

    /// Terminal 正常运行
    Running,

    /// Terminal 创建或运行过程中发生错误
    Failed(TerminalError),

    /// Terminal 已正常退出
    Exited { code: Option<i32> },
}
