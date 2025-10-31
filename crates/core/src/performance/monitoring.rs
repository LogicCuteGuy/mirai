//! Unified performance monitoring and metrics collection system
//!
//! This module combines the comprehensive monitoring from minecraft-server-core
//! with mirai's existing prometheus metrics to provide a unified monitoring solution.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime};
use std::collections::{HashMap, VecDeque};
use parking_lot::{RwLock, Mutex};
use prometheus_client::metrics::{counter::Counter, gauge::Gauge, histogram::Histogram};
use prometheus_client::registry::Registry;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};

/// Unified metrics collector that combines prometheus with custom metrics
pub struct MetricsCollector {
    prometheus_registry: Arc<Mutex<Registry>>,
    custom_metrics: Arc<RwLock<CustomMetrics>>,
    profiler: Arc<UnifiedProfiler>,
    alerting: Arc<UnifiedAlertingSystem>,
    system_monitor: Arc<SystemMonitor>,
    config: MonitoringConfig,
    start_time: Instant,
}

impl MetricsCollector {
    /// Create a new unified metrics collector
    pub fn new() -> Self {
        let config = MonitoringConfig::default();
        let prometheus_registry = Arc::new(Mutex::new(Registry::default()));
        
        Self {
            prometheus_registry,
            custom_metrics: Arc::new(RwLock::new(CustomMetrics::new())),
            profiler: Arc::new(UnifiedProfiler::new()),
            alerting: Arc::new(UnifiedAlertingSystem::new(config.alert_thresholds.clone())),
            system_monitor: Arc::new(SystemMonitor::new()),
            config,
            start_time: Instant::now(),
        }
    }

    /// Record a counter metric (compatible with prometheus)
    pub fn increment_counter(&self, name: &str, labels: Option<HashMap<String, String>>) {
        // Record in prometheus
        // Note: In a real implementation, we'd maintain a registry of prometheus counters
        
        // Record in custom metrics
        let mut metrics = self.custom_metrics.write();
        metrics.increment_counter(name, labels);
        
        debug!("Incremented counter: {}", name);
    }

    /// Record a gauge metric
    pub fn record_gauge(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        // Record in custom metrics
        let mut metrics = self.custom_metrics.write();
        metrics.record_gauge(name, value, labels);
        
        // Check for alerts
        self.alerting.check_metric(name, value);
        
        debug!("Recorded gauge: {} = {}", name, value);
    }

    /// Record a histogram metric (for timing measurements)
    pub fn record_histogram(&self, name: &str, value: Duration, labels: Option<HashMap<String, String>>) {
        let mut metrics = self.custom_metrics.write();
        metrics.record_histogram(name, value, labels);
        
        debug!("Recorded histogram: {} = {:?}", name, value);
    }

    /// Record a timing measurement
    pub fn record_timing(&self, name: &str, duration: Duration, labels: Option<HashMap<String, String>>) {
        self.record_histogram(name, duration, labels);
        
        // Also check for slow operations
        if duration > self.config.slow_operation_threshold {
            warn!("Slow operation detected: {} took {:?}", name, duration);
            self.alerting.record_slow_operation(name, duration);
        }
    }

    /// Start profiling a code section
    pub fn start_profile(&self, name: &str) -> UnifiedProfileGuard {
        self.profiler.start_profile(name)
    }

    /// Collect system metrics (CPU, memory, etc.)
    pub fn collect_system_metrics(&self) {
        let system_stats = self.system_monitor.collect_stats();
        
        self.record_gauge("system.cpu_usage", system_stats.cpu_usage, None);
        self.record_gauge("system.memory_usage_bytes", system_stats.memory_usage as f64, None);
        self.record_gauge("system.memory_total_bytes", system_stats.memory_total as f64, None);
        self.record_gauge("system.network_rx_bytes", system_stats.network_rx as f64, None);
        self.record_gauge("system.network_tx_bytes", system_stats.network_tx as f64, None);
        
        debug!("Collected system metrics");
    }

    /// Get comprehensive metrics summary
    pub fn summary(&self) -> MetricsSummary {
        let custom_metrics = self.custom_metrics.read();
        let profiler_stats = self.profiler.stats();
        let system_stats = self.system_monitor.collect_stats();
        let alerts = self.alerting.active_alerts();
        
        MetricsSummary {
            uptime: self.start_time.elapsed(),
            total_metrics: custom_metrics.total_metrics(),
            active_profiles: profiler_stats.profiles.len(),
            system_stats,
            active_alerts: alerts.len(),
            slow_operations: profiler_stats.slow_operations(self.config.slow_operation_threshold),
        }
    }

    /// Get detailed performance snapshot
    pub fn performance_snapshot(&self) -> UnifiedPerformanceSnapshot {
        let custom_metrics = self.custom_metrics.read();
        let profiler_stats = self.profiler.stats();
        let system_stats = self.system_monitor.collect_stats();
        
        UnifiedPerformanceSnapshot {
            timestamp: SystemTime::now(),
            uptime: self.start_time.elapsed(),
            metrics: custom_metrics.clone(),
            profiler_stats,
            system_stats,
            alerts: self.alerting.active_alerts(),
        }
    }

    /// Get prometheus registry for external integration
    pub fn prometheus_registry(&self) -> Arc<Mutex<Registry>> {
        Arc::clone(&self.prometheus_registry)
    }

    /// Flush all metrics and clear temporary data
    pub fn flush(&self) {
        let mut metrics = self.custom_metrics.write();
        metrics.flush_temporary_data();
        
        self.profiler.cleanup_old_profiles();
        self.alerting.cleanup_old_alerts();
        
        info!("Flushed metrics collector");
    }

    /// Enable or disable profiling
    pub fn set_profiling_enabled(&self, enabled: bool) {
        self.profiler.set_enabled(enabled);
    }

    /// Get monitoring configuration
    pub fn config(&self) -> &MonitoringConfig {
        &self.config
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom metrics storage (complementing prometheus)
#[derive(Debug, Clone)]
pub struct CustomMetrics {
    counters: HashMap<String, UnifiedMetricValue>,
    gauges: HashMap<String, UnifiedMetricValue>,
    histograms: HashMap<String, UnifiedHistogramValue>,
    last_updated: SystemTime,
}

impl CustomMetrics {
    fn new() -> Self {
        Self {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
            last_updated: SystemTime::now(),
        }
    }

    fn increment_counter(&mut self, name: &str, labels: Option<HashMap<String, String>>) {
        let metric = self.counters.entry(name.to_string()).or_insert_with(|| {
            UnifiedMetricValue::new(0.0, labels.clone())
        });
        metric.value += 1.0;
        metric.last_updated = SystemTime::now();
        self.last_updated = SystemTime::now();
    }

    fn record_gauge(&mut self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        let metric = UnifiedMetricValue::new(value, labels);
        self.gauges.insert(name.to_string(), metric);
        self.last_updated = SystemTime::now();
    }

    fn record_histogram(&mut self, name: &str, value: Duration, labels: Option<HashMap<String, String>>) {
        let histogram = self.histograms.entry(name.to_string()).or_insert_with(|| {
            UnifiedHistogramValue::new(labels.clone())
        });
        histogram.record(value);
        self.last_updated = SystemTime::now();
    }

    fn total_metrics(&self) -> usize {
        self.counters.len() + self.gauges.len() + self.histograms.len()
    }

    fn flush_temporary_data(&mut self) {
        // Keep only recent histogram samples
        for histogram in self.histograms.values_mut() {
            histogram.cleanup_old_samples();
        }
    }
}

/// Unified metric value with labels and metadata
#[derive(Debug, Clone)]
pub struct UnifiedMetricValue {
    pub value: f64,
    pub labels: Option<HashMap<String, String>>,
    pub last_updated: SystemTime,
    pub update_count: usize,
}

impl UnifiedMetricValue {
    fn new(value: f64, labels: Option<HashMap<String, String>>) -> Self {
        Self {
            value,
            labels,
            last_updated: SystemTime::now(),
            update_count: 1,
        }
    }
}

/// Enhanced histogram with percentile calculations
#[derive(Debug, Clone)]
pub struct UnifiedHistogramValue {
    pub samples: VecDeque<TimestampedSample>,
    pub labels: Option<HashMap<String, String>>,
    pub count: usize,
    pub sum: Duration,
    pub min: Duration,
    pub max: Duration,
    max_samples: usize,
    max_age: Duration,
}

#[derive(Debug, Clone)]
struct TimestampedSample {
    value: Duration,
    timestamp: SystemTime,
}

impl UnifiedHistogramValue {
    fn new(labels: Option<HashMap<String, String>>) -> Self {
        Self {
            samples: VecDeque::new(),
            labels,
            count: 0,
            sum: Duration::ZERO,
            min: Duration::MAX,
            max: Duration::ZERO,
            max_samples: 10000, // Keep last 10k samples
            max_age: Duration::from_secs(3600), // Keep samples for 1 hour
        }
    }

    fn record(&mut self, value: Duration) {
        let sample = TimestampedSample {
            value,
            timestamp: SystemTime::now(),
        };
        
        self.samples.push_back(sample);
        self.count += 1;
        self.sum += value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);

        // Cleanup old samples
        self.cleanup_old_samples();
    }

    fn cleanup_old_samples(&mut self) {
        let now = SystemTime::now();
        
        // Remove samples older than max_age
        while let Some(front) = self.samples.front() {
            if now.duration_since(front.timestamp).unwrap_or(Duration::ZERO) > self.max_age {
                if let Some(old_sample) = self.samples.pop_front() {
                    self.sum = self.sum.saturating_sub(old_sample.value);
                }
            } else {
                break;
            }
        }

        // Limit number of samples
        while self.samples.len() > self.max_samples {
            if let Some(old_sample) = self.samples.pop_front() {
                self.sum = self.sum.saturating_sub(old_sample.value);
            }
        }

        // Recalculate min/max if needed
        if self.samples.is_empty() {
            self.min = Duration::ZERO;
            self.max = Duration::ZERO;
            self.sum = Duration::ZERO;
        } else {
            self.min = self.samples.iter().map(|s| s.value).min().unwrap_or(Duration::ZERO);
            self.max = self.samples.iter().map(|s| s.value).max().unwrap_or(Duration::ZERO);
        }
    }

    /// Calculate average duration
    pub fn average(&self) -> Duration {
        if self.samples.is_empty() {
            Duration::ZERO
        } else {
            self.sum / self.samples.len() as u32
        }
    }

    /// Calculate percentile (0.0 to 1.0)
    pub fn percentile(&self, p: f64) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }

        let mut values: Vec<Duration> = self.samples.iter().map(|s| s.value).collect();
        values.sort();

        let index = ((values.len() - 1) as f64 * p) as usize;
        values[index]
    }

    /// Get rate (samples per second)
    pub fn rate(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }

        let oldest = self.samples.front().unwrap();
        let newest = self.samples.back().unwrap();
        
        let duration = newest.timestamp.duration_since(oldest.timestamp)
            .unwrap_or(Duration::from_secs(1));
        
        self.samples.len() as f64 / duration.as_secs_f64()
    }
}

/// Unified profiler with enhanced features
pub struct UnifiedProfiler {
    profiles: Arc<RwLock<HashMap<String, UnifiedProfileData>>>,
    enabled: Arc<AtomicBool>,
    cleanup_counter: AtomicUsize,
}

impl UnifiedProfiler {
    fn new() -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            enabled: Arc::new(AtomicBool::new(true)),
            cleanup_counter: AtomicUsize::new(0),
        }
    }

    fn start_profile(&self, name: &str) -> UnifiedProfileGuard {
        UnifiedProfileGuard::new(name.to_string(), self.profiles.clone(), self.enabled.clone())
    }

    fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    fn stats(&self) -> UnifiedProfilerStats {
        let profiles = self.profiles.read();
        UnifiedProfilerStats {
            profiles: profiles.clone(),
        }
    }

    fn cleanup_old_profiles(&self) {
        // Only cleanup periodically to avoid overhead
        if self.cleanup_counter.fetch_add(1, Ordering::Relaxed) % 1000 == 0 {
            let mut profiles = self.profiles.write();
            let now = SystemTime::now();
            
            profiles.retain(|_, profile| {
                now.duration_since(profile.last_call).unwrap_or(Duration::ZERO) < Duration::from_secs(3600)
            });
        }
    }
}

/// Enhanced profile data with more statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedProfileData {
    pub name: String,
    pub total_time: Duration,
    pub call_count: usize,
    pub average_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
    pub last_call: SystemTime,
    pub recent_calls: VecDeque<Duration>, // Last 100 calls for trend analysis
}

impl UnifiedProfileData {
    fn new(name: String) -> Self {
        Self {
            name,
            total_time: Duration::ZERO,
            call_count: 0,
            average_time: Duration::ZERO,
            min_time: Duration::MAX,
            max_time: Duration::ZERO,
            last_call: SystemTime::now(),
            recent_calls: VecDeque::new(),
        }
    }

    fn record(&mut self, duration: Duration) {
        self.total_time += duration;
        self.call_count += 1;
        self.average_time = self.total_time / self.call_count as u32;
        self.min_time = self.min_time.min(duration);
        self.max_time = self.max_time.max(duration);
        self.last_call = SystemTime::now();
        
        // Track recent calls for trend analysis
        self.recent_calls.push_back(duration);
        if self.recent_calls.len() > 100 {
            self.recent_calls.pop_front();
        }
    }

    /// Calculate trend (positive = getting slower, negative = getting faster)
    pub fn trend(&self) -> f64 {
        if self.recent_calls.len() < 10 {
            return 0.0;
        }

        let mid_point = self.recent_calls.len() / 2;
        let first_half: Duration = self.recent_calls.iter().take(mid_point).sum();
        let second_half: Duration = self.recent_calls.iter().skip(mid_point).sum();

        let first_avg = first_half.as_secs_f64() / mid_point as f64;
        let second_avg = second_half.as_secs_f64() / (self.recent_calls.len() - mid_point) as f64;

        (second_avg - first_avg) / first_avg
    }

    /// Check if this profile represents a slow operation
    pub fn is_slow(&self, threshold: Duration) -> bool {
        self.average_time > threshold || self.max_time > threshold * 2
    }
}

/// RAII guard for unified profiling
pub struct UnifiedProfileGuard {
    name: String,
    start_time: Instant,
    profiles: Arc<RwLock<HashMap<String, UnifiedProfileData>>>,
    enabled: Arc<AtomicBool>,
}

impl UnifiedProfileGuard {
    fn new(
        name: String,
        profiles: Arc<RwLock<HashMap<String, UnifiedProfileData>>>,
        enabled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            name,
            start_time: Instant::now(),
            profiles,
            enabled,
        }
    }
}

impl Drop for UnifiedProfileGuard {
    fn drop(&mut self) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        let duration = self.start_time.elapsed();
        let mut profiles = self.profiles.write();
        
        let profile = profiles.entry(self.name.clone()).or_insert_with(|| {
            UnifiedProfileData::new(self.name.clone())
        });
        
        profile.record(duration);
    }
}

/// Unified profiler statistics
#[derive(Debug, Clone)]
pub struct UnifiedProfilerStats {
    pub profiles: HashMap<String, UnifiedProfileData>,
}

impl UnifiedProfilerStats {
    /// Get the top N profiles by total time
    pub fn top_profiles(&self, n: usize) -> Vec<UnifiedProfileData> {
        let mut profiles: Vec<UnifiedProfileData> = self.profiles.values().cloned().collect();
        profiles.sort_by(|a, b| b.total_time.cmp(&a.total_time));
        profiles.truncate(n);
        profiles
    }

    /// Get profiles that are slower than threshold
    pub fn slow_operations(&self, threshold: Duration) -> Vec<UnifiedProfileData> {
        self.profiles
            .values()
            .filter(|p| p.is_slow(threshold))
            .cloned()
            .collect()
    }

    /// Get profiles with negative performance trends
    pub fn degrading_operations(&self) -> Vec<UnifiedProfileData> {
        self.profiles
            .values()
            .filter(|p| p.trend() > 0.1) // Getting 10% slower
            .cloned()
            .collect()
    }
}

/// Enhanced alerting system
pub struct UnifiedAlertingSystem {
    thresholds: UnifiedAlertThresholds,
    active_alerts: Arc<Mutex<Vec<UnifiedAlert>>>,
    slow_operations: Arc<Mutex<Vec<SlowOperationAlert>>>,
}

impl UnifiedAlertingSystem {
    fn new(thresholds: UnifiedAlertThresholds) -> Self {
        Self {
            thresholds,
            active_alerts: Arc::new(Mutex::new(Vec::new())),
            slow_operations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn check_metric(&self, name: &str, value: f64) {
        let should_alert = match name {
            "system.cpu_usage" => value > self.thresholds.cpu_usage,
            "system.memory_usage_bytes" => value > self.thresholds.memory_usage_bytes,
            "network.latency_ms" => value > self.thresholds.network_latency_ms,
            "server.error_rate" => value > self.thresholds.error_rate,
            _ => false,
        };

        if should_alert {
            let alert = UnifiedAlert {
                metric_name: name.to_string(),
                value,
                threshold: self.get_threshold(name),
                timestamp: SystemTime::now(),
                severity: self.calculate_severity(name, value),
                description: self.generate_description(name, value),
            };

            let mut alerts = self.active_alerts.lock();
            alerts.push(alert.clone());
            
            // Keep only recent alerts
            if alerts.len() > 1000 {
                alerts.drain(0..100); // Remove oldest 100 alerts
            }

            warn!("Performance alert: {}", alert.description);
        }
    }

    fn record_slow_operation(&self, name: &str, duration: Duration) {
        let alert = SlowOperationAlert {
            operation_name: name.to_string(),
            duration,
            threshold: self.thresholds.slow_operation_threshold,
            timestamp: SystemTime::now(),
        };

        let mut slow_ops = self.slow_operations.lock();
        slow_ops.push(alert);
        
        // Keep only recent slow operations
        if slow_ops.len() > 500 {
            slow_ops.drain(0..50);
        }
    }

    fn get_threshold(&self, name: &str) -> f64 {
        match name {
            "system.cpu_usage" => self.thresholds.cpu_usage,
            "system.memory_usage_bytes" => self.thresholds.memory_usage_bytes,
            "network.latency_ms" => self.thresholds.network_latency_ms,
            "server.error_rate" => self.thresholds.error_rate,
            _ => 0.0,
        }
    }

    fn calculate_severity(&self, name: &str, value: f64) -> UnifiedAlertSeverity {
        let threshold = self.get_threshold(name);
        let ratio = value / threshold;

        if ratio > 3.0 {
            UnifiedAlertSeverity::Critical
        } else if ratio > 2.0 {
            UnifiedAlertSeverity::High
        } else if ratio > 1.5 {
            UnifiedAlertSeverity::Medium
        } else {
            UnifiedAlertSeverity::Low
        }
    }

    fn generate_description(&self, name: &str, value: f64) -> String {
        match name {
            "system.cpu_usage" => format!("High CPU usage: {:.1}%", value),
            "system.memory_usage_bytes" => format!("High memory usage: {} MB", value as usize / 1024 / 1024),
            "network.latency_ms" => format!("High network latency: {:.1}ms", value),
            "server.error_rate" => format!("High error rate: {:.2}%", value * 100.0),
            _ => format!("Metric {} exceeded threshold: {}", name, value),
        }
    }

    fn active_alerts(&self) -> Vec<UnifiedAlert> {
        self.active_alerts.lock().clone()
    }

    fn cleanup_old_alerts(&self) {
        let now = SystemTime::now();
        let max_age = Duration::from_secs(3600); // Keep alerts for 1 hour

        {
            let mut alerts = self.active_alerts.lock();
            alerts.retain(|alert| {
                now.duration_since(alert.timestamp).unwrap_or(Duration::ZERO) < max_age
            });
        }

        {
            let mut slow_ops = self.slow_operations.lock();
            slow_ops.retain(|alert| {
                now.duration_since(alert.timestamp).unwrap_or(Duration::ZERO) < max_age
            });
        }
    }
}

/// Enhanced alert with more context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAlert {
    pub metric_name: String,
    pub value: f64,
    pub threshold: f64,
    pub timestamp: SystemTime,
    pub severity: UnifiedAlertSeverity,
    pub description: String,
}

/// Slow operation alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowOperationAlert {
    pub operation_name: String,
    pub duration: Duration,
    pub threshold: Duration,
    pub timestamp: SystemTime,
}

/// Enhanced alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnifiedAlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Enhanced alert thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAlertThresholds {
    pub cpu_usage: f64,
    pub memory_usage_bytes: f64,
    pub network_latency_ms: f64,
    pub error_rate: f64,
    pub slow_operation_threshold: Duration,
}

impl Default for UnifiedAlertThresholds {
    fn default() -> Self {
        Self {
            cpu_usage: 80.0,      // 80% CPU usage
            memory_usage_bytes: 2.0 * 1024.0 * 1024.0 * 1024.0, // 2GB memory usage
            network_latency_ms: 100.0, // 100ms network latency
            error_rate: 0.05,     // 5% error rate
            slow_operation_threshold: Duration::from_millis(100), // 100ms operations
        }
    }
}

/// System monitoring for collecting OS-level metrics
pub struct SystemMonitor {
    last_stats: Arc<Mutex<Option<SystemStats>>>,
}

impl SystemMonitor {
    fn new() -> Self {
        Self {
            last_stats: Arc::new(Mutex::new(None)),
        }
    }

    fn collect_stats(&self) -> SystemStats {
        // In a real implementation, this would collect actual system metrics
        // using libraries like sysinfo or platform-specific APIs
        let stats = SystemStats {
            cpu_usage: self.get_cpu_usage(),
            memory_usage: self.get_memory_usage(),
            memory_total: self.get_total_memory(),
            network_rx: 0, // Would collect actual network stats
            network_tx: 0,
            disk_read: 0,  // Would collect actual disk stats
            disk_write: 0,
        };

        // Cache the stats
        *self.last_stats.lock() = Some(stats.clone());
        stats
    }

    fn get_cpu_usage(&self) -> f64 {
        // Placeholder - would use actual CPU monitoring
        0.0
    }

    fn get_memory_usage(&self) -> usize {
        // Placeholder - would use actual memory monitoring
        0
    }

    fn get_total_memory(&self) -> usize {
        // Placeholder - would use actual system memory detection
        8 * 1024 * 1024 * 1024 // 8GB placeholder
    }
}

/// System statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub cpu_usage: f64,
    pub memory_usage: usize,
    pub memory_total: usize,
    pub network_rx: usize,
    pub network_tx: usize,
    pub disk_read: usize,
    pub disk_write: usize,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub profiling_enabled: bool,
    pub system_monitoring_enabled: bool,
    pub metrics_retention: Duration,
    pub alert_thresholds: UnifiedAlertThresholds,
    pub slow_operation_threshold: Duration,
    pub collection_interval: Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            profiling_enabled: true,
            system_monitoring_enabled: true,
            metrics_retention: Duration::from_secs(3600), // 1 hour
            alert_thresholds: UnifiedAlertThresholds::default(),
            slow_operation_threshold: Duration::from_millis(100),
            collection_interval: Duration::from_secs(10),
        }
    }
}

/// Comprehensive metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub uptime: Duration,
    pub total_metrics: usize,
    pub active_profiles: usize,
    pub system_stats: SystemStats,
    pub active_alerts: usize,
    pub slow_operations: Vec<UnifiedProfileData>,
}

/// Unified performance snapshot
#[derive(Debug, Clone)]
pub struct UnifiedPerformanceSnapshot {
    pub timestamp: SystemTime,
    pub uptime: Duration,
    pub metrics: CustomMetrics,
    pub profiler_stats: UnifiedProfilerStats,
    pub system_stats: SystemStats,
    pub alerts: Vec<UnifiedAlert>,
}

/// Macro for easy profiling with the unified system
#[macro_export]
macro_rules! unified_profile {
    ($collector:expr, $name:expr, $code:block) => {{
        let _guard = $collector.start_profile($name);
        $code
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let summary = collector.summary();
        
        assert!(summary.uptime < Duration::from_secs(1));
        assert_eq!(summary.total_metrics, 0);
    }

    #[test]
    fn test_metric_recording() {
        let collector = MetricsCollector::new();
        
        collector.increment_counter("test_counter", None);
        collector.record_gauge("test_gauge", 42.0, None);
        collector.record_histogram("test_histogram", Duration::from_millis(100), None);
        
        let summary = collector.summary();
        assert!(summary.total_metrics >= 3);
    }

    #[test]
    fn test_unified_profiling() {
        let collector = MetricsCollector::new();
        
        {
            let _guard = collector.start_profile("test_operation");
            thread::sleep(Duration::from_millis(10));
        }
        
        let snapshot = collector.performance_snapshot();
        assert!(snapshot.profiler_stats.profiles.contains_key("test_operation"));
        
        let profile = &snapshot.profiler_stats.profiles["test_operation"];
        assert_eq!(profile.call_count, 1);
        assert!(profile.total_time >= Duration::from_millis(10));
    }

    #[test]
    fn test_unified_histogram() {
        let mut histogram = UnifiedHistogramValue::new(None);
        
        histogram.record(Duration::from_millis(100));
        histogram.record(Duration::from_millis(200));
        histogram.record(Duration::from_millis(300));
        
        assert_eq!(histogram.count, 3);
        assert_eq!(histogram.min, Duration::from_millis(100));
        assert_eq!(histogram.max, Duration::from_millis(300));
        assert_eq!(histogram.average(), Duration::from_millis(200));
        
        // Test percentiles
        assert_eq!(histogram.percentile(0.0), Duration::from_millis(100));
        assert_eq!(histogram.percentile(0.5), Duration::from_millis(200));
        assert_eq!(histogram.percentile(1.0), Duration::from_millis(300));
    }

    #[test]
    fn test_unified_alerting() {
        let thresholds = UnifiedAlertThresholds {
            cpu_usage: 50.0,
            memory_usage_bytes: 1024.0 * 1024.0 * 1024.0, // 1GB
            network_latency_ms: 50.0,
            error_rate: 0.02, // 2%
            slow_operation_threshold: Duration::from_millis(50),
        };
        
        let alerting = UnifiedAlertingSystem::new(thresholds);
        
        // Should not trigger alert
        alerting.check_metric("system.cpu_usage", 30.0);
        assert_eq!(alerting.active_alerts().len(), 0);
        
        // Should trigger alert
        alerting.check_metric("system.cpu_usage", 80.0);
        assert_eq!(alerting.active_alerts().len(), 1);
        
        let alert = &alerting.active_alerts()[0];
        assert_eq!(alert.metric_name, "system.cpu_usage");
        assert_eq!(alert.value, 80.0);
        assert_eq!(alert.threshold, 50.0);
    }

    #[test]
    fn test_profile_trend_analysis() {
        let mut profile = UnifiedProfileData::new("test".to_string());
        
        // Add some samples with increasing duration (degrading performance)
        for i in 1..=20 {
            profile.record(Duration::from_millis(i * 10));
        }
        
        let trend = profile.trend();
        assert!(trend > 0.0); // Should show degrading performance
    }
}