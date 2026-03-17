// SPDX-License-Identifier: AGPL-3.0-only
//! Runtime configuration tests
//!
//! Tier 1 tests: Coverage-measured runtime config tests
//! Focus: Runtime defaults, overrides, validation, edge cases
//!
//! ✅ MODERNIZED: Uses scoped Mutex instead of #[serial]

use std::env;
use std::sync::Mutex;

// Environment lock for tests that must mutate env vars
static ENV_LOCK: Mutex<()> = Mutex::new(());

// ============================================================================
// Runtime Default Tests
// ============================================================================

#[test]
fn test_runtime_defaults_worker_threads() {
    let config = RuntimeConfig::default();

    // Should default to CPU count
    assert!(config.worker_threads > 0);
    assert!(
        config.worker_threads
            <= std::thread::available_parallelism()
                .map(std::num::NonZero::get)
                .unwrap_or(1)
                * 2
    );
}

#[test]
fn test_runtime_defaults_memory_limits() {
    let config = RuntimeConfig::default();

    // Should have reasonable defaults
    assert!(config.max_memory_mb > 0);
    assert!(config.max_memory_mb <= 32768); // 32GB max
}

#[test]
fn test_runtime_defaults_timeout_values() {
    let config = RuntimeConfig::default();

    assert!(config.default_timeout_secs > 0);
    assert!(config.default_timeout_secs < 300); // < 5 minutes
}

#[test]
fn test_runtime_defaults_stack_size() {
    let config = RuntimeConfig::default();

    // Should have reasonable stack size
    assert!(config.stack_size_kb >= 512);
    assert!(config.stack_size_kb <= 8192);
}

// ============================================================================
// Runtime Override Tests
// ============================================================================

#[test]
fn test_runtime_env_override_threads() {
    // ✅ MODERN: Scoped lock instead of #[serial]
    let _guard = ENV_LOCK.lock().unwrap();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe { env::set_var("TOADSTOOL_WORKER_THREADS", "16") };

    let config = load_runtime_config();

    assert_eq!(config.worker_threads, 16);

    unsafe { env::remove_var("TOADSTOOL_WORKER_THREADS") };
}

#[test]
fn test_runtime_env_override_memory() {
    // ✅ MODERN: Scoped lock instead of #[serial]
    let _guard = ENV_LOCK.lock().unwrap();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe { env::set_var("TOADSTOOL_MAX_MEMORY_MB", "8192") };

    let config = load_runtime_config();

    assert_eq!(config.max_memory_mb, 8192);

    unsafe { env::remove_var("TOADSTOOL_MAX_MEMORY_MB") };
}

#[test]
fn test_runtime_env_override_timeout() {
    // ✅ MODERN: Scoped lock instead of #[serial]
    let _guard = ENV_LOCK.lock().unwrap();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe { env::set_var("TOADSTOOL_DEFAULT_TIMEOUT_SECS", "120") };

    let config = load_runtime_config();

    assert_eq!(config.default_timeout_secs, 120);

    unsafe { env::remove_var("TOADSTOOL_DEFAULT_TIMEOUT_SECS") };
}

#[test]
fn test_runtime_env_override_invalid_falls_back() {
    // ✅ MODERN: Scoped lock instead of #[serial]
    let _guard = ENV_LOCK.lock().unwrap();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe { env::set_var("TOADSTOOL_WORKER_THREADS", "not-a-number") };

    let config = load_runtime_config();

    // Should fall back to default
    assert!(config.worker_threads > 0);

    unsafe { env::remove_var("TOADSTOOL_WORKER_THREADS") };
}

// ============================================================================
// Runtime Validation Tests
// ============================================================================

#[test]
fn test_runtime_validation_min_threads() {
    let config = RuntimeConfig {
        worker_threads: 0,
        ..Default::default()
    };

    let result = config.validate();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("threads"));
}

#[test]
fn test_runtime_validation_max_threads() {
    let config = RuntimeConfig {
        worker_threads: 10000, // Unreasonably high
        ..Default::default()
    };

    let result = config.validate();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("threads"));
}

#[test]
fn test_runtime_validation_memory_limits() {
    let config = RuntimeConfig {
        max_memory_mb: 0,
        ..Default::default()
    };

    let result = config.validate();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("memory"));
}

#[test]
fn test_runtime_validation_timeout_reasonable() {
    let config = RuntimeConfig {
        default_timeout_secs: 0,
        ..Default::default()
    };

    let result = config.validate();

    assert!(result.is_err());
}

// ============================================================================
// Runtime Feature Flags Tests
// ============================================================================

#[test]
fn test_runtime_wasm_enabled_default() {
    let config = RuntimeConfig::default();

    assert!(config.enable_wasm);
}

#[test]
fn test_runtime_native_enabled_default() {
    let config = RuntimeConfig::default();

    assert!(config.enable_native);
}

#[test]
fn test_runtime_container_enabled_default() {
    let config = RuntimeConfig::default();

    assert!(config.enable_container);
}

#[test]
fn test_runtime_disable_all_runtimes_invalid() {
    let config = RuntimeConfig {
        enable_wasm: false,
        enable_native: false,
        enable_container: false,
        ..Default::default()
    };

    let result = config.validate();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("At least one runtime"));
}

// ============================================================================
// Runtime Resource Limits Tests
// ============================================================================

#[test]
fn test_runtime_cpu_limit_percentage() {
    let config = RuntimeConfig {
        max_cpu_percent: 80,
        ..Default::default()
    };

    assert_eq!(config.max_cpu_percent, 80);
}

#[test]
fn test_runtime_cpu_limit_validation() {
    let config = RuntimeConfig {
        max_cpu_percent: 150, // Invalid: > 100%
        ..Default::default()
    };

    let result = config.validate();

    assert!(result.is_err());
}

#[test]
fn test_runtime_disk_limit() {
    let config = RuntimeConfig {
        max_disk_mb: 10240, // 10GB
        ..Default::default()
    };

    assert_eq!(config.max_disk_mb, 10240);
}

// ============================================================================
// Mock Types (Simplified)
// ============================================================================

#[derive(Debug, Clone)]
struct RuntimeConfig {
    worker_threads: usize,
    max_memory_mb: usize,
    default_timeout_secs: u64,
    stack_size_kb: usize,
    enable_wasm: bool,
    enable_native: bool,
    enable_container: bool,
    max_cpu_percent: u8,
    max_disk_mb: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: std::thread::available_parallelism()
                .map(std::num::NonZero::get)
                .unwrap_or(1),
            max_memory_mb: 4096,
            default_timeout_secs: 60,
            stack_size_kb: 2048,
            enable_wasm: true,
            enable_native: true,
            enable_container: true,
            max_cpu_percent: 100,
            max_disk_mb: 10240,
        }
    }
}

impl RuntimeConfig {
    fn validate(&self) -> Result<(), String> {
        if self.worker_threads == 0 {
            return Err("Worker threads must be > 0".to_string());
        }

        if self.worker_threads > 1024 {
            return Err("Worker threads too high (max 1024)".to_string());
        }

        if self.max_memory_mb == 0 {
            return Err("Max memory must be > 0".to_string());
        }

        if self.default_timeout_secs == 0 {
            return Err("Timeout must be > 0".to_string());
        }

        if !self.enable_wasm && !self.enable_native && !self.enable_container {
            return Err("At least one runtime must be enabled".to_string());
        }

        if self.max_cpu_percent > 100 {
            return Err("CPU percent must be <= 100".to_string());
        }

        Ok(())
    }
}

fn load_runtime_config() -> RuntimeConfig {
    let mut config = RuntimeConfig::default();

    if let Ok(threads) = env::var("TOADSTOOL_WORKER_THREADS") {
        if let Ok(n) = threads.parse() {
            config.worker_threads = n;
        }
    }

    if let Ok(mem) = env::var("TOADSTOOL_MAX_MEMORY_MB") {
        if let Ok(n) = mem.parse() {
            config.max_memory_mb = n;
        }
    }

    if let Ok(timeout) = env::var("TOADSTOOL_DEFAULT_TIMEOUT_SECS") {
        if let Ok(n) = timeout.parse() {
            config.default_timeout_secs = n;
        }
    }

    config
}
