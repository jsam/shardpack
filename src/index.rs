use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{error::Result, Error, StorageProvider};
use futures::stream::{self, StreamExt};
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct NativeIndex {
    pub entries: HashMap<String, Vec<IndexEntry>>,
    pub metadata: HashMap<String, Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
pub struct IndexEntry {
    pub shard_id: u64,
    pub offset: u64,  
    pub size: u64,
    pub checksum: [u8; 32],
}

impl IndexEntry {
    pub fn new(shard_id: u64, offset: u64, size: u64, checksum: [u8; 32]) -> Self {
        Self { shard_id, offset, size, checksum }
    }
}

impl Default for NativeIndex {
    fn default() -> Self {
        Self {
            entries: Default::default(),
            metadata: Default::default()
        }
    }
}

impl NativeIndex {
    pub async fn build<P: StorageProvider>(
        provider: &P,
        bucket: &str,
        parallelism: usize,
    ) -> Result<Self> {
        let index = Arc::new(Mutex::new(NativeIndex::default()));
        let files = provider.list(bucket.as_ref()).await?;
        
        stream::iter(files)
            .map(|file| {
                let provider = provider;
                let index = Arc::clone(&index);
                async move {
                    let data = provider.read(file.as_ref()).await?;
                    let mut index = index.lock().await;
                    // Process shard and update index
                    Self::process_shard(&mut index, &data)?;
                    Result::Ok(())
                }
            })
            .buffer_unordered(parallelism)
            .collect::<Vec<Result<()>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<()>>>()?;

        let index = Arc::try_unwrap(index)
            .map_err(|_| Error::Index("Failed to unwrap index".into()))?
            .into_inner();

        Ok(index)
    }

    fn process_shard(index: &mut NativeIndex, data: &[u8]) -> Result<()> {
        // Process shard data and update index entries
        // Parse shard header, entries, update hashmaps
        Ok(())
    }
}