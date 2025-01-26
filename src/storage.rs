use crate::error::Error;
use crate::types::Result;

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;

const DEFAULT_LOCAL_STORAGE_PATH: &str = "./local_bucket";

#[async_trait]
pub trait StorageProvider: Send + Sync + Default {
    async fn create_bucket(&self, name: &str) -> Result<()>;
    async fn delete_bucket(&self, name: &str) -> Result<()>;
    async fn bucket_exists(&self, name: &str) -> Result<bool>;
    async fn write(&self, path: &Path, data: &[u8]) -> Result<()>;
    async fn read(&self, path: &Path) -> Result<Vec<u8>>;
    async fn delete(&self, path: &Path) -> Result<()>;
    async fn list(&self, prefix: &Path) -> Result<Vec<String>>;
}

pub struct LocalStorageProvider {
    root: PathBuf,
}

impl LocalStorageProvider {
    pub async fn new<P: Into<PathBuf>>(root: P) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).await.map_err(Error::from)?;
        Ok(Self { root })
    }
}

impl Default for LocalStorageProvider {
    fn default() -> Self {
        Self { root: PathBuf::from(DEFAULT_LOCAL_STORAGE_PATH) }
    }
}

#[async_trait]
impl StorageProvider for LocalStorageProvider {
    async fn create_bucket(&self, name: &str) -> Result<()> {
        let path = self.root.join(name);
        fs::create_dir_all(&path).await.map_err(Error::from)
    }

    async fn delete_bucket(&self, name: &str) -> Result<()> {
        let path = self.root.join(name);
        fs::remove_dir_all(&path).await.map_err(Error::from)
    }

    async fn bucket_exists(&self, name: &str) -> Result<bool> {
        let path = self.root.join(name);
        Ok(path.exists())
    }

    async fn write(&self, path: &Path, data: &[u8]) -> Result<()> {
        let full_path = self.root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await.map_err(Error::from)?;
        }
        fs::write(full_path, data).await.map_err(Error::from)
    }

    async fn read(&self, path: &Path) -> Result<Vec<u8>> {
        let full_path = self.root.join(path);
        fs::read(full_path).await.map_err(Error::from)
    }

    async fn delete(&self, path: &Path) -> Result<()> {
        let full_path = self.root.join(path);
        fs::remove_file(full_path).await.map_err(Error::from)
    }

    async fn list(&self, prefix: &Path) -> Result<Vec<String>> {
        let full_path = self.root.join(prefix);
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(full_path).await.map_err(Error::from)?;
        
        while let Some(entry) = read_dir.next_entry().await.map_err(Error::from)? {
            if let Ok(path) = entry.path().strip_prefix(&self.root) {
                if let Some(path_str) = path.to_str() {
                    entries.push(path_str.to_string());
                }
            }
        }
        Ok(entries)
    }
}
