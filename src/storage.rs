use crate::error::{Error, Result};
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use std::path::{Path, PathBuf};
use tokio::fs;

#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn create_bucket(&self, name: &str) -> Result<()>;
    async fn delete_bucket(&self, name: &str) -> Result<()>;
    async fn bucket_exists(&self, name: &str) -> Result<bool>;
    async fn write(&self, path: &Path, data: Vec<u8>) -> Result<()>;
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

    async fn write(&self, path: &Path, data: Vec<u8>) -> Result<()> {
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

#[cfg(feature = "aws")]
pub struct S3StorageProvider {
    client: aws_sdk_s3::Client,
    region: String,
}

#[cfg(feature = "aws")]
impl S3StorageProvider {
    pub fn new(region: String) -> Result<Self> {
        let config = aws_sdk_s3::Config::builder()
            .region(region.parse().map_err(|e| Error::Storage(e.to_string()))?)
            .build();
        let client = aws_sdk_s3::Client::from_conf(config);
        Ok(Self { client, region })
    }
}

#[cfg(feature = "aws")]
#[async_trait]
impl StorageProvider for S3StorageProvider {
    // AWS S3 implementation would go here
}