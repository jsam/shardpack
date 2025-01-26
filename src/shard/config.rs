const SHARD_SIZE: usize = 256 * 1024 * 1024; // 256MB


pub fn shard_size() -> usize {
    // TODO: check env
    return SHARD_SIZE
}