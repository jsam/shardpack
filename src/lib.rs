mod bucket;
mod checksum;
mod storage;
mod shard;
mod error;
mod index;
mod types;

pub use bucket::Bucket;
pub use error::Error;
pub use storage::StorageProvider;




/*
Compression implementation
Parallel reading/writing
Error handling improvements
Memory efficiency (streaming records)
Checksum/validation
Chunked writing
File locking
Proper file entry content types
Record size limits
*/

// Example usage in tests
#[cfg(test)]
mod tests {
    
    
    // TODO:
    // #[tokio::test]
    // async fn test_bucket_operations() -> Result<()> {
    //     let provider = LocalStorageProvider::new("test_data").await?;
    //     let arc_provider = Arc::new(provider);
    //     let config = BucketConfig::default();

    //     let bucket = Bucket::new("test-bucket".to_string(), arc_provider, config);
        
    //     // Write data with metadata
    //     bucket.write(
    //         "key1",
    //         b"test data".as_ref(),
    //         Some(b"metadata".to_vec())
    //     ).await?;

    //     // Read data
    //     let data = bucket.read("key1").await?;
    //     assert_eq!(data, b"test data");

    //     // Get metadata
    //     let metadata = bucket.get_metadata("key1").await?;
    //     assert_eq!(metadata, Some(b"metadata".to_vec()));

    //     Ok(())
    // }
}