use protocol::{AuthMethod, Protocol};
use serde::{Deserialize, Serialize};

use crate::{
    error::{AuthenticationError, SessionError},
    id::SessionId,
};

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
// ============================================================
// Session
// ============================================================

#[derive(Debug, Clone)]
pub enum SessionStatus {
    /// 尚未连接
    Disconnected,

    /// 正在建立 TCP/SSH 连接
    Connecting,

    /// 正在进行 SSH Authentication
    Authenticating,

    /// Authentication 失败，需要用户处理
    ///
    /// 例如：
    /// - 密码错误
    /// - 私钥认证失败
    /// - keyboard-interactive 需要用户输入
    AuthenticationRequired { error: AuthenticationError },

    /// SSH Session 已建立
    Connected,

    /// 正在断开连接
    Disconnecting,

    /// Session 发生不可恢复错误
    Failed(SessionError),
}
