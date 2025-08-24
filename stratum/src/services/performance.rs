use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use sysinfo::System;
use tokio::sync::RwLock;
use tracing::debug;

use crate::error::{Result, StratumError};
use crate::services::metrics::ResourceUtilizationSummary;

/// System resource monitoring
#[derive(Debug)]
pub struct ResourceMonitor {
    system: Arc<Mutex<System>>,
    cpu_usage_history: Arc<RwLock<Vec<f32>>>,
    memory_usage_history: Arc<RwLock<Vec<u64>>>,
    max_history: usize,
}

impl ResourceMonitor {
    pub fn new(max_history: usize) -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            system: Arc::new(Mutex::new(system)),
            cpu_usage_history: Arc::new(RwLock::new(Vec::with_capacity(max_history))),
            memory_usage_history: Arc::new(RwLock::new(Vec::with_capacity(max_history))),
            max_history,
        }
    }

    /// Update system metrics
    pub async fn update_metrics(&self) -> Result<SystemMetrics> {
        // Extract values from system without holding the lock across await
        let (cpu_usage, memory_used, memory_total) = {
            let mut system = self.system.lock().map_err(|_| StratumError::Internal {
                message: "Failed to acquire system lock".to_string(),
            })?;

            system.refresh_all();
            let cpu_usage = system.global_cpu_usage();
            let memory_used = system.used_memory();
            let memory_total = system.total_memory();

            (cpu_usage, memory_used, memory_total)
        }; // system lock is dropped here

        let memory_usage_percent = (memory_used as f64 / memory_total as f64) * 100.0;

        // Update history
        {
            let mut cpu_history = self.cpu_usage_history.write().await;
            cpu_history.push(cpu_usage);
            if cpu_history.len() > self.max_history {
                cpu_history.remove(0);
            }
        }

        {
            let mut memory_history = self.memory_usage_history.write().await;
            memory_history.push(memory_used);
            if memory_history.len() > self.max_history {
                memory_history.remove(0);
            }
        }

        // Simplified implementation to avoid sysinfo API issues
        let disk_usage = Vec::new(); // Empty for now
        let network_usage = Vec::new(); // Empty for now

        Ok(SystemMetrics {
            cpu_usage,
            memory_used,
            memory_total,
            memory_usage_percent,
            disk_usage,
            network_usage,
            timestamp: SystemTime::now(),
        })
    }

    /// Get CPU usage history
    pub async fn get_cpu_history(&self) -> Vec<f32> {
        self.cpu_usage_history.read().await.clone()
    }

    /// Get memory usage history
    pub async fn get_memory_history(&self) -> Vec<u64> {
        self.memory_usage_history.read().await.clone()
    }

    /// Calculate average CPU usage over recent history
    pub async fn get_average_cpu_usage(&self) -> f32 {
        let history = self.cpu_usage_history.read().await;
        if history.is_empty() {
            0.0
        } else {
            history.iter().sum::<f32>() / history.len() as f32
        }
    }

    /// Check if system resources are under pressure
    pub async fn is_under_pressure(&self) -> ResourcePressure {
        let cpu_avg = self.get_average_cpu_usage().await;
        let memory_history = self.memory_usage_history.read().await;

        let memory_pressure = if let Some(&latest_memory) = memory_history.last() {
            latest_memory > 0 // Will be calculated properly with total memory
        } else {
            false
        };

        ResourcePressure {
            cpu_pressure: cpu_avg > 80.0,
            memory_pressure,
            high_cpu_usage: cpu_avg > 90.0,
            critical_memory: memory_pressure, // Placeholder - implement proper threshold
        }
    }

    /// Get current resource utilization summary
    pub async fn get_utilization_summary(&self) -> ResourceUtilizationSummary {
        // Get current system metrics
        let metrics = match self.update_metrics().await {
            Ok(m) => m,
            Err(_) => {
                // Return default/empty summary on error
                return ResourceUtilizationSummary::default();
            }
        };

        // Get history for calculations
        let cpu_avg = self.get_average_cpu_usage().await;
        let cpu_history = self.cpu_usage_history.read().await;
        let memory_history = self.memory_usage_history.read().await;

        let peak_cpu = cpu_history.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0) as f64;
        let peak_memory = memory_history.iter().max().copied().unwrap_or(0) as f64 / (1024.0 * 1024.0); // Convert to MB

        ResourceUtilizationSummary {
            current_memory_mb: metrics.memory_used as f64 / (1024.0 * 1024.0), // Convert bytes to MB
            peak_memory_mb: peak_memory,
            available_memory_mb: (metrics.memory_total - metrics.memory_used) as f64 / (1024.0 * 1024.0),
            connection_memory_bytes: 0, // Would need to track this separately
            current_cpu_percent: metrics.cpu_usage as f64,
            peak_cpu_percent: peak_cpu,
            cpu_sample_count: cpu_history.len() as u64,
            network_rx_bps: 0, // Would need network monitoring
            network_tx_bps: 0,
            peak_network_rx_bps: 0,
            peak_network_tx_bps: 0,
            load_avg_1min: 0.0,
            load_avg_5min: 0.0,
            load_avg_15min: 0.0,
            memory_per_connection_mb: 0.0,
            peak_memory_per_connection_mb: 0.0,
            resource_pressure_events: 0,
            last_pressure_event: None,
            last_updated: std::time::SystemTime::now(),
        }
    }
}

/// Performance profiler for measuring operation timing
#[derive(Debug)]
pub struct PerformanceProfiler {
    operation_times: Arc<DashMap<String, Vec<Duration>>>,
    operation_counts: Arc<DashMap<String, AtomicU64>>,
    max_samples: usize,
}

impl PerformanceProfiler {
    pub fn new(max_samples: usize) -> Self {
        Self {
            operation_times: Arc::new(DashMap::new()),
            operation_counts: Arc::new(DashMap::new()),
            max_samples,
        }
    }

    /// Start timing an operation
    pub fn start_timing(&self, operation: &str) -> OperationTimer {
        OperationTimer::new(
            operation.to_string(),
            Arc::clone(&self.operation_times),
            self.max_samples,
        )
    }

    /// Record an operation duration manually
    pub async fn record_duration(&self, operation: &str, duration: Duration) {
        let mut operation_times = self
            .operation_times
            .entry(operation.to_string())
            .or_insert_with(Vec::new);

        operation_times.push(duration);
        if operation_times.len() > self.max_samples {
            operation_times.remove(0);
        }

        // Update count
        self.operation_counts
            .entry(operation.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get performance statistics for an operation
    pub async fn get_stats(&self, operation: &str) -> Option<OperationStats> {
        if let Some(operation_times) = self.operation_times.get(operation) {
            if operation_times.is_empty() {
                return None;
            }

            let total_count = self
                .operation_counts
                .get(operation)
                .map(|c| c.load(Ordering::Relaxed))
                .unwrap_or(0);

            let mut sorted_times = operation_times.clone();
            sorted_times.sort();

            let sum: Duration = sorted_times.iter().sum();
            let avg = sum / sorted_times.len() as u32;

            let median = if sorted_times.len() % 2 == 0 {
                let mid = sorted_times.len() / 2;
                (sorted_times[mid - 1] + sorted_times[mid]) / 2
            } else {
                sorted_times[sorted_times.len() / 2]
            };

            let p95_idx = (sorted_times.len() as f64 * 0.95) as usize;
            let p95 = sorted_times
                .get(p95_idx.min(sorted_times.len() - 1))
                .copied()
                .unwrap_or(Duration::ZERO);

            let p99_idx = (sorted_times.len() as f64 * 0.99) as usize;
            let p99 = sorted_times
                .get(p99_idx.min(sorted_times.len() - 1))
                .copied()
                .unwrap_or(Duration::ZERO);

            Some(OperationStats {
                operation: operation.to_string(),
                total_count,
                sample_count: sorted_times.len(),
                min: *sorted_times.first().unwrap(),
                max: *sorted_times.last().unwrap(),
                avg,
                median,
                p95,
                p99,
            })
        } else {
            None
        }
    }

    /// Get all operation statistics
    pub async fn get_all_stats(&self) -> HashMap<String, OperationStats> {
        let mut all_stats = HashMap::new();

        for entry in self.operation_times.iter() {
            let operation = entry.key();
            if let Some(stats) = self.get_stats(operation).await {
                all_stats.insert(operation.clone(), stats);
            }
        }

        all_stats
    }

    /// Identify slow operations (above threshold)
    pub async fn get_slow_operations(&self, threshold: Duration) -> Vec<String> {
        let mut slow_ops = Vec::new();

        for entry in self.operation_times.iter() {
            let (operation, operation_times) = (entry.key(), entry.value());
            if let Some(&max_time) = operation_times.iter().max() {
                if max_time > threshold {
                    slow_ops.push(operation.clone());
                }
            }
        }

        slow_ops
    }
}

/// Timer for individual operations
#[derive(Debug)]
pub struct OperationTimer {
    operation: String,
    start_time: Instant,
    times_store: Arc<DashMap<String, Vec<Duration>>>,
    max_samples: usize,
}

impl OperationTimer {
    fn new(
        operation: String,
        times_store: Arc<DashMap<String, Vec<Duration>>>,
        max_samples: usize,
    ) -> Self {
        Self {
            operation,
            start_time: Instant::now(),
            times_store,
            max_samples,
        }
    }

    /// Finish timing and record the result
    pub async fn finish(self) -> Duration {
        let duration = self.start_time.elapsed();

        let mut operation_times = self
            .times_store
            .entry(self.operation.clone())
            .or_insert_with(Vec::new);

        operation_times.push(duration);
        if operation_times.len() > self.max_samples {
            operation_times.remove(0);
        }

        debug!("Operation '{}' completed in {:?}", self.operation, duration);
        duration
    }
}

/// Memory pool for reducing allocation overhead
/// Note: Simplified to avoid lock contention in async environments
pub struct MemoryPool<T> {
    factory: Arc<dyn Fn() -> T + Send + Sync>,
    max_size: usize,
    created_count: AtomicU64,
    reused_count: AtomicU64, // Always 0 in this lock-free version
}

impl<T> MemoryPool<T>
where
    T: Send + 'static,
{
    pub fn new<F>(factory: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            factory: Arc::new(factory),
            max_size,
            created_count: AtomicU64::new(0),
            reused_count: AtomicU64::new(0),
        }
    }

    /// Get an item from the pool or create a new one
    /// Note: In this lock-free version, always creates new items
    pub fn get(&self) -> PooledItem<T> {
        self.created_count.fetch_add(1, Ordering::Relaxed);
        let item = (self.factory)();
        PooledItem::new(item, self.max_size)
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            pool_size: 0, // Always 0 in lock-free version
            max_size: self.max_size,
            created_count: self.created_count.load(Ordering::Relaxed),
            reused_count: self.reused_count.load(Ordering::Relaxed),
        }
    }
}

/// Pooled item that returns to pool when dropped
/// Note: In lock-free version, items are simply dropped
pub struct PooledItem<T> {
    item: Option<T>,
    #[allow(unused)]
    max_size: usize,
}

impl<T> PooledItem<T> {
    fn new(item: T, max_size: usize) -> Self {
        Self {
            item: Some(item),
            max_size,
        }
    }

    /// Get a reference to the pooled item
    pub fn as_ref(&self) -> &T {
        self.item.as_ref().unwrap()
    }

    /// Get a mutable reference to the pooled item
    pub fn as_mut(&mut self) -> &mut T {
        self.item.as_mut().unwrap()
    }
}

impl<T> Drop for PooledItem<T> {
    fn drop(&mut self) {
        if let Some(item) = self.item.take() {
            // In lock-free version, simply drop the item
            // This eliminates lock contention at the cost of some allocations
            drop(item);
        }
    }
}

impl<T> std::ops::Deref for PooledItem<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> std::ops::DerefMut for PooledItem<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

/// Performance optimization utilities
pub struct PerformanceUtils;

impl PerformanceUtils {
    /// Create an optimized string pool for frequent string allocations
    pub fn create_string_pool(max_size: usize) -> MemoryPool<String> {
        MemoryPool::new(|| String::with_capacity(256), max_size)
    }

    /// Create an optimized byte buffer pool
    pub fn create_buffer_pool(buffer_size: usize, max_size: usize) -> MemoryPool<Vec<u8>> {
        MemoryPool::new(move || Vec::with_capacity(buffer_size), max_size)
    }

    /// Batch process items to reduce per-item overhead
    pub async fn batch_process<T, F, R>(items: Vec<T>, batch_size: usize, processor: F) -> Vec<R>
    where
        F: Fn(Vec<T>) -> Vec<R> + Send,
        T: Send + Clone,
        R: Send,
    {
        let mut results = Vec::with_capacity(items.len());

        for chunk in items.chunks(batch_size) {
            let batch_results = processor(chunk.to_vec());
            results.extend(batch_results);
        }

        results
    }

    /// Measure memory usage of a closure
    pub fn measure_memory<F, R>(operation: F) -> (R, u64)
    where
        F: FnOnce() -> R,
    {
        let initial_memory = Self::get_memory_usage();
        let result = operation();
        let final_memory = Self::get_memory_usage();

        let memory_delta = final_memory.saturating_sub(initial_memory);
        (result, memory_delta)
    }

    /// Get current memory usage (approximate)
    fn get_memory_usage() -> u64 {
        // This is a simplified implementation
        // In production, you might use more sophisticated memory tracking
        0
    }

    /// Optimize string operations by reusing allocations
    pub fn optimize_string_concat(parts: &[&str], separator: &str) -> String {
        let total_len: usize = parts.iter().map(|s| s.len()).sum::<usize>()
            + separator.len() * (parts.len().saturating_sub(1));

        let mut result = String::with_capacity(total_len);
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                result.push_str(separator);
            }
            result.push_str(part);
        }
        result
    }
}

// Data structures for metrics

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub memory_usage_percent: f64,
    pub disk_usage: Vec<DiskUsage>,
    pub network_usage: Vec<NetworkUsage>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone)]
pub struct DiskUsage {
    pub name: String,
    pub available_space: u64,
    pub total_space: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Clone)]
pub struct NetworkUsage {
    pub interface: String,
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
    pub packets_received: u64,
    pub packets_transmitted: u64,
}

#[derive(Debug, Clone)]
pub struct ResourcePressure {
    pub cpu_pressure: bool,
    pub memory_pressure: bool,
    pub high_cpu_usage: bool,
    pub critical_memory: bool,
}

#[derive(Debug, Clone)]
pub struct OperationStats {
    pub operation: String,
    pub total_count: u64,
    pub sample_count: usize,
    pub min: Duration,
    pub max: Duration,
    pub avg: Duration,
    pub median: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

#[derive(Debug)]
pub struct PoolStats {
    pub pool_size: usize,
    pub max_size: usize,
    pub created_count: u64,
    pub reused_count: u64,
}
