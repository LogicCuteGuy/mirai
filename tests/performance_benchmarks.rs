//! Performance benchmark tests for CI integration
//!
//! This module provides the main entry point for running performance benchmarks
//! in CI environments and local development.

use mirai::tests::benchmarks::{
    runner::{BenchmarkSuiteRunner, BenchmarkSuiteConfig},
    regression_detection::CIIntegration,
};

#[tokio::test]
#[ignore] // Only run when explicitly requested
async fn benchmark_suite_runner() {
    // This test runs the complete benchmark suite
    let config = BenchmarkSuiteConfig::default();
    let mut runner = BenchmarkSuiteRunner::new(config);
    
    let results = runner.run_all_benchmarks()
        .expect("Benchmark suite should complete successfully");
    
    // Assert that we got results
    assert!(results.total_duration.as_secs() > 0);
    
    // Check if we should fail based on regressions
    if let Some(regression_analysis) = &results.regression_analysis {
        if !regression_analysis.regressions.is_empty() {
            println!("⚠️ Performance regressions detected:");
            for regression in &regression_analysis.regressions {
                println!("  - {}: {:.3} → {:.3} ({:+.1}%)",
                    regression.benchmark_name,
                    regression.baseline_value,
                    regression.current_value,
                    regression.degradation_percent
                );
            }
        }
    }
    
    // Print summary
    println!("Benchmark suite completed:");
    println!("  Status: {:?}", results.overall_status);
    println!("  Duration: {:?}", results.total_duration);
    
    if let Some(baseline) = &results.baseline_comparison {
        println!("  Baseline benchmarks: {}", baseline.len());
    }
    if let Some(memory) = &results.memory_benchmarks {
        println!("  Memory benchmarks: {}", memory.len());
    }
    if let Some(ecs) = &results.ecs_overhead {
        println!("  ECS overhead benchmarks: {}", ecs.len());
    }
    if let Some(plugin) = &results.plugin_performance {
        println!("  Plugin performance benchmarks: {}", plugin.len());
    }
}

#[tokio::test]
#[ignore]
async fn ci_benchmark_runner() {
    // Simplified benchmark runner for CI environments
    let success = BenchmarkSuiteRunner::run_ci_benchmarks()
        .expect("CI benchmarks should complete");
    
    if !success {
        panic!("CI benchmarks failed due to performance regressions");
    }
    
    println!("✅ CI benchmarks passed");
}

#[tokio::test]
#[ignore]
async fn baseline_comparison_only() {
    // Run only baseline comparison benchmarks (faster for development)
    let config = BenchmarkSuiteConfig {
        run_baseline_comparison: true,
        run_memory_benchmarks: false,
        run_ecs_overhead: false,
        run_plugin_performance: false,
        run_regression_detection: false,
        ..Default::default()
    };
    
    let mut runner = BenchmarkSuiteRunner::new(config);
    let results = runner.run_all_benchmarks()
        .expect("Baseline comparison should complete");
    
    assert!(results.baseline_comparison.is_some());
    println!("Baseline comparison completed with {} benchmarks", 
             results.baseline_comparison.unwrap().len());
}

#[tokio::test]
#[ignore]
async fn memory_benchmarks_only() {
    // Run only memory benchmarks
    let config = BenchmarkSuiteConfig {
        run_baseline_comparison: false,
        run_memory_benchmarks: true,
        run_ecs_overhead: false,
        run_plugin_performance: false,
        run_regression_detection: false,
        ..Default::default()
    };
    
    let mut runner = BenchmarkSuiteRunner::new(config);
    let results = runner.run_all_benchmarks()
        .expect("Memory benchmarks should complete");
    
    assert!(results.memory_benchmarks.is_some());
    println!("Memory benchmarks completed with {} benchmarks", 
             results.memory_benchmarks.unwrap().len());
}

#[tokio::test]
#[ignore]
async fn ecs_overhead_benchmarks_only() {
    // Run only ECS overhead benchmarks
    let config = BenchmarkSuiteConfig {
        run_baseline_comparison: false,
        run_memory_benchmarks: false,
        run_ecs_overhead: true,
        run_plugin_performance: false,
        run_regression_detection: false,
        ..Default::default()
    };
    
    let mut runner = BenchmarkSuiteRunner::new(config);
    let results = runner.run_all_benchmarks()
        .expect("ECS overhead benchmarks should complete");
    
    assert!(results.ecs_overhead.is_some());
    println!("ECS overhead benchmarks completed with {} benchmarks", 
             results.ecs_overhead.unwrap().len());
}

#[tokio::test]
#[ignore]
async fn plugin_performance_benchmarks_only() {
    // Run only plugin performance benchmarks
    let config = BenchmarkSuiteConfig {
        run_baseline_comparison: false,
        run_memory_benchmarks: false,
        run_ecs_overhead: false,
        run_plugin_performance: true,
        run_regression_detection: false,
        ..Default::default()
    };
    
    let mut runner = BenchmarkSuiteRunner::new(config);
    let results = runner.run_all_benchmarks()
        .expect("Plugin performance benchmarks should complete");
    
    assert!(results.plugin_performance.is_some());
    println!("Plugin performance benchmarks completed with {} benchmarks", 
             results.plugin_performance.unwrap().len());
}

#[test]
fn test_regression_detection_integration() {
    // Test regression detection with mock data
    use mirai::tests::benchmarks::{BenchmarkResult, BenchmarkMetadata, SystemInfo};
    
    let current_results = vec![
        BenchmarkResult {
            name: "test_benchmark".to_string(),
            value: 1.2,
            unit: "seconds".to_string(),
            lower_is_better: true,
            metadata: BenchmarkMetadata {
                timestamp: 1234567890,
                git_commit: Some("test".to_string()),
                test_config: "test".to_string(),
                system_info: SystemInfo {
                    os: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    cpu_cores: 8,
                    memory_gb: 16.0,
                },
            },
        },
    ];
    
    // Test CI integration (should handle missing baseline gracefully)
    let success = CIIntegration::run_ci_regression_check(&current_results, "nonexistent_baseline.json")
        .expect("Regression check should complete");
    
    // Should succeed when no baseline exists
    assert!(success);
    println!("✅ Regression detection integration test passed");
}

// Helper functions for benchmark development and debugging

#[allow(dead_code)]
fn run_quick_benchmark_check() {
    // Quick benchmark check for development
    println!("Running quick benchmark check...");
    
    use std::time::Instant;
    use mirai_core::PerformanceManager;
    
    let performance_manager = PerformanceManager::new();
    
    // Quick memory pool test
    let start = Instant::now();
    let mut buffers = Vec::new();
    for _ in 0..1000 {
        buffers.push(performance_manager.memory_pools().entity_pool.get());
    }
    let allocation_time = start.elapsed();
    
    drop(buffers);
    let cleanup_time = start.elapsed() - allocation_time;
    
    println!("Memory pool allocation: {:?}", allocation_time);
    println!("Memory pool cleanup: {:?}", cleanup_time);
    
    // Quick metrics test
    let metrics = performance_manager.metrics_collector();
    metrics.increment_counter("test_counter", None);
    metrics.record_gauge("test_gauge", 42.0, None);
    
    let summary = metrics.summary();
    println!("Metrics collected: {}", summary.total_metrics);
    
    println!("✅ Quick benchmark check completed");
}

#[cfg(test)]
mod development_helpers {
    use super::*;
    
    #[test]
    #[ignore]
    fn quick_performance_check() {
        run_quick_benchmark_check();
    }
    
    #[test]
    fn benchmark_result_serialization() {
        use mirai::tests::benchmarks::{BenchmarkResult, BenchmarkMetadata, SystemInfo};
        
        let result = BenchmarkResult {
            name: "test".to_string(),
            value: 1.23,
            unit: "seconds".to_string(),
            lower_is_better: true,
            metadata: BenchmarkMetadata {
                timestamp: 1234567890,
                git_commit: Some("abc123".to_string()),
                test_config: "test".to_string(),
                system_info: SystemInfo {
                    os: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    cpu_cores: 8,
                    memory_gb: 16.0,
                },
            },
        };
        
        let json = serde_json::to_string_pretty(&result).unwrap();
        println!("Serialized benchmark result:\n{}", json);
        
        let deserialized: BenchmarkResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.name, deserialized.name);
        assert_eq!(result.value, deserialized.value);
    }
}