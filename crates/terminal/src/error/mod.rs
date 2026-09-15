// ============================================================
// Runtime Error
// ============================================================
//
// RuntimeError 是 Runtime 内部统一的错误类型。
// 它不是 Store 的状态模型。
//
// Runtime 层：
//     Result<T, RuntimeError>
//
// Store 层：
//     SessionStatus / TerminalStatus
//
// UI 层：
//     根据具体 Error 决定 Dialog / Banner / Notification
//

use protocol::error::ProtocolError;

#[derive(Debug, Clone)]
pub enum RuntimeError {
    Session(SessionError),

    Terminal(TerminalError),
}

// ============================================================
// Session Error
// ============================================================

#[derive(Debug, Clone)]
pub enum SessionError {
    /// TCP / DNS / 网络连接错误
    Connection(ConnectionError),

    /// SSH Authentication 错误
    Authentication(AuthenticationError),

    /// SSH Host Key 验证错误
    HostKey(HostKeyError),
    /// SSH 协议层错误
    Protocol { message: String },
    /// 其他 Session 级别错误
    Other { message: String },
}

// ============================================================
// Connection Error
// ============================================================

#[derive(Debug, Clone)]
pub enum ConnectionError {
    /// DNS 解析失败
    Dns ,

    /// 连接超时
    Timeout ,

    /// 目标端口拒绝连接
    Refused ,

    /// 网络不可达
    Unreachable ,

    /// 连接被重置
    Reset,

    /// 其他连接错误
    Other { message: String },
}

// ============================================================
// Authentication Error
// ============================================================

#[derive(Debug, Clone)]
pub enum AuthenticationError {
    /// Password authentication 被服务器拒绝
    PasswordRejected,

    /// Public Key authentication 被服务器拒绝
    PublicKeyRejected,

    /// 私钥文件不存在
    PrivateKeyNotFound { path: String },

    /// 私钥格式错误 / 无法解析
    PrivateKeyInvalid { message: String },

    /// Keyboard Interactive authentication 失败
    KeyboardInteractiveFailed,

    /// Server 没有支持的认证方式
    NoSupportedMethod,

    /// 权限被拒绝
    PermissionDenied,

    /// 其他 Authentication 错误
    Other { message: String },
}

// ============================================================
// Host Key Error
// ============================================================

#[derive(Debug, Clone)]
pub enum HostKeyError {
    /// 首次连接，客户端没有保存该 Host Key
    UnknownHost { fingerprint: String },

    /// Server Host Key 与之前保存的不一致
    Changed {
        old_fingerprint: String,
        new_fingerprint: String,
    },

    /// 需要用户确认 Host Key
    VerificationRequired { fingerprint: String },

    /// 用户拒绝 Host Key
    Rejected,
}

// ============================================================
// Terminal Error
// ============================================================

#[derive(Debug, Clone)]
pub enum TerminalError {
    /// SSH Channel 创建失败
    ChannelOpen { message: String },

    /// PTY 请求失败
    PtyRequest { message: String },

    /// PTY Resize 失败
    PtyResize { message: String },

    /// Shell 请求失败
    ShellRequest { message: String },

    /// Exec 请求失败
    Exec { command: String, message: String },

    /// Terminal 初始化失败
    Initialization { message: String },

    /// Channel 被远端关闭
    ChannelClosed { message: Option<String> },

    /// Terminal IO 错误
    Io { message: String },

    /// 其他 Terminal 错误
    Other { message: String },
}
impl From<ProtocolError> for RuntimeError {
    fn from(error: ProtocolError) -> Self {
        match error {
            ProtocolError::Disconnect =>  RuntimeError::Session(SessionError::Connection(ConnectionError::Refused )),
            ProtocolError::Io(_error) => RuntimeError::Session(SessionError::Connection(ConnectionError::Timeout )),
            ProtocolError::ChannelOpenFailure { reason } =>  RuntimeError::Session(SessionError::Other {
                message: format!("channel 打开失败{}",reason),
            }),
            ProtocolError::NotAuthenticated => todo!(),
            ProtocolError::UnsupportedAuthMethod =>  RuntimeError::Session(SessionError::Other {
                message: format!("当前认证方式不支持"),
            }),
            ProtocolError::NoAuthMethod =>  RuntimeError::Session(SessionError::Other {
                message: format!("没有可认证的方式"),
            }),
            ProtocolError::RequestDenied => RuntimeError::Session(SessionError::Other {
                message: format!("请求被服务器拒绝"),
            }),
            _ => RuntimeError::Session(SessionError::Other {
                message: format!("{error:?}"),
            }),
        }
    }
}
