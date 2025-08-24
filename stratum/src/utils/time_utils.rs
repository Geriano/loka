use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// High-performance time utilities optimized for mining operations
pub struct TimeUtils;

impl TimeUtils {
    /// Get current Unix timestamp in seconds
    pub fn unix_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get current Unix timestamp in milliseconds
    pub fn unix_timestamp_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Get current Unix timestamp in microseconds
    pub fn unix_timestamp_micros() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }

    /// Convert duration to human-readable string
    pub fn format_duration(duration: Duration) -> String {
        let secs = duration.as_secs();
        let millis = duration.subsec_millis();

        if secs >= 3600 {
            format!("{:.1}h", secs as f64 / 3600.0)
        } else if secs >= 60 {
            format!("{:.1}m", secs as f64 / 60.0)
        } else if secs >= 1 {
            format!("{secs}.{millis:03}s")
        } else {
            format!("{}ms", millis + duration.subsec_millis())
        }
    }

    /// Calculate time difference in seconds
    pub fn time_diff_seconds(start: SystemTime, end: SystemTime) -> f64 {
        end.duration_since(start).unwrap_or_default().as_secs_f64()
    }

    /// Check if timestamp is within time window
    pub fn is_within_window(timestamp: u64, window_seconds: u64) -> bool {
        let now = Self::unix_timestamp();
        let age = now.saturating_sub(timestamp);
        age <= window_seconds
    }

    /// Round timestamp to nearest interval (for bucketing)
    pub fn round_to_interval(timestamp: u64, interval_seconds: u64) -> u64 {
        (timestamp / interval_seconds) * interval_seconds
    }

    /// Get time until next interval boundary
    pub fn time_until_next_interval(timestamp: u64, interval_seconds: u64) -> u64 {
        let next_boundary = Self::round_to_interval(timestamp, interval_seconds) + interval_seconds;
        next_boundary.saturating_sub(timestamp)
    }
}

/// High-precision timer for performance measurements
#[derive(Debug)]
pub struct PrecisionTimer {
    start: Instant,
    measurements: Vec<Duration>,
    name: String,
}

impl PrecisionTimer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            measurements: Vec::new(),
            name: name.into(),
        }
    }

    /// Start a new measurement
    pub fn start(&mut self) {
        self.start = Instant::now();
    }

    /// Record current measurement
    pub fn record(&mut self) -> Duration {
        let elapsed = self.start.elapsed();
        self.measurements.push(elapsed);
        elapsed
    }

    /// Get all measurements
    pub fn measurements(&self) -> &[Duration] {
        &self.measurements
    }

    /// Get statistics for all measurements
    pub fn stats(&self) -> TimerStats {
        if self.measurements.is_empty() {
            return TimerStats::default();
        }

        let mut sorted = self.measurements.clone();
        sorted.sort();

        let total: Duration = sorted.iter().sum();
        let count = sorted.len();
        let avg = total / count as u32;

        let median = if count % 2 == 0 {
            (sorted[count / 2 - 1] + sorted[count / 2]) / 2
        } else {
            sorted[count / 2]
        };

        let p95_index = (count as f64 * 0.95) as usize;
        let p95 = sorted.get(p95_index).copied().unwrap_or(Duration::ZERO);

        let p99_index = (count as f64 * 0.99) as usize;
        let p99 = sorted.get(p99_index).copied().unwrap_or(Duration::ZERO);

        TimerStats {
            name: self.name.clone(),
            count,
            total,
            min: *sorted.first().unwrap(),
            max: *sorted.last().unwrap(),
            avg,
            median,
            p95,
            p99,
        }
    }

    /// Clear all measurements
    pub fn clear(&mut self) {
        self.measurements.clear();
    }
}

/// Time-based rate limiter
#[derive(Debug)]
pub struct RateLimiter {
    max_tokens: u32,
    current_tokens: f64,
    last_refill: Instant,
    refill_rate: f64, // tokens per second
}

impl RateLimiter {
    /// Create a new rate limiter
    /// - max_rate: maximum operations per second
    /// - burst_size: maximum burst size
    pub fn new(max_rate: f64, burst_size: u32) -> Self {
        Self {
            max_tokens: burst_size,
            current_tokens: burst_size as f64,
            last_refill: Instant::now(),
            refill_rate: max_rate,
        }
    }

    /// Try to consume a token, returns true if allowed
    pub fn try_consume(&mut self) -> bool {
        self.refill_tokens();

        if self.current_tokens >= 1.0 {
            self.current_tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Try to consume multiple tokens
    pub fn try_consume_multiple(&mut self, tokens: u32) -> bool {
        self.refill_tokens();

        if self.current_tokens >= tokens as f64 {
            self.current_tokens -= tokens as f64;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill_tokens(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        let tokens_to_add = elapsed * self.refill_rate;
        self.current_tokens = (self.current_tokens + tokens_to_add).min(self.max_tokens as f64);
        self.last_refill = now;
    }

    /// Get current token count
    pub fn available_tokens(&mut self) -> u32 {
        self.refill_tokens();
        self.current_tokens as u32
    }

    /// Reset the rate limiter
    pub fn reset(&mut self) {
        self.current_tokens = self.max_tokens as f64;
        self.last_refill = Instant::now();
    }
}

/// Time window aggregator for metrics
#[derive(Debug)]
pub struct TimeWindowAggregator {
    window_duration: Duration,
    buckets: Vec<TimeWindowBucket>,
    bucket_duration: Duration,
    current_bucket: usize,
    last_rotation: Instant,
}

impl TimeWindowAggregator {
    /// Create a new time window aggregator
    /// - window_duration: total time window to track
    /// - bucket_count: number of time buckets
    pub fn new(window_duration: Duration, bucket_count: usize) -> Self {
        let bucket_duration = window_duration / bucket_count as u32;
        let buckets = vec![TimeWindowBucket::default(); bucket_count];

        Self {
            window_duration,
            buckets,
            bucket_duration,
            current_bucket: 0,
            last_rotation: Instant::now(),
        }
    }

    /// Add a value to the current time bucket
    pub fn add_value(&mut self, value: u64) {
        self.rotate_buckets();
        self.buckets[self.current_bucket].add_value(value);
    }

    /// Increment counter in current bucket
    pub fn increment(&mut self) {
        self.rotate_buckets();
        self.buckets[self.current_bucket].increment();
    }

    /// Get total count across all buckets
    pub fn total_count(&mut self) -> u64 {
        self.rotate_buckets();
        self.buckets.iter().map(|b| b.count).sum()
    }

    /// Get total sum across all buckets
    pub fn total_sum(&mut self) -> u64 {
        self.rotate_buckets();
        self.buckets.iter().map(|b| b.sum).sum()
    }

    /// Get average across all buckets
    pub fn average(&mut self) -> f64 {
        self.rotate_buckets();
        let total_count = self.total_count();
        if total_count == 0 {
            0.0
        } else {
            self.total_sum() as f64 / total_count as f64
        }
    }

    /// Get rate per second
    pub fn rate_per_second(&mut self) -> f64 {
        let count = self.total_count();
        count as f64 / self.window_duration.as_secs_f64()
    }

    /// Rotate buckets based on elapsed time
    fn rotate_buckets(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_rotation);

        if elapsed >= self.bucket_duration {
            let buckets_to_rotate =
                (elapsed.as_secs_f64() / self.bucket_duration.as_secs_f64()) as usize;

            for _ in 0..buckets_to_rotate.min(self.buckets.len()) {
                self.current_bucket = (self.current_bucket + 1) % self.buckets.len();
                self.buckets[self.current_bucket].reset();
            }

            self.last_rotation = now;
        }
    }
}

#[derive(Debug, Default, Clone)]
struct TimeWindowBucket {
    count: u64,
    sum: u64,
    min: u64,
    max: u64,
}

impl TimeWindowBucket {
    fn add_value(&mut self, value: u64) {
        if self.count == 0 {
            self.min = value;
            self.max = value;
        } else {
            self.min = self.min.min(value);
            self.max = self.max.max(value);
        }

        self.count += 1;
        self.sum += value;
    }

    fn increment(&mut self) {
        self.add_value(1);
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Timeout helper for async operations
pub struct AsyncTimeout;

impl AsyncTimeout {
    /// Run an operation with a timeout
    pub async fn with_timeout<F, T>(
        future: F,
        timeout: Duration,
    ) -> Result<T, tokio::time::error::Elapsed>
    where
        F: std::future::Future<Output = T>,
    {
        tokio::time::timeout(timeout, future).await
    }

    /// Run with timeout and default value on timeout
    pub async fn with_timeout_or_default<F, T>(future: F, timeout: Duration, default: T) -> T
    where
        F: std::future::Future<Output = T>,
        T: Clone,
    {
        match tokio::time::timeout(timeout, future).await {
            Ok(result) => result,
            Err(_) => default,
        }
    }
}

// Data structures

#[derive(Debug, Default, Clone)]
pub struct TimerStats {
    pub name: String,
    pub count: usize,
    pub total: Duration,
    pub min: Duration,
    pub max: Duration,
    pub avg: Duration,
    pub median: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_timestamp() {
        let timestamp = TimeUtils::unix_timestamp();
        assert!(timestamp > 0);

        let millis = TimeUtils::unix_timestamp_millis();
        assert!(millis > timestamp * 1000);
    }

    #[test]
    fn test_format_duration() {
        let duration = Duration::from_millis(1500);
        let formatted = TimeUtils::format_duration(duration);
        assert!(formatted.contains("1.500s"));
    }

    #[test]
    fn test_precision_timer() {
        let mut timer = PrecisionTimer::new("test");
        timer.start();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.record();

        assert!(elapsed >= Duration::from_millis(10));
        assert_eq!(timer.measurements().len(), 1);
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(10.0, 5); // 10 ops/sec, burst of 5

        // Should allow burst
        for _ in 0..5 {
            assert!(limiter.try_consume());
        }

        // Should reject after burst
        assert!(!limiter.try_consume());
    }

    #[tokio::test]
    async fn test_async_timeout() {
        let result = AsyncTimeout::with_timeout(async { 42 }, Duration::from_millis(100)).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_time_window_aggregator() {
        let mut agg = TimeWindowAggregator::new(Duration::from_secs(60), 6);

        agg.add_value(100);
        agg.increment();

        assert_eq!(agg.total_count(), 2);
        assert_eq!(agg.total_sum(), 101); // 100 + 1
    }
}
