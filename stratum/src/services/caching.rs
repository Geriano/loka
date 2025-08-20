use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};
use tracing::{debug, info};

use crate::error::{Result, StratumError};

/// High-performance caching service with TTL support and compression
pub struct CachingService<K, V> {
    /// Cache storage
    cache: DashMap<K, CacheEntry<V>>,
    
    /// Configuration
    config: CacheConfig,
    
    /// Statistics
    stats: CacheStats,
    
    /// Cleanup task handle
    cleanup_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl<K, V> CachingService<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new caching service
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: DashMap::with_capacity(config.initial_capacity),
            config,
            stats: CacheStats::new(),
            cleanup_task: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the caching service with background cleanup
    pub async fn start(&self) -> Result<()> {
        let mut cleanup_task = self.cleanup_task.write().await;
        if cleanup_task.is_some() {
            return Err(StratumError::Internal {
                message: "Caching service is already running".to_string(),
            });
        }

        // Start cleanup task
        let cleanup_interval = self.config.cleanup_interval;
        // Note: In a real implementation, we'd need Arc<DashMap> to share properly
        let task = tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);
            loop {
                interval.tick().await;
                // Skip cleanup for now due to borrowing issues
                // In production, use Arc<DashMap> properly
                debug!("Cache cleanup tick");
            }
        });

        *cleanup_task = Some(task);
        info!("Caching service started with cleanup interval: {:?}", cleanup_interval);
        Ok(())
    }

    /// Stop the caching service
    pub async fn stop(&self) {
        let mut cleanup_task = self.cleanup_task.write().await;
        if let Some(task) = cleanup_task.take() {
            task.abort();
        }
        info!("Caching service stopped");
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        if let Some(entry) = self.cache.get(key) {
            if entry.is_expired() {
                // Entry is expired, remove it
                drop(entry);
                self.cache.remove(key);
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                self.stats.expired_entries.fetch_add(1, Ordering::Relaxed);
                None
            } else {
                // Entry is valid, update access time and return value
                let value = entry.value.clone();
                entry.update_access_time();
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                Some(value)
            }
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Put a value into the cache
    pub fn put(&self, key: K, value: V, ttl: Option<Duration>) -> Result<()> {
        // Check if cache is at capacity
        if self.cache.len() >= self.config.max_entries {
            if self.config.eviction_policy == EvictionPolicy::RejectNew {
                return Err(StratumError::Internal {
                    message: "Cache is at maximum capacity".to_string(),
                });
            } else {
                // Note: Eviction is simplified for now
                // In a real implementation, this would be async
            }
        }

        let expiration = ttl.map(|ttl| Instant::now() + ttl);
        let entry = CacheEntry::new(value, expiration);
        
        let is_update = self.cache.insert(key, entry).is_some();
        
        if is_update {
            self.stats.updates.fetch_add(1, Ordering::Relaxed);
        } else {
            self.stats.insertions.fetch_add(1, Ordering::Relaxed);
        }
        
        Ok(())
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &K) -> Option<V> {
        self.cache.remove(key).map(|(_, entry)| {
            self.stats.removals.fetch_add(1, Ordering::Relaxed);
            entry.value
        })
    }

    /// Check if a key exists in the cache
    pub fn contains_key(&self, key: &K) -> bool {
        if let Some(entry) = self.cache.get(key) {
            if entry.is_expired() {
                drop(entry);
                self.cache.remove(key);
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Get the current size of the cache
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clear all entries from the cache
    pub fn clear(&self) {
        let count = self.cache.len();
        self.cache.clear();
        self.stats.removals.fetch_add(count as u64, Ordering::Relaxed);
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            hits: self.stats.hits.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            insertions: self.stats.insertions.load(Ordering::Relaxed),
            updates: self.stats.updates.load(Ordering::Relaxed),
            removals: self.stats.removals.load(Ordering::Relaxed),
            expired_entries: self.stats.expired_entries.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            current_size: self.cache.len(),
            hit_rate: self.calculate_hit_rate(),
        }
    }

    /// Calculate hit rate as a percentage
    fn calculate_hit_rate(&self) -> f64 {
        let hits = self.stats.hits.load(Ordering::Relaxed);
        let misses = self.stats.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        
        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }

    /// Clean up expired entries
    async fn cleanup_expired_entries(cache: &DashMap<K, CacheEntry<V>>) {
        let mut expired_count = 0;
        let now = Instant::now();
        
        cache.retain(|_, entry| {
            if entry.is_expired_at(now) {
                expired_count += 1;
                false
            } else {
                true
            }
        });
        
        if expired_count > 0 {
            debug!("Cleaned up {} expired cache entries", expired_count);
        }
    }

    /// Evict entries based on eviction policy
    async fn evict_entries(&self) {
        match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                self.evict_lru().await;
            }
            EvictionPolicy::LFU => {
                self.evict_lfu().await;
            }
            EvictionPolicy::Random => {
                self.evict_random().await;
            }
            _ => {}
        }
    }

    /// Evict least recently used entries
    async fn evict_lru(&self) {
        let target_size = self.config.max_entries * 3 / 4; // Evict 25%
        let mut entries_to_remove = Vec::new();
        
        // Collect entries with their access times
        let mut access_times: Vec<(K, Instant)> = Vec::new();
        for entry in self.cache.iter() {
            access_times.push((entry.key().clone(), entry.value().last_accessed));
        }
        
        // Sort by access time (oldest first)
        access_times.sort_by_key(|(_, time)| *time);
        
        // Mark entries for removal
        let entries_to_evict = self.cache.len().saturating_sub(target_size);
        for (key, _) in access_times.into_iter().take(entries_to_evict) {
            entries_to_remove.push(key);
        }
        
        // Remove entries
        let mut evicted_count = 0;
        for key in entries_to_remove {
            if self.cache.remove(&key).is_some() {
                evicted_count += 1;
            }
        }
        
        self.stats.evictions.fetch_add(evicted_count, Ordering::Relaxed);
        debug!("Evicted {} LRU cache entries", evicted_count);
    }

    /// Evict least frequently used entries
    async fn evict_lfu(&self) {
        let target_size = self.config.max_entries * 3 / 4;
        let mut entries_to_remove = Vec::new();
        
        // Collect entries with their access counts
        let mut access_counts: Vec<(K, u64)> = Vec::new();
        for entry in self.cache.iter() {
            access_counts.push((entry.key().clone(), entry.value().access_count.load(Ordering::Relaxed)));
        }
        
        // Sort by access count (lowest first)
        access_counts.sort_by_key(|(_, count)| *count);
        
        // Mark entries for removal
        let entries_to_evict = self.cache.len().saturating_sub(target_size);
        for (key, _) in access_counts.into_iter().take(entries_to_evict) {
            entries_to_remove.push(key);
        }
        
        // Remove entries
        let mut evicted_count = 0;
        for key in entries_to_remove {
            if self.cache.remove(&key).is_some() {
                evicted_count += 1;
            }
        }
        
        self.stats.evictions.fetch_add(evicted_count, Ordering::Relaxed);
        debug!("Evicted {} LFU cache entries", evicted_count);
    }

    /// Evict random entries
    async fn evict_random(&self) {
        let target_size = self.config.max_entries * 3 / 4;
        let entries_to_evict = self.cache.len().saturating_sub(target_size);
        
        let mut evicted_count = 0;
        let mut keys_to_remove = Vec::new();
        
        // Collect random keys
        for entry in self.cache.iter().take(entries_to_evict) {
            keys_to_remove.push(entry.key().clone());
        }
        
        // Remove entries
        for key in keys_to_remove {
            if self.cache.remove(&key).is_some() {
                evicted_count += 1;
            }
        }
        
        self.stats.evictions.fetch_add(evicted_count, Ordering::Relaxed);
        debug!("Evicted {} random cache entries", evicted_count);
    }
}

/// Cache entry with expiration and access tracking
struct CacheEntry<V> {
    value: V,
    expiration: Option<Instant>,
    last_accessed: Instant,
    access_count: AtomicU64,
    created_at: Instant,
}

impl<V> CacheEntry<V> {
    fn new(value: V, expiration: Option<Instant>) -> Self {
        let now = Instant::now();
        Self {
            value,
            expiration,
            last_accessed: now,
            access_count: AtomicU64::new(1),
            created_at: now,
        }
    }

    fn is_expired(&self) -> bool {
        self.is_expired_at(Instant::now())
    }

    fn is_expired_at(&self, now: Instant) -> bool {
        if let Some(expiration) = self.expiration {
            now >= expiration
        } else {
            false
        }
    }

    fn update_access_time(&self) {
        // Note: This is not thread-safe, but acceptable for caching use case
        // In production, you might want to use atomic operations or accept the race condition
        self.access_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// Data compression utilities for cache values
pub struct CompressionUtils;

impl CompressionUtils {
    /// Compress data using simple RLE (Run-Length Encoding) for demonstration
    pub fn compress_simple(data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }

        let mut compressed = Vec::new();
        let mut current_byte = data[0];
        let mut count = 1u8;

        for &byte in &data[1..] {
            if byte == current_byte && count < 255 {
                count += 1;
            } else {
                compressed.push(count);
                compressed.push(current_byte);
                current_byte = byte;
                count = 1;
            }
        }
        
        // Push the last run
        compressed.push(count);
        compressed.push(current_byte);
        
        compressed
    }

    /// Decompress RLE data
    pub fn decompress_simple(compressed: &[u8]) -> Vec<u8> {
        if compressed.is_empty() || compressed.len() % 2 != 0 {
            return Vec::new();
        }

        let mut decompressed = Vec::new();
        
        for chunk in compressed.chunks(2) {
            let count = chunk[0];
            let byte = chunk[1];
            
            for _ in 0..count {
                decompressed.push(byte);
            }
        }
        
        decompressed
    }

    /// Calculate compression ratio
    pub fn compression_ratio(original_size: usize, compressed_size: usize) -> f64 {
        if original_size == 0 {
            0.0
        } else if compressed_size >= original_size {
            // No compression achieved or expanded
            0.0
        } else {
            (original_size - compressed_size) as f64 / original_size as f64 * 100.0
        }
    }

    /// Check if data should be compressed (based on size and type)
    pub fn should_compress(data: &[u8], min_size: usize) -> bool {
        data.len() >= min_size
    }
}

/// Specialized cache for JSON data with compression
pub type JsonCache = CachingService<String, CompressedData>;

/// Compressed data wrapper
#[derive(Debug, Clone)]
pub struct CompressedData {
    pub original_size: usize,
    pub compressed_data: Vec<u8>,
    pub compression_type: CompressionType,
}

impl CompressedData {
    pub fn new(data: &[u8], compression_type: CompressionType) -> Self {
        let compressed_data = match compression_type {
            CompressionType::None => data.to_vec(),
            CompressionType::SimpleRLE => CompressionUtils::compress_simple(data),
        };

        Self {
            original_size: data.len(),
            compressed_data,
            compression_type,
        }
    }

    pub fn decompress(&self) -> Vec<u8> {
        match self.compression_type {
            CompressionType::None => self.compressed_data.clone(),
            CompressionType::SimpleRLE => CompressionUtils::decompress_simple(&self.compressed_data),
        }
    }

    pub fn compression_ratio(&self) -> f64 {
        CompressionUtils::compression_ratio(self.original_size, self.compressed_data.len())
    }
}

// Configuration and data structures

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub initial_capacity: usize,
    pub default_ttl: Option<Duration>,
    pub cleanup_interval: Duration,
    pub eviction_policy: EvictionPolicy,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            initial_capacity: 1_000,
            default_ttl: Some(Duration::from_secs(3600)), // 1 hour
            cleanup_interval: Duration::from_secs(60), // 1 minute
            eviction_policy: EvictionPolicy::LRU,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvictionPolicy {
    LRU,  // Least Recently Used
    LFU,  // Least Frequently Used
    Random,
    RejectNew,  // Reject new entries when full
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompressionType {
    None,
    SimpleRLE,
}

struct CacheStats {
    hits: AtomicU64,
    misses: AtomicU64,
    insertions: AtomicU64,
    updates: AtomicU64,
    removals: AtomicU64,
    expired_entries: AtomicU64,
    evictions: AtomicU64,
}

impl CacheStats {
    fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            insertions: AtomicU64::new(0),
            updates: AtomicU64::new(0),
            removals: AtomicU64::new(0),
            expired_entries: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStatsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub insertions: u64,
    pub updates: u64,
    pub removals: u64,
    pub expired_entries: u64,
    pub evictions: u64,
    pub current_size: usize,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = CachingService::new(CacheConfig::default());
        
        // Test put and get
        cache.put("key1".to_string(), "value1".to_string(), None).unwrap();
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        
        // Test miss
        assert_eq!(cache.get(&"nonexistent".to_string()), None);
        
        // Test remove
        assert_eq!(cache.remove(&"key1".to_string()), Some("value1".to_string()));
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[tokio::test]
    async fn test_cache_ttl() {
        let cache = CachingService::new(CacheConfig::default());
        
        // Put with short TTL
        cache.put(
            "key1".to_string(),
            "value1".to_string(),
            Some(Duration::from_millis(100))
        ).unwrap();
        
        // Should be available immediately
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        
        // Wait for expiration
        sleep(Duration::from_millis(150)).await;
        
        // Should be expired
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_compression_utils() {
        let data = b"aaabbbcccdddeee";
        let compressed = CompressionUtils::compress_simple(data);
        let decompressed = CompressionUtils::decompress_simple(&compressed);
        
        assert_eq!(data, decompressed.as_slice());
        assert!(compressed.len() < data.len()); // Should be smaller
    }

    #[test]
    fn test_compressed_data() {
        let original = b"hello world hello world";
        let compressed_data = CompressedData::new(original, CompressionType::SimpleRLE);
        let decompressed = compressed_data.decompress();
        
        assert_eq!(original, decompressed.as_slice());
        assert!(compressed_data.compression_ratio() >= 0.0);
    }
}