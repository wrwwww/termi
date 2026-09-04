use protocol::{SessionId, TransferId};
pub struct TransferStore {}
impl TransferStore {
    pub fn new() -> Self {
        Self {}
    }
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
pub enum TransferStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}
pub struct SftpRuntimeHandle {}
