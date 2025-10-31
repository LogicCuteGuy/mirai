//! Main benchmark runner for performance testing
//!
//! Orchestrates all benchmark suites and generates comprehensive reports

use super::*;
use crate::benchmarks::{
    baseline_comparison::BaselineComparison,
    memory_benchmarks::MemoryBenchmarks,
    ecs_overhead::EcsOverheadBenchmarks,
    plugin_performance::PluginPerformanceBenchmarks,
    regression_detection::{RegressionDetector, CIIntegration},
};
use std::time::{Duration, Instant};
use std::fs;
use std::path::Path;

/// Main benchmark suite runner
pub struct BenchmarkSuiteRunner {
    config: BenchmarkSuiteConfig,
    results: Vec<BenchmarkResult>,
}

/// Configuration for benchmark suite
#[derive(Debug, Clone)]
pub struct BenchmarkSuiteConfig {
    pub run_baseline_comparison: bool,
    pub run_memory_benchmarks: bool,
    pub run_ecs_overhead: bool,
    pub run_plugin_performance: bool,
    pub run_regression_detection: bool,
    pub output_dir: String,
    pub baseline_path: String,
    pub generate_reports: bool,
    pub fail_on_regression: bool,
}

impl Default for BenchmarkSuiteConfig {
    fn default() -> Self {
        Self {
            run_baseline_comparison: true,
            run_memory_benchmarks: true,
            run_ecs_overhead: true,
            run_plugin_performance: true,
            run_regression_detection: true,
            output_dir: "benchmark_results".to_string(),
            baseline_path: "baseline_benchmarks.json".to_string(),
            generate_reports: true,
            fail_on_regression: true,
        }
    }
}

impl BenchmarkSuiteRunner {
    /// Create a new benchmark suite runner
    pub fn new(config: BenchmarkSuiteConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
        }
    }

    /// Run all configured benchmark suites
    pub fn run_all_benchmarks(&mut self) -> Result<BenchmarkSuiteResults, Box<dyn std::error::Error>> {
        println!("Starting comprehensive benchmark suite...");
        let start_time = Instant::now();

        // Ensure output directory exists
        fs::create_dir_all(&self.config.output_dir)?;

        let mut suite_results = BenchmarkSuiteResults {
            total_duration: Duration::ZERO,
            baseline_comparison: None,
            memory_benchmarks: None,
            ecs_overhead: None,
            plugin_performance: None,
            regression_analysis: None,
            overall_status: BenchmarkStatus::Pass,
        };

        // Run baseline comparison benchmarks
        if self.config.run_baseline_comparison {
            println!("\n=== Running Baseline Comparison Benchmarks ===");
            let mut baseline_comparison = BaselineComparison::new();
            let results = baseline_comparison.run_all_benchmarks();
            
            self.results.extend(results.clone());
            suite_results.baseline_comparison = Some(results);
            
            // Save individual results
            baseline_comparison.save_results(&format!("{}/baseline_comparison.json", self.config.output_dir))?;
        }

        // Run memory benchmarks
        if self.config.run_memory_benchmarks {
            println!("\n=== Running Memory Benchmarks ===");
            let mut memory_benchmarks = MemoryBenchmarks::new();
            let results = memory_benchmarks.run_all_benchmarks();
            
            self.results.extend(results.clone());
            suite_results.memory_benchmarks = Some(results);
            
            memory_benchmarks.save_results(&format!("{}/memory_benchmarks.json", self.config.output_dir))?;
        }

        // Run ECS overhead benchmarks
        if self.config.run_ecs_overhead {
            println!("\n=== Running ECS Overhead Benchmarks ===");
            let mut ecs_benchmarks = EcsOverheadBenchmarks::new();
            let results = ecs_benchmarks.run_all_benchmarks();
            
            self.results.extend(results.clone());
            suite_results.ecs_overhead = Some(results);
            
            ecs_benchmarks.save_results(&format!("{}/ecs_overhead.json", self.config.output_dir))?;
        }

        // Run plugin performance benchmarks
        if self.config.run_plugin_performance {
            println!("\n=== Running Plugin Performance Benchmarks ===");
            let mut plugin_benchmarks = PluginPerformanceBenchmarks::new();
            let results = plugin_benchmarks.run_all_benchmarks();
            
            self.results.extend(results.clone());
            suite_results.plugin_performance = Some(results);
            
            plugin_benchmarks.save_results(&format!("{}/plugin_performance.json", self.config.output_dir))?;
        }

        // Run regression detection
        if self.config.run_regression_detection {
            println!("\n=== Running Regression Detection ===");
            let detector = RegressionDetector::new(self.config.baseline_path.clone());
            let analysis = detector.analyze_regressions(&self.results)?;
            
            suite_results.regression_analysis = Some(analysis.clone());
            
            // Generate regression report
            let report = detector.generate_report(&analysis);
            fs::write(format!("{}/regression_report.md", self.config.output_dir), report)?;
            
            // Update overall status based on regression analysis
            suite_results.overall_status = match analysis.overall_status {
                crate::benchmarks::regression_detection::RegressionStatus::Pass => BenchmarkStatus::Pass,
                crate::benchmarks::regression_detection::RegressionStatus::Warning => BenchmarkStatus::Warning,
                crate::benchmarks::regression_detection::RegressionStatus::Fail => BenchmarkStatus::Fail,
            };
        }

        suite_results.total_duration = start_time.elapsed();

        // Save comprehensive results
        self.save_comprehensive_results(&suite_results)?;

        // Generate reports if configured
        if self.config.generate_reports {
            self.generate_comprehensive_report(&suite_results)?;
        }

        println!("\n=== Benchmark Suite Complete ===");
        println!("Total duration: {:?}", suite_results.total_duration);
        println!("Overall status: {:?}", suite_results.overall_status);

        Ok(suite_results)
    }

    /// Save comprehensive benchmark results
    fn save_comprehensive_results(&self, results: &BenchmarkSuiteResults) -> Result<(), Box<dyn std::error::Error>> {
        // Save all individual benchmark results
        let json = serde_json::to_string_pretty(&self.results)?;
        fs::write(format!("{}/all_benchmarks.json", self.config.output_dir), json)?;

        // Save suite summary
        let summary = BenchmarkSuiteSummary {
            total_benchmarks: self.results.len(),
            total_duration: results.total_duration,
            overall_status: results.overall_status,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            git_commit: std::env::var("GIT_COMMIT").ok(),
            system_info: SystemInfo {
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
                cpu_cores: num_cpus::get(),
                memory_gb: 8.0, // Placeholder
            },
        };

        let summary_json = serde_json::to_string_pretty(&summary)?;
        fs::write(format!("{}/benchmark_summary.json", self.config.output_dir), summary_json)?;

        Ok(())
    }

    /// Generate comprehensive benchmark report
    fn generate_comprehensive_report(&self, results: &BenchmarkSuiteResults) -> Result<(), Box<dyn std::error::Error>> {
        let mut report = Vec::new();
        
        report.push("# Comprehensive Benchmark Report\n".to_string());
        
        // Executive summary
        report.push("## Executive Summary".to_string());
        report.push(format!("- **Overall Status**: {:?}", results.overall_status));
        report.push(format!("- **Total Benchmarks**: {}", self.results.len()));
        report.push(format!("- **Total Duration**: {:?}", results.total_duration));
        report.push(format!("- **Timestamp**: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        if let Some(git_commit) = std::env::var("GIT_COMMIT").ok() {
            report.push(format!("- **Git Commit**: {}", git_commit));
        }
        report.push("".to_string());

        // Performance highlights
        report.push("## Performance Highlights".to_string());
        let highlights = self.generate_performance_highlights();
        for highlight in highlights {
            report.push(format!("- {}", highlight));
        }
        report.push("".to_string());

        // Benchmark category summaries
        if results.baseline_comparison.is_some() {
            report.push("### Baseline Comparison".to_string());
            report.push("✅ Server performance compared against baseline Mirai implementation".to_string());
            report.push("".to_string());
        }

        if results.memory_benchmarks.is_some() {
            report.push("### Memory Performance".to_string());
            report.push("✅ Memory allocation patterns and pool efficiency validated".to_string());
            report.push("".to_string());
        }

        if results.ecs_overhead.is_some() {
            report.push("### ECS Integration".to_string());
            report.push("✅ ECS overhead impact on packet processing measured".to_string());
            report.push("".to_string());
        }

        if results.plugin_performance.is_some() {
            report.push("### Plugin System".to_string());
            report.push("✅ Plugin system impact on server tick rate analyzed".to_string());
            report.push("".to_string());
        }

        // Regression analysis
        if let Some(regression_analysis) = &results.regression_analysis {
            report.push("## Regression Analysis".to_string());
            
            if !regression_analysis.regressions.is_empty() {
                report.push(format!("⚠️ **{} performance regressions detected**", regression_analysis.regressions.len()));
                for regression in &regression_analysis.regressions {
                    report.push(format!(
                        "  - {}: {:.3} → {:.3} ({:+.1}%)",
                        regression.benchmark_name,
                        regression.baseline_value,
                        regression.current_value,
                        regression.degradation_percent
                    ));
                }
            } else {
                report.push("✅ **No performance regressions detected**".to_string());
            }
            
            if !regression_analysis.improvements.is_empty() {
                report.push(format!("🚀 **{} performance improvements detected**", regression_analysis.improvements.len()));
                for improvement in &regression_analysis.improvements {
                    report.push(format!(
                        "  - {}: {:.3} → {:.3} ({:+.1}%)",
                        improvement.benchmark_name,
                        improvement.baseline_value,
                        improvement.current_value,
                        improvement.improvement_percent
                    ));
                }
            }
            report.push("".to_string());
        }

        // Performance metrics table
        report.push("## Key Performance Metrics".to_string());
        report.push("| Metric | Value | Unit | Status |".to_string());
        report.push("|--------|-------|------|--------|".to_string());
        
        let key_metrics = self.extract_key_metrics();
        for metric in key_metrics {
            let status = if metric.meets_threshold { "✅" } else { "⚠️" };
            report.push(format!(
                "| {} | {:.3} | {} | {} |",
                metric.name, metric.value, metric.unit, status
            ));
        }
        report.push("".to_string());

        // Recommendations
        report.push("## Recommendations".to_string());
        let recommendations = self.generate_recommendations(results);
        for recommendation in recommendations {
            report.push(format!("- {}", recommendation));
        }
        report.push("".to_string());

        // Technical details
        report.push("## Technical Details".to_string());
        report.push(format!("- **OS**: {}", std::env::consts::OS));
        report.push(format!("- **Architecture**: {}", std::env::consts::ARCH));
        report.push(format!("- **CPU Cores**: {}", num_cpus::get()));
        report.push("".to_string());

        // Save report
        let report_content = report.join("\n");
        fs::write(format!("{}/comprehensive_report.md", self.config.output_dir), report_content)?;

        Ok(())
    }

    /// Generate performance highlights
    fn generate_performance_highlights(&self) -> Vec<String> {
        let mut highlights = Vec::new();
        
        // Find best and worst performing benchmarks
        if let Some(fastest) = self.results.iter()
            .filter(|r| r.lower_is_better && r.unit.contains("seconds"))
            .min_by(|a, b| a.value.partial_cmp(&b.value).unwrap()) {
            highlights.push(format!("Fastest operation: {} ({:.3} {})", fastest.name, fastest.value, fastest.unit));
        }

        if let Some(highest_throughput) = self.results.iter()
            .filter(|r| !r.lower_is_better && (r.unit.contains("/sec") || r.unit.contains("ops")))
            .max_by(|a, b| a.value.partial_cmp(&b.value).unwrap()) {
            highlights.push(format!("Highest throughput: {} ({:.1} {})", highest_throughput.name, highest_throughput.value, highest_throughput.unit));
        }

        // Memory efficiency
        if let Some(memory_efficiency) = self.results.iter()
            .find(|r| r.name.contains("memory_pool_efficiency")) {
            highlights.push(format!("Memory pool efficiency: {:.1}%", memory_efficiency.value * 100.0));
        }

        highlights
    }

    /// Extract key performance metrics
    fn extract_key_metrics(&self) -> Vec<KeyMetric> {
        let mut metrics = Vec::new();
        
        // Define key metrics to track
        let key_metric_names = vec![
            ("server_startup_avg", 2.0, "seconds"),
            ("packet_processing_ecs_overhead", 1.5, "ratio"),
            ("memory_pool_efficiency", 0.8, "ratio"),
            ("tick_rate_lightweight_impact", 1.2, "ratio"),
            ("sustained_tick_rate_with_plugins", 20.0, "ticks/sec"),
        ];

        for (name, threshold, unit) in key_metric_names {
            if let Some(result) = self.results.iter().find(|r| r.name == name) {
                let meets_threshold = if result.lower_is_better {
                    result.value <= threshold
                } else {
                    result.value >= threshold
                };

                metrics.push(KeyMetric {
                    name: name.to_string(),
                    value: result.value,
                    unit: unit.to_string(),
                    meets_threshold,
                });
            }
        }

        metrics
    }

    /// Generate performance recommendations
    fn generate_recommendations(&self, results: &BenchmarkSuiteResults) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Check for high ECS overhead
        if let Some(ecs_overhead) = self.results.iter()
            .find(|r| r.name == "packet_processing_ecs_overhead") {
            if ecs_overhead.value > 1.5 {
                recommendations.push("Consider optimizing ECS integration to reduce packet processing overhead".to_string());
            }
        }

        // Check for low memory efficiency
        if let Some(memory_efficiency) = self.results.iter()
            .find(|r| r.name.contains("memory_pool_efficiency")) {
            if memory_efficiency.value < 0.7 {
                recommendations.push("Memory pool efficiency is low - consider tuning pool sizes or allocation patterns".to_string());
            }
        }

        // Check for plugin performance impact
        if let Some(plugin_impact) = self.results.iter()
            .find(|r| r.name == "tick_rate_heavy_impact") {
            if plugin_impact.value > 2.0 {
                recommendations.push("Heavy plugins significantly impact tick rate - consider plugin optimization".to_string());
            }
        }

        // Check for regressions
        if let Some(regression_analysis) = &results.regression_analysis {
            if !regression_analysis.regressions.is_empty() {
                recommendations.push(format!(
                    "Address {} performance regressions before release",
                    regression_analysis.regressions.len()
                ));
            }
        }

        if recommendations.is_empty() {
            recommendations.push("All performance metrics are within acceptable ranges".to_string());
        }

        recommendations
    }

    /// Run benchmarks for CI environment
    pub fn run_ci_benchmarks() -> Result<bool, Box<dyn std::error::Error>> {
        let config = BenchmarkSuiteConfig {
            output_dir: "ci_benchmark_results".to_string(),
            baseline_path: "baseline_benchmarks.json".to_string(),
            fail_on_regression: true,
            ..Default::default()
        };

        let mut runner = BenchmarkSuiteRunner::new(config);
        let results = runner.run_all_benchmarks()?;

        // Generate CI-specific outputs
        if let Some(regression_analysis) = &results.regression_analysis {
            let github_output = CIIntegration::generate_github_actions_output(regression_analysis);
            println!("{}", github_output);
        }

        // Return success/failure for CI
        Ok(results.overall_status != BenchmarkStatus::Fail)
    }
}

/// Results from complete benchmark suite
#[derive(Debug, Clone)]
pub struct BenchmarkSuiteResults {
    pub total_duration: Duration,
    pub baseline_comparison: Option<Vec<BenchmarkResult>>,
    pub memory_benchmarks: Option<Vec<BenchmarkResult>>,
    pub ecs_overhead: Option<Vec<BenchmarkResult>>,
    pub plugin_performance: Option<Vec<BenchmarkResult>>,
    pub regression_analysis: Option<crate::benchmarks::regression_detection::RegressionAnalysis>,
    pub overall_status: BenchmarkStatus,
}

/// Overall benchmark status
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BenchmarkStatus {
    Pass,
    Warning,
    Fail,
}

/// Benchmark suite summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkSuiteSummary {
    pub total_benchmarks: usize,
    pub total_duration: Duration,
    pub overall_status: BenchmarkStatus,
    pub timestamp: u64,
    pub git_commit: Option<String>,
    pub system_info: SystemInfo,
}

/// Key performance metric
#[derive(Debug, Clone)]
struct KeyMetric {
    name: String,
    value: f64,
    unit: String,
    meets_threshold: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_benchmark_suite_runner() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().to_str().unwrap().to_string();
        
        let config = BenchmarkSuiteConfig {
            run_baseline_comparison: true,
            run_memory_benchmarks: false, // Skip to speed up test
            run_ecs_overhead: false,
            run_plugin_performance: false,
            run_regression_detection: false,
            output_dir,
            generate_reports: true,
            ..Default::default()
        };

        let mut runner = BenchmarkSuiteRunner::new(config);
        let results = runner.run_all_benchmarks().unwrap();
        
        assert!(results.baseline_comparison.is_some());
        assert!(results.total_duration > Duration::ZERO);
    }

    #[test]
    fn test_key_metrics_extraction() {
        let runner = BenchmarkSuiteRunner::new(BenchmarkSuiteConfig::default());
        let metrics = runner.extract_key_metrics();
        
        // Should handle empty results gracefully
        assert!(metrics.is_empty());
    }

    #[test]
    fn test_performance_highlights() {
        let mut runner = BenchmarkSuiteRunner::new(BenchmarkSuiteConfig::default());
        
        // Add some test results
        runner.results.push(BenchmarkResult {
            name: "test_fast_operation".to_string(),
            value: 0.001,
            unit: "seconds".to_string(),
            lower_is_better: true,
            metadata: create_test_metadata(),
        });

        let highlights = runner.generate_performance_highlights();
        assert!(!highlights.is_empty());
    }

    fn create_test_metadata() -> BenchmarkMetadata {
        BenchmarkMetadata {
            timestamp: 1234567890,
            git_commit: Some("test".to_string()),
            test_config: "test".to_string(),
            system_info: SystemInfo {
                os: "test".to_string(),
                arch: "test".to_string(),
                cpu_cores: 4,
                memory_gb: 8.0,
            },
        }
    }
}