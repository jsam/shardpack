use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::StorageProvider;
use crate::types::Result;

#[derive(Serialize, Deserialize)]
pub struct ShardReader<W: StorageProvider> {
    reader: W,
    path: PathBuf
}

impl<W: StorageProvider> Default for ShardReader<W> {
    fn default() -> Self {
        Self { reader: Default::default(), path: Default::default() }
    }
}

impl<W: StorageProvider> ShardReader<W> {
    
    async fn read_all(&self) -> Result<Vec<u8>> {
        self.reader.read(&self.path).await
    }


}