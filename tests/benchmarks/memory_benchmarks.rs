//! Memory usage benchmarks and regression testing
//!
//! Tests memory allocation patterns, pool efficiency, and leak detection

use super::*;
use mirai_core::{PerformanceManager, performance::{EnhancedMemoryMonitor, GlobalEnhancedPools}};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Memory benchmark suite
pub struct MemoryBenchmarks {
    runner: BenchmarkRunner,
}

impl MemoryBenchmarks {
    pub fn new() -> Self {
        Self {
            runner: BenchmarkRunner::new(BenchmarkConfig {
                iterations: 5, // Fewer iterations for memory tests
                warmup_iterations: 2,
                timeout: Duration::from_secs(120),
                memory_limit_mb: 4096,
            }),
        }
    }

    /// Run all memory benchmarks
    pub fn run_all_benchmarks(&mut self) -> Vec<BenchmarkResult> {
        println!("Running memory benchmarks...");

        self.benchmark_memory_pool_efficiency();
        self.benchmark_memory_allocation_patterns();
        self.benchmark_memory_leak_detection();
        self.benchmark_memory_pressure();
        self.benchmark_garbage_collection_impact();

        self.runner.results().to_vec()
    }

    /// Benchmark memory pool efficiency
    fn benchmark_memory_pool_efficiency(&mut self) {
        println!("Benchmarking memory pool efficiency...");

        let performance_manager = PerformanceManager::new();
        let pools = performance_manager.memory_pools();

        // Benchmark pool allocation vs direct allocation
        let direct_allocation_time = self.runner.benchmark("direct_allocation_1000", || {
            let mut buffers = Vec::new();
            for _ in 0..1000 {
                buffers.push(Vec::<u8>::with_capacity(1024));
            }
            buffers
        });

        let pool_allocation_time = self.runner.benchmark("pool_allocation_1000", || {
            let mut buffers = Vec::new();
            for _ in 0..1000 {
                buffers.push(pools.packet_buffer_pool.get());
            }
            buffers
        });

        // Benchmark pool reuse efficiency
        self.runner.benchmark("pool_reuse_efficiency", || {
            let mut buffers = Vec::new();
            
            // Allocate
            for _ in 0..100 {
                buffers.push(pools.entity_pool.get());
            }
            
            // Release (return to pool)
            drop(buffers);
            
            // Reallocate (should reuse)
            let mut reused_buffers = Vec::new();
            for _ in 0..100 {
                reused_buffers.push(pools.entity_pool.get());
            }
            
            reused_buffers
        });

        // Record pool statistics as metrics
        let stats = pools.all_stats();
        self.runner.results.push(BenchmarkResult {
            name: "memory_pool_efficiency".to_string(),
            value: stats.overall_efficiency(),
            unit: "ratio".to_string(),
            lower_is_better: false,
            metadata: self.runner.create_metadata("memory_pool_efficiency"),
        });

        self.runner.results.push(BenchmarkResult {
            name: "memory_pool_utilization".to_string(),
            value: stats.entity_pool.utilization(),
            unit: "ratio".to_string(),
            lower_is_better: false,
            metadata: self.runner.create_metadata("memory_pool_utilization"),
        });
    }

    /// Benchmark different memory allocation patterns
    fn benchmark_memory_allocation_patterns(&mut self) {
        println!("Benchmarking memory allocation patterns...");

        let performance_manager = PerformanceManager::new();
        let pools = performance_manager.memory_pools();

        // Pattern 1: Frequent small allocations
        self.runner.benchmark("frequent_small_allocations", || {
            let mut allocations = Vec::new();
            for _ in 0..1000 {
                let mut buffer = pools.string_pool.get();
                buffer.push_str("small allocation test");
                allocations.push(buffer);
            }
            allocations
        });

        // Pattern 2: Large batch allocations
        self.runner.benchmark("large_batch_allocations", || {
            let mut allocations = Vec::new();
            for _ in 0..10 {
                let mut buffer = pools.chunk_data_pool.get();
                buffer.resize(65536, 0); // 64KB
                allocations.push(buffer);
            }
            allocations
        });

        // Pattern 3: Mixed allocation sizes
        self.runner.benchmark("mixed_allocation_sizes", || {
            let mut allocations = Vec::new();
            for i in 0..100 {
                let size = match i % 4 {
                    0 => 64,    // Small
                    1 => 1024,  // Medium
                    2 => 8192,  // Large
                    _ => 32768, // Very large
                };
                
                let mut buffer = pools.packet_buffer_pool.get();
                buffer.resize(size, 0);
                allocations.push(buffer);
            }
            allocations
        });

        // Pattern 4: Allocation churn (allocate and immediately free)
        self.runner.benchmark("allocation_churn", || {
            for _ in 0..1000 {
                let buffer = pools.entity_pool.get();
                // Immediately drop to simulate churn
                drop(buffer);
            }
        });
    }

    /// Benchmark memory leak detection capabilities
    fn benchmark_memory_leak_detection(&mut self) {
        println!("Benchmarking memory leak detection...");

        let monitor = Arc::new(EnhancedMemoryMonitor::new(
            "leak_test".to_string(),
            100, // Low threshold for testing
        ));

        // Simulate normal allocation/deallocation
        self.runner.benchmark("normal_allocation_pattern", || {
            for _ in 0..100 {
                monitor.record_allocation(1024);
                monitor.record_deallocation(1024);
            }
            
            let stats = monitor.stats();
            !stats.has_leaks()
        });

        // Simulate memory leak scenario
        self.runner.benchmark("leak_detection_time", || {
            let leak_monitor = EnhancedMemoryMonitor::new(
                "leak_detection_test".to_string(),
                50,
            );
            
            // Create leak scenario
            for _ in 0..200 {
                leak_monitor.record_allocation(1024);
                // Only deallocate half
                if rand::random::<bool>() {
                    leak_monitor.record_deallocation(1024);
                }
            }
            
            // Time leak detection
            let start = Instant::now();
            let has_leaks = leak_monitor.check_leaks();
            let detection_time = start.elapsed();
            
            (has_leaks, detection_time)
        });

        // Benchmark leak detection overhead
        let baseline_time = self.runner.benchmark("allocation_without_monitoring", || {
            let mut allocations = Vec::new();
            for _ in 0..1000 {
                allocations.push(Vec::<u8>::with_capacity(1024));
            }
            allocations
        });

        let monitored_time = self.runner.benchmark("allocation_with_monitoring", || {
            let mut allocations = Vec::new();
            for _ in 0..1000 {
                monitor.record_allocation(1024);
                allocations.push(Vec::<u8>::with_capacity(1024));
            }
            
            // Clean up monitoring
            for _ in 0..1000 {
                monitor.record_deallocation(1024);
            }
            
            allocations
        });

        // Calculate monitoring overhead
        let overhead_ratio = monitored_time.as_nanos() as f64 / baseline_time.as_nanos() as f64;
        self.runner.results.push(BenchmarkResult {
            name: "memory_monitoring_overhead".to_string(),
            value: overhead_ratio,
            unit: "ratio".to_string(),
            lower_is_better: true,
            metadata: self.runner.create_metadata("memory_monitoring_overhead"),
        });
    }

    /// Benchmark behavior under memory pressure
    fn benchmark_memory_pressure(&mut self) {
        println!("Benchmarking memory pressure scenarios...");

        let performance_manager = PerformanceManager::new();
        let pools = performance_manager.memory_pools();

        // Simulate high memory pressure
        self.runner.benchmark("high_memory_pressure", || {
            let mut allocations = Vec::new();
            
            // Allocate until we hit pool limits
            for _ in 0..2000 { // More than pool capacity
                allocations.push(pools.entity_pool.get());
            }
            
            // Measure allocation time under pressure
            let start = Instant::now();
            let _additional = pools.entity_pool.get();
            let pressure_time = start.elapsed();
            
            pressure_time
        });

        // Benchmark pool cleanup under pressure
        self.runner.benchmark("pool_cleanup_under_pressure", || {
            let mut allocations = Vec::new();
            
            // Fill pools
            for _ in 0..1000 {
                allocations.push(pools.packet_buffer_pool.get());
                allocations.push(pools.string_pool.get());
                allocations.push(pools.chunk_data_pool.get());
            }
            
            // Time cleanup
            let start = Instant::now();
            drop(allocations);
            performance_manager.maintenance();
            let cleanup_time = start.elapsed();
            
            cleanup_time
        });

        // Benchmark recovery from memory pressure
        self.runner.benchmark("memory_pressure_recovery", || {
            // Create pressure
            let mut pressure_allocations = Vec::new();
            for _ in 0..1500 {
                pressure_allocations.push(pools.entity_pool.get());
            }
            
            // Release pressure
            drop(pressure_allocations);
            
            // Measure recovery time
            let start = Instant::now();
            let mut recovery_allocations = Vec::new();
            for _ in 0..100 {
                recovery_allocations.push(pools.entity_pool.get());
            }
            let recovery_time = start.elapsed();
            
            recovery_time
        });
    }

    /// Benchmark garbage collection impact
    fn benchmark_garbage_collection_impact(&mut self) {
        println!("Benchmarking garbage collection impact...");

        let performance_manager = PerformanceManager::new();

        // Benchmark maintenance operation timing
        self.runner.benchmark("maintenance_operation_time", || {
            // Create some allocations
            let pools = performance_manager.memory_pools();
            let mut allocations = Vec::new();
            for _ in 0..500 {
                allocations.push(pools.entity_pool.get());
                allocations.push(pools.packet_buffer_pool.get());
            }
            
            // Time maintenance
            let start = Instant::now();
            performance_manager.maintenance();
            let maintenance_time = start.elapsed();
            
            maintenance_time
        });

        // Benchmark allocation performance before/after maintenance
        let before_maintenance = self.runner.benchmark("allocation_before_maintenance", || {
            let pools = performance_manager.memory_pools();
            let mut allocations = Vec::new();
            for _ in 0..100 {
                allocations.push(pools.entity_pool.get());
            }
            allocations
        });

        // Run maintenance
        performance_manager.maintenance();

        let after_maintenance = self.runner.benchmark("allocation_after_maintenance", || {
            let pools = performance_manager.memory_pools();
            let mut allocations = Vec::new();
            for _ in 0..100 {
                allocations.push(pools.entity_pool.get());
            }
            allocations
        });

        // Calculate maintenance impact
        let maintenance_impact = after_maintenance.as_nanos() as f64 / before_maintenance.as_nanos() as f64;
        self.runner.results.push(BenchmarkResult {
            name: "maintenance_impact_ratio".to_string(),
            value: maintenance_impact,
            unit: "ratio".to_string(),
            lower_is_better: true,
            metadata: self.runner.create_metadata("maintenance_impact"),
        });
    }

    /// Get current memory usage statistics
    fn get_memory_stats(&self) -> MemoryStats {
        let performance_manager = PerformanceManager::new();
        let stats = performance_manager.memory_pools().all_stats();
        
        MemoryStats {
            total_usage: stats.total_memory_usage(),
            pool_efficiency: stats.overall_efficiency(),
            has_leaks: stats.has_leaks(),
        }
    }

    /// Save benchmark results
    pub fn save_results(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.runner.save_results(path)
    }
}

/// Memory statistics snapshot
#[derive(Debug, Clone)]
struct MemoryStats {
    total_usage: usize,
    pool_efficiency: f64,
    has_leaks: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_benchmarks() {
        let mut benchmarks = MemoryBenchmarks::new();
        let results = benchmarks.run_all_benchmarks();
        
        assert!(!results.is_empty());
        
        // Verify we have memory-specific benchmarks
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("pool")));
        assert!(names.iter().any(|n| n.contains("allocation")));
        assert!(names.iter().any(|n| n.contains("leak")));
    }

    #[test]
    fn test_memory_pool_efficiency_measurement() {
        let performance_manager = PerformanceManager::new();
        let pools = performance_manager.memory_pools();
        
        // Use pools to generate statistics
        let mut buffers = Vec::new();
        for _ in 0..10 {
            buffers.push(pools.entity_pool.get());
        }
        drop(buffers);
        
        // Reuse should increase efficiency
        let mut reused = Vec::new();
        for _ in 0..5 {
            reused.push(pools.entity_pool.get());
        }
        
        let stats = pools.all_stats();
        assert!(stats.overall_efficiency() > 0.0);
    }

    #[test]
    fn test_memory_leak_detection() {
        let monitor = EnhancedMemoryMonitor::new("test".to_string(), 10);
        
        // Normal pattern - no leaks
        for _ in 0..5 {
            monitor.record_allocation(100);
            monitor.record_deallocation(100);
        }
        assert!(!monitor.check_leaks());
        
        // Leak pattern
        for _ in 0..20 {
            monitor.record_allocation(100);
            // Don't deallocate - creates leak
        }
        assert!(monitor.check_leaks());
    }
}