use crate::{error::{Error, Result}, index::IndexEntry};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use sha2::{Sha256, Digest};
use bincode;

const SHARD_SIZE: u64 = 256 * 1024 * 1024; // 256MB

#[derive(Serialize, Deserialize)]
pub struct ShardWriter<W: Write> {
    writer: W,
    current_size: u64,
    entries: Vec<IndexEntry>,
}

impl<W: Write> ShardWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            current_size: 0,
            entries: Vec::new(),
        }
    }

    pub fn write(&mut self, key: &str, data: &[u8], metadata: Option<&[u8]>) -> Result<()> {
        let data_len = data.len() as u64;
        if self.current_size + data_len > SHARD_SIZE {
            return Err(Error::Storage("Shard size limit exceeded".into()));
        }

        let mut hasher = Sha256::new();
        hasher.update(data);
        let checksum = hasher.finalize().into();

        let offset = self.current_size;
        let entry = IndexEntry::new(
            0, 
            offset,
            data_len,
            checksum,
        );

        self.writer.write_all(data)?;
        if let Some(meta) = metadata {
            self.writer.write_all(meta)?;
        }

        self.entries.push(entry);
        self.current_size += data_len;
        Ok(())
    }
}