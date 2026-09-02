use crate::{
    runtime_manager::{SessionRuntime, SessionRuntimeHandle},
    state::AppState,
};
use gpui::{Context, Entity, accesskit::Uuid};
use protocol::{Session, SessionId, ssh::SshConnection};
use utils::collections::HashMap;

// pub struct SessionManager {
//     inner: HashMap<String, Session>,
//     pub open_sessions: Vec<String>,
// }
// impl SessionManager {
//     pub fn new() -> Self {
//         let mut map = HashMap::default();
//         let session_1 = Session {
//             id: "7bf80381-a70d-41e3-8056-65c5c2c558aa".to_string(),
//             name: "192.168.117.4".to_string(),
//             group: "linux".to_string(),
//             hostname: "192.168.117.4".to_string(),
//             port: 22,
//             username: "wrw".to_string(),
//             protocol: Protocol::Ssh,
//             auth: AuthMethod::Password {
//                 password: "1006".to_string(),
//             },
//             // identity_file: Some("".to_string()),
//             // status: SessionStatus::Disconnected,
//             latencies_ms: vec![],
//         };
//         let session_2 = Session {
//             id: "7bf80381-a70d-41e3-8056-65c5c2c558ab".to_string(),
//             name: "192.168.117.129".to_string(),
//             group: "linux".to_string(),
//             hostname: "192.168.117.129".to_string(),
//             port: 22,
//             username: "wrw".to_string(),
//             protocol: Protocol::Ssh,
//             auth: AuthMethod::Password {
//                 password: "1234".to_string(),
//             },
//             // identity_file: Some("".to_string()),
//             // status: SessionStatus::Disconnected,
//             latencies_ms: vec![],
//         };

//         map.insert(session_1.id.clone(), session_1);
//         map.insert(session_2.id.clone(), session_2);
//         Self {
//             inner: map,
//             open_sessions: vec![],
//         }
//     }
//     pub fn save_session(&mut self, session: Session, is_connected: bool) {
//         self.inner.insert(session.id.clone(), session);
//     }
//     pub fn del_session(&mut self, session_id: &str) {
//         self.inner.remove(session_id);
//     }
//     pub fn copy_session(&mut self, session_id: &str) {
//         let mut res = self.inner.get(session_id).unwrap().clone();
//         res.id = Uuid::new_v4().to_string();
//         self.inner.insert(res.id.clone(), res);
//     }
//     pub fn list(&self) -> Vec<&Session> {
//         self.inner.values().collect()
//     }
//     /// Group sessions by their group name, in stable display order.
//     pub fn grouped_sessions(&self) -> BTreeMap<String, Vec<Session>> {
//         let mut map: BTreeMap<String, Vec<Session>> = BTreeMap::new();
//         for s in self.list() {
//             map.entry(s.group.clone()).or_default().push(s.clone());
//         }
//         map
//     }
//     pub fn open_session(&mut self, session_id: String) {
//         self.open_sessions.push(session_id);
//     }
//     pub fn connectioned(&self) -> Vec<Session> {
//         let list: Vec<Session> = self
//             .open_sessions
//             .iter()
//             .map(|id| self.inner.get(id).unwrap().clone())
//             .collect();

//         list
//     }
//     pub fn query(&self, session_id: &str) -> Option<&Session> {
//         self.inner.get(session_id)
//     }
// }

pub struct SessionManager {
    sessions: HashMap<SessionId, Session>,
    state: Entity<AppState>,
    runtimes: HashMap<SessionId, SessionRuntime>,
}

impl SessionManager {
    pub fn new(state: Entity<AppState>, cx: &Context<Self>) -> Self {
        let sessions = state.read(cx).sessions.clone();
        let sessions = sessions
            .into_iter()
            .map(|session| (session.id.clone(), session))
            .collect::<HashMap<_, _>>();
        Self {
            sessions,
            state,
            runtimes: HashMap::default(),
        }
    }
    // pub async fn connect(&mut self, session: Session) -> anyhow::Result<SessionRuntimeHandle> {
    //     let connection = SshConnection::connect(&session).await?;

    //     let (runtime, handle, _) = SessionRuntime::new(session.clone(), connection);

    //     self.runtimes.insert(session.id.clone(), runtime);
    //     // tokio::spawn(runtime.run());

    //     Ok(handle)
    // }
    fn sync_state(&self, cx: &mut Context<Self>) {
        let sessions = self.sessions.clone();
        self.state.update(cx, |state, cx| {
            state.sessions = sessions;
            state.groups = state
                .sessions
                .iter()
                .map(|session| session.group.trim().to_string())
                .filter(|group| !group.is_empty())
                .collect();
            state.groups.sort();
            state.groups.dedup();
            state.save();
            cx.notify();
        });
    }
    pub fn list(&self) -> &[Session] {
        &self.sessions
    }

    pub fn query(&self, session_id: SessionId) -> Option<&Session> {
        self.sessions
            .iter()
            .find(|session| session.id == session_id)
    }

    pub fn query_mut(&mut self, session_id: SessionId) -> Option<&mut Session> {
        self.sessions
            .iter_mut()
            .find(|session| session.id == session_id)
    }

    pub fn del_session(&mut self, session_id: SessionId, cx: &mut Context<Self>) -> bool {
        let old_len = self.sessions.len();

        self.sessions.retain(|session| session.id != session_id);

        let deleted = old_len != self.sessions.len();
        if deleted {
            self.sync_state(cx);
        }
        deleted
    }

    pub fn copy_session(
        &mut self,
        session_id: SessionId,
        cx: &mut Context<Self>,
    ) -> Option<SessionId> {
        let source = self.query(session_id)?.clone();

        let new_id = SessionId::new();

        let mut copied = source;

        copied.id = new_id;

        copied.name = format!("{} Copy", copied.name);

        // copied.status = protocol::SessionStatus::Disconnected;

        copied.latencies_ms = Vec::new();

        self.sessions.push(copied);
        self.sync_state(cx);

        Some(new_id)
    }

    pub fn open_session(&mut self, session_id: SessionId) {
        // 这里不要启动真正的 SSH 连接。
        //
        // SessionManager 只负责管理 session 生命周期。
        //
        // 真正连接应该进入：
        //
        // SessionManager
        //     -> Actor
        //     -> SSH Task
        //
        // if let Some(session) = self.query_mut(&session_id) {
        // session.status = protocol::SessionStatus::Disconnected;
        // }
    }

    pub fn save_session(&mut self, session: Session, is_connect: bool, cx: &mut Context<Self>) {
        let id = session.id.clone();

        match self.query_mut(id) {
            Some(existing) => {
                *existing = session;
            }

            None => {
                self.sessions.push(session);
            }
        }

        if is_connect {
            self.state.update(cx, |state, cx| {
                state.pending_open_session_id = Some(id);
                cx.notify();
            });
        }

        self.sync_state(cx);

        // TODO:
        //
        // persistence.save(&self.sessions)
    }

    pub fn grouped_sessions(&self) -> Vec<(String, Vec<Session>)> {
        let mut groups: HashMap<String, Vec<Session>> = HashMap::default();

        for session in &self.sessions {
            let group = if session.group.trim().is_empty() {
                "Default".to_string()
            } else {
                session.group.clone()
            };

            groups.entry(group).or_default().push(session.clone());
        }

        let mut result = groups.into_iter().collect::<Vec<_>>();

        result.sort_by(|a, b| a.0.cmp(&b.0));

        for (_, sessions) in &mut result {
            sessions.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }

        result
    }
}
