pub trait RemoteFileSystem {
    async fn list(&self, path: &str) -> anyhow::Result<Vec<RemoteFile>>;

    async fn mkdir(&self, path: &str) -> anyhow::Result<()>;

    async fn remove(&self, path: &str) -> anyhow::Result<()>;

    async fn rename(&self, old: &str, new: &str) -> anyhow::Result<()>;
}
#[derive(Clone, Debug)]
pub struct RemoteFile {
    pub name: String,

    pub path: String,

    pub size: u64,

    pub is_directory: bool,

    pub modified: Option<std::time::SystemTime>,
}
