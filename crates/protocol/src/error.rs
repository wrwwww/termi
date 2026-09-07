// protocol/error.rs

use std::fmt;
use std::io;

use russh::Error as RusshError;

#[derive(Debug)]
pub enum ProtocolError {
    /// SSH 握手初始化失败
    KexInit,

    /// SSH Key Exchange 失败
    Kex,

    /// SSH 协议版本错误
    Version,

    /// 没有找到双方都支持的算法
    NoCommonAlgorithm {
        // kind: String,
        ours: Vec<String>,
        theirs: Vec<String>,
    },

    /// 使用了未知算法
    UnknownAlgorithm,

    /// SSH 数据包认证失败
    PacketAuth,

    /// SSH 数据包大小非法
    PacketSize { size: usize },

    /// 解密失败
    Decryption,

    /// 压缩失败
    Compression,

    /// 解压失败
    Decompression,

    /// SSH 编码失败
    Encoding,

    /// 严格 Key Exchange 规则违反
    StrictKeyExchangeViolation {
        message_type: u8,
        sequence_number: usize,
    },

    /// SSH 连接被远端关闭
    Disconnect,

    /// TCP/IO 层错误
    Io(io::Error),

    /// russh 内部 channel 错误
    WrongChannel,

    /// Channel 打开失败
    ChannelOpenFailure { reason: String },

    /// 未认证就执行了需要认证的操作
    NotAuthenticated,

    /// 当前认证方式不支持
    UnsupportedAuthMethod,

    /// 没有可用认证方式
    NoAuthMethod,

    /// 请求被服务器拒绝
    RequestDenied,

    /// Server Key 发生变化
    KeyChanged { line: usize },

    /// Key 读取失败
    CouldNotReadKey,

    /// 未知 Key
    UnknownKey,

    /// Server Signature 验证失败
    WrongServerSignature,

    /// Signature 错误
    Signature { message: String },

    /// SSH Key 错误
    SshKey { message: String },

    /// SSH 编解码错误
    SshEncoding { message: String },

    /// russh 内部错误
    Internal { message: String },
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KexInit => {
                write!(f, "SSH key exchange initialization failed")
            }

            Self::Kex => {
                write!(f, "SSH key exchange failed")
            }

            Self::Version => {
                write!(f, "SSH protocol version error")
            }

            Self::NoCommonAlgorithm { ours, theirs } => {
                write!(
                    f,
                    "no common SSH algorithm for , ours={ours:?}, theirs={theirs:?}"
                )
            }

            Self::UnknownAlgorithm => {
                write!(f, "unknown SSH algorithm")
            }

            Self::PacketAuth => {
                write!(f, "SSH packet authentication failed")
            }

            Self::PacketSize { size } => {
                write!(f, "invalid SSH packet size: {size}")
            }

            Self::Decryption => {
                write!(f, "SSH packet decryption failed")
            }

            Self::Compression => {
                write!(f, "SSH compression failed")
            }

            Self::Decompression => {
                write!(f, "SSH decompression failed")
            }

            Self::Encoding => {
                write!(f, "SSH encoding failed")
            }

            Self::StrictKeyExchangeViolation {
                message_type,
                sequence_number,
            } => {
                write!(
                    f,
                    "strict key exchange violation: message_type={message_type}, sequence={sequence_number}"
                )
            }

            Self::Disconnect => {
                write!(f, "SSH connection disconnected")
            }

            Self::Io(error) => {
                write!(f, "SSH I/O error: {error}")
            }

            Self::WrongChannel => {
                write!(f, "invalid SSH channel")
            }

            Self::ChannelOpenFailure { reason } => {
                write!(f, "SSH channel open failed: {reason}")
            }

            Self::NotAuthenticated => {
                write!(f, "SSH client is not authenticated")
            }

            Self::UnsupportedAuthMethod => {
                write!(f, "SSH authentication method is not supported")
            }

            Self::NoAuthMethod => {
                write!(f, "no supported SSH authentication method")
            }

            Self::RequestDenied => {
                write!(f, "SSH request was denied")
            }

            Self::KeyChanged { line } => {
                write!(f, "SSH host key changed at known_hosts line {line}")
            }

            Self::CouldNotReadKey => {
                write!(f, "could not read SSH key")
            }

            Self::UnknownKey => {
                write!(f, "unknown SSH key")
            }

            Self::WrongServerSignature => {
                write!(f, "SSH server signature verification failed")
            }

            Self::Signature { message } => {
                write!(f, "SSH signature error: {message}")
            }

            Self::SshKey { message } => {
                write!(f, "SSH key error: {message}")
            }

            Self::SshEncoding { message } => {
                write!(f, "SSH encoding error: {message}")
            }

            Self::Internal { message } => {
                write!(f, "SSH protocol internal error: {message}")
            }
        }
    }
}

impl std::error::Error for ProtocolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

// ============================================================
// russh -> ProtocolError
// ============================================================

impl From<RusshError> for ProtocolError {
    fn from(error: RusshError) -> Self {
        match error {
            RusshError::KexInit => Self::KexInit,

            RusshError::Kex => Self::Kex,

            RusshError::Version => Self::Version,

            RusshError::UnknownAlgo => Self::UnknownAlgorithm,

            RusshError::NoCommonAlgo { kind, ours, theirs } => Self::NoCommonAlgorithm {
                ours: ours.into_iter().map(|value| value.to_string()).collect(),
                theirs: theirs.into_iter().map(|value| value.to_string()).collect(),
            },

            RusshError::PacketAuth => Self::PacketAuth,

            RusshError::PacketSize(size) => Self::PacketSize { size },

            RusshError::DecryptionError => Self::Decryption,

            RusshError::Compress(_) => Self::Compression,

            RusshError::Decompress(_) => Self::Decompression,

            RusshError::SshEncoding(_) => Self::Encoding,

            RusshError::StrictKeyExchangeViolation {
                message_type,
                sequence_number,
            } => Self::StrictKeyExchangeViolation {
                message_type,
                sequence_number,
            },

            RusshError::Disconnect => Self::Disconnect,

            RusshError::IO(error) => Self::Io(error),

            RusshError::WrongChannel => Self::WrongChannel,

            RusshError::ChannelOpenFailure(error) => Self::ChannelOpenFailure {
                reason: error.description().to_string(),
            },

            RusshError::NotAuthenticated => Self::NotAuthenticated,

            RusshError::UnsupportedAuthMethod => Self::UnsupportedAuthMethod,

            RusshError::NoAuthMethod => Self::NoAuthMethod,

            RusshError::RequestDenied => Self::RequestDenied,

            RusshError::KeyChanged { line } => Self::KeyChanged { line },

            RusshError::CouldNotReadKey => Self::CouldNotReadKey,

            RusshError::UnknownKey => Self::UnknownKey,

            RusshError::WrongServerSig => Self::WrongServerSignature,

            RusshError::Signature(error) => Self::Signature {
                message: error.to_string(),
            },

            RusshError::SshKey(error) => Self::SshKey {
                message: error.to_string(),
            },

            RusshError::SshEncoding(error) => Self::SshEncoding {
                message: error.to_string(),
            },

            error => Self::Internal {
                message: error.to_string(),
            },
        }
    }
}
