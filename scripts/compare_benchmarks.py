#!/usr/bin/env python3
"""
Benchmark comparison script for the merged mirai system.
Compares current benchmark results with baseline performance.
"""

import json
import sys
import os
from typing import Dict, List, Any
from dataclasses import dataclass
from pathlib import Path

@dataclass
class BenchmarkResult:
    name: str
    value: float
    unit: str
    lower_is_better: bool = True

@dataclass
class ComparisonResult:
    name: str
    current: float
    baseline: float
    change_percent: float
    regression: bool
    improvement: bool

class BenchmarkComparator:
    def __init__(self, baseline_path: str = "baseline_benchmarks.json"):
        self.baseline_path = baseline_path
        self.regression_threshold = 0.05  # 5% regression threshold
        self.improvement_threshold = 0.05  # 5% improvement threshold
    
    def load_baseline(self) -> Dict[str, BenchmarkResult]:
        """Load baseline benchmark results."""
        if not os.path.exists(self.baseline_path):
            print(f"Warning: Baseline file {self.baseline_path} not found")
            return {}
        
        with open(self.baseline_path, 'r') as f:
            data = json.load(f)
        
        baseline = {}
        for bench in data.get('benchmarks', []):
            result = BenchmarkResult(
                name=bench['name'],
                value=bench['value'],
                unit=bench['unit'],
                lower_is_better=bench.get('lower_is_better', True)
            )
            baseline[result.name] = result
        
        return baseline
    
    def load_current(self, current_path: str) -> Dict[str, BenchmarkResult]:
        """Load current benchmark results."""
        with open(current_path, 'r') as f:
            data = json.load(f)
        
        current = {}
        for bench in data.get('benchmarks', []):
            result = BenchmarkResult(
                name=bench['name'],
                value=bench['value'],
                unit=bench['unit'],
                lower_is_better=bench.get('lower_is_better', True)
            )
            current[result.name] = result
        
        return current
    
    def compare_benchmarks(self, current: Dict[str, BenchmarkResult], 
                          baseline: Dict[str, BenchmarkResult]) -> List[ComparisonResult]:
        """Compare current benchmarks with baseline."""
        comparisons = []
        
        for name, current_result in current.items():
            if name not in baseline:
                print(f"Warning: No baseline for benchmark '{name}'")
                continue
            
            baseline_result = baseline[name]
            
            # Calculate percentage change
            if baseline_result.value == 0:
                change_percent = float('inf') if current_result.value > 0 else 0
            else:
                change_percent = (current_result.value - baseline_result.value) / baseline_result.value
            
            # Determine if this is a regression or improvement
            if current_result.lower_is_better:
                regression = change_percent > self.regression_threshold
                improvement = change_percent < -self.improvement_threshold
            else:
                regression = change_percent < -self.regression_threshold
                improvement = change_percent > self.improvement_threshold
            
            comparison = ComparisonResult(
                name=name,
                current=current_result.value,
                baseline=baseline_result.value,
                change_percent=change_percent * 100,
                regression=regression,
                improvement=improvement
            )
            
            comparisons.append(comparison)
        
        return comparisons
    
    def generate_report(self, comparisons: List[ComparisonResult]) -> str:
        """Generate a human-readable comparison report."""
        report = ["# Benchmark Comparison Report\n"]
        
        regressions = [c for c in comparisons if c.regression]
        improvements = [c for c in comparisons if c.improvement]
        stable = [c for c in comparisons if not c.regression and not c.improvement]
        
        report.append(f"## Summary")
        report.append(f"- **Total benchmarks**: {len(comparisons)}")
        report.append(f"- **Regressions**: {len(regressions)}")
        report.append(f"- **Improvements**: {len(improvements)}")
        report.append(f"- **Stable**: {len(stable)}\n")
        
        if regressions:
            report.append("## ⚠️ Performance Regressions")
            for comp in regressions:
                report.append(f"- **{comp.name}**: {comp.current:.3f} vs {comp.baseline:.3f} "
                            f"({comp.change_percent:+.1f}%)")
            report.append("")
        
        if improvements:
            report.append("## ✅ Performance Improvements")
            for comp in improvements:
                report.append(f"- **{comp.name}**: {comp.current:.3f} vs {comp.baseline:.3f} "
                            f"({comp.change_percent:+.1f}%)")
            report.append("")
        
        if stable:
            report.append("## 📊 Stable Performance")
            for comp in stable:
                report.append(f"- **{comp.name}**: {comp.current:.3f} vs {comp.baseline:.3f} "
                            f"({comp.change_percent:+.1f}%)")
            report.append("")
        
        return "\n".join(report)
    
    def save_current_as_baseline(self, current_path: str):
        """Save current results as new baseline."""
        import shutil
        shutil.copy2(current_path, self.baseline_path)
        print(f"Saved current results as new baseline: {self.baseline_path}")

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 compare_benchmarks.py <current_results.json> [--update-baseline]")
        sys.exit(1)
    
    current_path = sys.argv[1]
    update_baseline = "--update-baseline" in sys.argv
    
    comparator = BenchmarkComparator()
    
    try:
        current = comparator.load_current(current_path)
        baseline = comparator.load_baseline()
        
        if not baseline:
            print("No baseline found. Saving current results as baseline.")
            comparator.save_current_as_baseline(current_path)
            return
        
        comparisons = comparator.compare_benchmarks(current, baseline)
        report = comparator.generate_report(comparisons)
        
        print(report)
        
        # Save report to file
        with open("benchmark_report.md", "w") as f:
            f.write(report)
        
        # Check for regressions
        regressions = [c for c in comparisons if c.regression]
        if regressions:
            print(f"\n❌ Found {len(regressions)} performance regressions!")
            if not update_baseline:
                sys.exit(1)
        
        if update_baseline:
            comparator.save_current_as_baseline(current_path)
        
        print("\n✅ Benchmark comparison completed successfully")
        
    except Exception as e:
        print(f"Error comparing benchmarks: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()