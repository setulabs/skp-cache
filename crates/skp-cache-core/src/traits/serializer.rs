//! Pluggable serialization trait

use crate::CacheError;
use serde::{de::DeserializeOwned, Serialize};

/// Trait for pluggable serialization formats
///
/// Implement this trait to add custom serialization formats.
/// Built-in implementations: JSON, MessagePack, Bincode.
pub trait Serializer: Send + Sync + Clone + 'static {
    /// Name of the serializer (for debugging/metrics)
    fn name(&self) -> &str;

    /// Serialize a value to bytes
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, CacheError>;

    /// Deserialize bytes to a value
    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, CacheError>;
}

/// JSON serializer (default)
///
/// Human-readable, widely compatible, good for debugging.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonSerializer;

impl Serializer for JsonSerializer {
    fn name(&self) -> &str {
        "json"
    }

    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, CacheError> {
        serde_json::to_vec(value).map_err(|e| CacheError::Serialization(e.to_string()))
    }

    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, CacheError> {
        serde_json::from_slice(bytes).map_err(|e| CacheError::Deserialization(e.to_string()))
    }
}

/// MessagePack serializer (optional)
///
/// Faster and more compact than JSON, but not human-readable.
/// Enable with `msgpack` feature.
#[cfg(feature = "msgpack")]
#[derive(Debug, Clone, Copy, Default)]
pub struct MsgPackSerializer;

#[cfg(feature = "msgpack")]
impl Serializer for MsgPackSerializer {
    fn name(&self) -> &str {
        "msgpack"
    }

    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, CacheError> {
        rmp_serde::to_vec(value).map_err(|e| CacheError::Serialization(e.to_string()))
    }

    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, CacheError> {
        rmp_serde::from_slice(bytes).map_err(|e| CacheError::Deserialization(e.to_string()))
    }
}

/// Bincode serializer (optional)
///
/// Fastest and most compact, but not human-readable or cross-language.
/// Enable with `bincode` feature.
#[cfg(feature = "bincode")]
#[derive(Debug, Clone, Copy, Default)]
pub struct BincodeSerializer;

#[cfg(feature = "bincode")]
impl Serializer for BincodeSerializer {
    fn name(&self) -> &str {
        "bincode"
    }

    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, CacheError> {
        bincode::serde::encode_to_vec(value, bincode::config::standard())
            .map_err(|e| CacheError::Serialization(e.to_string()))
    }

    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, CacheError> {
        let (val, _len) = bincode::serde::decode_from_slice(bytes, bincode::config::standard())
            .map_err(|e| CacheError::Deserialization(e.to_string()))?;
        Ok(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_roundtrip() {
        let serializer = JsonSerializer;
        let value = vec![1, 2, 3, 4, 5];

        let bytes = serializer.serialize(&value).unwrap();
        let decoded: Vec<i32> = serializer.deserialize(&bytes).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_json_struct() {
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct TestStruct {
            name: String,
            value: i32,
        }

        let serializer = JsonSerializer;
        let value = TestStruct {
            name: "test".to_string(),
            value: 42,
        };

        let bytes = serializer.serialize(&value).unwrap();
        let decoded: TestStruct = serializer.deserialize(&bytes).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_json_serializer_name() {
        assert_eq!(JsonSerializer.name(), "json");
    }
}
