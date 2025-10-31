//! Performance benchmarking module for the unified mirai server
//!
//! This module provides comprehensive benchmarking capabilities to compare
//! the unified server performance against baseline Mirai and detect regressions.

pub mod baseline_comparison;
pub mod memory_benchmarks;
pub mod ecs_overhead;
pub mod plugin_performance;
pub mod regression_detection;
pub mod runner;

use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Benchmark result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub lower_is_better: bool,
    pub metadata: BenchmarkMetadata,
}

/// Metadata for benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetadata {
    pub timestamp: u64,
    pub git_commit: Option<String>,
    pub test_config: String,
    pub system_info: SystemInfo,
}

/// System information for benchmark context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu_cores: usize,
    pub memory_gb: f64,
}

/// Benchmark suite configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub timeout: Duration,
    pub memory_limit_mb: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 10,
            warmup_iterations: 3,
            timeout: Duration::from_secs(300),
            memory_limit_mb: 2048,
        }
    }
}

/// Benchmark runner that executes and collects results
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
    results: Vec<BenchmarkResult>,
}

impl BenchmarkRunner {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
        }
    }

    /// Run a benchmark function and collect timing results
    pub fn benchmark<F, R>(&mut self, name: &str, mut benchmark_fn: F) -> R
    where
        F: FnMut() -> R,
    {
        // Warmup iterations
        for _ in 0..self.config.warmup_iterations {
            let _ = benchmark_fn();
        }

        // Actual benchmark iterations
        let mut durations = Vec::new();
        let mut last_result = None;

        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let result = benchmark_fn();
            let duration = start.elapsed();
            
            durations.push(duration);
            last_result = Some(result);
        }

        // Calculate statistics
        let avg_duration = durations.iter().sum::<Duration>() / durations.len() as u32;
        let min_duration = durations.iter().min().unwrap();
        let max_duration = durations.iter().max().unwrap();

        // Store results
        self.results.push(BenchmarkResult {
            name: format!("{}_avg", name),
            value: avg_duration.as_secs_f64(),
            unit: "seconds".to_string(),
            lower_is_better: true,
            metadata: self.create_metadata(name),
        });

        self.results.push(BenchmarkResult {
            name: format!("{}_min", name),
            value: min_duration.as_secs_f64(),
            unit: "seconds".to_string(),
            lower_is_better: true,
            metadata: self.create_metadata(name),
        });

        self.results.push(BenchmarkResult {
            name: format!("{}_max", name),
            value: max_duration.as_secs_f64(),
            unit: "seconds".to_string(),
            lower_is_better: true,
            metadata: self.create_metadata(name),
        });

        last_result.unwrap()
    }

    /// Run a throughput benchmark (operations per second)
    pub fn benchmark_throughput<F>(&mut self, name: &str, operations: usize, mut benchmark_fn: F)
    where
        F: FnMut(),
    {
        // Warmup
        for _ in 0..self.config.warmup_iterations {
            benchmark_fn();
        }

        // Benchmark
        let start = Instant::now();
        for _ in 0..operations {
            benchmark_fn();
        }
        let duration = start.elapsed();

        let ops_per_second = operations as f64 / duration.as_secs_f64();

        self.results.push(BenchmarkResult {
            name: format!("{}_throughput", name),
            value: ops_per_second,
            unit: "ops/sec".to_string(),
            lower_is_better: false,
            metadata: self.create_metadata(name),
        });
    }

    /// Get all benchmark results
    pub fn results(&self) -> &[BenchmarkResult] {
        &self.results
    }

    /// Clear all results
    pub fn clear_results(&mut self) {
        self.results.clear();
    }

    /// Save results to JSON file
    pub fn save_results(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self.results)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn create_metadata(&self, test_name: &str) -> BenchmarkMetadata {
        BenchmarkMetadata {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            git_commit: std::env::var("GIT_COMMIT").ok(),
            test_config: test_name.to_string(),
            system_info: SystemInfo {
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
                cpu_cores: num_cpus::get(),
                memory_gb: get_system_memory_gb(),
            },
        }
    }
}

fn get_system_memory_gb() -> f64 {
    // Simple estimation - in a real implementation would use proper system info
    8.0 // Default to 8GB
}

/// Macro for easy benchmarking
#[macro_export]
macro_rules! benchmark {
    ($runner:expr, $name:expr, $code:block) => {
        $runner.benchmark($name, || $code)
    };
}

/// Macro for throughput benchmarking
#[macro_export]
macro_rules! benchmark_throughput {
    ($runner:expr, $name:expr, $ops:expr, $code:block) => {
        $runner.benchmark_throughput($name, $ops, || $code)
    };
}