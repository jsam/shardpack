use sha2::{Sha256, Digest};

use crate::Error;
use crate::types::Result;

/// Computes the SHA-256 checksum of the provided data.
///
/// # Arguments
///
/// * `data` - A slice of bytes representing the data to hash.
///
/// # Returns
///
/// A 32-byte array representing the computed SHA-256 checksum.
pub fn compute_checksum(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}
 
// Verify the checksum
pub fn verify_checksum(data: &[u8], expected: &[u8; 32]) -> Result<()> {
    let actual = compute_checksum(data);
    if actual == *expected {
        Ok(())
    } else {
        Err(Error::Storage("Checksum mismatch".into()))
    }
}