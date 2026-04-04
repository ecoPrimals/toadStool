// SPDX-License-Identifier: AGPL-3.0-only
//! Resource limit constants
//!
//! Default resource limits for workloads, biomes, and system operations.

// ============================================================================
// Memory Limits
// ============================================================================

/// Default memory limit for biomes (512 MB)
pub const DEFAULT_MEMORY_LIMIT_MB: u64 = 512;

/// Minimum memory limit (64 MB)
pub const MIN_MEMORY_LIMIT_MB: u64 = 64;

/// Maximum memory limit (16 GB)
pub const MAX_MEMORY_LIMIT_MB: u64 = 16384;

/// Default memory limit string
pub const DEFAULT_MEMORY_LIMIT: &str = "512Mi";

// ============================================================================
// CPU Limits
// ============================================================================

/// Default CPU limit (1.0 cores)
pub const DEFAULT_CPU_LIMIT: f64 = 1.0;

/// Minimum CPU limit (0.1 cores)
pub const MIN_CPU_LIMIT: f64 = 0.1;

/// Maximum CPU limit (16.0 cores)
pub const MAX_CPU_LIMIT: f64 = 16.0;

/// Default CPU shares
pub const DEFAULT_CPU_SHARES: u32 = 1024;

// ============================================================================
// Storage Limits
// ============================================================================

/// Default storage limit (1 GB)
pub const DEFAULT_STORAGE_LIMIT_GB: u64 = 1;

/// Minimum storage limit (100 MB)
pub const MIN_STORAGE_LIMIT_MB: u64 = 100;

/// Maximum storage limit (1 TB)
pub const MAX_STORAGE_LIMIT_GB: u64 = 1024;

/// Default temporary storage limit (500 MB)
pub const DEFAULT_TMP_STORAGE_MB: u64 = 500;

// ============================================================================
// Network Limits
// ============================================================================

/// Default network bandwidth limit (100 Mbps)
pub const DEFAULT_NETWORK_BANDWIDTH_MBPS: u64 = 100;

/// Maximum concurrent connections
pub const MAX_CONCURRENT_CONNECTIONS: u32 = 1000;

/// Connection pool size
pub const DEFAULT_POOL_SIZE: u32 = 10;

/// Maximum connection pool size
pub const MAX_POOL_SIZE: u32 = 100;

// ============================================================================
// Concurrency Limits
// ============================================================================

/// Default worker thread count
pub const DEFAULT_WORKER_THREADS: usize = 4;

/// Maximum worker thread count
pub const MAX_WORKER_THREADS: usize = 128;

/// Default task queue size
pub const DEFAULT_TASK_QUEUE_SIZE: usize = 1000;

/// Maximum task queue size
pub const MAX_TASK_QUEUE_SIZE: usize = 100_000;

// ============================================================================
// File Limits
// ============================================================================

/// Maximum file size for uploads (100 MB)
pub const MAX_UPLOAD_SIZE_MB: u64 = 100;

/// Maximum manifest file size (1 MB)
pub const MAX_MANIFEST_SIZE_KB: u64 = 1024;

/// Maximum log file size (10 MB)
pub const MAX_LOG_FILE_SIZE_MB: u64 = 10;

/// Maximum number of open files
pub const MAX_OPEN_FILES: u64 = 1024;

// ============================================================================
// Cache Limits
// ============================================================================

/// Default cache size (100 MB)
pub const DEFAULT_CACHE_SIZE_MB: u64 = 100;

/// Maximum cache entries
pub const MAX_CACHE_ENTRIES: usize = 10000;

/// WASM module cache size (500 MB)
pub const WASM_CACHE_SIZE_MB: u64 = 500;

// ============================================================================
// Process Limits
// ============================================================================

/// Maximum biomes per host
pub const MAX_BIOMES_PER_HOST: u32 = 100;

/// Maximum processes per biome
pub const MAX_PROCESSES_PER_BIOME: u32 = 10;

/// Default process priority
pub const DEFAULT_PROCESS_PRIORITY: i32 = 0;

// ============================================================================
// Buffer Sizes
// ============================================================================

/// Default buffer size (64 KB)
pub const DEFAULT_BUFFER_SIZE_KB: usize = 64;

/// `WebSocket` message buffer size (1 MB)
pub const WS_BUFFER_SIZE_KB: usize = 1024;

/// Log buffer size (10 KB)
pub const LOG_BUFFER_SIZE_KB: usize = 10;

// ============================================================================
// Rate Limits
// ============================================================================

/// API requests per second
pub const API_RATE_LIMIT_PER_SEC: u32 = 100;

/// API requests per minute
pub const API_RATE_LIMIT_PER_MIN: u32 = 6000;

/// Maximum burst size
pub const API_BURST_LIMIT: u32 = 200;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[expect(
    clippy::assertions_on_constants,
    reason = "compile-time assertion by design"
)] // Constants validated at compile time serve as documentation
mod tests {
    use super::*;

    #[test]
    fn test_memory_min_default_max_ordering() {
        assert!(MIN_MEMORY_LIMIT_MB <= DEFAULT_MEMORY_LIMIT_MB);
        assert!(DEFAULT_MEMORY_LIMIT_MB <= MAX_MEMORY_LIMIT_MB);
    }

    #[test]
    fn test_cpu_min_default_max_ordering() {
        assert!(MIN_CPU_LIMIT <= DEFAULT_CPU_LIMIT);
        assert!(DEFAULT_CPU_LIMIT <= MAX_CPU_LIMIT);
    }

    #[test]
    fn test_storage_limits_ordering() {
        let min_storage_mb = MIN_STORAGE_LIMIT_MB;
        let default_storage_mb = DEFAULT_STORAGE_LIMIT_GB * 1024;
        let max_storage_mb = MAX_STORAGE_LIMIT_GB * 1024;
        assert!(min_storage_mb <= default_storage_mb);
        assert!(default_storage_mb <= max_storage_mb);
    }

    #[test]
    fn test_tmp_storage_within_storage_bounds() {
        assert!(DEFAULT_TMP_STORAGE_MB >= MIN_STORAGE_LIMIT_MB);
        assert!(DEFAULT_TMP_STORAGE_MB <= MAX_STORAGE_LIMIT_GB * 1024);
    }

    #[test]
    fn test_pool_size_ordering() {
        assert!(DEFAULT_POOL_SIZE <= MAX_POOL_SIZE);
    }

    #[test]
    fn test_worker_thread_ordering() {
        assert!(DEFAULT_WORKER_THREADS <= MAX_WORKER_THREADS);
    }

    #[test]
    fn test_task_queue_size_ordering() {
        assert!(DEFAULT_TASK_QUEUE_SIZE <= MAX_TASK_QUEUE_SIZE);
    }

    #[test]
    fn test_rate_limit_per_sec_times_60_le_per_min() {
        assert!(u64::from(API_RATE_LIMIT_PER_SEC) * 60 <= u64::from(API_RATE_LIMIT_PER_MIN));
    }

    #[test]
    fn test_default_memory_sensible() {
        assert!(DEFAULT_MEMORY_LIMIT_MB >= 64);
        assert!(DEFAULT_MEMORY_LIMIT_MB <= 4096);
    }

    #[test]
    fn test_default_cpu_sensible() {
        assert!(DEFAULT_CPU_LIMIT >= 0.1);
        assert!(DEFAULT_CPU_LIMIT <= 16.0);
    }

    #[test]
    fn test_all_maximums_positive() {
        assert!(MAX_MEMORY_LIMIT_MB > 0);
        assert!(MAX_CPU_LIMIT > 0.0);
        assert!(MAX_STORAGE_LIMIT_GB > 0);
        assert!(MAX_POOL_SIZE > 0);
        assert!(MAX_WORKER_THREADS > 0);
        assert!(MAX_TASK_QUEUE_SIZE > 0);
        assert!(MAX_CACHE_ENTRIES > 0);
    }

    #[test]
    fn test_cpu_limits_sensible_range() {
        assert!(MIN_CPU_LIMIT > 0.0);
        assert!(MAX_CPU_LIMIT <= 256.0);
    }

    #[test]
    fn test_buffer_sizes_positive() {
        assert!(DEFAULT_BUFFER_SIZE_KB > 0);
        assert!(WS_BUFFER_SIZE_KB > 0);
        assert!(LOG_BUFFER_SIZE_KB > 0);
    }

    #[test]
    fn test_buffer_sizes_reasonable() {
        assert!(DEFAULT_BUFFER_SIZE_KB <= 1024);
        assert!(WS_BUFFER_SIZE_KB <= 8192);
        assert!(LOG_BUFFER_SIZE_KB <= 1024);
    }

    #[test]
    fn test_file_limits_positive() {
        assert!(MAX_UPLOAD_SIZE_MB > 0);
        assert!(MAX_MANIFEST_SIZE_KB > 0);
        assert!(MAX_LOG_FILE_SIZE_MB > 0);
        assert!(MAX_OPEN_FILES > 0);
    }

    #[test]
    fn test_cache_limits_sensible() {
        assert!(DEFAULT_CACHE_SIZE_MB > 0);
        assert!(MAX_CACHE_ENTRIES > 0);
        assert!(WASM_CACHE_SIZE_MB > 0);
    }

    #[test]
    fn test_process_limits_positive() {
        assert!(MAX_BIOMES_PER_HOST > 0);
        assert!(MAX_PROCESSES_PER_BIOME > 0);
    }

    #[test]
    fn test_api_burst_limit_within_rate_limits() {
        assert!(API_BURST_LIMIT >= API_RATE_LIMIT_PER_SEC);
    }
}
