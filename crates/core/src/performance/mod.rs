//! Performance optimization systems for the unified mirai server
//!
//! This module provides comprehensive performance optimizations including:
//! - Enhanced memory management and object pooling
//! - Threading and concurrency improvements
//! - Performance monitoring and metrics collection

pub mod memory;
pub mod threading;
pub mod monitoring;

#[cfg(test)]
mod tests;

pub use memory::*;
pub use threading::*;
pub use monitoring::*;

use std::sync::Arc;
use tracing::info;
use std::time::Duration;

/// Global performance manager that coordinates all performance systems
pub struct PerformanceManager {
    memory_pools: GlobalEnhancedPools,
    thread_manager: EnhancedThreadManager,
    metrics_collector: MetricsCollector,
}

impl PerformanceManager {
    /// Create a new performance manager with default configurations
    pub fn new() -> Self {
        info!("Initializing unified performance management system");
        
        Self {
            memory_pools: GlobalEnhancedPools::new(),
            thread_manager: EnhancedThreadManager::new(),
            metrics_collector: MetricsCollector::new(),
        }
    }

    /// Get access to the global memory pools
    pub fn memory_pools(&self) -> &GlobalEnhancedPools {
        &self.memory_pools
    }

    /// Get access to the thread manager
    pub fn thread_manager(&self) -> &EnhancedThreadManager {
        &self.thread_manager
    }

    /// Get access to the metrics collector
    pub fn metrics_collector(&self) -> &MetricsCollector {
        &self.metrics_collector
    }

    /// Get comprehensive performance statistics
    pub fn performance_stats(&self) -> PerformanceStats {
        PerformanceStats {
            memory: self.memory_pools.all_stats(),
            threading: self.thread_manager.stats(),
            metrics: self.metrics_collector.summary(),
        }
    }

    /// Perform maintenance tasks (cleanup, leak detection, etc.)
    pub fn maintenance(&self) {
        info!("Running performance system maintenance");
        
        // Check for memory leaks
        if self.memory_pools.check_leaks() {
            tracing::warn!("Memory leaks detected during maintenance");
        }

        // Collect metrics
        self.metrics_collector.collect_system_metrics();
        
        // Thread pool maintenance
        self.thread_manager.maintenance();
    }

    /// Shutdown all performance systems gracefully
    pub fn shutdown(&self) {
        info!("Shutting down performance management system");
        
        self.thread_manager.shutdown();
        self.memory_pools.clear_all();
        self.metrics_collector.flush();
    }
}

impl Default for PerformanceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub memory: GlobalEnhancedPoolStats,
    pub threading: ThreadingStats,
    pub metrics: MetricsSummary,
}

impl PerformanceStats {
    /// Generate a human-readable performance report
    pub fn report(&self) -> String {
        format!(
            "Performance Report:\n\
             Memory Efficiency: {:.2}%\n\
             Memory Usage: {}\n\
             Thread Pool Utilization: {:.2}%\n\
             Active Threads: {}\n\
             Metrics Collected: {}",
            self.memory.overall_efficiency() * 100.0,
            self.memory.format_total_usage(),
            self.threading.utilization() * 100.0,
            self.threading.active_threads(),
            self.metrics.total_metrics
        )
    }
}

/// Global enhanced memory pools for frequently allocated objects
pub struct GlobalEnhancedPools {
    pub entity_pool: EnhancedResettingPool<Vec<u8>>, // For entity data
    pub packet_buffer_pool: EnhancedResettingPool<Vec<u8>>, // For packet buffers
    pub string_pool: EnhancedResettingPool<String>, // For string operations
    pub chunk_data_pool: EnhancedResettingPool<Vec<u8>>, // For chunk data
    global_monitor: Arc<EnhancedMemoryMonitor>,
}

impl GlobalEnhancedPools {
    /// Create new global enhanced pools with optimized configurations
    pub fn new() -> Self {
        let global_monitor = Arc::new(EnhancedMemoryMonitor::new(
            "global_memory".to_string(),
            1000, // Global leak threshold
        ));

        Self {
            entity_pool: EnhancedResettingPool::new(
                "entity_data".to_string(),
                || Vec::with_capacity(512), // Pre-allocate for entity data
                1000, // Pool up to 1000 entity buffers
            ),
            packet_buffer_pool: EnhancedResettingPool::new(
                "packet_buffers".to_string(),
                || Vec::with_capacity(1024), // Pre-allocate 1KB buffers
                500, // Pool up to 500 packet buffers
            ),
            string_pool: EnhancedResettingPool::new(
                "strings".to_string(),
                || String::with_capacity(256), // Pre-allocate string capacity
                200, // Pool up to 200 strings
            ),
            chunk_data_pool: EnhancedResettingPool::new(
                "chunk_data".to_string(),
                || Vec::with_capacity(65536), // Pre-allocate 64KB for chunk data
                100, // Pool up to 100 chunk buffers
            ),
            global_monitor,
        }
    }

    /// Get statistics for all enhanced pools
    pub fn all_stats(&self) -> GlobalEnhancedPoolStats {
        GlobalEnhancedPoolStats {
            entity_pool: self.entity_pool.stats(),
            packet_buffer_pool: self.packet_buffer_pool.stats(),
            string_pool: self.string_pool.stats(),
            chunk_data_pool: self.chunk_data_pool.stats(),
            global_memory: self.global_monitor.stats(),
        }
    }

    /// Clear all enhanced pools
    pub fn clear_all(&self) {
        self.entity_pool.clear();
        self.packet_buffer_pool.clear();
        self.string_pool.clear();
        self.chunk_data_pool.clear();
        self.global_monitor.reset();
        info!("Cleared all global enhanced pools");
    }

    /// Check for memory leaks across all pools
    pub fn check_leaks(&self) -> bool {
        let mut has_leaks = false;
        
        has_leaks |= self.entity_pool.check_leaks();
        has_leaks |= self.packet_buffer_pool.check_leaks();
        has_leaks |= self.string_pool.check_leaks();
        has_leaks |= self.chunk_data_pool.check_leaks();
        has_leaks |= self.global_monitor.check_leaks();
        
        has_leaks
    }
}

impl Default for GlobalEnhancedPools {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for all global enhanced pools
#[derive(Debug, Clone)]
pub struct GlobalEnhancedPoolStats {
    pub entity_pool: EnhancedPoolStats,
    pub packet_buffer_pool: EnhancedPoolStats,
    pub string_pool: EnhancedPoolStats,
    pub chunk_data_pool: EnhancedPoolStats,
    pub global_memory: EnhancedMemoryStats,
}

impl GlobalEnhancedPoolStats {
    /// Calculate overall efficiency across all pools
    pub fn overall_efficiency(&self) -> f64 {
        let pools = [
            &self.entity_pool,
            &self.packet_buffer_pool,
            &self.string_pool,
            &self.chunk_data_pool,
        ];

        let total_reused: usize = pools.iter().map(|p| p.reused_count).sum();
        let total_created: usize = pools.iter().map(|p| p.created_count).sum();
        let total_operations = total_reused + total_created;

        if total_operations == 0 {
            0.0
        } else {
            total_reused as f64 / total_operations as f64
        }
    }

    /// Get total memory usage across all pools
    pub fn total_memory_usage(&self) -> usize {
        self.global_memory.current_usage
    }

    /// Format total memory usage
    pub fn format_total_usage(&self) -> String {
        format_bytes(self.total_memory_usage())
    }

    /// Check if any pool has memory leaks
    pub fn has_leaks(&self) -> bool {
        self.global_memory.has_leaks()
    }
}