//! LayerNorm Optimization Comparison Benchmark
//!
//! Compares original vs optimized LayerNorm (both 3-pass, but with 4 optimizations)
//! Target: 2.6x improvement on LLaMA-scale tensors

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use ml_inference_showcase::wgpu::*;
use tokio::runtime::Runtime;

/// Benchmark original LayerNorm (3-pass)
fn bench_layernorm_original(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let executor = rt.block_on(WgpuExecutor::new()).unwrap();

    let mut group = c.benchmark_group("LayerNorm_Original_3Pass");

    let configs = vec![
        ("bert_384k", 512 * 768),  // BERT: 393,216 elements
        ("gpt2_1m", 1024 * 1024),  // GPT-2: 1,048,576 elements
        ("llama_8m", 2048 * 4096), // LLaMA: 8,388,608 elements
    ];

    for (name, size) in configs {
        let input: Vec<f32> = vec![0.5; size];
        let config = NormConfig {
            epsilon: 1e-5,
            gamma: Some(vec![1.0; size]),
            beta: Some(vec![0.0; size]),
        };

        group.bench_with_input(BenchmarkId::from_parameter(name), &name, |bencher, _| {
            bencher.iter(|| {
                rt.block_on(executor.execute_layernorm(black_box(&input), config.clone()))
            });
        });
    }

    group.finish();
}

/// Benchmark optimized LayerNorm (3-pass with 4 optimizations)
fn bench_layernorm_optimized(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let executor = rt.block_on(WgpuExecutor::new()).unwrap();

    let mut group = c.benchmark_group("LayerNorm_Optimized_3Pass");

    let configs = vec![
        ("bert_384k", 512 * 768),  // BERT: 393,216 elements
        ("gpt2_1m", 1024 * 1024),  // GPT-2: 1,048,576 elements
        ("llama_8m", 2048 * 4096), // LLaMA: 8,388,608 elements
    ];

    for (name, size) in configs {
        let input: Vec<f32> = vec![0.5; size];
        let config = NormConfig {
            epsilon: 1e-5,
            gamma: Some(vec![1.0; size]),
            beta: Some(vec![0.0; size]),
        };

        group.bench_with_input(BenchmarkId::from_parameter(name), &name, |bencher, _| {
            bencher.iter(|| {
                rt.block_on(executor.execute_layernorm_optimized(black_box(&input), config.clone()))
            });
        });
    }

    group.finish();
}

/// Quick comparison benchmark
fn bench_comparison(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let executor = rt.block_on(WgpuExecutor::new()).unwrap();

    // LLaMA scale (the critical bottleneck)
    let size = 2048 * 4096; // 8.4M elements
    let input: Vec<f32> = vec![0.5; size];
    let config = NormConfig {
        epsilon: 1e-5,
        gamma: Some(vec![1.0; size]),
        beta: Some(vec![0.0; size]),
    };

    c.bench_function("LayerNorm_LLaMA_Original", |bencher| {
        bencher.iter(|| rt.block_on(executor.execute_layernorm(black_box(&input), config.clone())));
    });

    c.bench_function("LayerNorm_LLaMA_Optimized", |bencher| {
        bencher.iter(|| {
            rt.block_on(executor.execute_layernorm_optimized(black_box(&input), config.clone()))
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_layernorm_original, bench_layernorm_optimized, bench_comparison
);
criterion_main!(benches);
