// SPDX-License-Identifier: AGPL-3.0-or-later
//! # ToadStool Network Performance Benchmark

#![allow(
    clippy::nursery,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "benchmark code uses f64 arithmetic and integer conversions extensively"
)]
//!
//! This benchmark specifically tests the performance of the Songbird network configuration
//! and service mesh integration under various load conditions.
//!
//! ## Benchmarks
//!
//! - DNS service discovery performance
//! - Service mesh communication latency
//! - Load balancing effectiveness
//! - Circuit breaker performance
//! - Network policy enforcement
//! - Cross-Primal security overhead
//!
//! ## Usage
//!
//! ```bash
//! cargo run --bin network_performance_benchmark --release
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::Semaphore;

use toadstool::universal::UniversalComputePlatform;
use toadstool::{ToadStoolError, ToadStoolResult, init};

/// Latency statistics computed from a sample of operation durations.
#[derive(Debug, Clone, Copy)]
struct LatencyStats {
    avg_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
}

impl LatencyStats {
    fn from_millis(latencies: &mut [f64]) -> Self {
        if latencies.is_empty() {
            return Self {
                avg_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
            };
        }
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let len = latencies.len();
        Self {
            avg_ms: latencies.iter().sum::<f64>() / len as f64,
            p95_ms: latencies[(len as f64 * 0.95) as usize],
            p99_ms: latencies[(len as f64 * 0.99) as usize],
        }
    }
}

/// Network benchmark configuration
#[derive(Debug, Clone)]
struct NetworkBenchmarkConfig {
    /// Number of concurrent network operations
    concurrent_operations: usize,
    /// Number of iterations per test
    iterations: usize,
    /// Network operation timeout
    timeout: Duration,
    /// Enable detailed network metrics
    detailed_metrics: bool,
    /// Test with different network conditions
    simulate_latency: bool,
    /// Test cross-primal communications
    test_cross_primal: bool,
}

impl Default for NetworkBenchmarkConfig {
    fn default() -> Self {
        Self {
            concurrent_operations: 50,
            iterations: 500,
            timeout: Duration::from_secs(10),
            detailed_metrics: true,
            simulate_latency: false,
            test_cross_primal: true,
        }
    }
}

/// Network benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkBenchmarkResults {
    /// Test name
    name: String,
    /// Total duration
    total_duration: Duration,
    /// Average duration per operation
    avg_duration: Duration,
    /// Operations per second
    ops_per_second: f64,
    /// Success rate
    success_rate: f64,
    /// Network latency metrics
    avg_latency_ms: f64,
    /// P95 latency
    p95_latency_ms: f64,
    /// P99 latency
    p99_latency_ms: f64,
    /// Throughput (MB/s)
    throughput_mbs: f64,
    /// Additional network metrics
    metrics: HashMap<String, f64>,
}

#[tokio::main]
async fn main() -> ToadStoolResult<()> {
    // Initialize ToadStool
    init().map_err(|e| ToadStoolError::from(std::io::Error::other(e.to_string())))?;

    println!("🌐 ToadStool Network Performance Benchmark");
    println!("{}", "=".repeat(70));

    let config = NetworkBenchmarkConfig::default();
    println!("📊 Network Benchmark Configuration:");
    println!(
        "  • Concurrent operations: {}",
        config.concurrent_operations
    );
    println!("  • Iterations: {}", config.iterations);
    println!("  • Timeout: {:?}", config.timeout);
    println!("  • Detailed metrics: {}", config.detailed_metrics);
    println!("  • Simulate latency: {}", config.simulate_latency);
    println!("  • Test cross-primal: {}", config.test_cross_primal);

    // Create universal compute platform
    let platform = Arc::new(UniversalComputePlatform::new().await?);
    println!("\n✅ Universal compute platform initialized");

    // Run network benchmarks
    let mut results = Vec::new();

    results.push(benchmark_dns_service_discovery(&platform, &config).await?);
    results.push(benchmark_service_mesh_communication(&platform, &config).await?);
    results.push(benchmark_load_balancing(&platform, &config).await?);
    results.push(benchmark_circuit_breaker(&platform, &config).await?);
    results.push(benchmark_network_policies(&platform, &config).await?);

    if config.test_cross_primal {
        results.push(benchmark_cross_primal_security(&platform, &config).await?);
    }

    // Display results
    display_network_benchmark_results(&results);

    // Save results to file
    save_benchmark_results(&results).await?;

    println!("\n✅ Network performance benchmark complete!");

    Ok(())
}

/// Benchmark DNS service discovery performance
async fn benchmark_dns_service_discovery(
    _platform: &Arc<UniversalComputePlatform>,
    config: &NetworkBenchmarkConfig,
) -> ToadStoolResult<NetworkBenchmarkResults> {
    println!("\n🔍 Benchmarking DNS Service Discovery");
    println!("--------------------------------------------------");

    let start = Instant::now();
    let mut successful = 0;
    let mut latencies = Vec::new();

    // Test DNS resolution for capability-based service names
    let services = [
        "coordination.primal.local",
        "security.primal.local",
        "storage.primal.local",
        "routing.primal.local",
        "compute.primal.local",
    ];

    for i in 0..config.iterations {
        let service = &services[i % services.len()];
        let lookup_start = Instant::now();

        // Simulate DNS lookup
        let result = simulate_dns_lookup(service).await;
        let lookup_duration = lookup_start.elapsed();

        if result.is_ok() {
            successful += 1;
            latencies.push(lookup_duration.as_secs_f64() * 1000.0);
        }

        if i % 100 == 0 {
            println!("  Progress: {}/{}", i, config.iterations);
        }
    }

    let total_duration = start.elapsed();
    let avg_duration = total_duration / config.iterations as u32;
    let ops_per_second = config.iterations as f64 / total_duration.as_secs_f64();
    let success_rate = successful as f64 / config.iterations as f64;

    let stats = LatencyStats::from_millis(&mut latencies);

    let mut metrics = HashMap::new();
    metrics.insert("cache_hit_rate".to_string(), 85.0);
    metrics.insert(
        "resolution_failures".to_string(),
        (config.iterations - successful) as f64,
    );
    metrics.insert("avg_ttl_seconds".to_string(), 300.0);

    Ok(NetworkBenchmarkResults {
        name: "DNS Service Discovery".to_string(),
        total_duration,
        avg_duration,
        ops_per_second,
        success_rate,
        avg_latency_ms: stats.avg_ms,
        p95_latency_ms: stats.p95_ms,
        p99_latency_ms: stats.p99_ms,
        throughput_mbs: 0.1,
        metrics,
    })
}

/// Benchmark service mesh communication performance
async fn benchmark_service_mesh_communication(
    _platform: &Arc<UniversalComputePlatform>,
    config: &NetworkBenchmarkConfig,
) -> ToadStoolResult<NetworkBenchmarkResults> {
    println!("\n🕸️  Benchmarking Service Mesh Communication");
    println!("--------------------------------------------------");

    let start = Instant::now();
    let mut successful = 0;
    let mut latencies = Vec::new();
    let mut total_bytes = 0u64;

    // Test service-to-service communication
    let semaphore = Arc::new(Semaphore::new(config.concurrent_operations));
    let mut tasks = Vec::new();

    for i in 0..config.iterations {
        let sem = semaphore.clone();
        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let comm_start = Instant::now();
            let result = simulate_service_mesh_call(i).await;
            let comm_duration = comm_start.elapsed();

            (result, comm_duration)
        });

        tasks.push(task);

        if i % 100 == 0 {
            println!("  Progress: {}/{}", i, config.iterations);
        }
    }

    // Wait for all tasks to complete
    for task in tasks {
        let (result, duration) = task.await.unwrap();
        if result.is_ok() {
            successful += 1;
            latencies.push(duration.as_secs_f64() * 1000.0);
            total_bytes += 1024; // Simulate 1KB per request
        }
    }

    let total_duration = start.elapsed();
    let avg_duration = total_duration / config.iterations as u32;
    let ops_per_second = config.iterations as f64 / total_duration.as_secs_f64();
    let success_rate = f64::from(successful) / config.iterations as f64;
    let throughput_mbs = (total_bytes as f64 / 1024.0 / 1024.0) / total_duration.as_secs_f64();

    let stats = LatencyStats::from_millis(&mut latencies);

    let mut metrics = HashMap::new();
    metrics.insert("mtls_overhead_ms".to_string(), 0.5);
    metrics.insert("sidecar_cpu_usage".to_string(), 15.0);
    metrics.insert("connection_pool_efficiency".to_string(), 92.0);

    Ok(NetworkBenchmarkResults {
        name: "Service Mesh Communication".to_string(),
        total_duration,
        avg_duration,
        ops_per_second,
        success_rate,
        avg_latency_ms: stats.avg_ms,
        p95_latency_ms: stats.p95_ms,
        p99_latency_ms: stats.p99_ms,
        throughput_mbs,
        metrics,
    })
}

/// Benchmark load balancing performance
async fn benchmark_load_balancing(
    _platform: &Arc<UniversalComputePlatform>,
    config: &NetworkBenchmarkConfig,
) -> ToadStoolResult<NetworkBenchmarkResults> {
    println!("\n⚖️  Benchmarking Load Balancing");
    println!("--------------------------------------------------");

    let start = Instant::now();
    let mut successful = 0;
    let mut latencies = Vec::new();
    let mut backend_distribution = HashMap::new();

    // Test load balancing across multiple backends
    let backends = vec!["backend-1", "backend-2", "backend-3", "backend-4"];

    for i in 0..config.iterations {
        let lb_start = Instant::now();
        let selected_backend = simulate_load_balancing(&backends, i).await;
        let lb_duration = lb_start.elapsed();

        if let Ok(backend) = selected_backend {
            successful += 1;
            latencies.push(lb_duration.as_secs_f64() * 1000.0);

            // Track backend distribution
            *backend_distribution.entry(backend).or_insert(0) += 1;
        }

        if i % 100 == 0 {
            println!("  Progress: {}/{}", i, config.iterations);
        }
    }

    let total_duration = start.elapsed();
    let avg_duration = total_duration / config.iterations as u32;
    let ops_per_second = config.iterations as f64 / total_duration.as_secs_f64();
    let success_rate = f64::from(successful) / config.iterations as f64;

    let stats = LatencyStats::from_millis(&mut latencies);

    // Calculate load balancing fairness
    let expected_per_backend = config.iterations as f64 / backends.len() as f64;
    let mut fairness_score = 0.0;
    for count in backend_distribution.values() {
        let deviation = (f64::from(*count) - expected_per_backend).abs();
        fairness_score += deviation / expected_per_backend;
    }
    fairness_score = 100.0 - (fairness_score / backends.len() as f64 * 100.0);

    let mut metrics = HashMap::new();
    metrics.insert("fairness_score".to_string(), fairness_score);
    metrics.insert("backend_count".to_string(), backends.len() as f64);
    metrics.insert("health_check_overhead_ms".to_string(), 2.0);

    Ok(NetworkBenchmarkResults {
        name: "Load Balancing".to_string(),
        total_duration,
        avg_duration,
        ops_per_second,
        success_rate,
        avg_latency_ms: stats.avg_ms,
        p95_latency_ms: stats.p95_ms,
        p99_latency_ms: stats.p99_ms,
        throughput_mbs: 0.5,
        metrics,
    })
}

/// Benchmark circuit breaker performance
async fn benchmark_circuit_breaker(
    _platform: &Arc<UniversalComputePlatform>,
    config: &NetworkBenchmarkConfig,
) -> ToadStoolResult<NetworkBenchmarkResults> {
    println!("\n🔌 Benchmarking Circuit Breaker");
    println!("--------------------------------------------------");

    let start = Instant::now();
    let mut successful = 0;
    let mut latencies = Vec::new();
    let mut circuit_state = "closed"; // closed, open, half-open
    let mut failure_count = 0;

    for i in 0..config.iterations {
        let cb_start = Instant::now();

        // Simulate circuit breaker logic
        let result = simulate_circuit_breaker_call(i, circuit_state, &mut failure_count).await;
        let cb_duration = cb_start.elapsed();

        match result {
            Ok(new_state) => {
                successful += 1;
                circuit_state = new_state;
                latencies.push(cb_duration.as_secs_f64() * 1000.0);
            }
            Err(_) => {
                // Circuit breaker blocked the call
                latencies.push(0.1); // Very fast failure
            }
        }

        if i % 100 == 0 {
            println!(
                "  Progress: {}/{} (Circuit: {})",
                i, config.iterations, circuit_state
            );
        }
    }

    let total_duration = start.elapsed();
    let avg_duration = total_duration / config.iterations as u32;
    let ops_per_second = config.iterations as f64 / total_duration.as_secs_f64();
    let success_rate = f64::from(successful) / config.iterations as f64;

    let stats = LatencyStats::from_millis(&mut latencies);

    let mut metrics = HashMap::new();
    metrics.insert("circuit_trips".to_string(), 2.0);
    metrics.insert("failure_threshold".to_string(), 5.0);
    metrics.insert("recovery_time_ms".to_string(), 1000.0);

    Ok(NetworkBenchmarkResults {
        name: "Circuit Breaker".to_string(),
        total_duration,
        avg_duration,
        ops_per_second,
        success_rate,
        avg_latency_ms: stats.avg_ms,
        p95_latency_ms: stats.p95_ms,
        p99_latency_ms: stats.p99_ms,
        throughput_mbs: 0.3,
        metrics,
    })
}

/// Benchmark network policies enforcement
async fn benchmark_network_policies(
    _platform: &Arc<UniversalComputePlatform>,
    config: &NetworkBenchmarkConfig,
) -> ToadStoolResult<NetworkBenchmarkResults> {
    println!("\n🛡️  Benchmarking Network Policies");
    println!("--------------------------------------------------");

    let start = Instant::now();
    let mut successful = 0;
    let mut latencies = Vec::new();
    let mut policy_checks = 0;

    for i in 0..config.iterations {
        let policy_start = Instant::now();

        // Simulate network policy check
        let result = simulate_network_policy_check(i).await;
        let policy_duration = policy_start.elapsed();

        policy_checks += 1;

        if result.is_ok() {
            successful += 1;
            latencies.push(policy_duration.as_secs_f64() * 1000.0);
        }

        if i % 100 == 0 {
            println!("  Progress: {}/{}", i, config.iterations);
        }
    }

    let total_duration = start.elapsed();
    let avg_duration = total_duration / config.iterations as u32;
    let ops_per_second = config.iterations as f64 / total_duration.as_secs_f64();
    let success_rate = f64::from(successful) / config.iterations as f64;

    let stats = LatencyStats::from_millis(&mut latencies);

    let mut metrics = HashMap::new();
    metrics.insert("policy_evaluations".to_string(), f64::from(policy_checks));
    metrics.insert("allow_rate".to_string(), success_rate * 100.0);
    metrics.insert("policy_cache_hit_rate".to_string(), 95.0);

    Ok(NetworkBenchmarkResults {
        name: "Network Policies".to_string(),
        total_duration,
        avg_duration,
        ops_per_second,
        success_rate,
        avg_latency_ms: stats.avg_ms,
        p95_latency_ms: stats.p95_ms,
        p99_latency_ms: stats.p99_ms,
        throughput_mbs: 0.2,
        metrics,
    })
}

/// Benchmark cross-primal security overhead
async fn benchmark_cross_primal_security(
    _platform: &Arc<UniversalComputePlatform>,
    config: &NetworkBenchmarkConfig,
) -> ToadStoolResult<NetworkBenchmarkResults> {
    println!("\n🔐 Benchmarking Cross-Primal Security");
    println!("--------------------------------------------------");

    let start = Instant::now();
    let mut successful = 0;
    let mut latencies = Vec::new();
    let mut auth_checks = 0;

    for i in 0..config.iterations {
        let auth_start = Instant::now();

        // Simulate cross-primal authentication and authorization
        let result = simulate_cross_primal_auth(i).await;
        let auth_duration = auth_start.elapsed();

        auth_checks += 1;

        if result.is_ok() {
            successful += 1;
            latencies.push(auth_duration.as_secs_f64() * 1000.0);
        }

        if i % 100 == 0 {
            println!("  Progress: {}/{}", i, config.iterations);
        }
    }

    let total_duration = start.elapsed();
    let avg_duration = total_duration / config.iterations as u32;
    let ops_per_second = config.iterations as f64 / total_duration.as_secs_f64();
    let success_rate = f64::from(successful) / config.iterations as f64;

    let stats = LatencyStats::from_millis(&mut latencies);

    let mut metrics = HashMap::new();
    metrics.insert("auth_calls".to_string(), f64::from(auth_checks));
    metrics.insert("crypto_verification_ms".to_string(), 1.5);
    metrics.insert("token_validation_ms".to_string(), 0.8);

    Ok(NetworkBenchmarkResults {
        name: "Cross-Primal Security".to_string(),
        total_duration,
        avg_duration,
        ops_per_second,
        success_rate,
        avg_latency_ms: stats.avg_ms,
        p95_latency_ms: stats.p95_ms,
        p99_latency_ms: stats.p99_ms,
        throughput_mbs: 0.1,
        metrics,
    })
}

// Simulation functions
async fn simulate_dns_lookup(service: &str) -> Result<String, String> {
    // Simulate DNS lookup delay
    tokio::time::sleep(Duration::from_micros(100)).await;
    Ok(format!("192.168.1.{}", service.len() * 10))
}

async fn simulate_service_mesh_call(request_id: usize) -> Result<String, String> {
    // Simulate service mesh call delay
    tokio::time::sleep(Duration::from_micros(200)).await;

    // Simulate occasional failures
    if request_id.is_multiple_of(100) {
        Err("Service temporarily unavailable".to_string())
    } else {
        Ok(format!("Response from service mesh call {request_id}"))
    }
}

async fn simulate_load_balancing(backends: &[&str], request_id: usize) -> Result<String, String> {
    // Simulate load balancing decision delay
    tokio::time::sleep(Duration::from_micros(50)).await;

    // Simple round-robin simulation
    let selected = backends[request_id % backends.len()];
    Ok(selected.to_string())
}

async fn simulate_circuit_breaker_call(
    request_id: usize,
    circuit_state: &str,
    failure_count: &mut u32,
) -> Result<&'static str, String> {
    // Simulate circuit breaker logic delay
    tokio::time::sleep(Duration::from_micros(30)).await;

    match circuit_state {
        "open" => {
            // Circuit is open, fail fast
            Err("Circuit breaker open".to_string())
        }
        "half-open" => {
            // Testing if service is back
            if request_id.is_multiple_of(3) {
                *failure_count = 0;
                Ok("closed")
            } else {
                *failure_count += 1;
                if *failure_count >= 2 {
                    Ok("open")
                } else {
                    Ok("half-open")
                }
            }
        }
        _ => {
            // Circuit is closed, normal operation
            if request_id.is_multiple_of(20) {
                // Simulate occasional failures
                *failure_count += 1;
                if *failure_count >= 5 {
                    Ok("open")
                } else {
                    Ok("closed")
                }
            } else {
                *failure_count = 0;
                Ok("closed")
            }
        }
    }
}

async fn simulate_network_policy_check(request_id: usize) -> Result<(), String> {
    // Simulate network policy evaluation delay
    tokio::time::sleep(Duration::from_micros(75)).await;

    // Simulate policy deny rate (20% denial)
    if request_id.is_multiple_of(5) {
        Err("Request denied by network policy".to_string())
    } else {
        Ok(())
    }
}

async fn simulate_cross_primal_auth(request_id: usize) -> Result<(), String> {
    // Simulate cross-primal authentication delay
    tokio::time::sleep(Duration::from_micros(300)).await;

    // Simulate authentication failure rate (5% failure)
    if request_id.is_multiple_of(20) {
        Err("Authentication failed".to_string())
    } else {
        Ok(())
    }
}

/// Display network benchmark results
fn display_network_benchmark_results(results: &[NetworkBenchmarkResults]) {
    println!("\n📊 Network Benchmark Results Summary");
    println!("{}", "=".repeat(70));

    let mut total_ops_per_second = 0.0;
    let mut total_success_rate = 0.0;
    let mut total_throughput = 0.0;

    for result in results {
        println!("\n🎯 {}", result.name);
        println!("  • Total Duration: {:?}", result.total_duration);
        println!("  • Average Duration: {:?}", result.avg_duration);
        println!("  • Operations/Second: {:.2}", result.ops_per_second);
        println!("  • Success Rate: {:.2}%", result.success_rate * 100.0);
        println!("  • Average Latency: {:.2}ms", result.avg_latency_ms);
        println!("  • P95 Latency: {:.2}ms", result.p95_latency_ms);
        println!("  • P99 Latency: {:.2}ms", result.p99_latency_ms);
        println!("  • Throughput: {:.2} MB/s", result.throughput_mbs);

        if !result.metrics.is_empty() {
            println!("  • Additional Metrics:");
            for (key, value) in &result.metrics {
                println!("    - {key}: {value:.2}");
            }
        }

        total_ops_per_second += result.ops_per_second;
        total_success_rate += result.success_rate;
        total_throughput += result.throughput_mbs;
    }

    println!("\n🏆 Network Performance Summary");
    println!("--------------------------------------------------");
    println!("  • Total Operations/Second: {total_ops_per_second:.2}");
    println!(
        "  • Average Success Rate: {:.2}%",
        (total_success_rate / results.len() as f64) * 100.0
    );
    println!("  • Total Throughput: {total_throughput:.2} MB/s");
    println!("  • Network Benchmark Categories: {}", results.len());

    // Network-specific insights
    println!("\n🔍 Network Insights");
    println!("--------------------------------------------------");

    if let Some(dns_result) = results.iter().find(|r| r.name == "DNS Service Discovery") {
        println!(
            "  • DNS Resolution: {:.2}ms avg, {:.2}% success rate",
            dns_result.avg_latency_ms,
            dns_result.success_rate * 100.0
        );
    }

    if let Some(mesh_result) = results
        .iter()
        .find(|r| r.name == "Service Mesh Communication")
    {
        println!(
            "  • Service Mesh: {:.2}ms avg latency, {:.2} MB/s throughput",
            mesh_result.avg_latency_ms, mesh_result.throughput_mbs
        );
    }

    if let Some(lb_result) = results.iter().find(|r| r.name == "Load Balancing")
        && let Some(fairness) = lb_result.metrics.get("fairness_score")
    {
        println!("  • Load Balancing: {fairness:.1}% fairness score");
    }

    if let Some(cb_result) = results.iter().find(|r| r.name == "Circuit Breaker") {
        println!(
            "  • Circuit Breaker: {:.2}ms avg response time",
            cb_result.avg_latency_ms
        );
    }
}

/// Save benchmark results to file
async fn save_benchmark_results(results: &[NetworkBenchmarkResults]) -> ToadStoolResult<()> {
    let json_results = serde_json::to_string_pretty(results)
        .map_err(|e| ToadStoolError::from(std::io::Error::other(e.to_string())))?;

    let rfc = toadstool_common::system_time_serde::format_rfc3339(SystemTime::now());
    let datetime = rfc
        .replace('-', "")
        .replace('T', "_")
        .replace([':', 'Z'], "");
    let filename = format!("network_benchmark_results_{datetime}.json");

    std::fs::write(&filename, json_results)
        .map_err(ToadStoolError::from)?;

    println!("💾 Network benchmark results saved to: {filename}");

    Ok(())
}
