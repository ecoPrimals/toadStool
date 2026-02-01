#!/usr/bin/env python3
"""
Pipeline Validation Results Analyzer

Analyzes the pipeline_validation_matrix.json results to identify:
1. Best pipeline for each workload type
2. Impact of chip ordering (NPU→GPU vs GPU→NPU)
3. Energy efficiency comparisons
4. Transfer overhead analysis
"""

import json
import sys
from typing import List, Dict
from collections import defaultdict

def load_results(filename: str) -> List[Dict]:
    """Load validation results from JSON file."""
    with open(filename, 'r') as f:
        return json.load(f)

def analyze_by_workload(results: List[Dict]):
    """Find best pipeline for each workload type."""
    print("═" * 70)
    print("BEST PIPELINE PER WORKLOAD TYPE")
    print("═" * 70)
    print()
    
    workloads = {}
    for result in results:
        workload = result['workload_type']
        if workload not in workloads:
            workloads[workload] = []
        workloads[workload].append(result)
    
    for workload, configs in sorted(workloads.items()):
        print(f"Workload: {workload}")
        print(f"  Sparsity: {configs[0]['sparsity'] * 100:.1f}%")
        print()
        
        # Sort by efficiency
        best_efficiency = max(configs, key=lambda x: x['ops_per_joule'])
        best_throughput = max(configs, key=lambda x: x['throughput_ops_per_sec'])
        best_latency = min(configs, key=lambda x: x['total_time_us'])
        
        print(f"  Best Efficiency: {best_efficiency['pipeline_config']}")
        print(f"    → {best_efficiency['ops_per_joule']:.0f} ops/J")
        print()
        
        print(f"  Best Throughput: {best_throughput['pipeline_config']}")
        print(f"    → {best_throughput['throughput_ops_per_sec']:.0f} ops/s")
        print()
        
        print(f"  Best Latency: {best_latency['pipeline_config']}")
        print(f"    → {best_latency['total_time_us'] / 1000:.2f} ms")
        print()

def analyze_ordering_impact(results: List[Dict]):
    """Compare NPU→GPU vs GPU→NPU to show ordering matters."""
    print("═" * 70)
    print("CHIP ORDERING IMPACT ANALYSIS")
    print("═" * 70)
    print()
    
    # Find NPU→GPU vs GPU→NPU pairs
    npu_gpu_results = [r for r in results if r['pipeline_config'] == 'NPU→GPU']
    gpu_npu_results = [r for r in results if r['pipeline_config'] == 'GPU→NPU']
    
    if not npu_gpu_results or not gpu_npu_results:
        print("  ⚠️  Ordering comparison data not available\n")
        return
    
    print("Comparing: NPU→GPU vs GPU→NPU\n")
    
    for npu_gpu in npu_gpu_results:
        workload = npu_gpu['workload_type']
        gpu_npu = next((r for r in gpu_npu_results if r['workload_type'] == workload), None)
        
        if gpu_npu:
            print(f"  Workload: {workload}")
            print(f"    Sparsity: {npu_gpu['sparsity'] * 100:.1f}%")
            print()
            
            print(f"    NPU→GPU:")
            print(f"      Throughput: {npu_gpu['throughput_ops_per_sec']:.0f} ops/s")
            print(f"      Efficiency: {npu_gpu['ops_per_joule']:.0f} ops/J")
            print(f"      Transfer: {npu_gpu['transfer_overhead_percent']:.2f}%")
            print()
            
            print(f"    GPU→NPU:")
            print(f"      Throughput: {gpu_npu['throughput_ops_per_sec']:.0f} ops/s")
            print(f"      Efficiency: {gpu_npu['ops_per_joule']:.0f} ops/J")
            print(f"      Transfer: {gpu_npu['transfer_overhead_percent']:.2f}%")
            print()
            
            # Calculate advantage
            if npu_gpu['ops_per_joule'] > gpu_npu['ops_per_joule']:
                advantage = npu_gpu['ops_per_joule'] / gpu_npu['ops_per_joule']
                print(f"    ⭐ Winner: NPU→GPU ({advantage:.1f}x better efficiency!)")
            else:
                advantage = gpu_npu['ops_per_joule'] / npu_gpu['ops_per_joule']
                print(f"    ⭐ Winner: GPU→NPU ({advantage:.1f}x better efficiency!)")
            print()

def analyze_parallel_vs_serial(results: List[Dict]):
    """Compare parallel vs serial configurations."""
    print("═" * 70)
    print("PARALLEL VS SERIAL COMPARISON")
    print("═" * 70)
    print()
    
    single_gpu = [r for r in results if r['pipeline_config'] == 'Single_GPU']
    dual_gpu = [r for r in results if r['pipeline_config'] == 'Dual_GPU_Parallel']
    
    if single_gpu and dual_gpu:
        print("Dual GPU Scaling Analysis:\n")
        for single, dual in zip(single_gpu, dual_gpu):
            if single['workload_type'] == dual['workload_type']:
                scaling = dual['throughput_ops_per_sec'] / single['throughput_ops_per_sec']
                print(f"  {single['workload_type']}: {scaling:.2f}x scaling")
        print()
    
    single_npu = [r for r in results if r['pipeline_config'] == 'Single_NPU']
    dual_npu = [r for r in results if r['pipeline_config'] == 'Dual_NPU_Parallel']
    
    if single_npu and dual_npu:
        print("Dual NPU Scaling Analysis:\n")
        for single, dual in zip(single_npu, dual_npu):
            if single['workload_type'] == dual['workload_type']:
                scaling = dual['throughput_ops_per_sec'] / single['throughput_ops_per_sec']
                print(f"  {single['workload_type']}: {scaling:.2f}x scaling")
        print()

def generate_summary_table(results: List[Dict]):
    """Generate summary comparison table."""
    print("═" * 70)
    print("SUMMARY: TOP 5 CONFIGURATIONS BY EFFICIENCY")
    print("═" * 70)
    print()
    
    # Group by pipeline, average across workloads
    pipeline_averages = defaultdict(list)
    for result in results:
        pipeline_averages[result['pipeline_config']].append(result['ops_per_joule'])
    
    avg_efficiencies = {
        pipeline: sum(efficiencies) / len(efficiencies)
        for pipeline, efficiencies in pipeline_averages.items()
    }
    
    # Sort by efficiency
    top_5 = sorted(avg_efficiencies.items(), key=lambda x: x[1], reverse=True)[:5]
    
    print(f"{'Rank':<6} {'Pipeline':<25} {'Avg Efficiency (ops/J)':<25}")
    print("-" * 70)
    for idx, (pipeline, efficiency) in enumerate(top_5, 1):
        print(f"{idx:<6} {pipeline:<25} {efficiency:>20.0f}")
    print()

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 analyze_pipeline_results.py <results.json>")
        print("Example: python3 analyze_pipeline_results.py pipeline_validation_matrix.json")
        sys.exit(1)
    
    results_file = sys.argv[1]
    
    print("\n" + "═" * 70)
    print("PIPELINE VALIDATION RESULTS ANALYSIS")
    print("═" * 70)
    print()
    
    try:
        results = load_results(results_file)
        print(f"✅ Loaded {len(results)} benchmark results\n")
        
        analyze_by_workload(results)
        analyze_ordering_impact(results)
        analyze_parallel_vs_serial(results)
        generate_summary_table(results)
        
        print("═" * 70)
        print("ANALYSIS COMPLETE!")
        print("═" * 70)
        print()
        
    except FileNotFoundError:
        print(f"❌ Error: File '{results_file}' not found")
        print("   Run the validation first:")
        print("   cargo run --example pipeline_validation_matrix --release")
        sys.exit(1)
    except json.JSONDecodeError:
        print(f"❌ Error: Invalid JSON in '{results_file}'")
        sys.exit(1)

if __name__ == "__main__":
    main()
