use serde::{Deserialize, Serialize};

use crate::StorageProvider;

#[derive(Serialize, Deserialize)]
pub struct ShardReader<W: StorageProvider> {
    reader: W,
}

impl<W: StorageProvider> Default for ShardReader<W> {
    fn default() -> Self {
        Self { reader: Default::default() }
    }
}
