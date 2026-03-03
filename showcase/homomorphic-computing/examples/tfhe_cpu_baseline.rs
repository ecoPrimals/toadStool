// SPDX-License-Identifier: AGPL-3.0-or-later
// 🔐 TFHE-rs CPU Baseline Validation
// ⚠️ VALIDATION HARNESS ONLY - NOT PRODUCTION CODE
//
// This validates ToadStool's CPU compute performance using
// public TFHE-rs encrypted operations as a benchmark reference.

use anyhow::Result;
use std::time::Instant;
use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ConfigBuilder, FheBool, FheUint16, FheUint8};

#[derive(Debug)]
struct BenchResult {
    operation: String,
    iterations: usize,
    encrypt_time_us: u128,
    compute_time_us: u128,
    decrypt_time_us: u128,
    throughput: f64,
}

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  TFHE-rs CPU Baseline Validation                        ║");
    println!("║  ⚠️  VALIDATION HARNESS - NOT PRODUCTION CODE  ⚠️       ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    println!("📊 Purpose: Validate ToadStool CPU compute against public benchmarks\n");
    println!("⚡ Setting up TFHE-rs...\n");

    // Generate keys (this is slow, done once)
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);
    set_server_key(server_key);

    println!("✅ Keys generated\n");

    // Run benchmarks
    println!("═══════════════════════════════════════════════════════════\n");
    println!("📊 Benchmark 1: Encrypted Boolean AND (10,000 ops)\n");

    let bool_result = bench_encrypted_bool_and(&client_key, 10_000)?;
    print_result(&bool_result);

    println!("\n═══════════════════════════════════════════════════════════\n");
    println!("📊 Benchmark 2: Encrypted 8-bit Addition (10,000 ops)\n");

    let add_result = bench_encrypted_u8_add(&client_key, 10_000)?;
    print_result(&add_result);

    println!("\n═══════════════════════════════════════════════════════════\n");
    println!("📊 Benchmark 3: Encrypted 8-bit Multiplication (1,000 ops)\n");

    let mul_result = bench_encrypted_u8_mul(&client_key, 1_000)?;
    print_result(&mul_result);

    println!("\n═══════════════════════════════════════════════════════════\n");
    println!("📊 Benchmark 4: Encrypted 16-bit Addition (5,000 ops)\n");

    let add16_result = bench_encrypted_u16_add(&client_key, 5_000)?;
    print_result(&add16_result);

    println!("\n═══════════════════════════════════════════════════════════\n");
    println!("🎯 CPU Baseline Validation Complete!\n");
    println!("Next Steps:");
    println!("  1. Run GPU validation: cargo run --example tfhe_gpu_validation --release");
    println!("  2. Run NPU validation: cargo run --example tfhe_npu_validation --release");
    println!("  3. Compare all: cargo run --example public_benchmark_comparison --release");
    println!("\n⚠️  This is validation infrastructure - ToadStool binary remains pure Rust!");

    Ok(())
}

fn bench_encrypted_bool_and(
    client_key: &tfhe::ClientKey,
    iterations: usize,
) -> Result<BenchResult> {
    let clear_a = true;
    let clear_b = false;

    // Encrypt
    let start = Instant::now();
    let enc_a = FheBool::encrypt(clear_a, client_key);
    let enc_b = FheBool::encrypt(clear_b, client_key);
    let encrypt_time = start.elapsed().as_micros();

    // Homomorphic AND operations
    let start = Instant::now();
    let mut enc_result = &enc_a & &enc_b;
    for _ in 1..iterations {
        enc_result = &enc_a & &enc_b;
    }
    let compute_time = start.elapsed().as_micros();

    // Decrypt
    let start = Instant::now();
    let result: bool = enc_result.decrypt(client_key);
    let decrypt_time = start.elapsed().as_micros();

    // Verify correctness
    assert_eq!(result, clear_a & clear_b);

    Ok(BenchResult {
        operation: "Encrypted Boolean AND".to_string(),
        iterations,
        encrypt_time_us: encrypt_time,
        compute_time_us: compute_time,
        decrypt_time_us: decrypt_time,
        throughput: (iterations as f64) / (compute_time as f64 / 1_000_000.0),
    })
}

fn bench_encrypted_u8_add(client_key: &tfhe::ClientKey, iterations: usize) -> Result<BenchResult> {
    let clear_a: u8 = 42;
    let clear_b: u8 = 128;

    // Encrypt
    let start = Instant::now();
    let enc_a = FheUint8::encrypt(clear_a, client_key);
    let enc_b = FheUint8::encrypt(clear_b, client_key);
    let encrypt_time = start.elapsed().as_micros();

    // Homomorphic addition
    let start = Instant::now();
    let mut enc_result = &enc_a + &enc_b;
    for _ in 1..iterations {
        enc_result = &enc_a + &enc_b;
    }
    let compute_time = start.elapsed().as_micros();

    // Decrypt
    let start = Instant::now();
    let result: u8 = enc_result.decrypt(client_key);
    let decrypt_time = start.elapsed().as_micros();

    // Verify correctness
    assert_eq!(result, clear_a.wrapping_add(clear_b));

    Ok(BenchResult {
        operation: "Encrypted u8 Addition".to_string(),
        iterations,
        encrypt_time_us: encrypt_time,
        compute_time_us: compute_time,
        decrypt_time_us: decrypt_time,
        throughput: (iterations as f64) / (compute_time as f64 / 1_000_000.0),
    })
}

fn bench_encrypted_u8_mul(client_key: &tfhe::ClientKey, iterations: usize) -> Result<BenchResult> {
    let clear_a: u8 = 5;
    let clear_b: u8 = 7;

    // Encrypt
    let start = Instant::now();
    let enc_a = FheUint8::encrypt(clear_a, client_key);
    let enc_b = FheUint8::encrypt(clear_b, client_key);
    let encrypt_time = start.elapsed().as_micros();

    // Homomorphic multiplication (more expensive!)
    let start = Instant::now();
    let mut enc_result = &enc_a * &enc_b;
    for _ in 1..iterations {
        enc_result = &enc_a * &enc_b;
    }
    let compute_time = start.elapsed().as_micros();

    // Decrypt
    let start = Instant::now();
    let result: u8 = enc_result.decrypt(client_key);
    let decrypt_time = start.elapsed().as_micros();

    // Verify correctness
    assert_eq!(result, clear_a.wrapping_mul(clear_b));

    Ok(BenchResult {
        operation: "Encrypted u8 Multiplication".to_string(),
        iterations,
        encrypt_time_us: encrypt_time,
        compute_time_us: compute_time,
        decrypt_time_us: decrypt_time,
        throughput: (iterations as f64) / (compute_time as f64 / 1_000_000.0),
    })
}

fn bench_encrypted_u16_add(client_key: &tfhe::ClientKey, iterations: usize) -> Result<BenchResult> {
    let clear_a: u16 = 1000;
    let clear_b: u16 = 2000;

    // Encrypt
    let start = Instant::now();
    let enc_a = FheUint16::encrypt(clear_a, client_key);
    let enc_b = FheUint16::encrypt(clear_b, client_key);
    let encrypt_time = start.elapsed().as_micros();

    // Homomorphic addition
    let start = Instant::now();
    let mut enc_result = &enc_a + &enc_b;
    for _ in 1..iterations {
        enc_result = &enc_a + &enc_b;
    }
    let compute_time = start.elapsed().as_micros();

    // Decrypt
    let start = Instant::now();
    let result: u16 = enc_result.decrypt(client_key);
    let decrypt_time = start.elapsed().as_micros();

    // Verify correctness
    assert_eq!(result, clear_a + clear_b);

    Ok(BenchResult {
        operation: "Encrypted u16 Addition".to_string(),
        iterations,
        encrypt_time_us: encrypt_time,
        compute_time_us: compute_time,
        decrypt_time_us: decrypt_time,
        throughput: (iterations as f64) / (compute_time as f64 / 1_000_000.0),
    })
}

fn print_result(result: &BenchResult) {
    println!("Operation: {}", result.operation);
    println!("Iterations: {}", result.iterations);
    println!("─────────────────────────────────────");
    println!(
        "Encrypt time:  {:>10} μs ({:.2} ms)",
        result.encrypt_time_us,
        result.encrypt_time_us as f64 / 1000.0
    );
    println!(
        "Compute time:  {:>10} μs ({:.2} ms)",
        result.compute_time_us,
        result.compute_time_us as f64 / 1000.0
    );
    println!(
        "Decrypt time:  {:>10} μs ({:.2} ms)",
        result.decrypt_time_us,
        result.decrypt_time_us as f64 / 1000.0
    );
    println!("─────────────────────────────────────");
    println!("Throughput:    {:>10.0} ops/sec", result.throughput);
    println!(
        "Avg latency:   {:>10.2} μs/op",
        result.compute_time_us as f64 / result.iterations as f64
    );
}
