use gpui::SharedString;
use protocol::{SessionId, TabId};
use utils::collections::HashMap;

pub struct TerminalStore {
    terminals: HashMap<TabId, TerminalEntry>,
}
impl TerminalStore {
    pub fn get(&self, tab_id: &TabId) -> Option<&TerminalEntry> {
        self.terminals.get(tab_id)
    }
}

pub struct TerminalEntry {
    pub tab_id: TabId,
    pub session_id: SessionId,
    pub title: SharedString,
    // pub runtime: TerminalRuntimeHandle,
}
