use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::Wrapping;

/// High-performance hash calculation utilities optimized for mining workloads
pub struct HashUtils;

impl HashUtils {
    /// Fast non-cryptographic hash for internal use
    pub fn fast_hash<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    /// Fast string hash using FNV-1a algorithm
    pub fn fast_string_hash(s: &str) -> u64 {
        const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
        const FNV_PRIME: u64 = 1099511628211;

        let mut hash = Wrapping(FNV_OFFSET_BASIS);
        for byte in s.bytes() {
            hash ^= Wrapping(byte as u64);
            hash *= Wrapping(FNV_PRIME);
        }
        hash.0
    }

    /// Compute difficulty from target (for mining calculations)
    pub fn target_to_difficulty(target: &[u8; 32]) -> f64 {
        // Calculate difficulty as max_target / current_target
        // This is a simplified version for demonstration
        let mut target_num = 0u64;

        // Take first 8 bytes for approximation
        for (i, &byte) in target[0..8].iter().enumerate() {
            target_num |= (byte as u64) << (8 * (7 - i));
        }

        if target_num == 0 {
            return f64::MAX;
        }

        // Bitcoin's max target approximation
        const MAX_TARGET_APPROX: u64 = 0x1d00ffff00000000u64;
        MAX_TARGET_APPROX as f64 / target_num as f64
    }

    /// Fast hash rate calculation (optimized for integer arithmetic)
    pub fn calculate_hash_rate_optimized(shares: u64, time_seconds: u64, difficulty: f64) -> u64 {
        if time_seconds == 0 {
            return 0;
        }

        // Convert difficulty to integer arithmetic to avoid floating point
        let difficulty_int = (difficulty * 1000.0) as u64; // Scale by 1000 for precision
        let hashes_per_share = (difficulty_int * 4_294_967_296) / 1000; // 2^32 scaled

        let total_hashes = shares * hashes_per_share;
        total_hashes / time_seconds
    }

    /// Combine multiple hashes efficiently
    pub fn combine_hashes(hashes: &[u64]) -> u64 {
        let mut result = 0u64;
        for &hash in hashes {
            result = result.rotate_left(1) ^ hash;
        }
        result
    }

    /// Check if hash meets difficulty target (simplified)
    pub fn meets_difficulty(hash: &[u8; 32], difficulty: f64) -> bool {
        // Count leading zeros in hash
        let leading_zeros = hash.iter().take_while(|&&byte| byte == 0).count() * 8
            + hash
                .iter()
                .find(|&&byte| byte != 0)
                .map(|&byte| byte.leading_zeros() as usize)
                .unwrap_or(0);

        // Approximate difficulty check
        let required_zeros = (difficulty.log2() / 4.0) as usize;
        leading_zeros >= required_zeros
    }

    /// Generate a simple checksum for validation
    pub fn checksum(data: &[u8]) -> u32 {
        let mut checksum = 0u32;
        for &byte in data {
            checksum = checksum.wrapping_mul(31).wrapping_add(byte as u32);
        }
        checksum
    }
}

/// Optimized hash table operations
pub struct OptimizedHashOps;

impl OptimizedHashOps {
    /// Pre-hash string to reduce HashMap lookup overhead
    pub fn pre_hash_string(s: &str) -> (u64, String) {
        let hash = HashUtils::fast_string_hash(s);
        (hash, s.to_string())
    }

    /// Batch hash multiple strings
    pub fn batch_hash_strings(strings: &[&str]) -> Vec<(u64, String)> {
        strings.iter().map(|&s| Self::pre_hash_string(s)).collect()
    }

    /// Compare hashes quickly before string comparison
    pub fn quick_string_compare(hash1: u64, str1: &str, hash2: u64, str2: &str) -> bool {
        hash1 == hash2 && str1 == str2
    }
}

/// Rolling hash for streaming data
pub struct RollingHash {
    hash: u64,
    power: u64,
    window_size: usize,
    buffer: Vec<u8>,
    base: u64,
}

impl RollingHash {
    const DEFAULT_BASE: u64 = 257;

    pub fn new(window_size: usize) -> Self {
        let base = Self::DEFAULT_BASE;
        let power = base.pow(window_size as u32 - 1);

        Self {
            hash: 0,
            power,
            window_size,
            buffer: Vec::with_capacity(window_size),
            base,
        }
    }

    pub fn add_byte(&mut self, byte: u8) -> u64 {
        if self.buffer.len() < self.window_size {
            // Growing phase
            self.buffer.push(byte);
            self.hash = self.hash * self.base + byte as u64;
        } else {
            // Rolling phase
            let old_byte = self.buffer[0];
            self.buffer.rotate_left(1);
            self.buffer[self.window_size - 1] = byte;

            self.hash = self.hash - (old_byte as u64 * self.power);
            self.hash = self.hash * self.base + byte as u64;
        }

        self.hash
    }

    pub fn current_hash(&self) -> u64 {
        self.hash
    }

    pub fn reset(&mut self) {
        self.hash = 0;
        self.buffer.clear();
    }
}

/// Hash-based bloom filter for fast membership testing
pub struct SimpleBloomFilter {
    bits: Vec<u64>,
    size: usize,
    hash_functions: u8,
}

impl SimpleBloomFilter {
    pub fn new(capacity: usize, false_positive_rate: f64) -> Self {
        // Calculate optimal size and hash functions
        let size = (-((capacity as f64) * false_positive_rate.ln()) / (2.0_f64.ln().powi(2))).ceil()
            as usize;
        let hash_functions = ((size as f64 / capacity as f64) * 2.0_f64.ln()).ceil() as u8;

        let num_words = (size + 63) / 64; // Round up to word boundary

        Self {
            bits: vec![0u64; num_words],
            size,
            hash_functions,
        }
    }

    pub fn insert<T: Hash>(&mut self, item: &T) {
        let hash1 = HashUtils::fast_hash(item);
        let hash2 = hash1.wrapping_mul(0x9e3779b97f4a7c15); // Golden ratio hash

        for i in 0..self.hash_functions {
            let hash = hash1.wrapping_add(hash2.wrapping_mul(i as u64));
            let bit_index = (hash as usize) % self.size;
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;

            self.bits[word_index] |= 1u64 << bit_offset;
        }
    }

    pub fn contains<T: Hash>(&self, item: &T) -> bool {
        let hash1 = HashUtils::fast_hash(item);
        let hash2 = hash1.wrapping_mul(0x9e3779b97f4a7c15);

        for i in 0..self.hash_functions {
            let hash = hash1.wrapping_add(hash2.wrapping_mul(i as u64));
            let bit_index = (hash as usize) % self.size;
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;

            if (self.bits[word_index] & (1u64 << bit_offset)) == 0 {
                return false;
            }
        }
        true
    }

    pub fn clear(&mut self) {
        self.bits.fill(0);
    }

    pub fn estimated_count(&self) -> usize {
        let set_bits = self
            .bits
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum::<usize>();

        if set_bits == 0 {
            return 0;
        }

        let m = self.size as f64;
        let k = self.hash_functions as f64;
        let x = set_bits as f64;

        (-(m / k) * (1.0 - x / m).ln()) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_string_hash() {
        let hash1 = HashUtils::fast_string_hash("hello");
        let hash2 = HashUtils::fast_string_hash("hello");
        let hash3 = HashUtils::fast_string_hash("world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_rate_calculation() {
        let hash_rate = HashUtils::calculate_hash_rate_optimized(10, 60, 1.0);
        assert!(hash_rate > 0);
    }

    #[test]
    fn test_rolling_hash() {
        let mut hasher = RollingHash::new(3);

        hasher.add_byte(b'a');
        hasher.add_byte(b'b');
        hasher.add_byte(b'c');
        let hash1 = hasher.current_hash();

        hasher.add_byte(b'd'); // Should roll off 'a'
        let hash2 = hasher.current_hash();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_bloom_filter() {
        let mut filter = SimpleBloomFilter::new(1000, 0.01);

        filter.insert(&"hello");
        filter.insert(&"world");

        assert!(filter.contains(&"hello"));
        assert!(filter.contains(&"world"));
        assert!(!filter.contains(&"not_inserted"));
    }
}
