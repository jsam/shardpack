use serde::{Deserialize, Serialize};
//use sha2::{Sha256, Digest};
use tokio::sync::RwLock;

use crate::checksum::{compute_checksum, verify_checksum};
use crate::error::Error;
use crate::index::bucket::BucketIndex;
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
    
    pub async fn write(&self, key: &str, data: &[u8], metadata: Option<Vec<u8>>) -> Result<()> {
        let index = self.index.write().await;
        let shard_id = self.get_next_shard_id().await?;
        let shard_path = self.get_shard_path(shard_id);
        
        // Calculate checksum before any compression
        let checksum = compute_checksum(data);
        let data_len = data.len();
        
        // TODO: we should use ShardWriter for this not write directely from the provider
        // Handle compression and writing
        match self.config.compression {
            CompressionType::None => self.provider.write(&shard_path, data).await?,
            CompressionType::Gzip => {
                let compressed = compress_gzip(data)?;
                self.provider.write(&shard_path, compressed.as_ref()).await?
            },
            CompressionType::Lz4 => {
                let compressed = compress_lz4(data)?;
                self.provider.write(&shard_path, compressed.as_ref()).await?
            },
            _ => return Err(Error::Storage("Unsupported compression".into()))
        };
        
        // TODO: index entry should be taken care of inside bucket index when we use shard writing API
        // let entry = IndexEntry::new(
        //     shard_id,
        //     0,
        //     data_len,
        //     checksum
        // );
     
        // index.entries.entry(key.to_string())
        //     .or_default()
        //     .push(entry);
     
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