 
pub struct TransferStore {}
impl TransferStore {
    pub fn new() -> Self {
        Self {}
    }
}

pub enum TransferStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}
