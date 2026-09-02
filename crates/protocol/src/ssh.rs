use std::sync::Arc;

use futures::lock::Mutex;
use russh::{
    Channel,
    client::{Handle, Msg},
};

use crate::{ClientHandler, Session};

pub struct SshConnection {
    handle: Arc<Mutex<Handle<ClientHandler>>>,
}
impl SshConnection {
    pub fn connect(session: &Session) -> anyhow::Result<Self> {
        let config = russh::client::Config {
            // connection_timeout: Some(std::time::Duration::from_secs(10)),
            ..Default::default()
        };
        let config = Arc::new(config);
        let sh = ClientHandler {};
        let handle = russh::client::connect(config, session.address.clone(), sh)?;
        Ok(Self {
            handle: Arc::new(Mutex::new(handle)),
        })
    }
}
pub struct TerminalChannel {
    channel: Channel<Msg>,
}
