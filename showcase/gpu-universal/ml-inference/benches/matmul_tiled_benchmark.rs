//! Tiled MatMul Performance Benchmark
//!
//! Compares naive (no shared memory) vs tiled (shared memory) MatMul
//! Expected: 2-3x speedup for large matrices

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ml_inference_showcase::wgpu::WgpuExecutor;
use tokio::runtime::Runtime;

fn bench_matmul_memory_optimization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let executor = rt.block_on(WgpuExecutor::new()).unwrap();
    
    eprintln!("🎯 Benchmarking MatMul Memory Optimization on: {}", executor.gpu_info());
    
    let mut group = c.benchmark_group("MatMul_Tiled_vs_Naive");
    
    // Test at multiple scales
    let sizes = vec![
        ("Small_128x128", 128),
        ("Medium_512x512", 512),
        ("Large_1024x1024", 1024),
        ("XLarge_2048x2048", 2048),
    ];
    
    for (name, size) in sizes {
        let a: Vec<f32> = (0..size * size).map(|i| ((i % 1000) as f32) * 0.001).collect();
        let b: Vec<f32> = (0..size * size).map(|i| (((i + 1) % 1000) as f32) * 0.001).collect();
        
        // Benchmark naive implementation
        group.bench_with_input(
            BenchmarkId::new("Naive", name),
            &size,
            |bench, &size| {
                bench.iter(|| {
                    rt.block_on(async {
                        executor
                            .execute_matmul(black_box(&a), black_box(&b), size, size, size)
                            .await
                            .unwrap()
                    })
                });
            },
        );
        
        // Benchmark tiled implementation
        group.bench_with_input(
            BenchmarkId::new("Tiled", name),
            &size,
            |bench, &size| {
                bench.iter(|| {
                    rt.block_on(async {
                        executor
                            .execute_matmul_tiled(black_box(&a), black_box(&b), size, size, size)
                            .await
                            .unwrap()
                    })
                });
            },
        );
    }
    
    group.finish();
}

criterion_group! {
    name = matmul_tiled;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(std::time::Duration::from_secs(20));
    targets = bench_matmul_memory_optimization
}

criterion_main!(matmul_tiled);
