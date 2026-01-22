//! Bloom filter for fast negative cache lookups
//!
//! A bloom filter is a probabilistic data structure that can quickly determine
//! if a key is definitely NOT in the cache, avoiding unnecessary backend lookups.
//! False positives are possible, but false negatives are not.

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// A thread-safe bloom filter optimized for cache negative lookups
///
/// Uses atomic operations for lock-free concurrent access.
pub struct BloomFilter {
    /// Bit array stored as atomic u64s
    bits: Box<[AtomicU64]>,
    /// Number of hash functions to use
    num_hashes: usize,
    /// Total number of bits
    num_bits: usize,
}

impl BloomFilter {
    /// Create a new bloom filter with specified capacity and false positive rate
    ///
    /// # Arguments
    /// * `expected_items` - Expected number of items to store
    /// * `false_positive_rate` - Desired false positive rate (e.g., 0.01 for 1%)
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        // Calculate optimal parameters
        // m = -n * ln(p) / (ln(2)^2)
        // k = (m/n) * ln(2)
        let ln2 = std::f64::consts::LN_2;
        let ln2_sq = ln2 * ln2;

        let num_bits = (-(expected_items as f64) * false_positive_rate.ln() / ln2_sq).ceil() as usize;
        let num_bits = num_bits.max(64); // Minimum 64 bits

        let num_hashes = ((num_bits as f64 / expected_items as f64) * ln2).ceil() as usize;
        let num_hashes = num_hashes.clamp(1, 16); // Between 1 and 16 hash functions

        // Round up to multiple of 64 for atomic storage
        let num_u64s = (num_bits + 63) / 64;
        let actual_bits = num_u64s * 64;

        let bits: Box<[AtomicU64]> = (0..num_u64s)
            .map(|_| AtomicU64::new(0))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            bits,
            num_hashes,
            num_bits: actual_bits,
        }
    }

    /// Create with specific size parameters
    pub fn with_size(num_bits: usize, num_hashes: usize) -> Self {
        let num_u64s = (num_bits + 63) / 64;
        let actual_bits = num_u64s * 64;

        let bits: Box<[AtomicU64]> = (0..num_u64s)
            .map(|_| AtomicU64::new(0))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            bits,
            num_hashes,
            num_bits: actual_bits,
        }
    }

    /// Insert a key into the bloom filter
    pub fn insert(&self, key: &str) {
        for i in 0..self.num_hashes {
            let bit_idx = self.hash_index(key, i);
            let word_idx = bit_idx / 64;
            let bit_pos = bit_idx % 64;

            self.bits[word_idx].fetch_or(1 << bit_pos, Ordering::Relaxed);
        }
    }

    /// Check if a key might be in the set
    ///
    /// Returns:
    /// - `false` if the key is definitely NOT in the set
    /// - `true` if the key MIGHT be in the set (could be false positive)
    pub fn might_contain(&self, key: &str) -> bool {
        for i in 0..self.num_hashes {
            let bit_idx = self.hash_index(key, i);
            let word_idx = bit_idx / 64;
            let bit_pos = bit_idx % 64;

            if self.bits[word_idx].load(Ordering::Relaxed) & (1 << bit_pos) == 0 {
                return false;
            }
        }
        true
    }

    /// Remove all entries (reset the filter)
    pub fn clear(&self) {
        for word in self.bits.iter() {
            word.store(0, Ordering::Relaxed);
        }
    }

    /// Get the number of bits in the filter
    pub fn num_bits(&self) -> usize {
        self.num_bits
    }

    /// Get the number of hash functions
    pub fn num_hashes(&self) -> usize {
        self.num_hashes
    }

    /// Compute hash index for a key and hash function number
    fn hash_index(&self, key: &str, hash_num: usize) -> usize {
        // Double hashing: h(i) = h1 + i * h2
        let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher1);
        let h1 = hasher1.finish();

        let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
        (key, 0x517cc1b727220a95u64).hash(&mut hasher2);
        let h2 = hasher2.finish();

        let combined = h1.wrapping_add((hash_num as u64).wrapping_mul(h2));
        (combined as usize) % self.num_bits
    }
}

impl Clone for BloomFilter {
    fn clone(&self) -> Self {
        let bits: Box<[AtomicU64]> = self
            .bits
            .iter()
            .map(|b| AtomicU64::new(b.load(Ordering::Relaxed)))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            bits,
            num_hashes: self.num_hashes,
            num_bits: self.num_bits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_check() {
        let filter = BloomFilter::new(1000, 0.01);

        filter.insert("key1");
        filter.insert("key2");
        filter.insert("key3");

        // These should all return true (definitely inserted)
        assert!(filter.might_contain("key1"));
        assert!(filter.might_contain("key2"));
        assert!(filter.might_contain("key3"));
    }

    #[test]
    fn test_negative_lookup() {
        let filter = BloomFilter::new(100, 0.01);

        // Insert some keys
        for i in 0..50 {
            filter.insert(&format!("key:{}", i));
        }

        // Check that many non-inserted keys return false
        let mut false_count = 0;
        for i in 1000..1100 {
            if !filter.might_contain(&format!("key:{}", i)) {
                false_count += 1;
            }
        }

        // With 1% FP rate, we expect most to return false
        assert!(false_count > 90, "False count was {}", false_count);
    }

    #[test]
    fn test_clear() {
        let filter = BloomFilter::new(100, 0.01);

        filter.insert("key1");
        assert!(filter.might_contain("key1"));

        filter.clear();
        assert!(!filter.might_contain("key1"));
    }

    #[test]
    fn test_parameters() {
        let filter = BloomFilter::new(1000, 0.01);
        assert!(filter.num_bits() > 0);
        assert!(filter.num_hashes() > 0);
    }
}
