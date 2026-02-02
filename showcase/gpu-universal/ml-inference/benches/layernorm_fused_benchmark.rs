// LayerNorm Fused Optimization Benchmark
//
// Compares 3-pass (original) vs 1-pass (fused) LayerNorm implementations
// Expected speedup: 8-12x for LLaMA-scale operations

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use ml_inference_showcase::wgpu::{NormConfig, WgpuExecutor};
use tokio::runtime::Runtime;

/// Benchmark LayerNorm implementations at different scales
fn bench_layernorm_comparison(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Initialize executor
    let executor = rt.block_on(WgpuExecutor::new()).unwrap();
    eprintln!("🎯 Benchmarking on: {}", executor.gpu_info());

    let mut group = c.benchmark_group("LayerNorm_Fused_vs_Original");

    // Test sizes (matching benchmark data)
    let sizes = vec![
        ("GPT-2 Hidden (768)", 768),
        ("GPT-2 Batch (768 * 128)", 768 * 128),
        ("LLaMA Hidden (4096)", 4096),
        ("LLaMA Batch (4096 * 256)", 4096 * 256), // This is the bottleneck!
    ];

    for (name, size) in sizes {
        let input: Vec<f32> = (0..size).map(|i| (i as f32) * 0.001).collect();
        let config = NormConfig {
            epsilon: 1e-5,
            gamma: None, // Default to all 1s
            beta: None,  // Default to all 0s
        };

        // Benchmark original (3-pass)
        group.bench_with_input(BenchmarkId::new("Original_3Pass", name), &size, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    executor
                        .execute_layernorm(black_box(&input), black_box(config.clone()))
                        .await
                        .unwrap()
                })
            });
        });

        // Benchmark fused (1-pass)
        group.bench_with_input(BenchmarkId::new("Fused_1Pass", name), &size, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    executor
                        .execute_layernorm_fused(black_box(&input), black_box(config.clone()))
                        .await
                        .unwrap()
                })
            });
        });
    }

    group.finish();
}

/// Focused benchmark on LLaMA scale (the critical bottleneck)
fn bench_llama_scale(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let executor = rt.block_on(WgpuExecutor::new()).unwrap();

    // LLaMA-scale: 4096 * 256 = 1,048,576 elements
    // This is where we saw 118-123ms in the original benchmarks
    let size = 4096 * 256;
    let input: Vec<f32> = (0..size).map(|i| (i as f32) * 0.001).collect();
    let config = NormConfig {
        epsilon: 1e-5,
        gamma: None,
        beta: None,
    };

    c.bench_function("LLaMA_LayerNorm_Original", |b| {
        b.iter(|| {
            rt.block_on(async {
                executor
                    .execute_layernorm(black_box(&input), black_box(config.clone()))
                    .await
                    .unwrap()
            })
        });
    });

    c.bench_function("LLaMA_LayerNorm_Fused", |b| {
        b.iter(|| {
            rt.block_on(async {
                executor
                    .execute_layernorm_fused(black_box(&input), black_box(config.clone()))
                    .await
                    .unwrap()
            })
        });
    });
}

criterion_group! {
    name = layernorm_fused;
    config = Criterion::default()
        .sample_size(20)  // Fewer samples for GPU benchmarks (more stable)
        .measurement_time(std::time::Duration::from_secs(30));
    targets = bench_layernorm_comparison, bench_llama_scale
}

criterion_main!(layernorm_fused);
