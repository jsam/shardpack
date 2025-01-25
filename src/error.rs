use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Index error: {0}")]
    Index(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
}

pub type Result<T> = std::result::Result<T, Error>;