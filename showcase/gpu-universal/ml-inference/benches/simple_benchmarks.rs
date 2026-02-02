//! Simple Benchmarks for Hot Path Operations
//!
//! Focuses on the most critical operations to identify optimization opportunities.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use ml_inference_showcase::wgpu::*;
use tokio::runtime::Runtime;

/// Benchmark MatMul - the most critical operation
fn bench_matmul(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Robust GPU selection using new substrate API
    let executor = if std::env::var("USE_AMD_GPU").is_ok() {
        eprintln!("🔴 Benchmarking on AMD GPU (explicit selection)");
        rt.block_on(WgpuExecutor::new_amd()).unwrap()
    } else if std::env::var("USE_NVIDIA_GPU").is_ok() {
        eprintln!("🟢 Benchmarking on NVIDIA GPU (explicit selection)");
        rt.block_on(WgpuExecutor::new_nvidia()).unwrap()
    } else {
        eprintln!("📊 Benchmarking on default GPU (first available)");
        let exec = rt.block_on(WgpuExecutor::new()).unwrap();
        eprintln!("   GPU: {}", exec.gpu_info());
        exec
    };

    let mut group = c.benchmark_group("MatMul");

    let configs = vec![
        ("32x32", 32),
        ("64x64", 64),
        ("128x128", 128),
        ("256x256", 256),
        ("512x512", 512),
        ("1024x1024", 1024),
    ];

    for (name, size) in configs {
        let a: Vec<f32> = vec![1.0; size * size];
        let b: Vec<f32> = vec![1.0; size * size];

        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    rt.block_on(executor.execute_matmul(
                        black_box(&a),
                        black_box(&b),
                        size,
                        size,
                        size,
                    ))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark BatchMatMul - critical for transformers
fn bench_batch_matmul(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let executor = rt.block_on(WgpuExecutor::new()).unwrap();

    let mut group = c.benchmark_group("BatchMatMul");

    let configs = vec![
        ("8_heads_64seq", 8, 64),
        ("12_heads_128seq", 12, 128),
        ("16_heads_256seq", 16, 256),
    ];

    for (name, batch, size) in configs {
        let a: Vec<f32> = vec![1.0; batch * size * size];
        let b: Vec<f32> = vec![1.0; batch * size * size];

        group.bench_with_input(BenchmarkId::from_parameter(name), &name, |bencher, _| {
            bencher.iter(|| {
                rt.block_on(executor.execute_batch_matmul(
                    black_box(&a),
                    black_box(&b),
                    batch,
                    size,
                    size,
                    size,
                ))
            });
        });
    }

    group.finish();
}

/// Benchmark common activations
fn bench_activations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let executor = rt.block_on(WgpuExecutor::new()).unwrap();

    let sizes = vec![("1k", 1024), ("64k", 65536), ("1m", 1048576)];

    for (name, size) in sizes.clone() {
        let input: Vec<f32> = vec![0.5; size];

        // ReLU
        c.bench_function(&format!("ReLU_{}", name), |bencher| {
            bencher.iter(|| rt.block_on(executor.execute_relu(black_box(&input))));
        });

        // GELU
        c.bench_function(&format!("GELU_{}", name), |bencher| {
            bencher.iter(|| rt.block_on(executor.execute_gelu(black_box(&input))));
        });

        // Sigmoid
        c.bench_function(&format!("Sigmoid_{}", name), |bencher| {
            bencher.iter(|| rt.block_on(executor.execute_sigmoid(black_box(&input))));
        });
    }
}

/// Benchmark LayerNorm - critical for transformers
fn bench_layernorm(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let executor = rt.block_on(WgpuExecutor::new()).unwrap();

    let mut group = c.benchmark_group("LayerNorm");

    let configs = vec![
        ("bert_512x768", 512 * 768),
        ("gpt2_1024x1024", 1024 * 1024),
        ("llama_2048x4096", 2048 * 4096),
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

/// Benchmark data operations
fn bench_data_ops(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let executor = rt.block_on(WgpuExecutor::new()).unwrap();

    // Concat
    let sizes = vec![1024, 65536, 1048576];

    for size in sizes {
        let a: Vec<f32> = vec![1.0; size];
        let b: Vec<f32> = vec![2.0; size];

        c.bench_function(&format!("Concat_{}", size), |bencher| {
            bencher.iter(|| rt.block_on(executor.execute_concat(black_box(&a), black_box(&b))));
        });
    }

    // Slice
    let input: Vec<f32> = vec![1.0; 1048576];
    c.bench_function("Slice_1m", |bencher| {
        bencher.iter(|| rt.block_on(executor.execute_slice(black_box(&input), 0, 524288)));
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_matmul, bench_batch_matmul, bench_activations, bench_layernorm, bench_data_ops
);
criterion_main!(benches);
