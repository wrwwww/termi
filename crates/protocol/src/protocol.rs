use std::{default, fmt::Display, sync::Arc};
pub mod ssh;
use anyhow::Context;
pub mod monitor;
pub mod file;
use futures::{SinkExt, channel::mpsc::UnboundedSender};
 
use russh::{
    ChannelMsg,
    client::{self},
    keys::ssh_key,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};
use tokio::sync::{
    Mutex,
    mpsc::{Sender, UnboundedReceiver},
};
use uuid::Uuid;

use crate::{file::RemoteFile, monitor::MetricSnapshot};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Deserialize, Serialize ,JsonSchema
)]
#[schemars(transparent)]
#[serde(transparent)]
pub struct SessionId(#[schemars(with = "String")] Uuid);
impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}
impl Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for TabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Deserialize, Serialize ,JsonSchema
)]
#[schemars(transparent)]
#[serde(transparent)]
pub struct  TransferId(#[schemars(with = "String")] Uuid);
impl TransferId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}
impl Display for  TransferId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,Deserialize, Serialize ,JsonSchema
)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct TabId(#[schemars(with = "String")] Uuid);
impl TabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}
pub enum RuntimeCommand {
       Terminal {
        tab_id: TabId,
        command: TerminalCommand,
    },
    Monitor(MonitorCommand),
    Files(FileCommand),
    Disconnect,
}
pub enum TerminalCommand {
    Open ,

    Input {
        data: Vec<u8>,
    },

    Resize {
        cols: u16,
        rows: u16,
    },

    Close  ,
}
pub enum MonitorCommand {
    Start,
    Stop,
    Refresh,
}
pub enum FileCommand {
    List {
        path: String,
    },

    Mkdir {
        path: String,
    },

    Delete {
        path: String,
    },

    Rename {
        from: String,
        to: String,
    },
}
pub enum RuntimeState {
    Connecting,
    Authenticating,
    Connected,
    Disconnected,
    Failed,
}

#[derive(Debug)]
pub enum RuntimeEvent {
    // =========================
    // Runtime
    // =========================

    Connected{
        session_id: SessionId,
    },

    Disconnected,

    Error {
        message: String,
    },

    // =========================
    // Terminal
    // =========================

    TerminalOutput {
        tab_id: TabId,
        bytes: Vec<u8>,
    },

    TerminalExit {
        tab_id: TabId,
    },

    // =========================
    // Monitor
    // =========================

    MetricsUpdated {
        metrics: MetricSnapshot,
    },

    // =========================
    // File
    // =========================

    DirectoryListed {
        path: String,
        entries: Vec<RemoteFile>,
    },

    // =========================
    // Transfer
    // =========================

    TransferStarted {
        transfer_id: TransferId,
    },

    TransferProgress {
        transfer_id: TransferId,
        transferred: u64,
        total: Option<u64>,
    },

    TransferCompleted {
        transfer_id: TransferId,
    },

    TransferFailed {
        transfer_id: TransferId,
        message: String,
    },
}




#[derive(Clone)]
pub enum SystemEvent {
    Output { tab_id: TabId, bytes: Vec<u8> },
    Status { tab_id: TabId, text: String },
    Connected { tab_id: TabId },
    Error { tab_id: TabId, message: String },
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

pub async fn open_session_terminal(
    mut events: UnboundedSender<SystemEvent>,
    session: Session,
    tab_id: TabId,
    mut cmd_rx: UnboundedReceiver<SshMessage>,
) {
    tokio::spawn(async move {
      
        // connect
        let addr = format!("{}:{}", session.hostname, session.port);
        let stream = match tokio::net::TcpStream::connect(&addr).await {
            Ok(stream) => stream,
            Err(error) => {
                let _ = events.unbounded_send(SystemEvent::Error { tab_id, message: format!("无法连接 {addr}: {error}") });
                return;
            }
        };
        
        let config = Arc::new(client::Config::default());
        let mut handle = match client::connect_stream(config, stream, ClientHandler).await {
            Ok(handle) => handle,
            Err(error) => {
                let _ = events.unbounded_send(SystemEvent::Error { tab_id, message: format!("SSH 握手失败: {error}") });
                return;
            }
        };
       
        // Authenticate before opening a shell. Password authentication is the
        // first complete flow supported by the connection form.
        let auth_result = match session.auth {
            AuthMethod::Password { password } => handle
                .authenticate_password(session.username.clone(), password)
                .await,
            AuthMethod::PublicKey { .. } | AuthMethod::Agent | AuthMethod::KeyboardInteractive => {
                let _ = events.unbounded_send(SystemEvent::Error { tab_id, message: "当前版本暂只支持密码认证连接".to_string() });
                return;
            }
        };
        let auth_result = match auth_result {
            Ok(result) => result,
            Err(error) => {
                let _ = events.unbounded_send(SystemEvent::Error { tab_id, message: format!("SSH 认证失败: {error}") });
                return;
            }
        };
        if !auth_result.success() {
            let _ = events.unbounded_send(SystemEvent::Error { tab_id, message: "用户名或密码不正确".to_string() });
            return;
        }

        let _ = events.unbounded_send(SystemEvent::Connected { tab_id: tab_id.clone() });

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
                        log::info!("ssh响应信息{:?}",data.clone().to_vec());
                    events.send( SystemEvent::Output{
                            tab_id: tab_id.clone(),
                            bytes: data.to_vec(),
                        }).await.unwrap();
                    // if let Err(e) = events.send(){
                    //         log::info!("发送错误：{:?}",e);
                    //     };
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
    }).await.unwrap();
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

// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub struct Session {
//     pub id: String,
//     pub name: String,
//     pub group: String,
//     pub hostname: String,
//     pub port: u16,
//     pub username: String,
//     pub protocol: Protocol,
//     pub auth: AuthMethod,
//     pub identity_file: Option<String>,
//     pub status: SessionStatus,
//     pub latencies_ms: Vec<u32>, // rolling window, last 60 samples
// }

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, EnumString, Display, EnumIter,
)]
pub enum Protocol {
    Ssh,
    Mosh,
    Telnet,
    Local,
}

// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub enum AuthMethod {
//     PublicKey {
//         private_key: String,
//         passphrase_secret_id: String,
//     },
//     Password {
//         password: String,
//     },
//     // Agent,
// }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Connected,
    Idle,
    Disconnected,
    Error,
}
impl Default for SessionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

/// SSH 会话认证方式。
///
/// 认证配置直接属于 Session，避免 Session 外部再维护
/// identity_file / password 等重复字段。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    /// 用户名 + 密码认证。
    Password { password: String },

    /// SSH 私钥认证。
    ///
    /// private_key:
    ///     私钥文件路径。
    ///
    /// passphrase:
    ///     私钥本身的密码。
    PublicKey {
        private_key: String,
        passphrase: Option<String>,
    },

    /// SSH Agent 认证。
    Agent,

    /// Keyboard Interactive 认证。
    ///
    /// 适用于需要服务端交互式询问的 SSH 服务，
    /// 例如某些堡垒机。
    KeyboardInteractive,
}

impl Default for AuthMethod {
    fn default() -> Self {
        Self::Password {
            password: String::new(),
        }
    }
}

impl AuthMethod {
    /// 用于 UI 显示。
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Password { .. } => "Password",
            Self::PublicKey { .. } => "SSH Key",
            Self::Agent => "Agent",
            Self::KeyboardInteractive => "Keyboard Interactive",
        }
    }

    /// 是否需要密码输入。
    pub fn requires_password(&self) -> bool {
        matches!(self, Self::Password { .. })
    }

    /// 是否需要私钥路径。
    pub fn requires_private_key(&self) -> bool {
        matches!(self, Self::PublicKey { .. })
    }

    /// 是否需要私钥 passphrase。
    pub fn requires_passphrase(&self) -> bool {
        matches!(self, Self::PublicKey { .. })
    }

    /// 是否使用 SSH Agent。
    pub fn is_agent(&self) -> bool {
        matches!(self, Self::Agent)
    }

    /// 是否使用 Keyboard Interactive。
    pub fn is_keyboard_interactive(&self) -> bool {
        matches!(self, Self::KeyboardInteractive)
    }
}

/// UI 中的认证方式。
///
/// 这个枚举只负责表示当前 UI 选择了哪个选项。
/// 真正的认证数据应该保存在 AuthMethod 中。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticationType {
    PublicKey,
    Password,
    Agent,
    KeyboardInteractive,
}

impl AuthenticationType {
    pub fn index(self) -> usize {
        match self {
            Self::PublicKey => 0,
            Self::Password => 1,
            Self::Agent => 2,
            Self::KeyboardInteractive => 3,
        }
    }

    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::PublicKey),
            1 => Some(Self::Password),
            2 => Some(Self::Agent),
            3 => Some(Self::KeyboardInteractive),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::PublicKey => "SSH Key",
            Self::Password => "Password",
            Self::Agent => "Agent",
            Self::KeyboardInteractive => "Keyboard Interactive",
        }
    }
}

impl Default for AuthenticationType {
    fn default() -> Self {
        Self::Password
    }
}

/// 你的会话实体。
///
/// 注意：
///
/// 不再单独保存 identity_file。
/// 私钥路径现在属于 AuthMethod::PublicKey。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    /// 唯一 ID。
    pub id: SessionId,

    /// UI 中显示的会话名称。
    pub name: String,

    /// 会话分组。
    pub group: String,

    /// SSH 主机地址。
    pub hostname: String,

    /// SSH 端口。
    pub port: u16,

    /// SSH 用户名。
    pub username: String,

    /// 网络协议。
    pub protocol: Protocol,

    /// 认证方式。
    pub auth: AuthMethod,

    /// 当前连接状态。
    ///
    /// 这种运行时状态通常不建议持久化。
    #[serde(skip)]
    pub status: SessionStatus,

    /// 延迟历史。
    ///
    /// 这是运行时数据，同样不建议持久化。
    #[serde(skip)]
    pub latencies_ms: Vec<u32>,
}

/// SSH 连接层使用的配置。
///
/// ConnectionDialog 不应该直接参与 SSH 连接。
/// UI 最终只需要生成这个结构交给 SessionManager / SSH Client。
#[derive(Clone, Debug)]
pub struct SshConnectConfig {
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
}

impl SshConnectConfig {
    pub fn from_session(session: &Session) -> Self {
        Self {
            hostname: session.hostname.clone(),
            port: session.port,
            username: session.username.clone(),
            auth: session.auth.clone(),
        }
    }
}

/// 将 Session 中的 AuthMethod 转换为 UI 所需要的认证类型。
pub fn authentication_type_from_auth(auth: &AuthMethod) -> AuthenticationType {
    match auth {
        AuthMethod::Password { .. } => AuthenticationType::Password,

        AuthMethod::PublicKey { .. } => AuthenticationType::PublicKey,

        AuthMethod::Agent => AuthenticationType::Agent,

        AuthMethod::KeyboardInteractive => AuthenticationType::KeyboardInteractive,
    }
}

/// 构造 ConnectionDialog 使用的认证状态。
///
/// 注意：
///
/// 这个函数只负责“读取 Session -> UI”。
/// 不负责修改 Session。
pub struct AuthFormState {
    pub authentication: AuthenticationType,

    /// Password 认证使用。
    pub password: String,

    /// PublicKey 认证使用。
    pub private_key: String,

    /// PublicKey 认证使用。
    pub passphrase: String,
}

impl AuthFormState {
    pub fn from_auth(auth: &AuthMethod) -> Self {
        match auth {
            AuthMethod::Password { password } => Self {
                authentication: AuthenticationType::Password,
                password: password.clone(),
                private_key: String::new(),
                passphrase: String::new(),
            },

            AuthMethod::PublicKey {
                private_key,
                passphrase,
            } => Self {
                authentication: AuthenticationType::PublicKey,
                password: String::new(),
                private_key: private_key.clone(),
                passphrase: passphrase.clone().unwrap_or_default(),
            },

            AuthMethod::Agent => Self {
                authentication: AuthenticationType::Agent,
                password: String::new(),
                private_key: String::new(),
                passphrase: String::new(),
            },

            AuthMethod::KeyboardInteractive => Self {
                authentication: AuthenticationType::KeyboardInteractive,
                password: String::new(),
                private_key: String::new(),
                passphrase: String::new(),
            },
        }
    }

    /// UI -> AuthMethod
    pub fn build_auth_method(&self) -> Result<AuthMethod, String> {
        match self.authentication {
            AuthenticationType::Password => Ok(AuthMethod::Password {
                password: self.password.clone(),
            }),

            AuthenticationType::PublicKey => {
                let private_key = self.private_key.trim();

                if private_key.is_empty() {
                    return Err("SSH Key 认证需要指定私钥文件".into());
                }

                let passphrase = self.passphrase.trim();

                Ok(AuthMethod::PublicKey {
                    private_key: private_key.to_string(),

                    passphrase: if passphrase.is_empty() {
                        None
                    } else {
                        Some(passphrase.to_string())
                    },
                })
            }

            AuthenticationType::Agent => Ok(AuthMethod::Agent),

            AuthenticationType::KeyboardInteractive => Ok(AuthMethod::KeyboardInteractive),
        }
    }
}
// pub async fn connect_ssh(config: SshConnectConfig) -> Result<(), SshError> {
//     match config.auth {
//         AuthMethod::Password { password } => {
//             connect_with_password(&config.hostname, config.port, &config.username, &password).await
//         }

//         AuthMethod::PublicKey {
//             private_key,
//             passphrase,
//         } => {
//             connect_with_private_key(
//                 &config.hostname,
//                 config.port,
//                 &config.username,
//                 &private_key,
//                 passphrase.as_deref(),
//             )
//             .await
//         }

//         AuthMethod::Agent => {
//             connect_with_agent(&config.hostname, config.port, &config.username).await
//         }

//         AuthMethod::KeyboardInteractive => {
//             connect_with_keyboard_interactive(&config.hostname, config.port, &config.username).await
//         }
//     }
// }
