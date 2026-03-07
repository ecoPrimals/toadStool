// SPDX-License-Identifier: AGPL-3.0-or-later
//! Performance benchmarks for secure enclave operations
//!
//! Validates performance claims:
//! - Decompression: ~5ms/MB (Zstd), ~2ms/MB (LZ4)
//! - Memory isolation: < 1ms overhead
//! - Audit logging: < 0.1ms per event
//! - Total overhead: < 10% vs plaintext

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use toadstool_runtime_secure_enclave::{
    decompress_isolated, AuditEventType, AuditLogger, CompressionAlgorithm, IsolatedMemoryRegion,
    SecureEnclaveRuntime,
};

/// Benchmark isolated memory allocation
fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    for size in &[1024, 4096, 65536, 1_048_576] {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let memory = IsolatedMemoryRegion::new(size).unwrap();
                black_box(memory);
            });
        });
    }

    group.finish();
}

/// Benchmark memory wiping
fn bench_memory_wiping(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_wiping");

    for size in &[4096, 65536, 1_048_576] {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut memory = IsolatedMemoryRegion::new(size).unwrap();
                memory.wipe();
                black_box(memory);
            });
        });
    }

    group.finish();
}

/// Benchmark Zstd decompression
fn bench_zstd_decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("zstd_decompression");

    // Prepare test data
    let sizes = [
        (1024, "1KB"),
        (10_240, "10KB"),
        (102_400, "100KB"),
        (1_048_576, "1MB"),
    ];

    for (size, label) in &sizes {
        let data = vec![42u8; *size];
        let compressed =
            ruzstd::encoding::compress_to_vec(&*data, ruzstd::encoding::CompressionLevel::Fastest);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("isolated", label),
            &compressed,
            |b, comp| {
                b.iter(|| {
                    let (memory, _stats) = decompress_isolated(
                        black_box(comp),
                        CompressionAlgorithm::Zstd,
                        Some(*size),
                    )
                    .unwrap();
                    black_box(memory);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark LZ4 decompression
fn bench_lz4_decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("lz4_decompression");

    let sizes = [(1024, "1KB"), (10_240, "10KB"), (102_400, "100KB")];

    for (size, label) in &sizes {
        let data = vec![42u8; *size];
        let compressed = lz4_flex::compress_prepend_size(&data);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("isolated", label),
            &compressed,
            |b, comp| {
                b.iter(|| {
                    let (memory, _stats) = decompress_isolated(
                        black_box(comp),
                        CompressionAlgorithm::Lz4,
                        Some(*size),
                    )
                    .unwrap();
                    black_box(memory);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark audit logging
fn bench_audit_logging(c: &mut Criterion) {
    let mut group = c.benchmark_group("audit_logging");

    group.bench_function("single_event", |b| {
        b.iter(|| {
            let mut logger = AuditLogger::new();
            logger
                .log(
                    black_box(AuditEventType::MemoryAllocated),
                    r#"{"size": 4096}"#,
                )
                .unwrap();
            black_box(logger);
        });
    });

    group.bench_function("chain_verification_10", |b| {
        let mut logger = AuditLogger::new();
        for i in 0..10 {
            logger
                .log(AuditEventType::MemoryAllocated, format!("event {i}"))
                .unwrap();
        }
        b.iter(|| {
            logger.verify_integrity().unwrap();
        });
    });

    group.bench_function("chain_verification_100", |b| {
        let mut logger = AuditLogger::new();
        for i in 0..100 {
            logger
                .log(AuditEventType::MemoryAllocated, format!("event {i}"))
                .unwrap();
        }
        b.iter(|| {
            logger.verify_integrity().unwrap();
        });
    });

    group.finish();
}

/// Benchmark complete workflow (decompress + process + audit)
fn bench_complete_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("complete_workflow");

    let data = vec![42u8; 10_240]; // 10KB
    let compressed =
        ruzstd::encoding::compress_to_vec(&*data, ruzstd::encoding::CompressionLevel::Fastest);

    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("decompress_process_audit", |b| {
        b.iter(|| {
            let mut runtime = SecureEnclaveRuntime::new().unwrap();

            // Decompress
            let (memory, _stats) = decompress_isolated(
                black_box(&compressed),
                CompressionAlgorithm::Zstd,
                Some(data.len()),
            )
            .unwrap();

            // Process
            let result = runtime
                .process_isolated(memory.as_slice(), |data| {
                    let sum: u64 = data.iter().map(|&b| u64::from(b)).sum();
                    Ok(sum)
                })
                .unwrap();

            // Verify audit trail
            if let Some(logger) = runtime.audit_logger() {
                logger.verify_integrity().unwrap();
            }

            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark runtime overhead vs plaintext
fn bench_overhead_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("overhead_comparison");

    let data = vec![42u8; 10_240]; // 10KB

    group.throughput(Throughput::Bytes(data.len() as u64));

    // Baseline: plaintext processing
    group.bench_function("plaintext_baseline", |b| {
        b.iter(|| {
            let sum: u64 = black_box(&data).iter().map(|&b| u64::from(b)).sum();
            black_box(sum);
        });
    });

    // With isolation
    group.bench_function("isolated_processing", |b| {
        b.iter(|| {
            let mut runtime = SecureEnclaveRuntime::new().unwrap();
            let result = runtime
                .process_isolated(black_box(&data), |data| {
                    let sum: u64 = data.iter().map(|&b| u64::from(b)).sum();
                    Ok(sum)
                })
                .unwrap();
            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_memory_allocation,
    bench_memory_wiping,
    bench_zstd_decompression,
    bench_lz4_decompression,
    bench_audit_logging,
    bench_complete_workflow,
    bench_overhead_comparison,
);

criterion_main!(benches);
