//! Time series metrics collection and windowing functionality.
//!
//! This module provides time-based metrics collection with configurable windows
//! for trend analysis and historical performance tracking.

use std::collections::VecDeque;
use std::time::{Duration, SystemTime};
use tracing::debug;

/// Time series metrics collector for trend analysis and historical data.
///
/// Maintains sliding windows of metrics data for computing moving averages,
/// detecting trends, and providing historical context for performance analysis.
///
/// # Examples
///
/// ```rust
/// use loka_stratum::services::metrics::TimeSeriesMetrics;
/// use std::time::Duration;
///
/// let mut ts = TimeSeriesMetrics::new(Duration::from_secs(60), 100);
/// 
/// // Record data points
/// ts.record_metric("connection_count", 150.0);
/// ts.record_metric("message_rate", 1200.5);
/// 
/// // Get moving averages
/// if let Some(avg) = ts.get_moving_average("connection_count", 10) {
///     println!("Average connections over last 10 samples: {:.2}", avg);
/// }
/// ```
#[derive(Debug)]
pub struct TimeSeriesMetrics {
    /// Window duration for data retention
    window_duration: Duration,
    /// Maximum number of data points to retain
    max_samples: usize,
    /// Historical data points organized by metric name
    data_points: std::collections::HashMap<String, VecDeque<(SystemTime, f64)>>,
}

impl TimeSeriesMetrics {
    /// Create a new time series metrics collector.
    ///
    /// # Arguments
    ///
    /// * `window_duration` - How long to retain data points
    /// * `max_samples` - Maximum number of samples per metric
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loka_stratum::services::metrics::TimeSeriesMetrics;
    /// use std::time::Duration;
    ///
    /// // Keep 5 minutes of data, max 300 samples
    /// let ts = TimeSeriesMetrics::new(Duration::from_secs(300), 300);
    /// ```
    pub fn new(window_duration: Duration, max_samples: usize) -> Self {
        Self {
            window_duration,
            max_samples,
            data_points: std::collections::HashMap::new(),
        }
    }

    /// Record a metric data point with current timestamp.
    ///
    /// # Arguments
    ///
    /// * `metric_name` - Name of the metric to record
    /// * `value` - Metric value to record
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use loka_stratum::services::metrics::TimeSeriesMetrics;
    /// # use std::time::Duration;
    /// # let mut ts = TimeSeriesMetrics::new(Duration::from_secs(60), 100);
    /// ts.record_metric("connection_count", 42.0);
    /// ts.record_metric("bytes_per_second", 1024.5);
    /// ```
    pub fn record_metric(&mut self, metric_name: &str, value: f64) {
        let now = SystemTime::now();
        
        let data = self.data_points
            .entry(metric_name.to_string())
            .or_default();
            
        // Add new data point
        data.push_back((now, value));
        
        // Remove old data points beyond window or max samples - need to handle borrowing
        let window_duration = self.window_duration;
        let max_samples = self.max_samples;
        Self::cleanup_old_data_static(data, window_duration, max_samples);
        
        debug!(
            metric = metric_name,
            value = value,
            samples = data.len(),
            "Recorded time series metric"
        );
    }

    /// Get moving average for a metric over the last N samples.
    ///
    /// # Arguments
    ///
    /// * `metric_name` - Name of the metric
    /// * `sample_count` - Number of recent samples to average
    ///
    /// # Returns
    ///
    /// Some(average) if enough samples exist, None otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use loka_stratum::services::metrics::TimeSeriesMetrics;
    /// # use std::time::Duration;
    /// # let mut ts = TimeSeriesMetrics::new(Duration::from_secs(60), 100);
    /// # ts.record_metric("test", 10.0);
    /// # ts.record_metric("test", 20.0);
    /// if let Some(avg) = ts.get_moving_average("test", 2) {
    ///     println!("Moving average: {:.2}", avg);
    /// }
    /// ```
    pub fn get_moving_average(&self, metric_name: &str, sample_count: usize) -> Option<f64> {
        let data = self.data_points.get(metric_name)?;
        
        if data.len() < sample_count {
            return None;
        }
        
        let sum: f64 = data.iter()
            .rev()
            .take(sample_count)
            .map(|(_, value)| value)
            .sum();
            
        Some(sum / sample_count as f64)
    }

    /// Get the most recent value for a metric.
    ///
    /// # Arguments
    ///
    /// * `metric_name` - Name of the metric
    ///
    /// # Returns
    ///
    /// Some(value) if data exists, None otherwise
    pub fn get_latest_value(&self, metric_name: &str) -> Option<f64> {
        self.data_points
            .get(metric_name)?
            .back()
            .map(|(_, value)| *value)
    }

    /// Get the trend direction for a metric over recent samples.
    ///
    /// # Arguments
    ///
    /// * `metric_name` - Name of the metric
    /// * `sample_count` - Number of recent samples to analyze
    ///
    /// # Returns
    ///
    /// Positive value for increasing trend, negative for decreasing,
    /// zero for stable, None if insufficient data.
    pub fn get_trend(&self, metric_name: &str, sample_count: usize) -> Option<f64> {
        let data = self.data_points.get(metric_name)?;
        
        if data.len() < sample_count.max(2) {
            return None;
        }
        
        let recent: Vec<f64> = data.iter()
            .rev()
            .take(sample_count)
            .map(|(_, value)| *value)
            .collect();
            
        if recent.len() < 2 {
            return None;
        }
        
        // Simple linear trend calculation
        let first_half_avg = recent[recent.len()/2..].iter().sum::<f64>() / (recent.len()/2) as f64;
        let second_half_avg = recent[..recent.len()/2].iter().sum::<f64>() / (recent.len()/2) as f64;
        
        Some(first_half_avg - second_half_avg)
    }

    /// Clean up old data points beyond window duration or max samples (static version).
    fn cleanup_old_data_static(data: &mut VecDeque<(SystemTime, f64)>, window_duration: Duration, max_samples: usize) {
        let cutoff_time = SystemTime::now()
            .checked_sub(window_duration)
            .unwrap_or(SystemTime::UNIX_EPOCH);
            
        // Remove points older than window duration
        while let Some((timestamp, _)) = data.front() {
            if *timestamp < cutoff_time {
                data.pop_front();
            } else {
                break;
            }
        }
        
        // Remove excess points beyond max samples
        while data.len() > max_samples {
            data.pop_front();
        }
    }

    /// Clean up old data points beyond window duration or max samples.
    #[allow(dead_code)]
    fn cleanup_old_data(&self, data: &mut VecDeque<(SystemTime, f64)>) {
        Self::cleanup_old_data_static(data, self.window_duration, self.max_samples);
    }

    /// Get the number of data points for a metric.
    pub fn sample_count(&self, metric_name: &str) -> usize {
        self.data_points
            .get(metric_name)
            .map(|data| data.len())
            .unwrap_or(0)
    }

    /// Get all available metric names.
    pub fn metric_names(&self) -> Vec<String> {
        self.data_points.keys().cloned().collect()
    }
}

impl Default for TimeSeriesMetrics {
    fn default() -> Self {
        Self::new(Duration::from_secs(300), 1000) // 5 minutes, 1000 samples
    }
}