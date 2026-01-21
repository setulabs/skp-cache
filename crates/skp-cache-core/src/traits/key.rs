//! Cache key trait and implementations

use std::fmt::Display;

/// Trait for types that can be used as cache keys
///
/// Implement this trait to use custom types as cache keys.
pub trait CacheKey: Send + Sync {
    /// Generate the key string
    fn cache_key(&self) -> String;

    /// Optional namespace for the key
    fn namespace(&self) -> Option<&str> {
        None
    }

    /// Get the full key including namespace
    fn full_key(&self) -> String {
        match self.namespace() {
            Some(ns) => format!("{}:{}", ns, self.cache_key()),
            None => self.cache_key(),
        }
    }
}

// Implementations for common types

impl CacheKey for String {
    fn cache_key(&self) -> String {
        self.clone()
    }
}

impl CacheKey for &str {
    fn cache_key(&self) -> String {
        self.to_string()
    }
}

impl CacheKey for &String {
    fn cache_key(&self) -> String {
        (*self).clone()
    }
}

// Tuple implementations for composite keys

impl<T: Display + Send + Sync> CacheKey for (T,) {
    fn cache_key(&self) -> String {
        self.0.to_string()
    }
}

impl<T1: Display + Send + Sync, T2: Display + Send + Sync> CacheKey for (T1, T2) {
    fn cache_key(&self) -> String {
        format!("{}:{}", self.0, self.1)
    }
}

impl<T1: Display + Send + Sync, T2: Display + Send + Sync, T3: Display + Send + Sync> CacheKey
    for (T1, T2, T3)
{
    fn cache_key(&self) -> String {
        format!("{}:{}:{}", self.0, self.1, self.2)
    }
}

impl<
        T1: Display + Send + Sync,
        T2: Display + Send + Sync,
        T3: Display + Send + Sync,
        T4: Display + Send + Sync,
    > CacheKey for (T1, T2, T3, T4)
{
    fn cache_key(&self) -> String {
        format!("{}:{}:{}:{}", self.0, self.1, self.2, self.3)
    }
}

/// Composite key builder for complex keys
#[derive(Debug, Clone)]
pub struct CompositeKey {
    parts: Vec<String>,
    ns: Option<String>,
}

impl CompositeKey {
    /// Create a new composite key builder
    pub fn new() -> Self {
        Self {
            parts: Vec::new(),
            ns: None,
        }
    }

    /// Set the namespace
    pub fn with_namespace(mut self, ns: impl Into<String>) -> Self {
        self.ns = Some(ns.into());
        self
    }

    /// Add a part to the key
    pub fn part(mut self, part: impl Display) -> Self {
        self.parts.push(part.to_string());
        self
    }

    /// Add multiple parts
    pub fn parts<I, S>(mut self, parts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Display,
    {
        self.parts.extend(parts.into_iter().map(|p| p.to_string()));
        self
    }

    /// Get the namespace
    pub fn get_namespace(&self) -> Option<&str> {
        self.ns.as_deref()
    }
}

impl Default for CompositeKey {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheKey for CompositeKey {
    fn cache_key(&self) -> String {
        self.parts.join(":")
    }

    fn namespace(&self) -> Option<&str> {
        self.ns.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_key() {
        let key = "my_key".to_string();
        assert_eq!(key.cache_key(), "my_key");
        assert_eq!(key.full_key(), "my_key");
    }

    #[test]
    fn test_str_key() {
        let key = "my_key";
        assert_eq!(key.cache_key(), "my_key");
    }

    #[test]
    fn test_tuple_key_2() {
        let key = ("user", 123);
        assert_eq!(key.cache_key(), "user:123");
    }

    #[test]
    fn test_tuple_key_3() {
        let key = ("org", 1, "user");
        assert_eq!(key.cache_key(), "org:1:user");
    }

    #[test]
    fn test_composite_key() {
        let key = CompositeKey::new()
            .with_namespace("myapp")
            .part("user")
            .part(123);

        assert_eq!(key.cache_key(), "user:123");
        assert_eq!(key.get_namespace(), Some("myapp"));
        assert_eq!(key.full_key(), "myapp:user:123");
    }

    #[test]
    fn test_composite_key_no_namespace() {
        let key = CompositeKey::new().part("session").part("abc123");

        assert_eq!(key.cache_key(), "session:abc123");
        assert_eq!(key.full_key(), "session:abc123");
    }
}
