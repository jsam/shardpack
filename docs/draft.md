# ShardPack File Format Specification

## 1. Introduction

**ShardPack** is a dataset format designed for large-scale deep learning workflows. It implements data packing of data samples in sharded container files for efficient I/O—but it provides native indexing, random access, and flexible metadata options without the overhead of relying on tar-based archives. 

Key features include:

1. **Sharding**: Store massive datasets in chunked files that are easy to distribute.  
2. **Native Indexing**: Each shard contains an end-of-file index for fast random access.  
3. **Flexible Metadata**: Every file within a shard can include per-sample metadata (e.g., MIME type, compression info) to ease decoding.  
4. **Parallel Processing**: Designed to handle concurrent reads, writes, and distributed training usage.  
5. **Portability**: A simple binary record format with minimal external dependencies.  

By reducing the reliance on external indexing tools (e.g., tar offset scanning) and making efficient random access a first-class citizen, ShardPack is ideal for scenarios where partial or selective data access is required—without sacrificing the ability to do pure streaming in large-scale training.

---

## 2. Shard Organization

### 2.1 Shard Files

- Each dataset consists of one or more **ShardPack** files, typically ending in `.shardpack`.
- Shards should be numbered sequentially. For instance:
  ```
  dataset-train-000000.shardpack
  dataset-train-000001.shardpack
  ...
  dataset-train-000973.shardpack
  ```
- Shard references may use brace notation or any equivalent range expansion. 

### 2.2 Shard Internal Layout

A single ShardPack file is structured as follows:

```
+-------------------------+-------------------------+--- ... ---+-------------------------+
|    Record 1 Block      |    Record 2 Block      |           |     Record N Block      |
+-------------------------+-------------------------+--- ... ---+-------------------------+
|                     Index + Shard Metadata (EOF)                                        |
+-----------------------------------------------------------------------------------------+
```

- **Record Blocks**: Each block corresponds to a **sample** (or combined sample) containing one or more data items (images, text, metadata, etc.).
- **Index + Shard Metadata**: An end-of-file (EOF) structure containing:
  - A table of offsets pointing to the start of each record block.
  - Shard-wide metadata (e.g., creation date, version, or user-defined metadata).
  - A magic number/header to help identify ShardPack files.

By placing the index at the end, ShardPack supports pure sequential streaming (you can read record blocks one after the other) but also allows for fast random access once the index is read.

---

## 3. Record Layout

Each **record block** represents one sample (or a micro-batch, or however you choose to group data). This record block is a small container of multiple **file entries** (the data that would traditionally be separate files in a tar-based approach). The structure is illustrated below:

```
+--------------------------------------+
|    Record Header (fixed-size)        |
+--------------------------------------+
|    Number of file entries (uint32)   |
|    Possibly record-level metadata    |
+--------------------------------------+
|    File Entry 1: header + data       |
+--------------------------------------+
|    File Entry 2: header + data       |
+--------------------------------------+
|                 ...                  |
+--------------------------------------+
|    File Entry M: header + data       |
+--------------------------------------+
```

### 3.1 Record Header

- **Record Size (u64)**: Number of bytes in this record block (including record header, file entries, and any inline metadata).
- **Record Key (variable length)**: A string identifier or key for the sample. This could be `images17/image194` or an auto-generated UUID. 
- **Optional Record-level Metadata**: Could store additional info about how to decode or the sample’s schema. This part is flexible in length and format (JSON, MsgPack, etc.).

### 3.2 File Entries

Each record contains **M** file entries (e.g., an image, a text annotation, a depth map). Every file entry has:

1. **File Entry Header**  
   - **File Name (string)**: e.g., `left.jpg`, `annotation.json`, etc.  
   - **File Content Type (string)**: e.g., `image/jpeg`, `application/json`.  
   - **File Size (u64)**: size in bytes of the file content.  
   - **(Optional) Compression / Encoding Info**: e.g., `gzip`, `lz4`, or `none`.  

2. **File Content (raw or compressed bytes)**

Because each entry includes a content type and optional encoding flag, libraries can decode or decompress entries as needed without guesswork. This also eliminates the need to rely solely on the file extension.

---

## 4. Index and Metadata

### 4.1 Index

After writing all record blocks, a ShardPack writer appends a binary index to the end of the file:

```
+-----------------------------------+
|    Number of Records (uint64)     |
+-----------------------------------+
|    [Record 1 Offset (u64)]        |
|    [Record 2 Offset (u64)]        |
|    ...                            |
|    [Record N Offset (u64)]        |
+-----------------------------------+
|    Shard-level Metadata (var.)    |
+-----------------------------------+
|    Magic Number / Footer (fixed)  |
+-----------------------------------+
```

- **Number of Records** indicates how many record blocks exist.
- Each **Record Offset** is a file-position pointer to the beginning of the corresponding record block (just after the previous record block ended).
- **Shard-level Metadata** might include user-defined fields (JSON, key-value pairs, etc.), creation timestamps, shard version, and so on.
- A **Magic Number** or footer label ensures the file can be correctly identified and validated as a ShardPack archive.

### 4.2 Random Access

Once an application reads the index from the end of a `.shardpack`, it can jump directly to any record offset using standard file-seeking operations. This substantially improves the speed of selective data access or multi-worker parallel data loading (without storing separate external index files).

---

## 5. Usage and Tools

### 5.1 Creating Shards

Example CLI usage (hypothetical `shardpack` command-line tool written in Rust):

```bash
shardpack create \
    --input input_data_directory \
    --output dataset-train-000000.shardpack \
    --record-size-limit 1GB \
    --compression lz4 \
    --metadata creator="MyName" training="true"
```

1. **Input** can be a directory or a structured layout specifying which files go in each record.  
2. **Record Size Limit** might be used to avoid giant record blocks (useful if you have multi-GB single samples or micro-batches).  
3. **Compression** can be applied on a per-file-entry basis or at a chunk level.  
4. **Metadata** can store custom shard-level fields.

### 5.2 Reading Shards

A typical data-loading library or script (Python/Rust/Julia) might do:

```python
import shardpack

dataset = shardpack.open("dataset-train-000000.shardpack")
for record in dataset.records():
    for entry in record.file_entries():
        if entry.content_type == "image/jpeg":
            image = decode_jpeg(entry.data)
        elif entry.content_type == "application/json":
            annotation = json.loads(entry.data)
    # do something with image, annotation ...
```

Or for random access:

```python
# read index once
index = dataset.get_index()
random_record_offset = index[123]
record = dataset.read_record_at_offset(random_record_offset)
print(f"Record key: {record.key}")
```

### 5.3 Parallel and Distributed Processing

- **Single Shard** can be streamed sequentially for simpler tasks.  
- **Multiple Shards** can be processed in parallel; each worker node grabs its own shard file or partial range of offsets.  
- Because the index is self-contained within each shard, there is no need for external indexing tools, though you can still build external shard-of-shards catalogs if desired.

---

## 6. Recommended File Types

ShardPack itself is file-type agnostic, but the built-in content type field in file entries encourages explicit specifications such as:

- **`image/jpeg`** for JPEG images  
- **`image/png`** for PNG images  
- **`application/json`** for JSON annotations  
- **`application/msgpack`** for MsgPack annotations  
- **`application/x-npy`** or **`application/x-npz`** for NumPy arrays  

This is more explicit and flexible than relying on `.jpg`, `.json` suffixes alone; it also opens the door for new or custom data formats to be recognized at runtime.

---

## 7. Advanced Use Cases

### 7.1 Column Storage

ShardPack supports building “columnar shards.” However, ShardPack’s approach to random access and indexing significantly reduces overhead. One could store left images in one ShardPack, right images in another, and depth data in a third, each with a consistent record-key ordering. The library can quickly jump to matching keys across these shards if needed.

> **Caution**: As with any multi-shard column storage approach, you must maintain consistent record ordering to keep columns aligned.

### 7.2 Updating Small Columns

Because ShardPack is designed for immutable writes (much like a tar-based format), storing frequently changing small columns (e.g., dynamic labels) is best done via either:
- A small ShardPack with the updated data keyed identically to the main dataset, combined at read time.  
- A dedicated database (SQLite, LMDB, or a key-value store) that your training pipelines can integrate seamlessly.

### 7.3 Streaming vs Random Access

- **Pure Streaming**: Read from the beginning of the file block-by-block, ignoring the index.  
- **Random Access**: Load the index once, then seek to any record offset in O(1) time.

ShardPack’s design is flexible enough to accommodate both styles efficiently.

---

## 8. Example ShardPack Layout Walkthrough

Suppose we have the following input sample grouping (like a stereo dataset with metadata):

```
sample_key = "images17/image194"
Files:
  - left.jpg (JPEG image)
  - right.jpg (JPEG image)
  - meta.json (JSON annotation)
```

In ShardPack, that single sample might be stored as one record:

```
Record Header:
  - Record Size = <some uint64>
  - Record Key = "images17/image194"
  - Possibly some record-level metadata

File Entries (3 total):
  1) File Entry Header
     - File Name = "left.jpg"
     - Content Type = "image/jpeg"
     - File Size = ...
     - Data = (raw JPEG bytes or compressed bytes)
  2) File Entry Header
     - File Name = "right.jpg"
     - Content Type = "image/jpeg"
     - File Size = ...
     - Data = (raw JPEG bytes)
  3) File Entry Header
     - File Name = "meta.json"
     - Content Type = "application/json"
     - File Size = ...
     - Data = {"stereo":true,"notes":"example"...}
```

---

## 9. Reference Implementation Sketch (Rust)

Below is a high-level outline of how a Rust library for **ShardPack** might look. (Note: This is an illustrative code sketch rather than a full production-ready library.)

```rust
/// shardpack/src/lib.rs
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Seek, SeekFrom, Write, Read};

/// Struct describing a file entry within a record
pub struct FileEntry {
    pub file_name: String,
    pub content_type: String,
    pub data: Vec<u8>,
    pub compressed: bool, // or store compression scheme
}

/// A single record inside a ShardPack shard
pub struct Record {
    pub key: String,
    pub metadata: Vec<u8>, // e.g., JSON or MsgPack
    pub file_entries: Vec<FileEntry>,
}

/// Writer for ShardPack
pub struct ShardPackWriter {
    writer: BufWriter<File>,
    offsets: Vec<u64>, // to store offsets for each record
    record_count: u64,
    shard_metadata: Vec<u8>,
}

impl ShardPackWriter {
    pub fn create(path: &str, shard_metadata: Option<&[u8]>) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
            offsets: vec![],
            record_count: 0,
            shard_metadata: shard_metadata.unwrap_or(&[]).to_vec(),
        })
    }

    pub fn write_record(&mut self, record: &Record) -> std::io::Result<()> {
        // track offset
        let offset = self.writer.seek(SeekFrom::Current(0))?;
        self.offsets.push(offset);

        // serialize record header (size placeholder, key, metadata, etc.)
        // write file entries
        // handle compression if needed
        // update record_count
        // ...
        Ok(())
    }

    /// Finalize the shard by writing the index at EOF
    pub fn finalize(&mut self) -> std::io::Result<()> {
        // write index: [record_count, offsets, shard_metadata, magic number]
        // flush and close writer
        Ok(())
    }
}

/// Reader for ShardPack
pub struct ShardPackReader {
    file: BufReader<File>,
    offsets: Vec<u64>,
    record_count: u64,
    shard_metadata: Vec<u8>,
}

impl ShardPackReader {
    pub fn open(path: &str) -> std::io::Result<Self> {
        let mut file = BufReader::new(File::open(path)?);
        // Seek to end, read index, parse into offsets, record_count, etc.
        // Seek back to 0 or handle as needed
        // ...
        Ok(Self {
            file,
            offsets: vec![],
            record_count: 0,
            shard_metadata: vec![],
        })
    }

    pub fn read_record(&mut self, index: usize) -> std::io::Result<Record> {
        // seek to offsets[index]
        // deserialize record header and file entries
        // return Record
        Ok(Record {
            key: "".to_string(),
            metadata: vec![],
            file_entries: vec![],
        })
    }

    pub fn iter_records(&mut self) -> RecordIterator<'_> {
        // return an iterator that reads records sequentially
        RecordIterator { reader: self }
    }
}

pub struct RecordIterator<'a> {
    reader: &'a mut ShardPackReader,
}

impl<'a> Iterator for RecordIterator<'a> {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        // read next record sequentially from current file position
        // stop after record_count
        None
    }
}
```

This example demonstrates the basic building blocks: a writer that accumulates record offsets and writes them out at the end, and a reader that loads these offsets for random access or streaming iteration.

---

## 10. Why ShardPack is “Better”

**1. Native Random Access**  
Unlike tar-based formats, which often require a separate indexing step (scanning the tar to find offsets) or external index files, ShardPack stores the index inside the same file. This significantly accelerates partial reading or seeking, especially in distributed contexts.

**2. Self-Describing**  
Each file entry has a clear content type or compression scheme, removing guesswork. This is particularly convenient for multi-language or multi-framework pipelines (e.g., Rust, Python, or Julia).

**3. Flexible Metadata**  
Shard-level metadata, record-level metadata, and file-level metadata all have dedicated places. This fosters better dataset documentation and self-describing data storage.

**4. Streaming + Random Access**  
By placing the index at the end, ShardPack can be generated in a single pass (streaming) and read from start to finish (like a pure streaming dataset). Once the entire file is complete, advanced usage (random access, skipping) becomes trivial.

**5. Parallelization**  
ShardPack splits large datasets into independent shards, each containing its own index. This design trivially supports multi-GPU or multi-node training scenarios where workers can pick up separate shards or read specific records on demand.

**6. Minimal Overhead**  
ShardPack is a straightforward binary format without extra layers like tar headers for each file entry, reducing redundancy and potentially improving I/O efficiency.

---

