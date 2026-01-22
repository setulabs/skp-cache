//! Compression support for cached values
//!
//! Provides zstd compression to reduce memory usage and network bandwidth.

use crate::CacheError;

/// Compression level (1-22, higher = better compression but slower)
pub const DEFAULT_COMPRESSION_LEVEL: i32 = 3;

/// Minimum size threshold for compression (bytes)
/// Values smaller than this won't be compressed
pub const MIN_COMPRESSION_SIZE: usize = 256;

/// Trait for compression implementations
pub trait Compressor: Send + Sync + Clone + 'static {
    /// Name of the compressor
    fn name(&self) -> &str;

    /// Compress data
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError>;

    /// Decompress data
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError>;

    /// Check if data should be compressed (based on size threshold)
    fn should_compress(&self, data: &[u8]) -> bool {
        data.len() >= MIN_COMPRESSION_SIZE
    }
}

/// No-op compressor (disabled compression)
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopCompressor;

impl Compressor for NoopCompressor {
    fn name(&self) -> &str {
        "none"
    }

    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        Ok(data.to_vec())
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        Ok(data.to_vec())
    }

    fn should_compress(&self, _data: &[u8]) -> bool {
        false
    }
}

/// Zstd compressor
#[cfg(feature = "compression")]
#[derive(Debug, Clone)]
pub struct ZstdCompressor {
    level: i32,
    min_size: usize,
}

#[cfg(feature = "compression")]
impl Default for ZstdCompressor {
    fn default() -> Self {
        Self::new(DEFAULT_COMPRESSION_LEVEL)
    }
}

#[cfg(feature = "compression")]
impl ZstdCompressor {
    /// Create a new zstd compressor with the given compression level (1-22)
    pub fn new(level: i32) -> Self {
        Self {
            level: level.clamp(1, 22),
            min_size: MIN_COMPRESSION_SIZE,
        }
    }

    /// Set minimum size for compression
    pub fn with_min_size(mut self, size: usize) -> Self {
        self.min_size = size;
        self
    }

    /// Get the compression level
    pub fn level(&self) -> i32 {
        self.level
    }
}

#[cfg(feature = "compression")]
impl Compressor for ZstdCompressor {
    fn name(&self) -> &str {
        "zstd"
    }

    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        zstd::encode_all(data, self.level)
            .map_err(|e| CacheError::Compression(e.to_string()))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        zstd::decode_all(data)
            .map_err(|e| CacheError::Decompression(e.to_string()))
    }

    fn should_compress(&self, data: &[u8]) -> bool {
        data.len() >= self.min_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_compressor() {
        let compressor = NoopCompressor;
        let data = b"hello world";

        let compressed = compressor.compress(data).unwrap();
        assert_eq!(compressed, data);

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);

        assert!(!compressor.should_compress(data));
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_zstd_compressor() {
        let compressor = ZstdCompressor::new(3);

        // Large data should compress
        let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();

        let compressed = compressor.compress(&data).unwrap();
        // Compressed should be smaller (for repetitive data)
        assert!(compressed.len() < data.len());

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_zstd_should_compress() {
        let compressor = ZstdCompressor::new(3);

        // Small data shouldn't be compressed
        assert!(!compressor.should_compress(b"small"));

        // Large data should be compressed
        let large: Vec<u8> = vec![0; 1024];
        assert!(compressor.should_compress(&large));
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_zstd_level_clamping() {
        let low = ZstdCompressor::new(-5);
        assert_eq!(low.level(), 1);

        let high = ZstdCompressor::new(100);
        assert_eq!(high.level(), 22);
    }
}
