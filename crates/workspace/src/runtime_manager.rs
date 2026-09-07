use std::collections::VecDeque;

use anyhow::Ok;
use futures::{
    StreamExt,
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
};

use gpui::Task;
use protocol::{
    RuntimeCommand, RuntimeEvent, Session, SessionId, TabId, TerminalCommand, TransferId,
    error::{ConnectionError, RuntimeError, SessionError},
    ssh::{SshConnection, TerminalChannel},
};

use russh::ChannelMsg;

use terminal::{
    Content, SessionRuntimeHandle, Terminal, TerminalBounds, new_term, normalize_terminal_bounds,
};
use utils::collections::HashMap;
use vte::ansi::{Processor, StdSyncHandler};

use crate::{
    monitor_store::MonitorRuntimeHandle,
    transfer_store::{SftpRuntimeHandle, TransferRuntimeHandle},
};
