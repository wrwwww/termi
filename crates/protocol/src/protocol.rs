use std::{default, fmt::Display, sync::Arc};
pub mod error;
pub mod ssh;
use anyhow::Context;
pub mod file;
pub mod monitor;
use futures::{SinkExt, channel::mpsc::UnboundedSender};

use russh::{
    ChannelMsg,
    client::{self},
    keys::ssh_key,
};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};
use tokio::sync::{
    Mutex,
    mpsc::{Sender, UnboundedReceiver},
};

use crate::{file::RemoteFile, monitor::MetricSnapshot};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, EnumString, Display, EnumIter,
)]
pub enum Protocol {
    Ssh,
    Mosh,
    Telnet,
    Local,
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
    // pub fn from_session(session: &Session) -> Self {
    //     Self {
    //         hostname: session.hostname.clone(),
    //         port: session.port,
    //         username: session.username.clone(),
    //         auth: session.auth.clone(),
    //     }
    // }
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
