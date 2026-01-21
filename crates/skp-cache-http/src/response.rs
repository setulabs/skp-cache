use serde::{Serialize, Deserialize};
use http::{StatusCode, HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;

/// Serializable wrapper around HTTP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl CachedResponse {
    /// Create new cached response
    pub fn new(status: u16, headers: HashMap<String, String>, body: Vec<u8>) -> Self {
        Self { status, headers, body }
    }
    
    /// Create from http::Response parts
    pub fn from_parts(status: StatusCode, headers: &HeaderMap, body: Vec<u8>) -> Self {
        let mut headers_map = HashMap::new();
        for (k, v) in headers.iter() {
            if let Ok(s) = v.to_str() {
                headers_map.insert(k.to_string(), s.to_string());
            }
        }
        
        Self {
            status: status.as_u16(),
            headers: headers_map,
            body,
        }
    }
    
    /// Convert headers to HeaderMap
    pub fn headers_map(&self) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in &self.headers {
             if let (Ok(name), Ok(val)) = (HeaderName::from_bytes(k.as_bytes()), HeaderValue::from_str(v)) {
                 map.insert(name, val);
             }
        }
        map
    }
}
