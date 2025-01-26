use crate::{checksum::compute_checksum, index::bucket::IndexEntry};
use crate::StorageProvider;
use crate::error::Error;
use crate::shard::config::shard_size;
use crate::types::Result;

use byte_counter::counter::ByteCounter;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a writer for writing data to a shard.
///
/// A `ShardWriter` is responsible for managing the writing of data into a shard,
/// keeping track of its size and maintaining an index of entries. It uses a
/// generic storage provider that implements the `StorageProvider` trait.
#[derive(Serialize, Deserialize)]
pub struct ShardWriter<W: StorageProvider> {
    /// A `ByteCounter` used to generate unique identifiers for data written to the shard.
    id: ByteCounter,

    /// The storage provider responsible for persisting data to the shard.
    provider: W,

    /// The current size of the shard in bytes.
    current_size: usize,

    /// A vector containing index entries, each representing a piece of data stored in the shard.
    entries: Vec<IndexEntry>,
}

/// Default implementation for `ShardWriter`.
///
/// This provides default values for all fields:
/// - `id`: An empty `ByteCounter` instance.
/// - `provider`: The default value of the storage provider type.
/// - `current_size`: 0, indicating that the shard is initially empty.
/// - `entries`: An empty vector, as there are no entries when a writer is first created.
impl<W: StorageProvider> Default for ShardWriter<W> {
    fn default() -> Self {
        Self {
            id: ByteCounter::default(),
            provider: Default::default(),
            current_size: Default::default(),
            entries: Default::default()
        }
    }
}

impl<W: StorageProvider> ShardWriter<W> {
    /// Creates a new instance of `ShardWriter` with the provided writer.
    ///
    /// # Arguments
    /// * `writer`: The writer to use for output operations. This can be any type that implements
    ///   the `Write` trait, such as file handles or network streams.
    ///
    /// # Returns
    /// A new `ShardWriter` instance initialized with the provided writer and default values:
    /// - `current_size` set to 0.
    /// - `entries` initialized as an empty vector of `IndexEntry`.
    pub fn new(id: ByteCounter, writer: W)  -> Self  {
        Self {
            id: id,
            provider: writer,
            current_size: 0,
            entries: Vec::new(),
         }
     }

    /// Writes data to the shard with an associated key and optional metadata.
    ///
    /// This method takes a `key`, `data`, and optional `metadata` as arguments. It performs several steps:
    /// 1. Calculates the size of the data to determine if adding it would exceed the shard's size limit.
    /// 2. Computes the SHA-256 checksum of the provided data for integrity verification.
    /// 3. Creates an `IndexEntry` containing metadata about the stored data, including its offset and length.
    /// 4. Writes the data to the underlying writer using the StorageProvider API.
    /// 5. If metadata is provided, it also writes the metadata immediately after the data.
    /// 6. Updates the current size of the shard to reflect the added data.
    ///
    /// # Arguments
    /// * `key` - A string slice representing the key associated with the data being stored.
    /// * `data` - A byte slice containing the actual data to be written to the shard.
    /// * `metadata` - An optional byte slice that can hold additional information about the data.
    ///
    /// # Returns
    /// * `Result<()>` indicating success or an error if writing fails, such as exceeding the shard size limit or I/O errors during write operations.
    pub async fn write(&mut self, key: &str, data: &[u8], metadata: Option<&[u8]>) -> Result<()> {
        let mut data_len = data.len();
        if metadata.is_some() {
            data_len += metadata.unwrap().len();
        }

        if self.current_size + data_len > shard_size() {
            return Err(Error::Storage("Shard size limit exceeded".into()));
        }

        // Compute the SHA-256 checksum of the data
        let checksum = compute_checksum(data);

        // Determine the offset for this entry and create a new IndexEntry
        let offset = self.current_size;
        let entry = IndexEntry::new(
            self.entries.len(),
            offset as usize,
            data_len as usize,
            checksum,
        );

        // Write the data to the underlying writer using the StorageProvider API
        let path = PathBuf::from(self.id.to_string());
        self.provider.write(&path, data).await?;

        // If metadata is provided, write it after the data
        if let Some(meta) = metadata {
            self.provider.write(&path, meta).await?;
        }

        // Record this entry in our list of entries and update current size
        self.entries.push(entry);
        self.current_size += data_len;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use sha2::{Sha256, Digest};

    use super::*;
    
    use std::path::Path;
    use mockall::mock;
    use mockall::predicate::*;
    use byte_counter::counter::ByteCounter;

    // Mocking StorageProvider for testing purposes
    mock! {
        pub FakeStorageProvider {}

        #[async_trait]
        impl StorageProvider for FakeStorageProvider {
            async fn create_bucket(&self, name: &str) -> Result<()>;
            async fn delete_bucket(&self, name: &str) -> Result<()>;
            async fn bucket_exists(&self, name: &str) -> Result<bool>;
            async fn write(&self, path: &Path, data: &[u8]) -> Result<()>;
            async fn read(&self, path: &Path) -> Result<Vec<u8>>;
            async fn delete(&self, path: &Path) -> Result<()>;
            async fn list(&self, prefix: &Path) -> Result<Vec<String>>;
        }
    }

    #[tokio::test]
    async fn test_storage_operations() {
        let mut mock = MockFakeStorageProvider::default();
        
        // Setup expectations
        mock.expect_create_bucket()
            .with(eq("test-bucket"))
            .times(1)
            .returning(|_| Ok(()));

        mock.expect_bucket_exists()
            .with(eq("test-bucket"))
            .times(1)
            .returning(|_| Ok(true));

        mock.expect_write()
            .with(eq(Path::new("test/path")), eq("test-data".as_bytes()))
            .times(1)
            .returning(|_, _| Ok(()));

        mock.expect_read()
            .with(eq(Path::new("test/path")))
            .times(1)
            .returning(|_| Ok(vec![1, 2, 3]));

        mock.expect_list()
            .with(eq(Path::new("test/")))
            .times(1)
            .returning(|_| Ok(vec!["test/path1".to_string(), "test/path2".to_string()]));

        mock.expect_delete()
            .with(eq(Path::new("test/path")))
            .times(1)
            .returning(|_| Ok(()));

        mock.expect_delete_bucket()
            .with(eq("test-bucket"))
            .times(1)
            .returning(|_| Ok(()));

        // Test the mock
        assert!(mock.create_bucket("test-bucket").await.is_ok());
        assert!(mock.bucket_exists("test-bucket").await.unwrap());
        assert!(mock.write(Path::new("test/path"), "test-data".as_bytes()).await.is_ok());
        assert_eq!(mock.read(Path::new("test/path")).await.unwrap(), vec![1, 2, 3]);
        assert_eq!(mock.list(Path::new("test/")).await.unwrap(), 
                  vec!["test/path1".to_string(), "test/path2".to_string()]);
        assert!(mock.delete(Path::new("test/path")).await.is_ok());
        assert!(mock.delete_bucket("test-bucket").await.is_ok());
    }

    #[test]
    fn test_shard_writer_new() {
        let mock_provider = MockFakeStorageProvider::default();
        let id = ByteCounter::default();
        let writer = ShardWriter::new(id, mock_provider);
        assert_eq!(writer.current_size, 0);
        assert!(writer.entries.is_empty());
    }

    #[tokio::test]
    async fn test_write_success_no_metadata() {
        let data = b"some_data";
        let key = "key1";
        let id = ByteCounter::default();

        let mut mock_provider = MockFakeStorageProvider::default();
        mock_provider.expect_write()
            .with(eq(PathBuf::from(id.to_string())), eq(data.clone()))
            .times(1)
            .returning(|_, _| Ok(()));
        
        let mut writer = ShardWriter::new(id, mock_provider);
        
        assert!(writer.write(key, data, None).await.is_ok());
        assert_eq!(writer.current_size, 9);
        assert_eq!(writer.entries.len(), 1);
        assert_eq!(writer.entries[0].offset, 0);
        assert_eq!(writer.entries[0].size, 9);
    }

    #[tokio::test]
    async fn test_write_success_with_metadata() {
        let data = b"some_data";
        let metadata = b"metadata";
        let key = "key1";
        let id = ByteCounter::default();
        let expected_path = PathBuf::from(id.clone().to_string());

        let mut mock_provider = MockFakeStorageProvider::default();
        
        let ep_1 = expected_path.clone();
        mock_provider.expect_write()
            .withf(move |path_arg, data_arg| {
                path_arg.as_os_str() == ep_1.as_os_str() && 
                data_arg == b"some_data"
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let ep_2 = expected_path.clone();
        mock_provider.expect_write()
            .withf(move |path_arg, data_arg| {
                path_arg.as_os_str() == ep_2.as_os_str() && 
                data_arg == b"metadata"
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let mut writer = ShardWriter::new(id, mock_provider);
        
        assert!(writer.write(key, data, Some(metadata)).await.is_ok());
        assert_eq!(writer.current_size, data.len() + metadata.len());
        assert_eq!(writer.entries.len(), 1);
        assert_eq!(writer.entries[0].offset, 0);
        assert_eq!(writer.entries[0].size, data.len() + metadata.len());
    }

    #[tokio::test]
    async fn test_write_exceeds_shard_size() {
        let mock_provider = MockFakeStorageProvider::default();
        let id = ByteCounter::default();
        let mut writer = ShardWriter::new(id, mock_provider);
        let data = vec![0; (shard_size() + 1) as usize];
        let key = "key1";

        assert!(writer.write(key, &data, None).await.is_err());
    }

    #[tokio::test]
    async fn test_write_multiple_entries() {
        let data1 = b"some_data";
        let data2 = b"more_data";
        let key1 = "key1";
        let key2 = "key2";
        let id = ByteCounter::default();
        
        let mut mock_provider = MockFakeStorageProvider::default();
        mock_provider.expect_write()
            .with(eq(PathBuf::from(id.to_string())), eq(data1.clone()))
            .times(1)
            .returning(|_, _| Ok(()));
        
        mock_provider.expect_write()
            .with(eq(PathBuf::from(id.to_string())), eq(data2.clone()))
            .times(1)
            .returning(|_, _| Ok(()));

        let mut writer = ShardWriter::new(id, mock_provider);
        
        assert!(writer.write(key1, data1, None).await.is_ok());
        assert_eq!(writer.current_size, 9);
        assert_eq!(writer.entries.len(), 1);

        assert!(writer.write(key2, data2, None).await.is_ok());
        assert_eq!(writer.current_size, 18);
        assert_eq!(writer.entries.len(), 2);
        assert_eq!(writer.entries[0].offset, 0);
        assert_eq!(writer.entries[0].size, 9);
        assert_eq!(writer.entries[1].offset, 9);
        assert_eq!(writer.entries[1].size, 9);
    }

    #[tokio::test]
    async fn test_write_with_large_metadata() {
        let data = b"some_data";
        let metadata_size = (shard_size() - data.len()) as usize;
        let metadata = vec![0; metadata_size];
        let key = "key1";
        let id = ByteCounter::default();
        let expected_path = PathBuf::from(id.clone().to_string());

        let mut mock_provider = MockFakeStorageProvider::default();
        
        let ep_1 = expected_path.clone();
        mock_provider.expect_write()
            .withf(move |path_arg, data_arg| {
                path_arg.as_os_str() == ep_1.as_os_str() && 
                data_arg == b"some_data"
            })
            .times(1)
            .returning(|_, _| Ok(()));
        
        let ep_2 = expected_path.clone();
        mock_provider.expect_write()
            .withf(move |path_arg, data_arg| {
                path_arg.as_os_str() == ep_2.as_os_str() && 
                data_arg.len() == metadata_size &&
                data_arg.iter().all(|&b| b == 0)
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let mut writer = ShardWriter::new(id, mock_provider);
        let result = writer.write(key, data, Some(&metadata)).await;
        assert!(result.is_ok());
        assert_eq!(writer.current_size, data.len() + metadata_size);
        assert_eq!(writer.entries.len(), 1);

        let additional_data = b"more_data";
        let key2 = "key2";
        assert!(writer.write(key2, additional_data, None).await.is_err());
    }


    #[tokio::test]
    async fn test_write_checksum() {
        let data = b"some_data";
        let key = "key1";
        let id = ByteCounter::default();

        let mut mock_provider = MockFakeStorageProvider::default();
        mock_provider.expect_write()
            .with(eq(PathBuf::from(id.to_string())), eq(data.clone()))
            .times(1)
            .returning(|_, _| Ok(()));
            
        let mut writer = ShardWriter::new(id, mock_provider);
        

        assert!(writer.write(key, data, None).await.is_ok());

        let mut hasher = Sha256::new();
        hasher.update(data);
        let expected_checksum: [u8; 32] = hasher.finalize().into();

        assert_eq!(writer.entries[0].checksum, expected_checksum);
    }
}