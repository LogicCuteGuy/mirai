#!/usr/bin/env python3
"""
Generate benchmark comparison charts for performance analysis.
"""

import json
import argparse
import sys
import os
from pathlib import Path
from typing import Dict, List, Any, Optional
import matplotlib.pyplot as plt
import seaborn as sns
import pandas as pd
import numpy as np

def load_benchmark_results(file_path: str) -> List[Dict[str, Any]]:
    """Load benchmark results from JSON file."""
    try:
        with open(file_path, 'r') as f:
            return json.load(f)
    except FileNotFoundError:
        print(f"Warning: Benchmark file {file_path} not found")
        return []
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON file {file_path}: {e}")
        return []

def create_performance_comparison_chart(current_data: List[Dict], baseline_data: List[Dict], output_dir: str):
    """Create performance comparison chart between current and baseline."""
    
    # Create baseline lookup
    baseline_lookup = {item['name']: item['value'] for item in baseline_data}
    
    # Prepare data for comparison
    comparison_data = []
    for current in current_data:
        name = current['name']
        current_value = current['value']
        baseline_value = baseline_lookup.get(name)
        
        if baseline_value is not None:
            # Calculate percentage change
            if baseline_value != 0:
                change_percent = ((current_value - baseline_value) / baseline_value) * 100
            else:
                change_percent = 0 if current_value == 0 else float('inf')
            
            comparison_data.append({
                'benchmark': name,
                'current': current_value,
                'baseline': baseline_value,
                'change_percent': change_percent,
                'unit': current['unit'],
                'lower_is_better': current['lower_is_better'],
                'category': categorize_benchmark(name)
            })
    
    if not comparison_data:
        print("No matching benchmarks found for comparison")
        return
    
    df = pd.DataFrame(comparison_data)
    
    # Create performance comparison chart
    plt.figure(figsize=(15, 10))
    
    # Sort by change percentage for better visualization
    df_sorted = df.sort_values('change_percent')
    
    # Color code based on performance change
    colors = []
    for _, row in df_sorted.iterrows():
        if row['lower_is_better']:
            # For lower-is-better metrics, negative change is good (green), positive is bad (red)
            if row['change_percent'] < -5:
                colors.append('green')
            elif row['change_percent'] > 5:
                colors.append('red')
            else:
                colors.append('orange')
        else:
            # For higher-is-better metrics, positive change is good (green), negative is bad (red)
            if row['change_percent'] > 5:
                colors.append('green')
            elif row['change_percent'] < -5:
                colors.append('red')
            else:
                colors.append('orange')
    
    bars = plt.barh(range(len(df_sorted)), df_sorted['change_percent'], color=colors, alpha=0.7)
    
    plt.yticks(range(len(df_sorted)), df_sorted['benchmark'], fontsize=8)
    plt.xlabel('Performance Change (%)')
    plt.title('Performance Comparison: Current vs Baseline')
    plt.axvline(x=0, color='black', linestyle='-', alpha=0.3)
    plt.axvline(x=5, color='red', linestyle='--', alpha=0.5, label='5% threshold')
    plt.axvline(x=-5, color='red', linestyle='--', alpha=0.5)
    
    # Add value labels on bars
    for i, (bar, row) in enumerate(zip(bars, df_sorted.itertuples())):
        width = bar.get_width()
        plt.text(width + (1 if width >= 0 else -1), bar.get_y() + bar.get_height()/2, 
                f'{width:.1f}%', ha='left' if width >= 0 else 'right', va='center', fontsize=7)
    
    plt.legend()
    plt.tight_layout()
    plt.savefig(f'{output_dir}/performance_comparison.png', dpi=300, bbox_inches='tight')
    plt.close()

def create_category_breakdown_chart(data: List[Dict], output_dir: str):
    """Create category breakdown chart."""
    
    # Categorize benchmarks
    categories = {}
    for item in data:
        category = categorize_benchmark(item['name'])
        if category not in categories:
            categories[category] = []
        categories[category].append(item)
    
    # Create subplots for each category
    fig, axes = plt.subplots(2, 3, figsize=(18, 12))
    axes = axes.flatten()
    
    category_names = list(categories.keys())
    
    for i, category in enumerate(category_names[:6]):  # Limit to 6 categories
        if i >= len(axes):
            break
            
        ax = axes[i]
        category_data = categories[category]
        
        # Create bar chart for this category
        names = [item['name'].replace(f'{category}_', '').replace('_', ' ') for item in category_data]
        values = [item['value'] for item in category_data]
        
        bars = ax.bar(range(len(names)), values, alpha=0.7)
        ax.set_xticks(range(len(names)))
        ax.set_xticklabels(names, rotation=45, ha='right', fontsize=8)
        ax.set_title(f'{category.title()} Benchmarks')
        ax.set_ylabel('Value')
        
        # Add value labels on bars
        for bar, value in zip(bars, values):
            height = bar.get_height()
            ax.text(bar.get_x() + bar.get_width()/2., height,
                   f'{value:.3f}', ha='center', va='bottom', fontsize=7)
    
    # Hide unused subplots
    for i in range(len(category_names), len(axes)):
        axes[i].set_visible(False)
    
    plt.tight_layout()
    plt.savefig(f'{output_dir}/category_breakdown.png', dpi=300, bbox_inches='tight')
    plt.close()

def create_performance_trends_chart(data: List[Dict], output_dir: str):
    """Create performance trends chart (simulated historical data)."""
    
    # For demonstration, create simulated trend data
    # In a real implementation, this would use historical benchmark data
    
    key_benchmarks = [
        'server_startup_avg',
        'packet_processing_ecs_overhead', 
        'memory_pool_efficiency',
        'tick_rate_lightweight_impact'
    ]
    
    # Filter to key benchmarks
    key_data = [item for item in data if item['name'] in key_benchmarks]
    
    if not key_data:
        print("No key benchmarks found for trends chart")
        return
    
    # Simulate historical data (in real implementation, load from database)
    dates = pd.date_range(end=pd.Timestamp.now(), periods=30, freq='D')
    
    plt.figure(figsize=(14, 8))
    
    for item in key_data:
        # Simulate trend data around current value
        current_value = item['value']
        trend_data = np.random.normal(current_value, current_value * 0.1, len(dates))
        
        # Add some realistic trend (slight degradation over time)
        trend_factor = np.linspace(0.95, 1.05, len(dates))
        trend_data = trend_data * trend_factor
        
        plt.plot(dates, trend_data, marker='o', markersize=3, label=item['name'], alpha=0.7)
    
    plt.xlabel('Date')
    plt.ylabel('Performance Value')
    plt.title('Performance Trends (Last 30 Days)')
    plt.legend()
    plt.xticks(rotation=45)
    plt.grid(True, alpha=0.3)
    plt.tight_layout()
    plt.savefig(f'{output_dir}/performance_trends.png', dpi=300, bbox_inches='tight')
    plt.close()

def create_memory_analysis_chart(data: List[Dict], output_dir: str):
    """Create memory-specific analysis chart."""
    
    memory_benchmarks = [item for item in data if 'memory' in item['name'].lower() or 'pool' in item['name'].lower()]
    
    if not memory_benchmarks:
        print("No memory benchmarks found")
        return
    
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 6))
    
    # Memory efficiency chart
    efficiency_data = [item for item in memory_benchmarks if 'efficiency' in item['name']]
    if efficiency_data:
        names = [item['name'].replace('memory_', '').replace('_', ' ') for item in efficiency_data]
        values = [item['value'] * 100 for item in efficiency_data]  # Convert to percentage
        
        bars = ax1.bar(names, values, color='skyblue', alpha=0.7)
        ax1.set_title('Memory Pool Efficiency')
        ax1.set_ylabel('Efficiency (%)')
        ax1.set_ylim(0, 100)
        
        # Add efficiency threshold line
        ax1.axhline(y=80, color='green', linestyle='--', alpha=0.7, label='Target (80%)')
        ax1.axhline(y=60, color='orange', linestyle='--', alpha=0.7, label='Warning (60%)')
        
        for bar, value in zip(bars, values):
            height = bar.get_height()
            ax1.text(bar.get_x() + bar.get_width()/2., height + 1,
                    f'{value:.1f}%', ha='center', va='bottom')
        
        ax1.legend()
        ax1.tick_params(axis='x', rotation=45)
    
    # Memory allocation performance
    allocation_data = [item for item in memory_benchmarks if 'allocation' in item['name'] and 'seconds' in item['unit']]
    if allocation_data:
        names = [item['name'].replace('memory_', '').replace('_', ' ') for item in allocation_data]
        values = [item['value'] * 1000 for item in allocation_data]  # Convert to milliseconds
        
        bars = ax2.bar(names, values, color='lightcoral', alpha=0.7)
        ax2.set_title('Memory Allocation Performance')
        ax2.set_ylabel('Time (ms)')
        
        for bar, value in zip(bars, values):
            height = bar.get_height()
            ax2.text(bar.get_x() + bar.get_width()/2., height + max(values) * 0.01,
                    f'{value:.2f}ms', ha='center', va='bottom', fontsize=8)
        
        ax2.tick_params(axis='x', rotation=45)
    
    plt.tight_layout()
    plt.savefig(f'{output_dir}/memory_analysis.png', dpi=300, bbox_inches='tight')
    plt.close()

def categorize_benchmark(name: str) -> str:
    """Categorize benchmark by name."""
    name_lower = name.lower()
    
    if 'memory' in name_lower or 'pool' in name_lower or 'allocation' in name_lower:
        return 'memory'
    elif 'packet' in name_lower or 'network' in name_lower:
        return 'network'
    elif 'ecs' in name_lower or 'entity' in name_lower or 'component' in name_lower:
        return 'ecs'
    elif 'plugin' in name_lower:
        return 'plugin'
    elif 'tick' in name_lower:
        return 'tick_rate'
    elif 'server' in name_lower or 'startup' in name_lower:
        return 'server'
    else:
        return 'general'

def generate_summary_report(current_data: List[Dict], baseline_data: List[Dict], output_dir: str):
    """Generate a summary report with key metrics."""
    
    baseline_lookup = {item['name']: item for item in baseline_data}
    
    # Key metrics to highlight
    key_metrics = [
        'server_startup_avg',
        'packet_processing_ecs_overhead',
        'memory_pool_efficiency', 
        'tick_rate_lightweight_impact',
        'sustained_tick_rate_with_plugins'
    ]
    
    report_lines = [
        "# Benchmark Summary Report\n",
        f"Generated: {pd.Timestamp.now().strftime('%Y-%m-%d %H:%M:%S')}\n",
        f"Total benchmarks: {len(current_data)}\n",
    ]
    
    if baseline_data:
        report_lines.append(f"Baseline benchmarks: {len(baseline_data)}\n")
        
        # Calculate summary statistics
        regressions = 0
        improvements = 0
        stable = 0
        
        for current in current_data:
            baseline = baseline_lookup.get(current['name'])
            if baseline:
                if baseline['value'] != 0:
                    change = (current['value'] - baseline['value']) / baseline['value']
                    if current['lower_is_better']:
                        if change > 0.05:
                            regressions += 1
                        elif change < -0.05:
                            improvements += 1
                        else:
                            stable += 1
                    else:
                        if change < -0.05:
                            regressions += 1
                        elif change > 0.05:
                            improvements += 1
                        else:
                            stable += 1
        
        report_lines.extend([
            f"\n## Performance Changes\n",
            f"- Regressions: {regressions}\n",
            f"- Improvements: {improvements}\n", 
            f"- Stable: {stable}\n",
        ])
    
    # Key metrics section
    report_lines.append("\n## Key Metrics\n")
    
    for metric_name in key_metrics:
        current_metric = next((item for item in current_data if item['name'] == metric_name), None)
        if current_metric:
            baseline_metric = baseline_lookup.get(metric_name)
            
            if baseline_metric:
                change = ((current_metric['value'] - baseline_metric['value']) / baseline_metric['value']) * 100
                change_str = f" ({change:+.1f}% vs baseline)"
            else:
                change_str = ""
            
            report_lines.append(f"- **{metric_name}**: {current_metric['value']:.3f} {current_metric['unit']}{change_str}\n")
    
    # Write report
    with open(f'{output_dir}/summary_report.md', 'w') as f:
        f.writelines(report_lines)

def main():
    parser = argparse.ArgumentParser(description='Generate benchmark comparison charts')
    parser.add_argument('--current', required=True, help='Current benchmark results JSON file')
    parser.add_argument('--baseline', help='Baseline benchmark results JSON file')
    parser.add_argument('--output', default='benchmark_charts', help='Output directory for charts')
    
    args = parser.parse_args()
    
    # Create output directory
    os.makedirs(args.output, exist_ok=True)
    
    # Load data
    current_data = load_benchmark_results(args.current)
    baseline_data = load_benchmark_results(args.baseline) if args.baseline else []
    
    if not current_data:
        print("No current benchmark data found")
        sys.exit(1)
    
    print(f"Loaded {len(current_data)} current benchmarks")
    if baseline_data:
        print(f"Loaded {len(baseline_data)} baseline benchmarks")
    
    # Set style
    plt.style.use('seaborn-v0_8')
    sns.set_palette("husl")
    
    try:
        # Generate charts
        print("Generating performance comparison chart...")
        if baseline_data:
            create_performance_comparison_chart(current_data, baseline_data, args.output)
        
        print("Generating category breakdown chart...")
        create_category_breakdown_chart(current_data, args.output)
        
        print("Generating performance trends chart...")
        create_performance_trends_chart(current_data, args.output)
        
        print("Generating memory analysis chart...")
        create_memory_analysis_chart(current_data, args.output)
        
        print("Generating summary report...")
        generate_summary_report(current_data, baseline_data, args.output)
        
        print(f"✅ Charts generated successfully in {args.output}/")
        
    except Exception as e:
        print(f"Error generating charts: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()