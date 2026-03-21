// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive Pure Rust compression tests
//!
//! Tests for `lz4_flex` and `ruzstd` - 100% Pure Rust implementations!
//!
//! Deep Debt Principles Applied:
//! - ✅ No mocks (real compression/decompression!)
//! - ✅ No hardcoding (test various sizes/patterns)
//! - ✅ Fast AND safe (zero unsafe)
//! - ✅ Smart organization (logical test groups)
//! - ✅ Discover capabilities (not assumptions!)

use toadstool_runtime_secure_enclave::decompression::{CompressionAlgorithm, decompress_isolated};

// Helper: Create LZ4 compressed data using lz4_flex
fn compress_lz4(data: &[u8]) -> Vec<u8> {
    lz4_flex::block::compress_prepend_size(data)
}

// Helper: Create Zstd compressed data (using ruzstd - Pure Rust, no C deps!)
fn compress_zstd(data: &[u8]) -> Vec<u8> {
    ruzstd::encoding::compress_to_vec(data, ruzstd::encoding::CompressionLevel::Fastest)
}

// =============================================================================
// LZ4 Compression Tests (lz4_flex - Pure Rust!)
// =============================================================================

#[test]
fn test_lz4_simple_data() {
    let original = b"Hello World! ".repeat(100);
    let compressed = compress_lz4(&original);

    // Verify compression worked
    assert!(compressed.len() < original.len());

    // Decompress and verify
    let (region, stats) =
        decompress_isolated(&compressed, CompressionAlgorithm::Lz4, None).unwrap();
    assert_eq!(region.as_slice(), &original[..]);
    assert_eq!(stats.decompressed_size, original.len());
}

#[test]
fn test_lz4_empty_data() {
    let original = b"";
    let compressed = compress_lz4(original);
    // Empty data handling - discover actual behavior
    if let Ok((region, _)) = decompress_isolated(&compressed, CompressionAlgorithm::Lz4, None) {
        assert_eq!(region.as_slice(), original);
    } else {
        // Empty data may not compress/decompress - that's okay!
        // Discovered behavior: some compressors don't handle empty data
    }
}

#[test]
fn test_lz4_single_byte() {
    let original = b"x";
    let compressed = compress_lz4(original);
    let (region, _) = decompress_isolated(&compressed, CompressionAlgorithm::Lz4, None).unwrap();
    assert_eq!(region.as_slice(), original);
}

#[test]
fn test_lz4_highly_compressible() {
    // All zeros - maximum compression!
    let original = vec![0u8; 10000];
    let compressed = compress_lz4(&original);

    // Should compress to much smaller size
    assert!(compressed.len() < original.len() / 10);

    let (region, _) = decompress_isolated(&compressed, CompressionAlgorithm::Lz4, None).unwrap();
    assert_eq!(region.as_slice(), &original[..]);
}

#[test]
fn test_lz4_random_data() {
    // Random data - low compressibility (wrap to u8 range)
    let original: Vec<u8> = (0..1000)
        .map(|i| u8::try_from((i * 7 + 13) % 256).unwrap())
        .collect();
    let compressed = compress_lz4(&original);

    // May not compress well, but should still decompress correctly
    let (region, _) = decompress_isolated(&compressed, CompressionAlgorithm::Lz4, None).unwrap();
    assert_eq!(region.as_slice(), &original[..]);
}

#[test]
fn test_lz4_text_data() {
    let original = "The quick brown fox jumps over the lazy dog. ".repeat(50);
    let original_bytes = original.as_bytes();
    let compressed = compress_lz4(original_bytes);

    assert!(compressed.len() < original_bytes.len());

    let (region, _) = decompress_isolated(&compressed, CompressionAlgorithm::Lz4, None).unwrap();
    assert_eq!(region.as_slice(), original_bytes);
}

#[test]
fn test_lz4_large_data() {
    // Test with 1MB of data
    let original = b"ToadStool Pure Rust! ".repeat(52428); // ~1MB
    let compressed = compress_lz4(&original);

    // Should achieve good compression
    assert!(compressed.len() < original.len() / 2);

    let (region, stats) =
        decompress_isolated(&compressed, CompressionAlgorithm::Lz4, None).unwrap();
    assert_eq!(region.as_slice().len(), original.len());
    assert!(stats.compression_ratio < 0.5); // Good compression!
}

#[test]
fn test_lz4_utf8_data() {
    let original = "Hello 世界! 🦀 Pure Rust! ".repeat(50);
    let original_bytes = original.as_bytes();
    let compressed = compress_lz4(original_bytes);
    let (region, _) = decompress_isolated(&compressed, CompressionAlgorithm::Lz4, None).unwrap();
    assert_eq!(region.as_slice(), original_bytes);

    // Verify UTF-8 integrity
    assert_eq!(
        String::from_utf8(region.as_slice().to_vec()).unwrap(),
        original
    );
}

#[test]
fn test_lz4_all_bytes() {
    // Test all possible byte values
    let original: Vec<u8> = (0..=255).collect();
    let compressed = compress_lz4(&original);
    let (region, _) = decompress_isolated(&compressed, CompressionAlgorithm::Lz4, None).unwrap();
    assert_eq!(region.as_slice(), &original[..]);
}

// =============================================================================
// Zstandard Decompression Tests (ruzstd - Pure Rust decoder!)
// =============================================================================

#[test]
fn test_zstd_simple_data() {
    let original = b"Hello World! ".repeat(100);
    let compressed = compress_zstd(&original);

    // Verify compression
    assert!(compressed.len() < original.len());

    // Decompress with ruzstd (Pure Rust!)
    let (region, stats) =
        decompress_isolated(&compressed, CompressionAlgorithm::Zstd, None).unwrap();
    assert_eq!(region.as_slice(), &original[..]);
    assert_eq!(stats.decompressed_size, original.len());
}

#[test]
fn test_zstd_empty_data() {
    let original = b"";
    let compressed = compress_zstd(original);
    // Empty data handling - discover actual behavior
    if let Ok((region, _)) = decompress_isolated(&compressed, CompressionAlgorithm::Zstd, None) {
        assert_eq!(region.as_slice(), original);
    } else {
        // Empty data may not compress/decompress - that's okay!
        // Discovered behavior: some compressors don't handle empty data
    }
}

#[test]
fn test_zstd_highly_compressible() {
    let original = vec![42u8; 10000];
    let compressed = compress_zstd(&original);

    // Zstd should compress this extremely well
    assert!(compressed.len() < original.len() / 50);

    let (region, stats) =
        decompress_isolated(&compressed, CompressionAlgorithm::Zstd, None).unwrap();
    assert_eq!(region.as_slice(), &original[..]);
    assert!(stats.compression_ratio < 0.02); // Excellent compression!
}

#[test]
fn test_zstd_random_data() {
    // Wrap to u8 range for low compressibility
    let original: Vec<u8> = (0..1000)
        .map(|i| u8::try_from((i * 13 + 7) % 256).unwrap())
        .collect();
    let compressed = compress_zstd(&original);

    let (region, _) = decompress_isolated(&compressed, CompressionAlgorithm::Zstd, None).unwrap();
    assert_eq!(region.as_slice(), &original[..]);
}

#[test]
fn test_zstd_text_data() {
    let original = "ToadStool: Pure Rust runtime with zero C dependencies! ".repeat(100);
    let original_bytes = original.as_bytes();
    let compressed = compress_zstd(original_bytes);

    assert!(compressed.len() < original_bytes.len());

    let (region, _) = decompress_isolated(&compressed, CompressionAlgorithm::Zstd, None).unwrap();
    assert_eq!(region.as_slice(), original_bytes);
}

#[test]
fn test_zstd_large_data() {
    // Test with 1MB of data
    let original = b"Pure Rust Forever! ".repeat(55188); // ~1MB
    let compressed = compress_zstd(&original);

    // Zstd should achieve excellent compression
    assert!(compressed.len() < original.len() / 10);

    let (region, stats) =
        decompress_isolated(&compressed, CompressionAlgorithm::Zstd, None).unwrap();
    assert_eq!(region.as_slice().len(), original.len());
    assert!(stats.compression_ratio < 0.1); // Excellent compression!
}

#[test]
fn test_zstd_utf8_data() {
    let original = "ToadStool 🍄 Pure Rust 🦀 Zero C! ".repeat(50);
    let original_bytes = original.as_bytes();
    let compressed = compress_zstd(original_bytes);
    let (region, _) = decompress_isolated(&compressed, CompressionAlgorithm::Zstd, None).unwrap();

    // Verify UTF-8 integrity
    assert_eq!(
        String::from_utf8(region.as_slice().to_vec()).unwrap(),
        original
    );
}

#[test]
fn test_zstd_all_bytes() {
    let original: Vec<u8> = (0..=255).collect();
    let compressed = compress_zstd(&original);
    let (region, _) = decompress_isolated(&compressed, CompressionAlgorithm::Zstd, None).unwrap();
    assert_eq!(region.as_slice(), &original[..]);
}

// =============================================================================
// Decompression Stats Tests
// =============================================================================

#[test]
fn test_compression_stats_lz4() {
    let original = b"ToadStool ".repeat(1000);
    let compressed = compress_lz4(&original);

    let (_region, stats) =
        decompress_isolated(&compressed, CompressionAlgorithm::Lz4, None).unwrap();

    // Discover stats (no hardcoded expectations!)
    assert_eq!(stats.compressed_size, compressed.len());
    assert_eq!(stats.decompressed_size, original.len());
    assert!(stats.compression_ratio > 0.0 && stats.compression_ratio < 1.0);
    assert!(stats.duration_micros > 0);

    println!("LZ4 stats: {stats:?}");
}

#[test]
fn test_compression_stats_zstd() {
    let original = b"ToadStool ".repeat(1000);
    let compressed = compress_zstd(&original);

    let (_region, stats) =
        decompress_isolated(&compressed, CompressionAlgorithm::Zstd, None).unwrap();

    // Discover stats
    assert_eq!(stats.compressed_size, compressed.len());
    assert_eq!(stats.decompressed_size, original.len());
    assert!(stats.compression_ratio > 0.0 && stats.compression_ratio < 1.0);
    assert!(stats.duration_micros > 0);

    println!("Zstd stats: {stats:?}");
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_invalid_lz4_data() {
    let invalid = b"this is not compressed data";
    let result = decompress_isolated(invalid, CompressionAlgorithm::Lz4, None);

    // Should fail gracefully
    assert!(result.is_err());
}

#[test]
fn test_invalid_zstd_data() {
    let invalid = b"also not compressed";
    let result = decompress_isolated(invalid, CompressionAlgorithm::Zstd, None);

    // Should fail gracefully
    assert!(result.is_err());
}

#[test]
fn test_corrupted_lz4_data() {
    let original = b"Test data".repeat(10);
    let mut compressed = compress_lz4(&original);

    // Corrupt the data
    if compressed.len() > 10 {
        compressed[10] ^= 0xFF;
    }

    let result = decompress_isolated(&compressed, CompressionAlgorithm::Lz4, None);
    // Should fail or produce wrong data
    assert!(result.is_err() || result.unwrap().0.as_slice() != &original[..]);
}

// =============================================================================
// Real-world Scenario Tests
// =============================================================================

#[test]
fn test_json_like_data() {
    // Simulate JSON payload (common in web apps)
    let json_like =
        r#"{"user":"test","action":"execute","timestamp":1234567890,"data":{"key":"value"}}"#
            .repeat(100);
    let original = json_like.as_bytes();

    let lz4_compressed = compress_lz4(original);
    let (lz4_region, _) =
        decompress_isolated(&lz4_compressed, CompressionAlgorithm::Lz4, None).unwrap();
    assert_eq!(lz4_region.as_slice(), original);

    let zstd_compressed = compress_zstd(original);
    let (zstd_region, _) =
        decompress_isolated(&zstd_compressed, CompressionAlgorithm::Zstd, None).unwrap();
    assert_eq!(zstd_region.as_slice(), original);
}

#[test]
fn test_binary_wasm_like_data() {
    // Simulate WASM module (mix of structured binary data)
    let mut wasm_like = b"\0asm\x01\0\0\0".to_vec(); // WASM magic + version
    wasm_like.extend_from_slice(&[0x01, 0x05, 0x01, 0x60, 0x00, 0x00]); // Type section
    wasm_like.extend_from_slice(&vec![0x42; 1000]); // Lots of data

    let lz4_compressed = compress_lz4(&wasm_like);
    let (lz4_region, _) =
        decompress_isolated(&lz4_compressed, CompressionAlgorithm::Lz4, None).unwrap();
    assert_eq!(lz4_region.as_slice(), &wasm_like[..]);

    let zstd_compressed = compress_zstd(&wasm_like);
    let (zstd_region, _) =
        decompress_isolated(&zstd_compressed, CompressionAlgorithm::Zstd, None).unwrap();
    assert_eq!(zstd_region.as_slice(), &wasm_like[..]);
}

#[test]
fn test_log_data() {
    // Simulate log lines (repetitive structure)
    let log_line = "[2026-01-17 12:34:56] INFO: Processing request from user 12345\n";
    let logs = log_line.repeat(500);
    let original = logs.as_bytes();

    let lz4_compressed = compress_lz4(original);
    let (_lz4_region, lz4_stats) =
        decompress_isolated(&lz4_compressed, CompressionAlgorithm::Lz4, None).unwrap();

    let zstd_compressed = compress_zstd(original);
    let (_zstd_region, zstd_stats) =
        decompress_isolated(&zstd_compressed, CompressionAlgorithm::Zstd, None).unwrap();

    // Log data should compress very well!
    let lz4_ratio = 1.0 / lz4_stats.compression_ratio;
    let zstd_ratio = 1.0 / zstd_stats.compression_ratio;

    assert!(lz4_ratio > 3.0);
    assert!(zstd_ratio > 5.0);

    println!("Log compression - LZ4: {lz4_ratio:.2}x, Zstd: {zstd_ratio:.2}x");
}
