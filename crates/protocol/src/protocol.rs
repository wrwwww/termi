use std::sync::Arc;
pub mod ssh;
use anyhow::Context;

use russh::{
    ChannelMsg,
    client::{self},
    keys::ssh_key,
};
use tokio::sync::{
    Mutex,
    mpsc::{self, Sender, UnboundedReceiver},
};

use crate::ssh::{AuthMethod, Session};
pub type TabId = String;
#[derive(Clone)]
pub enum SystemEvent {
    Output { tab_id: TabId, bytes: Vec<u8> },
    Status { tab_id: String, text: String },
    Connected { tab_id: TabId },
    Error(String),
    CommandComplete(String),
    TitleUpdate { tab_id: TabId, title: String },
    ClearScreen,
    ProcessStarted(u32),
    ProcessTerminated,
}

pub enum SshMessage {
    Input(Vec<u8>),
    Resize(u16, u16),
    Disconnect,
}

#[derive(Clone)]
pub enum BackendTx {
    Local(Sender<SshMessage>),
    Ssh(tokio::sync::mpsc::UnboundedSender<SshMessage>),
    Serial(tokio::sync::mpsc::UnboundedSender<SshMessage>),
}

impl BackendTx {
    pub fn send(&self, command: SshMessage) {
        match self {
            Self::Local(tx) => {
                let _ = tx.send(command);
            }
            Self::Ssh(tx) => {
                let _ = tx.send(command);
            }
            Self::Serial(tx) => {
                let _ = tx.send(command);
            }
        }
    }
}

pub fn open_session_terminal(
    events: std::sync::mpsc::Sender<SystemEvent>,
    session: Session,
    tab_id: TabId,
    mut cmd_rx: UnboundedReceiver<SshMessage>,
) {
    tokio::spawn(async move {
        // connect
        let addr = format!("{}:{}", session.config.hostname, session.config.port);
        let stream = tokio::net::TcpStream::connect(&addr)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP proxy connection failed: {}", e))
            .unwrap();
        let config = Arc::new(client::Config::default());
        let mut handle = client::connect_stream(config, stream, ClientHandler)
            .await
            .unwrap();

        // auth
        let a = match session.config.auth_method {
            AuthMethod::Password {
                remember,
                username,
                password,
            } => handle
                .authenticate_password(username, password)
                .await
                .context("context")
                .unwrap(),
            AuthMethod::PublicKey {
                private_key,
                passphrase_secret_id,
            } => handle
                .authenticate_password("".to_string(), "".to_string())
                .await
                .context("context")
                .unwrap(),
            AuthMethod::KeyboardInteractive => handle
                .authenticate_password("".to_string(), "".to_string())
                .await
                .context("context")
                .unwrap(),
            AuthMethod::GssApi => handle
                .authenticate_password("".to_string(), "".to_string())
                .await
                .context("context")
                .unwrap(),
        };
        if !a.success() {
            // 没有成功，发送授权失败事件
        }
        let handle = Arc::new(Mutex::new(handle));
        let mut channel = handle.lock().await.channel_open_session().await.unwrap();
        channel
            .request_pty(true, "xterm-256color", 0, 0, 0, 0, &[])
            .await
            .context("")
            .unwrap();
        channel
            .request_shell(true)
            .await
            .context("request shell")
            .unwrap();
        // channel 创建并初始化成功

        loop {
            tokio::select! {
                            command = cmd_rx.recv() => {
                                match command{
                                    Some(SshMessage::Input(data)) => {
                                        // 将终端发送过来的命令，发送给ssh服务器
                                        if let Err(e) =  channel.data_bytes(data).await{

                                        };
                                    },
                                    Some(SshMessage::Resize(col,row )) => {
                                       let _ = channel.window_change(col.into(), row.into(), 0, 0).await;
                                    },
                                    Some(SshMessage::Disconnect)|None => {
                                        channel.eof();
                                    },
                                }
                            }
                            message=channel.wait()=>{
                                match message{
                                   Some(ChannelMsg::Data { data }) | Some(ChannelMsg::ExtendedData { data, ext: _ }) => {
                                    // log::info!("ssh响应信息{:?}",data.clone().to_vec());
               match events.send( SystemEvent::Output{
                                        tab_id: tab_id.clone(),
                                        bytes: data.to_vec(),
                                    }) {
                    Ok(_) => {
                        log::info!("发送成功");
                    },
                    Err(_) =>{
                        log::info!("发失败");}
            ,
                }
                                // if let Err(e) = events.send(){
                                //         log::info!("发送错误：{:?}",e);
                                //     };
                                //     log::info!("发送完成");
                                }
                                Some(ChannelMsg::ExitStatus { exit_status: _ }) | Some(ChannelMsg::Eof) => {
                                }
                                Some(ChannelMsg::Close) => {

                                    break;
                                }
                                None => {

                                    break;
                                }
                                _ => {}
                                }
                            }
                        }
        }
    });
}
struct ClientHandler;

impl russh::client::Handler for ClientHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        println!("Server key: {:?}", server_public_key);

        Ok(true)
    }
}
