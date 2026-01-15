//! Capability Discovery Performance Benchmarks
//!
//! Measures the performance of our Deep Debt capability-based discovery system.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::HashMap;
use std::time::Duration;

/// Benchmark environment variable-based capability discovery (our new pattern)
fn bench_capability_discovery(c: &mut Criterion) {
    let mut group = c.benchmark_group("capability_discovery");
    
    // Setup: Set capability environment variables
    std::env::set_var("TOADSTOOL_COORDINATION_SERVICE_URL", "http://localhost:8080");
    std::env::set_var("TOADSTOOL_CRYPTO_SERVICE_URL", "http://localhost:8081");
    std::env::set_var("TOADSTOOL_STORAGE_SERVICE_URL", "http://localhost:8082");
    
    // Pattern 1: Direct environment check (our optimized approach)
    group.bench_function("env_check_direct", |b| {
        b.iter(|| {
            let url = std::env::var("TOADSTOOL_COORDINATION_SERVICE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
            black_box(url);
        });
    });
    
    // Pattern 2: HashMap cache lookup (common pattern)
    let mut cache = HashMap::new();
    cache.insert("coordination".to_string(), "http://localhost:8080".to_string());
    cache.insert("crypto".to_string(), "http://localhost:8081".to_string());
    cache.insert("storage".to_string(), "http://localhost:8082".to_string());
    
    group.bench_function("hashmap_lookup", |b| {
        b.iter(|| {
            let url = cache.get(black_box("coordination")).cloned()
                .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
            black_box(url);
        });
    });
    
    // Pattern 3: Entry API (our Quick Win optimization)
    let mut cache_entry = HashMap::new();
    
    group.bench_function("entry_api_insert", |b| {
        b.iter(|| {
            let key = black_box("coordination").to_string();
            let url = "http://localhost:8080".to_string();
            cache_entry.entry(key).or_insert_with(|| url);
        });
    });
    
    group.finish();
}

/// Benchmark string allocation patterns for service URLs
fn bench_service_url_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("service_url_construction");
    
    // Pattern 1: format! macro (common)
    group.bench_function("format_macro", |b| {
        b.iter(|| {
            let url = format!("http://{}:{}", black_box("127.0.0.1"), black_box(8080));
            black_box(url);
        });
    });
    
    // Pattern 2: String concatenation
    group.bench_function("string_concat", |b| {
        b.iter(|| {
            let mut url = String::from("http://");
            url.push_str(black_box("127.0.0.1"));
            url.push(':');
            url.push_str(&black_box(8080).to_string());
            black_box(url);
        });
    });
    
    // Pattern 3: Pre-allocated with capacity
    group.bench_function("with_capacity", |b| {
        b.iter(|| {
            let mut url = String::with_capacity(30);
            url.push_str("http://");
            url.push_str(black_box("127.0.0.1"));
            url.push(':');
            url.push_str(&black_box(8080).to_string());
            black_box(url);
        });
    });
    
    group.finish();
}

/// Benchmark interned strings vs heap allocation (our zero-copy optimization)
fn bench_string_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_interning");
    
    // Simulated interned constants
    const CAPABILITY_COORDINATION: &str = "coordination";
    const CAPABILITY_CRYPTO: &str = "crypto";
    const CAPABILITY_STORAGE: &str = "storage";
    
    // Pattern 1: Using interned strings (zero-copy)
    group.bench_function("interned_strings", |b| {
        b.iter(|| {
            let cap = black_box(CAPABILITY_COORDINATION);
            black_box(cap);
        });
    });
    
    // Pattern 2: String allocation
    group.bench_function("heap_allocation", |b| {
        b.iter(|| {
            let cap = black_box("coordination").to_string();
            black_box(cap);
        });
    });
    
    // Pattern 3: Static str slice (zero-copy but not as ergonomic)
    group.bench_function("str_slice", |b| {
        b.iter(|| {
            let cap: &str = black_box("coordination");
            black_box(cap);
        });
    });
    
    group.finish();
}

/// Benchmark Result error handling patterns
fn bench_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");
    
    // Pattern 1: Result with ? operator (idiomatic)
    fn with_question_mark() -> Result<String, std::env::VarError> {
        let val = std::env::var("TOADSTOOL_TEST")?;
        Ok(val)
    }
    
    group.bench_function("result_with_question", |b| {
        std::env::set_var("TOADSTOOL_TEST", "test_value");
        b.iter(|| {
            let result = with_question_mark();
            black_box(result);
        });
    });
    
    // Pattern 2: unwrap_or_else (graceful fallback)
    group.bench_function("unwrap_or_else", |b| {
        std::env::remove_var("TOADSTOOL_NONEXISTENT");
        b.iter(|| {
            let val = std::env::var("TOADSTOOL_NONEXISTENT")
                .unwrap_or_else(|_| "default".to_string());
            black_box(val);
        });
    });
    
    // Pattern 3: unwrap_or_default
    group.bench_function("unwrap_or_default", |b| {
        std::env::remove_var("TOADSTOOL_NONEXISTENT");
        b.iter(|| {
            let val = std::env::var("TOADSTOOL_NONEXISTENT").unwrap_or_default();
            black_box(val);
        });
    });
    
    group.finish();
}

/// Benchmark HashMap Entry API vs direct insert (our Quick Win)
fn bench_hashmap_entry_api(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashmap_entry_api");
    
    // Pattern 1: Direct insert (old pattern)
    group.bench_function("direct_insert", |b| {
        b.iter(|| {
            let mut cache = HashMap::new();
            for i in 0..10 {
                let key = format!("service_{}", i);
                let value = format!("url_{}", i);
                cache.insert(key.clone(), value.clone());
            }
            black_box(cache);
        });
    });
    
    // Pattern 2: Entry API or_insert_with (optimized)
    group.bench_function("entry_api_or_insert", |b| {
        b.iter(|| {
            let mut cache = HashMap::new();
            for i in 0..10 {
                let key = format!("service_{}", i);
                let value = format!("url_{}", i);
                cache.entry(key.clone()).or_insert_with(|| value.clone());
            }
            black_box(cache);
        });
    });
    
    // Pattern 3: Entry API with pre-existing keys (best case for optimization)
    group.bench_function("entry_api_existing_keys", |b| {
        let mut cache = HashMap::new();
        for i in 0..10 {
            cache.insert(format!("service_{}", i), format!("url_{}", i));
        }
        
        b.iter(|| {
            for i in 0..10 {
                let key = format!("service_{}", i);
                let value = format!("url_{}", i);
                cache.entry(key).or_insert_with(|| value);
            }
            black_box(&cache);
        });
    });
    
    group.finish();
}

// Configure criterion with reasonable defaults
criterion_group! {
    name = capability_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(1));
    targets =
        bench_capability_discovery,
        bench_service_url_construction,
        bench_string_interning,
        bench_error_handling,
        bench_hashmap_entry_api,
}

criterion_main!(capability_benches);
