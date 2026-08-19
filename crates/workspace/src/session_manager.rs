use std::collections::BTreeMap;

use gpui::accesskit::Uuid;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};
use utils::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub group: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub protocol: Protocol,
    pub auth: AuthMethod,
    pub identity_file: Option<String>,
    pub status: SessionStatus,
    pub latencies_ms: Vec<u32>, // rolling window, last 60 samples
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, EnumString, Display, EnumIter,
)]
pub enum Protocol {
    Ssh,
    Mosh,
    Telnet,
    Local,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthMethod {
    Key { path: String },
    Password,
    Agent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Connected,
    Idle,
    Disconnected,
    Error,
}
pub struct SessionManager {
    inner: HashMap<String, Session>,
    pub open_sessions: Vec<String>,
}
impl SessionManager {
    pub fn new() -> Self {
        let mut map = HashMap::default();
        let session_1 = Session {
            id: "7bf80381-a70d-41e3-8056-65c5c2c558aa".to_string(),
            name: "192.168.117.134".to_string(),
            group: "linux".to_string(),
            host: "192.168.117.134".to_string(),
            port: 22,
            username: "wrw".to_string(),
            protocol: Protocol::Ssh,
            auth: AuthMethod::Password,
            identity_file: Some("".to_string()),
            status: SessionStatus::Disconnected,
            latencies_ms: vec![],
        };
        map.insert(
            "7bf80381-a70d-41e3-8056-65c5c2c558aa".to_string(),
            session_1,
        );
        Self {
            inner: map,
            open_sessions: vec![],
        }
    }
    pub fn save_session(&mut self, session: Session, is_connected: bool) {
        self.inner.insert(session.id.clone(), session);
    }
    pub fn del_session(&mut self, session_id: &str) {
        self.inner.remove(session_id);
    }
    pub fn copy_session(&mut self, session_id: &str) {
        let mut res = self.inner.get(session_id).unwrap().clone();
        res.id = Uuid::new_v4().to_string();
        self.inner.insert(res.id.clone(), res);
    }
    pub fn list(&self) -> Vec<&Session> {
        self.inner.values().collect()
    }
    /// Group sessions by their group name, in stable display order.
    pub fn grouped_sessions(&self) -> BTreeMap<String, Vec<Session>> {
        let mut map: BTreeMap<String, Vec<Session>> = BTreeMap::new();
        for s in self.list() {
            map.entry(s.group.clone()).or_default().push(s.clone());
        }
        map
    }
    pub fn open_session(&mut self, session_id: String) {
        self.open_sessions.push(session_id);
    }
    pub fn connectioned(&self) -> Vec<Session> {
        let list: Vec<Session> = self
            .open_sessions
            .iter()
            .map(|id| self.inner.get(id).unwrap().clone())
            .collect();

        list
    }
    pub fn query(&self, session_id: &str) -> Option<&Session> {
        self.inner.get(session_id)
    }
}
