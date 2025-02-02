use byte_counter::counter::ByteCounter;

use crate::StorageProvider;

use super::{reader::ShardReader, writer::ShardWriter};


/// A `Shard` represents a segment of data that can be read from and written to.
///
/// The generic type `S` is expected to be some form of mutable reference to a binary file,
/// such as `&mut std::fs::File`. This allows the `ShardWriter` and `ShardReader` to operate
/// on the underlying file, keeping track of byte counts through `ByteCounter`.
pub struct Shard<S: StorageProvider> {
    id: ByteCounter,
    writer: ShardWriter<S>,
    reader: ShardReader<S>,
}

impl<S: StorageProvider> Default for Shard<S> {
    fn default() -> Self {
        Shard {
            id: ByteCounter::default(),
            writer: ShardWriter::default(),
            reader: ShardReader::default(),
        }
    }
}

impl<S: StorageProvider> Shard<S> {
    pub fn new() -> Self {
        Self { 
            id: ByteCounter::default(), 
            writer: ShardWriter::default(), 
            reader: ShardReader::default() 
        }
    }
}
