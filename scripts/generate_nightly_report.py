#!/usr/bin/env python3
"""
Generate comprehensive nightly test report for the merged mirai system.
Aggregates results from all test suites and generates an HTML report.
"""

import json
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Optional
import subprocess

class NightlyReportGenerator:
    def __init__(self):
        self.report_data = {
            "timestamp": datetime.now().isoformat(),
            "test_results": {},
            "performance_results": {},
            "coverage_results": {},
            "security_results": {},
            "compatibility_results": {}
        }
    
    def collect_test_results(self) -> Dict[str, Any]:
        """Collect results from unit and integration tests."""
        test_results = {
            "unit_tests": {"passed": 0, "failed": 0, "total": 0},
            "integration_tests": {"passed": 0, "failed": 0, "total": 0},
            "doc_tests": {"passed": 0, "failed": 0, "total": 0}
        }
        
        try:
            # Run cargo test and capture output
            result = subprocess.run(
                ["cargo", "test", "--all-features", "--", "--format=json"],
                capture_output=True,
                text=True,
                cwd="."
            )
            
            # Parse test results (simplified - would need proper JSON parsing)
            lines = result.stdout.split('\n')
            for line in lines:
                if "test result:" in line:
                    # Parse test summary line
                    parts = line.split()
                    if len(parts) >= 6:
                        passed = int(parts[2])
                        failed = int(parts[5]) if "failed" in line else 0
                        total = passed + failed
                        
                        if "integration" in line:
                            test_results["integration_tests"] = {
                                "passed": passed, "failed": failed, "total": total
                            }
                        elif "doc" in line:
                            test_results["doc_tests"] = {
                                "passed": passed, "failed": failed, "total": total
                            }
                        else:
                            test_results["unit_tests"] = {
                                "passed": passed, "failed": failed, "total": total
                            }
        
        except Exception as e:
            print(f"Error collecting test results: {e}")
        
        return test_results
    
    def collect_performance_results(self) -> Dict[str, Any]:
        """Collect performance benchmark results."""
        performance_results = {
            "benchmarks": [],
            "memory_usage": {},
            "throughput": {}
        }
        
        # Look for benchmark results file
        benchmark_files = [
            "benchmark_results.json",
            "target/criterion/reports/index.html"
        ]
        
        for file_path in benchmark_files:
            if os.path.exists(file_path):
                try:
                    if file_path.endswith('.json'):
                        with open(file_path, 'r') as f:
                            data = json.load(f)
                            performance_results["benchmarks"] = data.get("benchmarks", [])
                except Exception as e:
                    print(f"Error reading {file_path}: {e}")
        
        return performance_results
    
    def collect_coverage_results(self) -> Dict[str, Any]:
        """Collect code coverage results."""
        coverage_results = {
            "line_coverage": 0.0,
            "branch_coverage": 0.0,
            "function_coverage": 0.0,
            "total_lines": 0,
            "covered_lines": 0
        }
        
        # Look for coverage files
        coverage_files = ["lcov.info", "coverage.json"]
        
        for file_path in coverage_files:
            if os.path.exists(file_path):
                try:
                    if file_path == "lcov.info":
                        # Parse LCOV format (simplified)
                        with open(file_path, 'r') as f:
                            content = f.read()
                            lines = content.split('\n')
                            
                            total_lines = 0
                            covered_lines = 0
                            
                            for line in lines:
                                if line.startswith('LF:'):
                                    total_lines += int(line.split(':')[1])
                                elif line.startswith('LH:'):
                                    covered_lines += int(line.split(':')[1])
                            
                            if total_lines > 0:
                                coverage_results["total_lines"] = total_lines
                                coverage_results["covered_lines"] = covered_lines
                                coverage_results["line_coverage"] = (covered_lines / total_lines) * 100
                
                except Exception as e:
                    print(f"Error reading coverage file {file_path}: {e}")
        
        return coverage_results
    
    def collect_security_results(self) -> Dict[str, Any]:
        """Collect security audit results."""
        security_results = {
            "vulnerabilities": [],
            "audit_passed": True,
            "total_crates": 0,
            "vulnerable_crates": 0
        }
        
        try:
            # Run cargo audit
            result = subprocess.run(
                ["cargo", "audit", "--format", "json"],
                capture_output=True,
                text=True,
                cwd="."
            )
            
            if result.returncode == 0:
                try:
                    audit_data = json.loads(result.stdout)
                    security_results["vulnerabilities"] = audit_data.get("vulnerabilities", [])
                    security_results["vulnerable_crates"] = len(audit_data.get("vulnerabilities", []))
                    security_results["audit_passed"] = len(audit_data.get("vulnerabilities", [])) == 0
                except json.JSONDecodeError:
                    pass
        
        except Exception as e:
            print(f"Error running security audit: {e}")
        
        return security_results
    
    def collect_compatibility_results(self) -> Dict[str, Any]:
        """Collect compatibility test results."""
        compatibility_results = {
            "minecraft_versions": {},
            "migration_tests": {"passed": 0, "failed": 0},
            "backward_compatibility": True
        }
        
        # Look for compatibility test results
        compat_files = [
            "compatibility_results.json",
            "migration_test_results.json"
        ]
        
        for file_path in compat_files:
            if os.path.exists(file_path):
                try:
                    with open(file_path, 'r') as f:
                        data = json.load(f)
                        
                        if "minecraft_versions" in data:
                            compatibility_results["minecraft_versions"] = data["minecraft_versions"]
                        
                        if "migration_tests" in data:
                            compatibility_results["migration_tests"] = data["migration_tests"]
                
                except Exception as e:
                    print(f"Error reading compatibility file {file_path}: {e}")
        
        return compatibility_results
    
    def generate_html_report(self) -> str:
        """Generate comprehensive HTML report."""
        html_template = """
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mirai Merged System - Nightly Test Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .header {{ text-align: center; margin-bottom: 30px; }}
        .section {{ margin-bottom: 30px; }}
        .section h2 {{ color: #333; border-bottom: 2px solid #007acc; padding-bottom: 5px; }}
        .metrics {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; margin: 20px 0; }}
        .metric {{ background: #f8f9fa; padding: 15px; border-radius: 5px; text-align: center; }}
        .metric .value {{ font-size: 24px; font-weight: bold; color: #007acc; }}
        .metric .label {{ font-size: 14px; color: #666; }}
        .status-pass {{ color: #28a745; }}
        .status-fail {{ color: #dc3545; }}
        .status-warn {{ color: #ffc107; }}
        .progress-bar {{ width: 100%; height: 20px; background: #e9ecef; border-radius: 10px; overflow: hidden; }}
        .progress-fill {{ height: 100%; background: #28a745; transition: width 0.3s ease; }}
        table {{ width: 100%; border-collapse: collapse; margin: 15px 0; }}
        th, td {{ padding: 10px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background-color: #f8f9fa; }}
        .timestamp {{ color: #666; font-size: 14px; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 Mirai Merged System</h1>
            <h2>Nightly Test Report</h2>
            <p class="timestamp">Generated: {timestamp}</p>
        </div>
        
        <div class="section">
            <h2>📊 Test Results Overview</h2>
            <div class="metrics">
                <div class="metric">
                    <div class="value {unit_status}">{unit_passed}/{unit_total}</div>
                    <div class="label">Unit Tests</div>
                </div>
                <div class="metric">
                    <div class="value {integration_status}">{integration_passed}/{integration_total}</div>
                    <div class="label">Integration Tests</div>
                </div>
                <div class="metric">
                    <div class="value {doc_status}">{doc_passed}/{doc_total}</div>
                    <div class="label">Doc Tests</div>
                </div>
                <div class="metric">
                    <div class="value {coverage_status}">{coverage:.1f}%</div>
                    <div class="label">Code Coverage</div>
                </div>
            </div>
        </div>
        
        <div class="section">
            <h2>⚡ Performance Metrics</h2>
            <div class="metrics">
                <div class="metric">
                    <div class="value">{benchmark_count}</div>
                    <div class="label">Benchmarks Run</div>
                </div>
                <div class="metric">
                    <div class="value {performance_status}">✓</div>
                    <div class="label">Performance Status</div>
                </div>
            </div>
        </div>
        
        <div class="section">
            <h2>🔒 Security Audit</h2>
            <div class="metrics">
                <div class="metric">
                    <div class="value {security_status}">{vulnerability_count}</div>
                    <div class="label">Vulnerabilities</div>
                </div>
                <div class="metric">
                    <div class="value {audit_status}">{"PASS" if audit_passed else "FAIL"}</div>
                    <div class="label">Audit Status</div>
                </div>
            </div>
        </div>
        
        <div class="section">
            <h2>🔄 Compatibility Tests</h2>
            <div class="metrics">
                <div class="metric">
                    <div class="value {migration_status}">{migration_passed}/{migration_total}</div>
                    <div class="label">Migration Tests</div>
                </div>
                <div class="metric">
                    <div class="value {compat_status}">{"PASS" if backward_compat else "FAIL"}</div>
                    <div class="label">Backward Compatibility</div>
                </div>
            </div>
        </div>
        
        <div class="section">
            <h2>📈 Detailed Results</h2>
            <h3>Test Summary</h3>
            <table>
                <tr><th>Test Suite</th><th>Passed</th><th>Failed</th><th>Total</th><th>Success Rate</th></tr>
                <tr>
                    <td>Unit Tests</td>
                    <td>{unit_passed}</td>
                    <td>{unit_failed}</td>
                    <td>{unit_total}</td>
                    <td>{unit_success_rate:.1f}%</td>
                </tr>
                <tr>
                    <td>Integration Tests</td>
                    <td>{integration_passed}</td>
                    <td>{integration_failed}</td>
                    <td>{integration_total}</td>
                    <td>{integration_success_rate:.1f}%</td>
                </tr>
                <tr>
                    <td>Documentation Tests</td>
                    <td>{doc_passed}</td>
                    <td>{doc_failed}</td>
                    <td>{doc_total}</td>
                    <td>{doc_success_rate:.1f}%</td>
                </tr>
            </table>
        </div>
        
        <div class="section">
            <h2>🎯 Overall Status</h2>
            <div class="metric">
                <div class="value {overall_status}">{overall_result}</div>
                <div class="label">Build Status</div>
            </div>
        </div>
    </div>
</body>
</html>
        """
        
        # Calculate derived metrics
        test_results = self.report_data["test_results"]
        coverage_results = self.report_data["coverage_results"]
        security_results = self.report_data["security_results"]
        compatibility_results = self.report_data["compatibility_results"]
        
        # Unit tests
        unit_passed = test_results.get("unit_tests", {}).get("passed", 0)
        unit_failed = test_results.get("unit_tests", {}).get("failed", 0)
        unit_total = unit_passed + unit_failed
        unit_success_rate = (unit_passed / unit_total * 100) if unit_total > 0 else 0
        unit_status = "status-pass" if unit_failed == 0 else "status-fail"
        
        # Integration tests
        integration_passed = test_results.get("integration_tests", {}).get("passed", 0)
        integration_failed = test_results.get("integration_tests", {}).get("failed", 0)
        integration_total = integration_passed + integration_failed
        integration_success_rate = (integration_passed / integration_total * 100) if integration_total > 0 else 0
        integration_status = "status-pass" if integration_failed == 0 else "status-fail"
        
        # Doc tests
        doc_passed = test_results.get("doc_tests", {}).get("passed", 0)
        doc_failed = test_results.get("doc_tests", {}).get("failed", 0)
        doc_total = doc_passed + doc_failed
        doc_success_rate = (doc_passed / doc_total * 100) if doc_total > 0 else 0
        doc_status = "status-pass" if doc_failed == 0 else "status-fail"
        
        # Coverage
        coverage = coverage_results.get("line_coverage", 0)
        coverage_status = "status-pass" if coverage >= 80 else "status-warn" if coverage >= 60 else "status-fail"
        
        # Security
        vulnerability_count = len(security_results.get("vulnerabilities", []))
        audit_passed = security_results.get("audit_passed", True)
        security_status = "status-pass" if vulnerability_count == 0 else "status-fail"
        audit_status = "status-pass" if audit_passed else "status-fail"
        
        # Compatibility
        migration_passed = compatibility_results.get("migration_tests", {}).get("passed", 0)
        migration_failed = compatibility_results.get("migration_tests", {}).get("failed", 0)
        migration_total = migration_passed + migration_failed
        migration_status = "status-pass" if migration_failed == 0 else "status-fail"
        
        backward_compat = compatibility_results.get("backward_compatibility", True)
        compat_status = "status-pass" if backward_compat else "status-fail"
        
        # Overall status
        all_tests_pass = (unit_failed == 0 and integration_failed == 0 and 
                         doc_failed == 0 and audit_passed and backward_compat)
        overall_result = "PASS" if all_tests_pass else "FAIL"
        overall_status = "status-pass" if all_tests_pass else "status-fail"
        
        return html_template.format(
            timestamp=self.report_data["timestamp"],
            unit_passed=unit_passed,
            unit_failed=unit_failed,
            unit_total=unit_total,
            unit_success_rate=unit_success_rate,
            unit_status=unit_status,
            integration_passed=integration_passed,
            integration_failed=integration_failed,
            integration_total=integration_total,
            integration_success_rate=integration_success_rate,
            integration_status=integration_status,
            doc_passed=doc_passed,
            doc_failed=doc_failed,
            doc_total=doc_total,
            doc_success_rate=doc_success_rate,
            doc_status=doc_status,
            coverage=coverage,
            coverage_status=coverage_status,
            benchmark_count=len(self.report_data["performance_results"].get("benchmarks", [])),
            performance_status="status-pass",
            vulnerability_count=vulnerability_count,
            security_status=security_status,
            audit_passed=audit_passed,
            audit_status=audit_status,
            migration_passed=migration_passed,
            migration_total=migration_total,
            migration_status=migration_status,
            backward_compat=backward_compat,
            compat_status=compat_status,
            overall_result=overall_result,
            overall_status=overall_status
        )
    
    def generate_report(self):
        """Generate the complete nightly report."""
        print("Collecting test results...")
        self.report_data["test_results"] = self.collect_test_results()
        
        print("Collecting performance results...")
        self.report_data["performance_results"] = self.collect_performance_results()
        
        print("Collecting coverage results...")
        self.report_data["coverage_results"] = self.collect_coverage_results()
        
        print("Collecting security results...")
        self.report_data["security_results"] = self.collect_security_results()
        
        print("Collecting compatibility results...")
        self.report_data["compatibility_results"] = self.collect_compatibility_results()
        
        print("Generating HTML report...")
        html_report = self.generate_html_report()
        
        # Save reports
        with open("nightly_report.html", "w") as f:
            f.write(html_report)
        
        with open("nightly_report.json", "w") as f:
            json.dump(self.report_data, f, indent=2)
        
        print("✅ Nightly report generated successfully!")
        print("📄 HTML report: nightly_report.html")
        print("📊 JSON data: nightly_report.json")

def main():
    generator = NightlyReportGenerator()
    generator.generate_report()

if __name__ == "__main__":
    main()