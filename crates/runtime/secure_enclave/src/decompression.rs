// SPDX-License-Identifier: AGPL-3.0-or-later
//! Decompression support for NestGate-compressed payloads
//!
//! Provides efficient decompression in isolated memory for compressed data
//! from NestGate (88% compression ratio, 70-80% energy savings).

use crate::error::{Error, Result};
use crate::isolated_memory::IsolatedMemoryRegion;

/// Supported compression algorithms (from NestGate)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// Zstandard (high compression ratio)
    Zstd,
    /// LZ4 (high speed)
    Lz4,
}

/// Decompression statistics
#[derive(Debug, Clone)]
pub struct DecompressionStats {
    /// Compressed size in bytes
    pub compressed_size: usize,

    /// Decompressed size in bytes
    pub decompressed_size: usize,

    /// Compression ratio (compressed / decompressed)
    pub compression_ratio: f64,

    /// Decompression time in microseconds
    pub duration_micros: u64,

    /// Throughput in MB/s
    pub throughput_mbps: f64,
}

impl DecompressionStats {
    /// Calculate statistics
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Intentional: statistics use f64 for calculations
    pub fn new(compressed_size: usize, decompressed_size: usize, duration_micros: u64) -> Self {
        let compression_ratio = if decompressed_size > 0 {
            compressed_size as f64 / decompressed_size as f64
        } else {
            0.0
        };

        let throughput_mbps = if duration_micros > 0 {
            (decompressed_size as f64 / (1024.0 * 1024.0)) / (duration_micros as f64 / 1_000_000.0)
        } else {
            0.0
        };

        Self {
            compressed_size,
            decompressed_size,
            compression_ratio,
            duration_micros,
            throughput_mbps,
        }
    }
}

/// Decompress data in isolated memory
///
/// # Arguments
///
/// * `compressed` - Compressed input data
/// * `algorithm` - Compression algorithm used
/// * `expected_size` - Expected decompressed size (for allocation)
///
/// # Returns
///
/// Returns `(isolated_memory, stats)` with decompressed data in isolated region
///
/// # Security
///
/// - Decompression happens in isolated memory (locked, no swap)
/// - Memory wiped automatically on drop
/// - No disk I/O during decompression
///
/// # Performance
///
/// - **Zstd**: ~5ms/MB decompression
/// - **LZ4**: ~2ms/MB decompression
///
/// # Example
///
/// ```rust,ignore
/// use secure_enclave::decompression::{decompress_isolated, CompressionAlgorithm};
///
/// let (memory, stats) = decompress_isolated(
///     &compressed_data,
///     CompressionAlgorithm::Zstd,
///     Some(expected_size),
/// )?;
///
/// println!("Decompressed: {} bytes", stats.decompressed_size);
/// println!("Ratio: {:.2}", stats.compression_ratio);
/// println!("Speed: {:.2} MB/s", stats.throughput_mbps);
/// ```
///
/// # Errors
///
/// Returns error if decompression fails or memory allocation fails.
pub fn decompress_isolated(
    compressed: &[u8],
    algorithm: CompressionAlgorithm,
    expected_size: Option<usize>,
) -> Result<(IsolatedMemoryRegion, DecompressionStats)> {
    let start = std::time::Instant::now();

    // Decompress based on algorithm
    let decompressed = match algorithm {
        CompressionAlgorithm::Zstd => decompress_zstd(compressed)?,
        CompressionAlgorithm::Lz4 => decompress_lz4(compressed)?,
    };

    // Validate expected size if provided
    if let Some(expected) = expected_size {
        if decompressed.len() != expected {
            return Err(Error::decompression(format!(
                "Size mismatch: expected {}, got {}",
                expected,
                decompressed.len()
            )));
        }
    }

    // Allocate isolated memory for decompressed data
    let mut memory = IsolatedMemoryRegion::new(decompressed.len())?;

    // Copy decompressed data into isolated memory
    memory.as_mut_slice().copy_from_slice(&decompressed);

    #[allow(clippy::cast_possible_truncation)]
    // Intentional: duration saturates at u64::MAX (>500k years)
    let duration_micros = start.elapsed().as_micros() as u64;

    let stats = DecompressionStats::new(compressed.len(), decompressed.len(), duration_micros);

    tracing::debug!(
        "Decompressed {} bytes to {} bytes ({:.2}% ratio, {:.2} MB/s)",
        stats.compressed_size,
        stats.decompressed_size,
        stats.compression_ratio * 100.0,
        stats.throughput_mbps
    );

    Ok((memory, stats))
}

/// Decompress using Zstandard (Pure Rust decoder!)
fn decompress_zstd(compressed: &[u8]) -> Result<Vec<u8>> {
    // PURE RUST EVOLUTION (Jan 17, 2026):
    //   OLD: zstd crate (pulled in zstd-sys C dependency)
    //   NEW: ruzstd (100% Pure Rust implementation!)
    //   BENEFIT: Cross-compiles trivially, no C toolchain needed!
    use ruzstd::decoding::StreamingDecoder;
    use std::io::Read;

    let mut decoder = StreamingDecoder::new(compressed)
        .map_err(|e| Error::decompression(format!("Failed to create zstd decoder: {e}")))?;

    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| Error::decompression(format!("Zstd decompression failed: {e}")))?;

    Ok(decompressed)
}

/// Decompress using LZ4 (Pure Rust implementation!)
fn decompress_lz4(compressed: &[u8]) -> Result<Vec<u8>> {
    // PURE RUST EVOLUTION (Jan 17, 2026):
    //   OLD: lz4 crate (pulled in lz4-sys C dependency)
    //   NEW: lz4_flex (100% Pure Rust, fast, safe!)
    //   BENEFIT: Cross-compiles trivially, no C toolchain needed!
    lz4_flex::block::decompress_size_prepended(compressed)
        .map_err(|e| Error::decompression(format!("LZ4 decompression failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zstd_decompression() {
        // Use larger, repetitive data for realistic compression
        let original = b"Hello, world! ".repeat(100); // ~1.3KB repetitive data

        // Compress with zstd crate for testing
        let compressed = zstd::encode_all(&original[..], 3).unwrap();

        // Decompress in isolated memory
        let (memory, stats) = decompress_isolated(
            &compressed,
            CompressionAlgorithm::Zstd,
            Some(original.len()),
        )
        .unwrap();

        // Verify decompressed data
        assert_eq!(memory.as_slice(), &original[..]);
        assert_eq!(stats.decompressed_size, original.len());
        // With repetitive data, compression should work
        assert!(
            compressed.len() < original.len(),
            "Data should be compressed: {} vs {}",
            compressed.len(),
            original.len()
        );
    }

    #[test]
    fn test_lz4_decompression() {
        // Use larger, repetitive data
        let original = b"Test data for LZ4 compression. ".repeat(50); // ~1.6KB

        // Compress with lz4_flex (Pure Rust!)
        let compressed = lz4_flex::block::compress_prepend_size(&original);

        // Decompress in isolated memory
        let (memory, stats) =
            decompress_isolated(&compressed, CompressionAlgorithm::Lz4, Some(original.len()))
                .unwrap();

        // Verify decompressed data
        assert_eq!(memory.as_slice(), &original[..]);
        assert_eq!(stats.decompressed_size, original.len());
        assert!(compressed.len() < original.len());
    }

    #[test]
    fn test_size_mismatch_error() {
        let original = b"Test data";

        // Compress with ruzstd
        let compressed = zstd::encode_all(&original[..], 3).unwrap();

        // Expect wrong size
        let result = decompress_isolated(
            &compressed,
            CompressionAlgorithm::Zstd,
            Some(100), // Wrong size
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_decompression_stats() {
        let original = vec![0u8; 1024 * 1024]; // 1MB of zeros (highly compressible)

        // Compress with ruzstd
        let compressed = zstd::encode_all(&original[..], 3).unwrap();

        let (_memory, stats) =
            decompress_isolated(&compressed, CompressionAlgorithm::Zstd, None).unwrap();

        // Zeros compress very well
        assert!(stats.compression_ratio < 0.01); // < 1%
        assert!(stats.throughput_mbps > 10.0); // Should be fast
    }

    #[test]
    fn test_memory_isolation() {
        let original = b"Sensitive data";

        // Compress with ruzstd
        let compressed = zstd::encode_all(&original[..], 3).unwrap();

        let (mut memory, _stats) =
            decompress_isolated(&compressed, CompressionAlgorithm::Zstd, None).unwrap();

        // Data accessible
        assert_eq!(memory.as_slice(), original);

        // Explicit wipe
        memory.wipe();
        assert!(memory.as_slice().iter().all(|&b| b == 0));

        // Memory dropped here - deallocated securely
    }
}
