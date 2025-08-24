use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;

/// High-performance string interning pool for reducing memory allocations
/// Provides O(1) lookup and insertion with minimal overhead
#[derive(Debug)]
pub struct StringPool {
    /// Interned strings storage
    interned: DashMap<String, Arc<str>>,
    /// Statistics tracking
    stats: PoolStats,
    /// Configuration
    config: PoolConfig,
}

#[derive(Debug, Default)]
struct PoolStats {
    /// Total strings interned
    interned_count: AtomicU64,
    /// Cache hits (string already existed)
    hits: AtomicU64,
    /// Cache misses (new string added)
    misses: AtomicU64,
    /// Memory saved by interning
    memory_saved: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of strings to keep in pool
    pub max_entries: usize,
    /// Maximum string length to intern
    pub max_string_length: usize,
    /// Enable automatic cleanup of old entries
    pub auto_cleanup: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_string_length: 1024,
            auto_cleanup: true,
        }
    }
}

impl StringPool {
    /// Create a new string pool with default configuration
    pub fn new() -> Self {
        Self::with_config(PoolConfig::default())
    }

    /// Create a new string pool with custom configuration
    pub fn with_config(config: PoolConfig) -> Self {
        Self {
            interned: DashMap::with_capacity(config.max_entries),
            stats: PoolStats::default(),
            config,
        }
    }

    /// Intern a string, returning a reference-counted string
    /// If the string already exists, returns the existing reference
    pub fn intern(&self, s: &str) -> Arc<str> {
        // Don't intern very long strings to avoid memory bloat
        if s.len() > self.config.max_string_length {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            return s.into();
        }

        // Check if string already exists
        if let Some(existing) = self.interned.get(s) {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            return Arc::clone(existing.value());
        }

        // Cleanup if needed
        if self.config.auto_cleanup && self.interned.len() >= self.config.max_entries {
            self.cleanup_oldest_entries();
        }

        // Create new interned string
        let interned_string: Arc<str> = s.into();
        let result = Arc::clone(&interned_string);

        self.interned.insert(s.to_string(), interned_string);

        self.stats.interned_count.fetch_add(1, Ordering::Relaxed);
        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        self.stats
            .memory_saved
            .fetch_add(s.len() as u64, Ordering::Relaxed);

        result
    }

    /// Intern a string from an owned String, potentially reusing the allocation
    pub fn intern_owned(&self, s: String) -> Arc<str> {
        // Don't intern very long strings
        if s.len() > self.config.max_string_length {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            return s.into();
        }

        // Check if string already exists
        if let Some(existing) = self.interned.get(&s) {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            return Arc::clone(existing.value());
        }

        // Cleanup if needed
        if self.config.auto_cleanup && self.interned.len() >= self.config.max_entries {
            self.cleanup_oldest_entries();
        }

        // Create new interned string from owned string
        let memory_saved = s.len() as u64;
        let interned_string: Arc<str> = s.clone().into();
        let result = Arc::clone(&interned_string);

        self.interned.insert(s, interned_string);

        self.stats.interned_count.fetch_add(1, Ordering::Relaxed);
        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        self.stats
            .memory_saved
            .fetch_add(memory_saved, Ordering::Relaxed);

        result
    }

    /// Check if a string is already interned
    pub fn contains(&self, s: &str) -> bool {
        self.interned.contains_key(s)
    }

    /// Get the number of interned strings
    pub fn len(&self) -> usize {
        self.interned.len()
    }

    /// Check if the pool is empty
    pub fn is_empty(&self) -> bool {
        self.interned.is_empty()
    }

    /// Clear all interned strings
    pub fn clear(&self) {
        self.interned.clear();
    }

    /// Get pool statistics
    pub fn stats(&self) -> StringPoolStats {
        StringPoolStats {
            interned_count: self.stats.interned_count.load(Ordering::Relaxed),
            hits: self.stats.hits.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            memory_saved: self.stats.memory_saved.load(Ordering::Relaxed),
            pool_size: self.interned.len(),
            hit_rate: {
                let hits = self.stats.hits.load(Ordering::Relaxed);
                let total = hits + self.stats.misses.load(Ordering::Relaxed);
                if total > 0 {
                    (hits as f64 / total as f64) * 100.0
                } else {
                    0.0
                }
            },
        }
    }

    /// Cleanup oldest entries when pool is full
    /// This is a simple FIFO cleanup - in production you might want LRU
    fn cleanup_oldest_entries(&self) {
        let target_size = self.config.max_entries * 3 / 4; // Remove 25% of entries
        let current_size = self.interned.len();

        if current_size <= target_size {
            return;
        }

        let to_remove = current_size - target_size;
        let mut removed = 0;

        // Simple cleanup - remove arbitrary entries
        // In production, you might want to track access time for LRU
        self.interned.retain(|_, _| {
            if removed < to_remove {
                removed += 1;
                false
            } else {
                true
            }
        });
    }
}

impl Default for StringPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Global string pool instance for application-wide string interning
static GLOBAL_STRING_POOL: std::sync::OnceLock<StringPool> = std::sync::OnceLock::new();

/// Get the global string pool instance
pub fn global_pool() -> &'static StringPool {
    GLOBAL_STRING_POOL.get_or_init(StringPool::new)
}

/// Intern a string using the global pool
pub fn intern(s: &str) -> Arc<str> {
    global_pool().intern(s)
}

/// Intern an owned string using the global pool
pub fn intern_owned(s: String) -> Arc<str> {
    global_pool().intern_owned(s)
}

/// Optimized string builder for reducing allocations
#[derive(Debug)]
pub struct OptimizedStringBuilder {
    buffer: String,
    pool: Option<Arc<StringPool>>,
}

impl OptimizedStringBuilder {
    /// Create a new builder with default capacity
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    /// Create a new builder with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
            pool: None,
        }
    }

    /// Create a new builder that uses a string pool for final result
    pub fn with_pool(pool: Arc<StringPool>) -> Self {
        Self {
            buffer: String::with_capacity(64),
            pool: Some(pool),
        }
    }

    /// Append a string slice
    pub fn push_str(&mut self, s: &str) -> &mut Self {
        self.buffer.push_str(s);
        self
    }

    /// Append a single character
    pub fn push(&mut self, ch: char) -> &mut Self {
        self.buffer.push(ch);
        self
    }

    /// Append formatted arguments
    pub fn push_fmt(&mut self, args: std::fmt::Arguments<'_>) -> &mut Self {
        use std::fmt::Write;
        let _ = self.buffer.write_fmt(args);
        self
    }

    /// Reserve additional capacity
    pub fn reserve(&mut self, additional: usize) -> &mut Self {
        self.buffer.reserve(additional);
        self
    }

    /// Clear the buffer for reuse
    pub fn clear(&mut self) -> &mut Self {
        self.buffer.clear();
        self
    }

    /// Get the current length
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Build the final string, potentially using string pool
    pub fn build(self) -> Arc<str> {
        if let Some(pool) = self.pool {
            pool.intern_owned(self.buffer)
        } else {
            self.buffer.into()
        }
    }

    /// Build into a regular String
    pub fn build_string(self) -> String {
        self.buffer
    }

    /// Get a reference to the current buffer
    pub fn as_str(&self) -> &str {
        &self.buffer
    }
}

impl Default for OptimizedStringBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility for optimizing common string operations
pub struct StringOptimizer;

impl StringOptimizer {
    /// Optimized string concatenation with pre-allocated capacity
    pub fn concat_optimized(parts: &[&str]) -> String {
        let total_len: usize = parts.iter().map(|s| s.len()).sum();
        let mut result = String::with_capacity(total_len);

        for part in parts {
            result.push_str(part);
        }

        result
    }

    /// Join strings with separator, optimized for performance
    pub fn join_optimized(parts: &[&str], separator: &str) -> String {
        if parts.is_empty() {
            return String::new();
        }

        if parts.len() == 1 {
            return parts[0].to_string();
        }

        let total_len: usize =
            parts.iter().map(|s| s.len()).sum::<usize>() + separator.len() * (parts.len() - 1);

        let mut result = String::with_capacity(total_len);

        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                result.push_str(separator);
            }
            result.push_str(part);
        }

        result
    }

    /// Replace all occurrences with pre-allocated capacity
    pub fn replace_all_optimized(source: &str, from: &str, to: &str) -> String {
        let count = source.matches(from).count();
        if count == 0 {
            return source.to_string();
        }

        let new_len = source.len() + count * (to.len().saturating_sub(from.len()));
        let mut result = String::with_capacity(new_len);

        let mut last_end = 0;
        for (start, part) in source.match_indices(from) {
            result.push_str(&source[last_end..start]);
            result.push_str(to);
            last_end = start + part.len();
        }
        result.push_str(&source[last_end..]);

        result
    }

    /// Trim whitespace without allocation if not needed
    pub fn trim_optimized(s: &str) -> &str {
        s.trim()
    }

    /// Split string and collect into Vec with pre-allocated capacity
    pub fn split_optimized(s: &str, delimiter: char) -> Vec<&str> {
        // Pre-count the number of parts to avoid reallocations
        let count = s.matches(delimiter).count() + 1;
        let mut result = Vec::with_capacity(count);
        result.extend(s.split(delimiter));
        result
    }

    /// Parse JSON field efficiently for common Stratum fields
    pub fn extract_json_field<'a>(json: &'a str, field: &str) -> Option<&'a str> {
        // Simple JSON field extraction for performance-critical paths
        let field_pattern = format!("\"{field}\":");
        if let Some(start) = json.find(&field_pattern) {
            let value_start = start + field_pattern.len();
            let json_after_field = &json[value_start..];

            // Skip whitespace
            let json_after_field = json_after_field.trim_start();

            if json_after_field.starts_with('"') {
                // String value
                if let Some(end_quote) = json_after_field[1..].find('"') {
                    return Some(&json_after_field[1..end_quote + 1]);
                }
            } else {
                // Number or boolean value
                let end_pos = json_after_field
                    .find(&[',', '}', ']', ' ', '\n', '\r', '\t'][..])
                    .unwrap_or(json_after_field.len());
                return Some(&json_after_field[..end_pos]);
            }
        }
        None
    }
}

/// Batch string operations for processing multiple strings efficiently
pub struct BatchStringProcessor {
    pool: Arc<StringPool>,
    buffer: Vec<String>,
}

impl BatchStringProcessor {
    pub fn new(pool: Arc<StringPool>) -> Self {
        Self {
            pool,
            buffer: Vec::new(),
        }
    }

    pub fn with_capacity(pool: Arc<StringPool>, capacity: usize) -> Self {
        Self {
            pool,
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Add string to batch
    pub fn add(&mut self, s: String) {
        self.buffer.push(s);
    }

    /// Process all strings in the batch and intern them
    pub fn process_batch(self) -> Vec<Arc<str>> {
        self.buffer
            .into_iter()
            .map(|s| self.pool.intern_owned(s))
            .collect()
    }

    /// Process batch with transformation function
    pub fn process_batch_with<F>(self, transform: F) -> Vec<Arc<str>>
    where
        F: Fn(String) -> String,
    {
        self.buffer
            .into_iter()
            .map(transform)
            .map(|s| self.pool.intern_owned(s))
            .collect()
    }
}

// Data structures

#[derive(Debug, Clone)]
pub struct StringPoolStats {
    pub interned_count: u64,
    pub hits: u64,
    pub misses: u64,
    pub memory_saved: u64,
    pub pool_size: usize,
    pub hit_rate: f64,
}

// Thread-local string pools for high-performance single-threaded scenarios
thread_local! {
    static LOCAL_POOL: StringPool = StringPool::new();
}

/// Intern using thread-local pool (fastest for single-threaded scenarios)
pub fn intern_local(s: &str) -> Arc<str> {
    LOCAL_POOL.with(|pool| pool.intern(s))
}

/// Intern owned string using thread-local pool
pub fn intern_owned_local(s: String) -> Arc<str> {
    LOCAL_POOL.with(|pool| pool.intern_owned(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_pool_basic() {
        let pool = StringPool::new();

        let s1 = pool.intern("hello");
        let s2 = pool.intern("hello");

        assert_eq!(s1.as_ref(), s2.as_ref());
        assert!(Arc::ptr_eq(&s1, &s2));

        let stats = pool.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_string_builder() {
        let mut builder = OptimizedStringBuilder::new();
        builder.push_str("hello").push(' ').push_str("world");

        let result = builder.build();
        assert_eq!(result.as_ref(), "hello world");
    }

    #[test]
    fn test_string_optimizer() {
        let parts = &["hello", " ", "world"];
        let result = StringOptimizer::concat_optimized(parts);
        assert_eq!(result, "hello world");

        let joined = StringOptimizer::join_optimized(parts, "");
        assert_eq!(joined, "hello world");
    }
}
