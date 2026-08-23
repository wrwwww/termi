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
