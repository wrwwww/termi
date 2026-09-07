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

    /// 其他 Session 级别错误
    Other { message: String },
}

// ============================================================
// Connection Error
// ============================================================

#[derive(Debug, Clone)]
pub enum ConnectionError {
    /// DNS 解析失败
    Dns { host: String, message: String },

    /// 连接超时
    Timeout { host: String, port: u16 },

    /// 目标端口拒绝连接
    Refused { host: String, port: u16 },

    /// 网络不可达
    Unreachable { host: String, port: u16 },

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
    fn from(value: ProtocolError) -> Self {
        match value {
            ProtocolError::KexInit => {
                RuntimeError::Session(SessionError::Connection(ConnectionError::Other {
                    message: "".to_string(),
                }))
            }
            ProtocolError::Kex => todo!(),
            ProtocolError::Version => todo!(),
            ProtocolError::NoCommonAlgorithm { ours, theirs } => todo!(),
            ProtocolError::UnknownAlgorithm => todo!(),
            ProtocolError::PacketAuth => todo!(),
            ProtocolError::PacketSize { size } => todo!(),
            ProtocolError::Decryption => todo!(),
            ProtocolError::Compression => todo!(),
            ProtocolError::Decompression => todo!(),
            ProtocolError::Encoding => todo!(),
            ProtocolError::StrictKeyExchangeViolation {
                message_type,
                sequence_number,
            } => todo!(),
            ProtocolError::Disconnect => todo!(),
            ProtocolError::Io(error) => todo!(),
            ProtocolError::WrongChannel => todo!(),
            ProtocolError::ChannelOpenFailure { reason } => todo!(),
            ProtocolError::NotAuthenticated => todo!(),
            ProtocolError::UnsupportedAuthMethod => todo!(),
            ProtocolError::NoAuthMethod => todo!(),
            ProtocolError::RequestDenied => todo!(),
            ProtocolError::KeyChanged { line } => todo!(),
            ProtocolError::CouldNotReadKey => todo!(),
            ProtocolError::UnknownKey => todo!(),
            ProtocolError::WrongServerSignature => todo!(),
            ProtocolError::Signature { message } => todo!(),
            ProtocolError::SshKey { message } => todo!(),
            ProtocolError::SshEncoding { message } => todo!(),
            ProtocolError::Internal { message } => todo!(),
        }
    }
}
