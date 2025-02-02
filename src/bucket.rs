use serde::{Deserialize, Serialize};
//use sha2::{Sha256, Digest};
use tokio::sync::RwLock;

use crate::checksum::{compute_checksum, verify_checksum};
use crate::error::Error;
use crate::index::bucket::{BucketIndex, IndexEntry};
use crate::shard::config::shard_size;
use crate::shard::shard::Shard;
use crate::types::Result;
use crate::storage::StorageProvider;
use std::io::{Read, Write};
use std::sync::Arc;


#[derive(Clone, Debug, Serialize, Deserialize)]
#[derive(Default)]
pub enum CompressionType {
   #[default]
   None,
   Gzip,
   Lz4,
   Zstd,
   Snappy,
}




 fn compress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish().map_err(Error::from)
}

fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

fn compress_lz4(data: &[u8]) -> Result<Vec<u8>> {
    Ok(lz4_flex::block::compress(data))
}
 
 fn decompress_lz4(data: &[u8]) -> Result<Vec<u8>> {
    lz4_flex::block::decompress(data, data.len() * 3)
        .map_err(|e| Error::Storage(e.to_string()))
 }


#[derive(Clone)]
#[derive(Default)]
pub struct BucketConfig {
    pub compression: CompressionType,
    pub parallelism: usize,
}

impl BucketConfig {
    pub fn new(compression: CompressionType, parallelism: usize) -> Self {
        Self { compression, parallelism }
    }
}

pub struct Bucket<P: StorageProvider> {
    name: String,
    provider: Arc<P>,
    index: RwLock<BucketIndex>,
    shards: Vec<Shard<P>>,
    config: BucketConfig,
}


impl<P: StorageProvider> Bucket<P> {
    pub fn new(name: String, provider: Arc<P>, config: BucketConfig) -> Self {
        Self { 
            name, 
            provider, 
            index: Default::default(), 
            shards: Default::default(), 
            config 
        }
    }
    
    pub async fn write(&mut self, key:  &str, data:  &Vec<u8>, metadata: Option<Vec<u8>>)  -> Result<()> {
        let index = self.index.write().await;
        let current_shards = self.shards.len();

        if current_shards == 0 {
            // No active shards yet; create a new one
            let new_shard = Shard::new();
            self.shards.push(new_shard);
        }

        let last_shard_idx = current_shards - 1;
        let last_shard_path = self.get_shard_path(last_shard_idx);

        // Read the existing data from the last shard to check its size
        let mut file = self.provider.read(&last_shard_path).await?;
        // let mut buffer = Vec::new();
        // file.read_to_end(&mut buffer)?;
        let current_size = file.len();

        // Determine if we need to create a new shard or use the existing one
        let (shard_id, offset) = {
            let available_space = shard_size() - current_size;
            if available_space < data.len() {
                // The data is too large for the current shard; create a new one
                self.shards.push(Shard::new());
                (current_shards, 0)
            } else {
                // Write to the current shard
                (last_shard_idx, current_size)
            }
        };

        let index_entry = IndexEntry::new(
            shard_id,
            offset,
            data.len(),
            compute_checksum(data)
        );

        // Handle compression based on config
        let compressed_data = match self.config.compression {
            CompressionType::None => data.clone(),
            CompressionType::Gzip => compress_gzip(data)?,
            CompressionType::Lz4 => compress_lz4(data)?,
            _ => return Err(Error::Storage("Unsupported compression".into()))
        };

        // Write the compressed data to the appropriate shard
        self.provider.write(&last_shard_path, &compressed_data).await?;

        // if let Some(meta) = metadata {
        //     index.metadata.insert(key.to_string(), meta);
        // }
        Ok(())
    }
 
    pub async fn read(&self, key: &str) -> Result<Vec<u8>> {
        let index = self.index.read().await;
        let entries = index.entries.get(key)
            .ok_or_else(|| Error::Storage("Key not found".into()))?;
 
        let mut data = Vec::new();
        for entry in entries {
            let shard_path = self.get_shard_path(entry.shard_id);
            let chunk = self.provider.read(&shard_path).await?;
            
            let decompressed = match self.config.compression {
                CompressionType::None => chunk,
                CompressionType::Gzip => decompress_gzip(&chunk)?,
                CompressionType::Lz4 => decompress_lz4(&chunk)?,
                _ => return Err(Error::Storage("Unsupported compression".into()))
            };
 
            verify_checksum(&decompressed, &entry.checksum)?;
            data.extend_from_slice(&decompressed);
        }
 
        Ok(data)
    }
 
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut index = self.index.write().await;
        
        if let Some(entries) = index.entries.remove(key) {
            for entry in entries {
                let shard_path = self.get_shard_path(entry.shard_id);
                self.provider.delete(&shard_path).await?;
            }
        }
 
        index.metadata.remove(key);
        Ok(())
    }
 
    pub async fn get_metadata(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let index = self.index.read().await;
        Ok(index.metadata.get(key).cloned())
    }
 
    async fn get_next_shard_id(&self) -> Result<usize> {
        // Implementation for generating unique shard IDs
        Ok(0)
    }
 
    fn get_shard_path(&self, shard_id: usize) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.name).join(format!("shard_{:016x}", shard_id))
    }
    
 }