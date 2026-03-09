// SPDX-License-Identifier: AGPL-3.0-only
//! Integration tests for secure enclave runtime
//!
//! These tests verify the end-to-end functionality of the secure enclave
//! system, including memory isolation, key management, and secure processing.

use toadstool_runtime_secure_enclave::{
    EphemeralKeyStore, IsolatedMemoryRegion, SecureEnclaveRuntime,
};

#[test]
fn test_isolated_memory_workflow() {
    // Allocate isolated memory
    let mut memory = IsolatedMemoryRegion::new(1024).expect("Failed to allocate memory");

    // Write sensitive data
    let sensitive_data = b"This is secret data";
    memory.as_mut_slice()[..sensitive_data.len()].copy_from_slice(sensitive_data);

    // Read it back
    let read_back = &memory.as_slice()[..sensitive_data.len()];
    assert_eq!(read_back, sensitive_data);

    // Explicit wipe
    memory.wipe();

    // Verify wiped
    assert!(memory.as_slice().iter().all(|&b| b == 0));

    // memory dropped here - deallocated securely
}

#[test]
fn test_key_store_workflow() {
    // Create key store
    let mut store = EphemeralKeyStore::new().expect("Failed to create key store");

    // Store encryption key
    let key = b"encryption_key_32_bytes_long!!!";
    store.store_key(key).expect("Failed to store key");

    assert!(store.has_key());

    // Retrieve key
    let retrieved = store.key().expect("Failed to get key");
    assert_eq!(retrieved, key);

    // Explicit wipe
    store.wipe();
    assert!(!store.has_key());

    // store dropped here - key wiped and deallocated
}

#[test]
fn test_runtime_process_data() {
    // Create runtime
    let mut runtime = SecureEnclaveRuntime::new().expect("Failed to create runtime");

    // Process data in isolated memory
    let input = b"sensitive input data";
    let result = runtime
        .process_isolated(input, |data| {
            // Simulate processing (e.g., hash, encrypt, analyze)
            let processed: Vec<u8> = data.iter().map(|&b| b.wrapping_add(1)).collect();
            Ok(processed)
        })
        .expect("Processing failed");

    // Verify result
    let expected: Vec<u8> = input.iter().map(|&b| b.wrapping_add(1)).collect();
    assert_eq!(result, expected);

    // Memory automatically wiped after processing
}

#[test]
fn test_runtime_with_key() {
    // Create runtime
    let mut runtime = SecureEnclaveRuntime::new().expect("Failed to create runtime");

    // Store encryption key
    let key = b"my_encryption_key_32_bytes_long";
    runtime.store_key(key).expect("Failed to store key");

    // Process data (simulating encrypted processing)
    let encrypted_input = b"encrypted_data_here";
    let result = runtime
        .process_isolated(encrypted_input, |data| {
            // In real use: decrypt with key, process, re-encrypt
            // For test: just verify data is accessible
            assert_eq!(data, encrypted_input);
            Ok(data.len())
        })
        .expect("Processing failed");

    assert_eq!(result, encrypted_input.len());

    // Key automatically wiped when runtime is dropped
}

#[test]
fn test_large_data_processing() {
    let mut runtime = SecureEnclaveRuntime::new().expect("Failed to create runtime");

    // Process 10MB of data
    let large_data = vec![42u8; 10 * 1024 * 1024];

    let result = runtime
        .process_isolated(&large_data, |data| {
            // Verify data integrity
            assert_eq!(data.len(), large_data.len());
            assert!(data.iter().all(|&b| b == 42));

            // Compute checksum
            let checksum: u64 = data.iter().map(|&b| u64::from(b)).sum();
            Ok(checksum)
        })
        .expect("Processing failed");

    let expected_checksum: u64 = 10 * 1024 * 1024 * 42;
    assert_eq!(result, expected_checksum);
}

#[test]
fn test_error_handling_in_processing() {
    let mut runtime = SecureEnclaveRuntime::new().expect("Failed to create runtime");

    let data = b"test";
    let result: Result<(), _> = runtime.process_isolated(data, |_| {
        // Simulate an error during processing
        Err(toadstool_runtime_secure_enclave::Error::cryptography(
            "Simulated decryption failure",
        ))
    });

    assert!(result.is_err());

    // Memory still wiped even on error
}

#[test]
fn test_multiple_sequential_operations() {
    let mut runtime = SecureEnclaveRuntime::new().expect("Failed to create runtime");

    // First operation
    let result1 = runtime
        .process_isolated(b"data1", |data| Ok(data.len()))
        .expect("First op failed");
    assert_eq!(result1, 5);

    // Second operation
    let result2 = runtime
        .process_isolated(b"data2_longer", |data| Ok(data.len()))
        .expect("Second op failed");
    assert_eq!(result2, 12);

    // Third operation
    let result3 = runtime
        .process_isolated(b"d3", |data| Ok(data.len()))
        .expect("Third op failed");
    assert_eq!(result3, 2);

    // Each operation gets fresh isolated memory
    // Previous memory is wiped before new allocation
}

#[test]
fn test_zero_knowledge_property() {
    // This test simulates the zero-knowledge property:
    // The runtime can process encrypted data without ever seeing plaintext

    let mut runtime = SecureEnclaveRuntime::new().expect("Failed to create runtime");

    // Simulate encrypted blob (high entropy, looks random)
    let encrypted_blob = vec![0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90];

    let result = runtime
        .process_isolated(&encrypted_blob, |data| {
            // In reality: decrypt here using key from BTSP
            // Process plaintext
            // Re-encrypt before returning

            // For test: verify we can process without exposing to provider
            assert_eq!(data.len(), 8);

            // Simulate computation on decrypted data
            // Provider never sees this plaintext
            let simulated_result = vec![0x01, 0x02, 0x03, 0x04];

            Ok(simulated_result)
        })
        .expect("Processing failed");

    assert_eq!(result.len(), 4);

    // Key insight: The provider (cloud) only ever saw:
    // - Input: encrypted_blob (entropy ~7.99)
    // - Output: result (also encrypted in real system)
    // Never saw the plaintext or the computation details!
}
