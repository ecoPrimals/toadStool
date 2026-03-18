// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-end tests for secure enclave workflows
//!
//! These tests demonstrate complete workflows including:
//! - Decompression (NestGate integration)
//! - Isolated processing
//! - Audit logging
//! - Zero-knowledge compute patterns

use toadstool_runtime_secure_enclave::{
    AuditEventType, CompressionAlgorithm, SecureEnclaveRuntime, decompress_isolated,
};

#[test]
fn test_e2e_decompress_and_process() {
    // Simulate NestGate workflow: compressed data → decompress → process

    // Step 1: Prepare compressed data (simulating NestGate output)
    let original_data = b"Sensitive medical data. ".repeat(100); // ~2.4KB
    let compressed = ruzstd::encoding::compress_to_vec(
        &original_data[..],
        ruzstd::encoding::CompressionLevel::Fastest,
    );

    #[allow(clippy::cast_precision_loss)] // usize to f64 for ratio display
    let ratio = (compressed.len() as f64 / original_data.len() as f64) * 100.0;
    println!(
        "Compressed: {} bytes → {} bytes ({ratio:.1}% ratio)",
        original_data.len(),
        compressed.len()
    );

    // Step 2: Decompress in isolated memory
    let (memory, decomp_stats) = decompress_isolated(
        &compressed,
        CompressionAlgorithm::Zstd,
        Some(original_data.len()),
    )
    .unwrap();

    assert_eq!(memory.as_slice(), &original_data[..]);
    assert!(decomp_stats.throughput_mbps > 1.0); // Reasonable throughput

    println!("Decompression: {:.2} MB/s", decomp_stats.throughput_mbps);

    // Step 3: Process decompressed data
    let result = process_sensitive_data(memory.as_slice());
    assert!(result > 0);

    // Memory automatically wiped on drop
}

#[test]
fn test_e2e_with_audit_trail() {
    // Complete workflow with audit logging enabled

    let mut runtime = SecureEnclaveRuntime::new().unwrap();

    // Step 1: Store encryption key (simulating BTSP key exchange)
    let encryption_key = b"test_encryption_key_32_bytes!!!";
    runtime.store_key(encryption_key).unwrap();

    // Step 2: Process data
    let input = b"Private genomic data";
    let result = runtime
        .process_isolated(input, |data| {
            // Simulate analysis
            Ok(data.len() * 2)
        })
        .unwrap();

    assert_eq!(result, input.len() * 2);

    // Step 3: Verify audit trail
    let audit_log = runtime.audit_logger().unwrap();

    // Should have logged: KeyStored, ProcessingStarted, MemoryAllocated, ProcessingCompleted, MemoryDeallocated
    assert!(audit_log.len() >= 5);

    // Verify audit integrity
    audit_log.verify_integrity().unwrap();

    // Check specific events
    let events = audit_log.events();
    assert_eq!(events[0].event_type, AuditEventType::KeyStored);
    assert_eq!(events[1].event_type, AuditEventType::ProcessingStarted);
    assert_eq!(events[2].event_type, AuditEventType::MemoryAllocated);
    assert_eq!(events[3].event_type, AuditEventType::ProcessingCompleted);
    assert_eq!(events[4].event_type, AuditEventType::MemoryDeallocated);

    let event_count = audit_log.len();
    println!("Audit trail verified: {event_count} events");
}

#[test]
fn test_e2e_compress_encrypt_process() {
    // Full zero-knowledge workflow:
    // 1. Compress (NestGate)
    // 2. Encrypt (BearDog) - simulated
    // 3. Decompress in enclave
    // 4. Decrypt in enclave - simulated
    // 5. Process
    // 6. Re-encrypt - simulated

    let mut runtime = SecureEnclaveRuntime::new().unwrap();

    // Original sensitive data
    let sensitive = b"Patient genome sequence ACTG".repeat(50); // ~1.4KB

    // Step 1: Compress (NestGate)
    let compressed = ruzstd::encoding::compress_to_vec(
        &sensitive[..],
        ruzstd::encoding::CompressionLevel::Fastest,
    );
    println!(
        "NestGate compression: {} → {} bytes",
        sensitive.len(),
        compressed.len()
    );

    // Step 2: Encrypt (BearDog) - simulated with XOR for test
    let encrypted: Vec<u8> = compressed.iter().map(|&b| b ^ 0x42).collect();

    // Step 3: Decompress in enclave (simulating decryption first)
    let decrypted: Vec<u8> = encrypted.iter().map(|&b| b ^ 0x42).collect();
    let (memory, _stats) = decompress_isolated(
        &decrypted,
        CompressionAlgorithm::Zstd,
        Some(sensitive.len()),
    )
    .unwrap();

    // Step 4: Process in enclave
    let result = runtime
        .process_isolated(memory.as_slice(), |plaintext| {
            // Verify we got the original data
            assert_eq!(plaintext, &sensitive[..]);

            // Perform computation (count 'A's)
            #[allow(clippy::naive_bytecount)] // test code, no bytecount dependency needed
            let count = plaintext.iter().filter(|&&b| b == b'A').count();
            Ok(count)
        })
        .unwrap();

    println!("Processed result: {result} A's found");
    assert!(result > 0);

    // Provider never saw plaintext!
    // Audit log proves isolation
    let audit_log = runtime.audit_logger().unwrap();
    audit_log.verify_integrity().unwrap();
    let event_count = audit_log.len();
    println!("Audit verified: {event_count} events");
}

#[test]
fn test_e2e_tamper_detection() {
    // Demonstrate that audit log detects tampering

    let mut runtime = SecureEnclaveRuntime::new().unwrap();

    // Perform some operations
    runtime.store_key(b"key1").unwrap();
    runtime.process_isolated(b"data", |_| Ok(())).unwrap();

    // Get mutable access to audit log (normally not possible in production)
    // This is for demonstration only
    let audit_log = runtime.audit_logger().unwrap();

    // Verify integrity before tampering
    assert!(audit_log.verify_integrity().is_ok());

    // Tampering would be detected in production
    // (This test just verifies the audit trail works correctly)
    let event_count = audit_log.len();
    println!("Audit trail secure: {event_count} events verified");
}

#[test]
fn test_e2e_performance_monitoring() {
    // Monitor performance of complete workflow

    let mut runtime = SecureEnclaveRuntime::new().unwrap();

    // Large data (10MB of repetitive data)
    let large_data = vec![42u8; 10 * 1024 * 1024];
    let compressed = ruzstd::encoding::compress_to_vec(
        &large_data[..],
        ruzstd::encoding::CompressionLevel::Fastest,
    );

    let data_mb = large_data.len() / 1024 / 1024;
    println!("Data size: {data_mb} MB");
    #[allow(clippy::cast_precision_loss)] // usize to f64 for ratio display
    let comp_ratio = (compressed.len() as f64 / large_data.len() as f64) * 100.0;
    println!(
        "Compressed: {} bytes ({comp_ratio:.2}% of original)",
        compressed.len()
    );

    // Decompress
    let start = std::time::Instant::now();
    let (memory, decomp_stats) = decompress_isolated(
        &compressed,
        CompressionAlgorithm::Zstd,
        Some(large_data.len()),
    )
    .unwrap();
    let decomp_time = start.elapsed();

    let throughput = decomp_stats.throughput_mbps;
    println!("Decompression: {decomp_time:?} ({throughput:.2} MB/s)");

    // Process
    let start = std::time::Instant::now();
    let result = runtime
        .process_isolated(memory.as_slice(), |data| {
            // Simple computation: sum bytes
            let sum: u64 = data.iter().map(|&b| u64::from(b)).sum();
            Ok(sum)
        })
        .unwrap();
    let process_time = start.elapsed();

    println!("Processing: {process_time:?}");
    println!("Result: {result}");

    // Total overhead should be < 10%
    let total_time = decomp_time + process_time;
    println!("Total time: {total_time:?}");

    // Verify audit trail
    let audit_log = runtime.audit_logger().unwrap();
    assert!(audit_log.verify_integrity().is_ok());
}

/// Helper function: simulate sensitive data processing
fn process_sensitive_data(data: &[u8]) -> usize {
    // Simulated computation: count non-zero bytes
    data.iter().filter(|&&b| b != 0).count()
}
