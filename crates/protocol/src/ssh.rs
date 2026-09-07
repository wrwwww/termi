use anyhow::{Context, Result};
use bytes::Bytes;
use futures::lock::Mutex;
use russh::{
    Channel, ChannelMsg,
    client::{self, Config, Handle, Msg},
    keys::ssh_key,
};
use std::{path::Path, sync::Arc};

use crate::{AuthMethod, error::ProtocolError, file::RemoteFile};
pub struct SshConfig {
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
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

pub struct SshConnection {
    handle: Arc<Mutex<Handle<ClientHandler>>>,
}
impl SshConnection {
    /// 建立 SSH 连接
    pub async fn connect(session: &SshConfig) -> Result<Self, ProtocolError> {
        let addr = format!("{}:{}", session.hostname, session.port);
        let stream = tokio::net::TcpStream::connect(&addr)
            .await
            .map_err(|err| ProtocolError::Io(err))?;
        // .with_context(|| format!("连接服务器失败: {addr}"))?;

        let config = Arc::new(Config::default());

        let mut handle = client::connect_stream(config, stream, ClientHandler)
            .await
            .map_err(|err| ProtocolError::KexInit)?;
        // .context()?;

        match &session.auth {
            AuthMethod::Password { password } => {
                let result = handle
                    .authenticate_password(session.username.clone(), password.clone())
                    .await
                    .map_err(|err| ProtocolError::RequestDenied)?;

                if !result.success() {
                    // anyhow::bail!("用户名或密码不正确");
                }
            }

            _ => {
                // anyhow::bail!("当前只支持密码认证");
                return Err(ProtocolError::NoAuthMethod);
            }
        }

        Ok(Self {
            handle: Arc::new(Mutex::new(handle)),
        })
    }

    /// 打开一个 SSH Terminal Channel
    pub async fn open_terminal(&self) -> Result<TerminalChannel> {
        let channel = self
            .handle
            .lock()
            .await
            .channel_open_session()
            .await
            .context("打开 SSH terminal channel 失败")?;

        channel
            .request_pty(true, "xterm-256color", 80, 24, 0, 0, &[])
            .await
            .context("申请 PTY 失败")?;

        channel
            .request_shell(true)
            .await
            .context("启动 shell 失败")?;

        Ok(TerminalChannel::new(channel))
    }

    /// 在 SSH 上执行一次命令
    pub async fn execute(&self, command: &str) -> Result<CommandOutput> {
        let mut channel = self
            .handle
            .lock()
            .await
            .channel_open_session()
            .await
            .context("打开 command channel 失败")?;

        channel
            .exec(true, command)
            .await
            .with_context(|| format!("执行命令失败: {command}"))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_status = None;

        while let Some(msg) = channel.wait().await {
            match msg {
                russh::ChannelMsg::Data { data } => {
                    stdout.extend_from_slice(&data);
                }

                russh::ChannelMsg::ExtendedData { data, .. } => {
                    stderr.extend_from_slice(&data);
                }

                russh::ChannelMsg::ExitStatus { exit_status: code } => {
                    exit_status = Some(code);
                }

                russh::ChannelMsg::Eof => {
                    channel.close().await.ok();
                    break;
                }

                russh::ChannelMsg::Close => {
                    break;
                }

                _ => {}
            }
        }

        Ok(CommandOutput {
            stdout,
            stderr,
            exit_status,
        })
    }

    /// 打开 SFTP
    pub async fn open_sftp(&self) -> Result<SftpClient> {
        let channel = self
            .handle
            .lock()
            .await
            .channel_open_session()
            .await
            .context("打开 SFTP channel 失败")?;

        channel
            .request_subsystem(true, "sftp")
            .await
            .context("启动 SFTP subsystem 失败")?;

        SftpClient::from_channel(channel).await
    }
}
pub struct TerminalChannel {
    channel: Channel<Msg>,
}

impl TerminalChannel {
    fn new(channel: Channel<Msg>) -> Self {
        Self { channel }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        self.channel
            .data(data)
            .await
            .context("向 SSH terminal channel 写入数据失败")?;
        Ok(())
    }

    pub async fn read(&mut self) -> Option<ProtocolChannelMsg> {
        match self.channel.wait().await {
            Some(msg) => match msg {
                ChannelMsg::Data { data } => Some(ProtocolChannelMsg::Data { data }),
                ChannelMsg::ExtendedData { data, ext } => {
                    Some(ProtocolChannelMsg::ExtendedData { data, ext })
                }
                ChannelMsg::Eof => Some(ProtocolChannelMsg::Eof),
                ChannelMsg::Close => Some(ProtocolChannelMsg::Close),
                _ => Some(ProtocolChannelMsg::Other),
            },
            None => None,
        }
    }
    pub async fn window_change(&mut self, cols: u32, rows: u32) -> Result<()> {
        self.channel
            .window_change(cols, rows, 0, 0)
            .await
            .context("向 SSH terminal channel 发送 window change 失败")?;
        Ok(())
    }
    pub async fn eof(&mut self) -> Result<()> {
        self.channel
            .eof()
            .await
            .context("向 SSH terminal channel 发送 EOF 失败")?;
        Ok(())
    }
}

pub enum ProtocolChannelMsg {
    Data { data: Bytes },
    ExtendedData { data: Bytes, ext: u32 },
    Eof,
    Close,
    Other,
}
pub struct CommandOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_status: Option<u32>,
}
pub struct SftpClient {
    // russh SFTP client
}

impl SftpClient {
    async fn from_channel(channel: Channel<client::Msg>) -> Result<Self> {
        // 创建 russh SFTP client
        todo!()
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<RemoteFile>> {
        todo!()
    }

    pub async fn upload(&self, local: &Path, remote: &str) -> Result<()> {
        todo!()
    }

    pub async fn download(&self, remote: &str, local: &Path) -> Result<()> {
        todo!()
    }
}
