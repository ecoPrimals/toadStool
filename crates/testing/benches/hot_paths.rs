// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hot Path Benchmarks for ToadStool
//!
//! Benchmarks for the most frequently executed code paths to identify
//! optimization opportunities through data-driven analysis.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::time::Duration;

/// Benchmark string allocation patterns (common hot spot)
fn bench_string_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_allocations");

    // Pattern 1: to_string()
    group.bench_function("to_string", |b| {
        b.iter(|| {
            let s = black_box("test_string").to_string();
            black_box(s);
        });
    });

    // Pattern 2: into()
    group.bench_function("into", |b| {
        b.iter(|| {
            let s: String = black_box("test_string").into();
            black_box(s);
        });
    });

    // Pattern 3: String::from()
    group.bench_function("string_from", |b| {
        b.iter(|| {
            let s = String::from(black_box("test_string"));
            black_box(s);
        });
    });

    group.finish();
}

/// Benchmark `HashMap` operations (common in BYOB deployment)
fn bench_hashmap_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashmap_operations");

    // Setup: Create a HashMap with test data
    let mut map = HashMap::new();
    for i in 0..100 {
        map.insert(format!("key_{i}"), format!("value_{i}"));
    }

    // Pattern 1: Clone entire HashMap
    group.bench_function("clone_hashmap", |b| {
        b.iter(|| {
            let cloned = black_box(&map).clone();
            black_box(cloned);
        });
    });

    // Pattern 2: Clone only keys
    group.bench_function("clone_keys", |b| {
        b.iter(|| {
            let keys: Vec<_> = black_box(&map).keys().cloned().collect();
            black_box(keys);
        });
    });

    // Pattern 3: Iterate by reference
    group.bench_function("iterate_reference", |b| {
        b.iter(|| {
            let mut count = 0;
            for (k, v) in black_box(&map) {
                count += k.len() + v.len();
            }
            black_box(count);
        });
    });

    group.finish();
}

/// Benchmark Vec operations (common in request handling)
fn bench_vec_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec_operations");

    // Setup: Create test data
    let data: Vec<String> = (0..1000).map(|i| format!("item_{i}")).collect();

    // Pattern 1: Clone entire Vec
    group.bench_function("clone_vec", |b| {
        b.iter(|| {
            let cloned = black_box(&data).clone();
            black_box(cloned);
        });
    });

    // Pattern 2: Clone via iter().cloned()
    group.bench_function("iter_cloned", |b| {
        b.iter(|| {
            let collected = black_box(&data).clone();
            black_box(collected);
        });
    });

    // Pattern 3: Map to references
    group.bench_function("map_references", |b| {
        b.iter(|| {
            let refs: Vec<_> = black_box(&data)
                .iter()
                .map(std::string::String::as_str)
                .collect();
            black_box(refs);
        });
    });

    // Pattern 4: Pre-allocated Vec
    group.bench_function("preallocated", |b| {
        b.iter(|| {
            let mut vec = Vec::with_capacity(1000);
            for item in black_box(&data) {
                vec.push(item.clone());
            }
            black_box(vec);
        });
    });

    group.finish();
}

/// Benchmark JSON serialization (common in API responses)
fn bench_json_operations(c: &mut Criterion) {
    use serde_json::{json, Value};

    let mut group = c.benchmark_group("json_operations");

    // Setup: Create test JSON data
    let data = json!({
        "name": "test_primal",
        "type": "songbird",
        "version": "1.0.0",
        "capabilities": ["discovery", "routing", "coordination"],
        "status": "active"
    });

    // Pattern 1: to_string() on Value
    group.bench_function("value_to_string", |b| {
        b.iter(|| {
            let s = black_box(&data).to_string();
            black_box(s);
        });
    });

    // Pattern 2: serde_json::to_string()
    group.bench_function("serde_to_string", |b| {
        b.iter(|| {
            #[allow(clippy::unwrap_used)] // Benchmark: panic on failure is acceptable
            let s = serde_json::to_string(black_box(&data)).unwrap();
            black_box(s);
        });
    });

    // Pattern 3: Parse from string
    group.bench_function("parse_json", |b| {
        let json_str = data.to_string();
        b.iter(|| {
            #[allow(clippy::unwrap_used)] // Benchmark: panic on failure is acceptable
            let parsed: Value = serde_json::from_str(black_box(&json_str)).unwrap();
            black_box(parsed);
        });
    });

    group.finish();
}

/// Benchmark common configuration parsing patterns
fn bench_config_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_parsing");

    // Simulate environment variable reads
    std::env::set_var("TEST_VAR", "test_value");

    group.bench_function("env_var_read", |b| {
        b.iter(|| {
            let val = std::env::var(black_box("TEST_VAR")).unwrap_or_default();
            black_box(val);
        });
    });

    group.bench_function("env_var_with_default", |b| {
        b.iter(|| {
            let val =
                std::env::var(black_box("NONEXISTENT")).unwrap_or_else(|_| "default".to_string());
            black_box(val);
        });
    });

    group.finish();
}

// Configure criterion groups
criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10));
    targets =
        bench_string_allocations,
        bench_hashmap_operations,
        bench_vec_operations,
        bench_json_operations,
        bench_config_parsing,
}

criterion_main!(benches);
