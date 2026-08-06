use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TabId(pub Uuid);
impl TabId {
    // 创建新的随机 SessionId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    // 从已有 UUID 创建
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    // 获取内部 UUID 引用
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    // 转换为 UUID
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub Uuid);
impl SessionId {
    // 创建新的随机 SessionId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    // 从已有 UUID 创建
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    // 获取内部 UUID 引用
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    // 转换为 UUID
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum SessionKind {
    #[default] // 指定默认变体
    Ssh,
    Telnet,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthMethod {
    Password {
        remember: bool,
        // secret_id: Option<String>,
        username: String,
        password: String,
    },

    PublicKey {
        private_key: PathBuf,
        passphrase_secret_id: Option<String>,
    },

    KeyboardInteractive,

    GssApi,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionConfig {
    pub hostname: String,
    pub port: u32,
    pub auth_method: AuthMethod,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum SessionStatus {
    #[default] // 指定默认变体
    Connected,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,

    // 显示名称
    pub name: String,

    // 连接类型
    pub kind: SessionKind,

    // 连接配置
    pub config: SessionConfig,

    // 当前状态
    #[serde(skip)]
    pub status: SessionStatus,
}
impl Session {
    pub fn get_hostname(&self) -> String {
        self.config.hostname.clone()
    }
}
