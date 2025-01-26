use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use futures::stream::{self, StreamExt};
use tokio::sync::Mutex;
use std::sync::Arc;

use crate::{Error, StorageProvider};
use crate::types::Result;


/// Represents an index structure that maps shard identifiers to their respective entries and metadata.
///
/// # Fields
///
/// * `entries` - A hashmap mapping file keys to a vector of `IndexEntry` objects representing the shards.
/// * `metadata` - A hashmap containing additional metadata for each file key.
#[derive(Serialize, Deserialize)]
pub struct BucketIndex {
    pub entries: HashMap<String, Vec<IndexEntry>>,
    pub metadata: HashMap<String, Vec<u8>>,
}

/// Represents an entry in the index corresponding to a shard within a file.
///
/// # Fields
///
/// * `shard_id` - A unique identifier for the shard.
/// * `offset` - The offset of the shard within the file.
/// * `size` - The size of the shard in bytes.
/// * `checksum` - A 32-byte SHA-256 checksum of the shard data.
#[derive(Serialize, Deserialize)]
pub struct IndexEntry {
    pub shard_id: usize,
    pub offset: usize,  
    pub size: usize,
    pub checksum: [u8; 32],
}

impl IndexEntry {
    /// Constructs a new `IndexEntry` with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - A unique identifier for the shard.
    /// * `offset` - The offset of the shard within the file.
    /// * `size` - The size of the shard in bytes.
    /// * `checksum` - A 32-byte SHA-256 checksum of the shard data.
    ///
    /// # Returns
    ///
    /// A new `IndexEntry` instance with the specified properties.
    pub fn new(shard_id: usize, offset: usize, size: usize, checksum: [u8; 32]) -> Self {
        Self { shard_id, offset, size, checksum }
    }
}

impl Default for BucketIndex {
    /// Constructs a default `NativeIndex` with empty entries and metadata.
    ///
    /// # Returns
    ///
    /// A new `NativeIndex` instance with empty hashmaps.
    fn default() -> Self {
        Self {
            entries: Default::default(),
            metadata: Default::default()
        }
    }
}

impl BucketIndex {
    /// Builds a new index by processing shards from the specified storage provider and bucket concurrently.
    ///
    /// # Arguments
    ///
    /// * `provider` - A reference to a storage provider that handles file operations.
    /// * `bucket` - The name of the bucket containing files to process.
    /// * `parallelism` - The number of concurrent tasks to use for processing shards.
    ///
    /// # Returns
    ///
    /// A new `NativeIndex` instance containing entries and metadata from processed shards, or an error if any step fails.
    pub async fn build<P: StorageProvider>(
        provider: &P,
        bucket: &str,
        parallelism: usize,
    ) -> Result<Self> {
        let index = Arc::new(Mutex::new(BucketIndex::default()));
        let files = provider.list(bucket.as_ref()).await?;
        
        stream::iter(files)
            .map(|file| {
                let provider = provider;
                let index = Arc::clone(&index);
                async move {
                    let path  = PathBuf::from(file);
                    let data = provider.read(&path).await?;
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

    /// Processes a single shard's data and updates the index entries and metadata accordingly.
    ///
    /// # Arguments
    ///
    /// * `index` - A mutable reference to the `NativeIndex` being updated.
    /// * `data` - The byte slice containing the shard data to process.
    ///
    /// # Returns
    ///
    /// An empty result indicating success or an error if processing fails.
    fn process_shard(index: &mut BucketIndex, data: &[u8]) -> Result<()> {
        // Process shard data and update index entries
        // Parse shard header, entries, update hashmaps
        Ok(())
    }
}
