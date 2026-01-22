//! Error types for cache operations

use thiserror::Error;

/// Main error type for all cache operations
#[derive(Error, Debug, Clone)]
pub enum CacheError {
    /// Key not found in cache
    #[error("key not found: {0}")]
    NotFound(String),

    /// Serialization failed
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Deserialization failed
    #[error("deserialization error: {0}")]
    Deserialization(String),

    /// Backend connection failed
    #[error("connection error: {0}")]
    Connection(String),

    /// Backend operation failed
    #[error("backend error: {0}")]
    Backend(String),

    /// Cyclic dependency detected
    #[error("cyclic dependency detected for key: {0}")]
    CyclicDependency(String),

    /// Lock acquisition failed
    #[error("lock conflict for key: {0}")]
    LockConflict(String),

    /// Version mismatch for conditional operation
    #[error("version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u64, actual: u64 },

    /// Capacity exceeded
    #[error("capacity exceeded")]
    CapacityExceeded,

    /// Internal error
    #[error("internal error: {0}")]
    Internal(String),

    /// Compression failed
    #[error("compression error: {0}")]
    Compression(String),

    /// Decompression failed
    #[error("decompression error: {0}")]
    Decompression(String),

    /// Timeout
    #[error("operation timed out")]
    Timeout,
}

/// Result type alias for cache operations
pub type Result<T> = std::result::Result<T, CacheError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CacheError::NotFound("test_key".to_string());
        assert_eq!(err.to_string(), "key not found: test_key");

        let err = CacheError::Serialization("failed".to_string());
        assert_eq!(err.to_string(), "serialization error: failed");

        let err = CacheError::VersionMismatch {
            expected: 1,
            actual: 2,
        };
        assert_eq!(err.to_string(), "version mismatch: expected 1, got 2");
    }

    #[test]
    fn test_error_clone() {
        let err = CacheError::Timeout;
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }
}
