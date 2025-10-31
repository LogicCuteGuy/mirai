//! Performance regression detection system
//!
//! Automated detection of performance regressions in CI pipeline

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Performance regression detector
pub struct RegressionDetector {
    baseline_path: String,
    threshold_config: RegressionThresholds,
}

/// Regression detection thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionThresholds {
    /// Maximum acceptable performance degradation (as ratio)
    pub max_degradation: f64,
    /// Minimum performance improvement to be considered significant
    pub min_improvement: f64,
    /// Number of consecutive regressions before alerting
    pub regression_count_threshold: usize,
    /// Specific thresholds for different benchmark categories
    pub category_thresholds: HashMap<String, f64>,
}

impl Default for RegressionThresholds {
    fn default() -> Self {
        let mut category_thresholds = HashMap::new();
        category_thresholds.insert("memory".to_string(), 0.10); // 10% for memory benchmarks
        category_thresholds.insert("packet_processing".to_string(), 0.05); // 5% for packet processing
        category_thresholds.insert("ecs".to_string(), 0.15); // 15% for ECS benchmarks
        category_thresholds.insert("plugin".to_string(), 0.20); // 20% for plugin benchmarks
        category_thresholds.insert("tick_rate".to_string(), 0.05); // 5% for tick rate
        
        Self {
            max_degradation: 0.10, // 10% default degradation threshold
            min_improvement: 0.05, // 5% minimum improvement
            regression_count_threshold: 3,
            category_thresholds,
        }
    }
}

/// Regression analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionAnalysis {
    pub total_benchmarks: usize,
    pub regressions: Vec<RegressionResult>,
    pub improvements: Vec<ImprovementResult>,
    pub stable: Vec<StableResult>,
    pub overall_status: RegressionStatus,
    pub analysis_timestamp: u64,
}

/// Individual regression result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionResult {
    pub benchmark_name: String,
    pub current_value: f64,
    pub baseline_value: f64,
    pub degradation_percent: f64,
    pub threshold_exceeded: bool,
    pub severity: RegressionSeverity,
    pub category: String,
}

/// Performance improvement result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementResult {
    pub benchmark_name: String,
    pub current_value: f64,
    pub baseline_value: f64,
    pub improvement_percent: f64,
    pub category: String,
}

/// Stable performance result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StableResult {
    pub benchmark_name: String,
    pub current_value: f64,
    pub baseline_value: f64,
    pub change_percent: f64,
    pub category: String,
}

/// Overall regression status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegressionStatus {
    Pass,
    Warning,
    Fail,
}

/// Regression severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegressionSeverity {
    Minor,
    Moderate,
    Severe,
    Critical,
}

impl RegressionDetector {
    /// Create a new regression detector
    pub fn new(baseline_path: String) -> Self {
        Self {
            baseline_path,
            threshold_config: RegressionThresholds::default(),
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(baseline_path: String, thresholds: RegressionThresholds) -> Self {
        Self {
            baseline_path,
            threshold_config: thresholds,
        }
    }

    /// Analyze current benchmark results against baseline
    pub fn analyze_regressions(&self, current_results: &[BenchmarkResult]) -> Result<RegressionAnalysis, Box<dyn std::error::Error>> {
        let baseline_results = self.load_baseline()?;
        
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();
        let mut stable = Vec::new();

        for current in current_results {
            if let Some(baseline) = baseline_results.iter().find(|b| b.name == current.name) {
                let analysis = self.analyze_single_benchmark(current, baseline);
                
                match analysis {
                    SingleBenchmarkAnalysis::Regression(reg) => regressions.push(reg),
                    SingleBenchmarkAnalysis::Improvement(imp) => improvements.push(imp),
                    SingleBenchmarkAnalysis::Stable(stable_result) => stable.push(stable_result),
                }
            }
        }

        let overall_status = self.determine_overall_status(&regressions);

        Ok(RegressionAnalysis {
            total_benchmarks: current_results.len(),
            regressions,
            improvements,
            stable,
            overall_status,
            analysis_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    /// Load baseline benchmark results
    fn load_baseline(&self) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
        if !Path::new(&self.baseline_path).exists() {
            return Ok(Vec::new()); // No baseline available
        }

        let content = fs::read_to_string(&self.baseline_path)?;
        let results: Vec<BenchmarkResult> = serde_json::from_str(&content)?;
        Ok(results)
    }

    /// Analyze a single benchmark against its baseline
    fn analyze_single_benchmark(&self, current: &BenchmarkResult, baseline: &BenchmarkResult) -> SingleBenchmarkAnalysis {
        let category = self.determine_category(&current.name);
        let threshold = self.get_threshold_for_category(&category);

        // Calculate percentage change
        let change_percent = if baseline.value == 0.0 {
            if current.value == 0.0 { 0.0 } else { f64::INFINITY }
        } else {
            (current.value - baseline.value) / baseline.value
        };

        // Determine if this is a regression, improvement, or stable
        let is_regression = if current.lower_is_better {
            change_percent > threshold
        } else {
            change_percent < -threshold
        };

        let is_improvement = if current.lower_is_better {
            change_percent < -self.threshold_config.min_improvement
        } else {
            change_percent > self.threshold_config.min_improvement
        };

        if is_regression {
            let severity = self.calculate_severity(change_percent.abs(), threshold);
            SingleBenchmarkAnalysis::Regression(RegressionResult {
                benchmark_name: current.name.clone(),
                current_value: current.value,
                baseline_value: baseline.value,
                degradation_percent: change_percent.abs() * 100.0,
                threshold_exceeded: change_percent.abs() > threshold,
                severity,
                category,
            })
        } else if is_improvement {
            SingleBenchmarkAnalysis::Improvement(ImprovementResult {
                benchmark_name: current.name.clone(),
                current_value: current.value,
                baseline_value: baseline.value,
                improvement_percent: change_percent.abs() * 100.0,
                category,
            })
        } else {
            SingleBenchmarkAnalysis::Stable(StableResult {
                benchmark_name: current.name.clone(),
                current_value: current.value,
                baseline_value: baseline.value,
                change_percent: change_percent * 100.0,
                category,
            })
        }
    }

    /// Determine benchmark category from name
    fn determine_category(&self, name: &str) -> String {
        if name.contains("memory") || name.contains("pool") || name.contains("allocation") {
            "memory".to_string()
        } else if name.contains("packet") || name.contains("network") {
            "packet_processing".to_string()
        } else if name.contains("ecs") || name.contains("entity") || name.contains("component") {
            "ecs".to_string()
        } else if name.contains("plugin") {
            "plugin".to_string()
        } else if name.contains("tick") {
            "tick_rate".to_string()
        } else {
            "general".to_string()
        }
    }

    /// Get threshold for specific category
    fn get_threshold_for_category(&self, category: &str) -> f64 {
        self.threshold_config.category_thresholds
            .get(category)
            .copied()
            .unwrap_or(self.threshold_config.max_degradation)
    }

    /// Calculate regression severity
    fn calculate_severity(&self, change_percent: f64, threshold: f64) -> RegressionSeverity {
        let severity_ratio = change_percent / threshold;
        
        if severity_ratio >= 5.0 {
            RegressionSeverity::Critical
        } else if severity_ratio >= 3.0 {
            RegressionSeverity::Severe
        } else if severity_ratio >= 2.0 {
            RegressionSeverity::Moderate
        } else {
            RegressionSeverity::Minor
        }
    }

    /// Determine overall regression status
    fn determine_overall_status(&self, regressions: &[RegressionResult]) -> RegressionStatus {
        let critical_count = regressions.iter().filter(|r| r.severity == RegressionSeverity::Critical).count();
        let severe_count = regressions.iter().filter(|r| r.severity == RegressionSeverity::Severe).count();
        let moderate_count = regressions.iter().filter(|r| r.severity == RegressionSeverity::Moderate).count();

        if critical_count > 0 || severe_count >= 2 {
            RegressionStatus::Fail
        } else if severe_count > 0 || moderate_count >= 3 {
            RegressionStatus::Warning
        } else {
            RegressionStatus::Pass
        }
    }

    /// Generate regression report
    pub fn generate_report(&self, analysis: &RegressionAnalysis) -> String {
        let mut report = Vec::new();
        
        report.push("# Performance Regression Analysis Report\n".to_string());
        
        // Summary
        report.push(format!("## Summary"));
        report.push(format!("- **Status**: {:?}", analysis.overall_status));
        report.push(format!("- **Total Benchmarks**: {}", analysis.total_benchmarks));
        report.push(format!("- **Regressions**: {}", analysis.regressions.len()));
        report.push(format!("- **Improvements**: {}", analysis.improvements.len()));
        report.push(format!("- **Stable**: {}", analysis.stable.len()));
        report.push("".to_string());

        // Regressions
        if !analysis.regressions.is_empty() {
            report.push("## ⚠️ Performance Regressions".to_string());
            
            for regression in &analysis.regressions {
                let severity_emoji = match regression.severity {
                    RegressionSeverity::Critical => "🔴",
                    RegressionSeverity::Severe => "🟠",
                    RegressionSeverity::Moderate => "🟡",
                    RegressionSeverity::Minor => "🟢",
                };
                
                report.push(format!(
                    "- {} **{}** ({:?}): {:.3} → {:.3} ({:+.1}%)",
                    severity_emoji,
                    regression.benchmark_name,
                    regression.severity,
                    regression.baseline_value,
                    regression.current_value,
                    regression.degradation_percent
                ));
            }
            report.push("".to_string());
        }

        // Improvements
        if !analysis.improvements.is_empty() {
            report.push("## ✅ Performance Improvements".to_string());
            
            for improvement in &analysis.improvements {
                report.push(format!(
                    "- **{}**: {:.3} → {:.3} ({:+.1}%)",
                    improvement.benchmark_name,
                    improvement.baseline_value,
                    improvement.current_value,
                    improvement.improvement_percent
                ));
            }
            report.push("".to_string());
        }

        // Category breakdown
        report.push("## 📊 Category Breakdown".to_string());
        let mut category_stats: HashMap<String, (usize, usize, usize)> = HashMap::new();
        
        for regression in &analysis.regressions {
            let entry = category_stats.entry(regression.category.clone()).or_insert((0, 0, 0));
            entry.0 += 1;
        }
        
        for improvement in &analysis.improvements {
            let entry = category_stats.entry(improvement.category.clone()).or_insert((0, 0, 0));
            entry.1 += 1;
        }
        
        for stable in &analysis.stable {
            let entry = category_stats.entry(stable.category.clone()).or_insert((0, 0, 0));
            entry.2 += 1;
        }

        for (category, (regressions, improvements, stable)) in category_stats {
            report.push(format!(
                "- **{}**: {} regressions, {} improvements, {} stable",
                category, regressions, improvements, stable
            ));
        }

        report.join("\n")
    }

    /// Save current results as new baseline
    pub fn update_baseline(&self, current_results: &[BenchmarkResult]) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(current_results)?;
        fs::write(&self.baseline_path, json)?;
        println!("Updated baseline: {}", self.baseline_path);
        Ok(())
    }

    /// Check if CI should fail based on regression analysis
    pub fn should_fail_ci(&self, analysis: &RegressionAnalysis) -> bool {
        analysis.overall_status == RegressionStatus::Fail
    }

    /// Get regression detection configuration
    pub fn get_config(&self) -> &RegressionThresholds {
        &self.threshold_config
    }
}

/// Internal enum for single benchmark analysis
enum SingleBenchmarkAnalysis {
    Regression(RegressionResult),
    Improvement(ImprovementResult),
    Stable(StableResult),
}

/// CI integration helper
pub struct CIIntegration;

impl CIIntegration {
    /// Run regression detection in CI environment
    pub fn run_ci_regression_check(
        current_results: &[BenchmarkResult],
        baseline_path: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let detector = RegressionDetector::new(baseline_path.to_string());
        let analysis = detector.analyze_regressions(current_results)?;
        
        // Generate and save report
        let report = detector.generate_report(&analysis);
        fs::write("regression_report.md", &report)?;
        
        // Print summary to console
        println!("{}", report);
        
        // Check if we should update baseline (if no regressions)
        if analysis.overall_status == RegressionStatus::Pass && std::env::var("UPDATE_BASELINE").is_ok() {
            detector.update_baseline(current_results)?;
        }
        
        // Return whether CI should pass
        Ok(!detector.should_fail_ci(&analysis))
    }

    /// Generate GitHub Actions output
    pub fn generate_github_actions_output(analysis: &RegressionAnalysis) -> String {
        let status = match analysis.overall_status {
            RegressionStatus::Pass => "success",
            RegressionStatus::Warning => "warning",
            RegressionStatus::Fail => "failure",
        };
        
        format!(
            "::set-output name=status::{}\n::set-output name=regressions::{}\n::set-output name=improvements::{}",
            status,
            analysis.regressions.len(),
            analysis.improvements.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_regression_detection() {
        let temp_file = NamedTempFile::new().unwrap();
        let baseline_path = temp_file.path().to_str().unwrap().to_string();
        
        // Create baseline results
        let baseline_results = vec![
            BenchmarkResult {
                name: "test_benchmark".to_string(),
                value: 1.0,
                unit: "seconds".to_string(),
                lower_is_better: true,
                metadata: create_test_metadata(),
            },
        ];
        
        let detector = RegressionDetector::new(baseline_path.clone());
        detector.update_baseline(&baseline_results).unwrap();
        
        // Create current results with regression
        let current_results = vec![
            BenchmarkResult {
                name: "test_benchmark".to_string(),
                value: 1.2, // 20% slower
                unit: "seconds".to_string(),
                lower_is_better: true,
                metadata: create_test_metadata(),
            },
        ];
        
        let analysis = detector.analyze_regressions(&current_results).unwrap();
        
        assert_eq!(analysis.regressions.len(), 1);
        assert_eq!(analysis.improvements.len(), 0);
        assert_eq!(analysis.stable.len(), 0);
        assert_eq!(analysis.overall_status, RegressionStatus::Warning);
    }

    #[test]
    fn test_improvement_detection() {
        let temp_file = NamedTempFile::new().unwrap();
        let baseline_path = temp_file.path().to_str().unwrap().to_string();
        
        let baseline_results = vec![
            BenchmarkResult {
                name: "test_benchmark".to_string(),
                value: 1.0,
                unit: "seconds".to_string(),
                lower_is_better: true,
                metadata: create_test_metadata(),
            },
        ];
        
        let detector = RegressionDetector::new(baseline_path);
        detector.update_baseline(&baseline_results).unwrap();
        
        // Create current results with improvement
        let current_results = vec![
            BenchmarkResult {
                name: "test_benchmark".to_string(),
                value: 0.8, // 20% faster
                unit: "seconds".to_string(),
                lower_is_better: true,
                metadata: create_test_metadata(),
            },
        ];
        
        let analysis = detector.analyze_regressions(&current_results).unwrap();
        
        assert_eq!(analysis.regressions.len(), 0);
        assert_eq!(analysis.improvements.len(), 1);
        assert_eq!(analysis.stable.len(), 0);
        assert_eq!(analysis.overall_status, RegressionStatus::Pass);
    }

    #[test]
    fn test_category_thresholds() {
        let mut thresholds = RegressionThresholds::default();
        thresholds.category_thresholds.insert("memory".to_string(), 0.05); // 5% for memory
        
        let detector = RegressionDetector::with_thresholds("test".to_string(), thresholds);
        
        assert_eq!(detector.get_threshold_for_category("memory"), 0.05);
        assert_eq!(detector.get_threshold_for_category("unknown"), 0.10); // default
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