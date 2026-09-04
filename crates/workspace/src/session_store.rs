use gpui::Context;
use protocol::{AuthMethod, Protocol, Session, SessionId, SessionStatus};
use utils::collections::HashMap;

pub struct SessionStore {
    sessions: HashMap<SessionId, Session>,
}

impl SessionStore {
    pub fn new(cx: &Context<Self>) -> Self {
        // let sessions = state.read(cx).sessions.clone();
        // let sessions = sessions
        //     .into_iter()
        //     .map(|session| (session.id.clone(), session))
        //     .collect::<HashMap<_, _>>();
        let session_1 = Session {
            id: SessionId::new(),
            name: "192.168.117.4".to_string(),
            group: "linux".to_string(),
            hostname: "192.168.117.4".to_string(),
            port: 22,
            username: "wrw".to_string(),
            protocol: Protocol::Ssh,
            auth: AuthMethod::Password {
                password: "1006".to_string(),
            },
            // identity_file: Some("".to_string()),
            status: SessionStatus::Disconnected,
            latencies_ms: vec![],
        };
        let session_2 = Session {
            id: SessionId::new(),
            name: "192.168.117.129".to_string(),
            group: "linux".to_string(),
            hostname: "192.168.117.129".to_string(),
            port: 22,
            username: "wrw".to_string(),
            protocol: Protocol::Ssh,
            auth: AuthMethod::Password {
                password: "1234".to_string(),
            },
            // identity_file: Some("".to_string()),
            status: SessionStatus::Disconnected,
            latencies_ms: vec![],
        };
        let mut map = HashMap::default();
        map.insert(session_1.id, session_1);
        map.insert(session_2.id, session_2);
        Self { sessions: map }
    }

    // fn sync_state(&self, cx: &mut Context<Self>) {
    //     let sessions = self.list();
    //     self.state.update(cx, |state, cx| {
    //         state.sessions = sessions;
    //         state.groups = state
    //             .sessions
    //             .iter()
    //             .map(|session| session.group.trim().to_string())
    //             .filter(|group| !group.is_empty())
    //             .collect();
    //         state.groups.sort();
    //         state.groups.dedup();
    //         // state.save();
    //         cx.notify();
    //     });
    // }
    pub fn list(&self) -> Vec<Session> {
        self.sessions.values().cloned().collect::<Vec<_>>()
    }

    pub fn query(&self, session_id: SessionId) -> Option<&Session> {
        self.sessions.get(&session_id)
    }

    pub fn query_mut(&mut self, session_id: SessionId) -> Option<&mut Session> {
        self.sessions.get_mut(&session_id)
    }

    pub fn del_session(&mut self, session_id: SessionId, cx: &mut Context<Self>) -> bool {
        let old_len = self.sessions.len();

        self.sessions.remove(&session_id);

        let deleted = old_len != self.sessions.len();
        if deleted {
            // self.sync_state(cx);
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

        copied.status = protocol::SessionStatus::Disconnected;

        copied.latencies_ms = Vec::new();

        self.sessions.insert(new_id, copied);
        // self.sync_state(cx);

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
        // self.query(session_id).map(|session| {
        //     let _ = SessionRuntimeHandle::start(session.clone());
        // });
    }

    pub fn save_session(&mut self, session: Session, is_connect: bool, cx: &mut Context<Self>) {
        let id = session.id.clone();

        match self.query_mut(id) {
            Some(existing) => {
                *existing = session;
            }

            None => {
                self.sessions.insert(id, session);
            }
        }

        if is_connect {
            // self.state.update(cx, |state, cx| {
            //     state.pending_open_session_id = Some(id);
            //     cx.notify();
            // });
        }

        // self.sync_state(cx);

        // TODO:
        //
        // persistence.save(&self.sessions)
    }

    pub fn grouped_sessions(&self) -> Vec<(String, Vec<Session>)> {
        let mut groups: HashMap<String, Vec<Session>> = HashMap::default();

        for session in self.sessions.values() {
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
