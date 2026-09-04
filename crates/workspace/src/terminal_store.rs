use gpui::{Entity, SharedString};
use protocol::{SessionId, TabId};
use terminal::Terminal;
use utils::collections::HashMap;

pub struct TerminalStore {
    terminals: HashMap<TabId, TerminalEntry>,
}
impl TerminalStore {
    pub fn new() -> Self {
        Self {
            terminals: HashMap::default(),
        }
    }
    pub fn get(&self, tab_id: &TabId) -> Option<&TerminalEntry> {
        self.terminals.get(tab_id)
    }
    pub fn insert(&mut self, terminal: TerminalEntry) {
        self.terminals.insert(terminal.tab_id, terminal);
    }
}

pub struct TerminalEntry {
    pub tab_id: TabId,
    pub session_id: SessionId,
    pub title: SharedString,
    pub runtime: Entity<Terminal>,
}
