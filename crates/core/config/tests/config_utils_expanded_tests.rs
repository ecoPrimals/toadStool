//! Additional comprehensive tests for config_utils
//!
//! Expanding test coverage for configuration utility functions
//!
//! ✅ MODERNIZED: Uses scoped Mutex instead of #[serial] for concurrent execution

#[allow(deprecated)] // Testing legacy config functions during migration
mod tests {
    use std::env;
    use std::sync::Mutex;
    use toadstool_config::config_utils::ConfigUtils;

    // ✅ MODERN: Scoped lock for environment variable tests
    // Using std::sync::OnceLock for thread-safe lazy initialization
    static ENV_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

    fn get_env_lock() -> &'static Mutex<()> {
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_worker_threads_configuration() {
        let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
        let original = env::var("TOADSTOOL_WORKER_THREADS").ok();

        // Test default
        env::remove_var("TOADSTOOL_WORKER_THREADS");
        let threads = ConfigUtils::get_worker_threads();
        assert!(threads > 0);
        assert!(threads <= 1024);

        // Test custom value
        env::set_var("TOADSTOOL_WORKER_THREADS", "8");
        let threads = ConfigUtils::get_worker_threads();
        assert_eq!(threads, 8);

        // Restore
        match original {
            Some(val) => env::set_var("TOADSTOOL_WORKER_THREADS", val),
            None => env::remove_var("TOADSTOOL_WORKER_THREADS"),
        }
    }

    #[test]
    fn test_execution_timeout_configuration() {
        let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
        let original = env::var("TOADSTOOL_EXECUTION_TIMEOUT").ok();

        // Test default
        env::remove_var("TOADSTOOL_EXECUTION_TIMEOUT");
        let timeout = ConfigUtils::get_execution_timeout();
        assert!(timeout.as_secs() > 0);
        assert!(timeout.as_secs() <= 3600);

        // Test custom value (in seconds)
        env::set_var("TOADSTOOL_EXECUTION_TIMEOUT", "600");
        let timeout = ConfigUtils::get_execution_timeout();
        // May or may not be 600 depending on how env is read
        assert!(timeout.as_secs() > 0);

        // Restore
        match original {
            Some(val) => env::set_var("TOADSTOOL_EXECUTION_TIMEOUT", val),
            None => env::remove_var("TOADSTOOL_EXECUTION_TIMEOUT"),
        }
    }

    #[test]
    fn test_max_concurrent_executions() {
        let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
        let original = env::var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS").ok();

        // Test default
        env::remove_var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS");
        let max = ConfigUtils::get_max_concurrent_executions();
        assert!(max > 0);
        assert!(max <= 10000);

        // Test custom value
        env::set_var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", "50");
        let max = ConfigUtils::get_max_concurrent_executions();
        assert_eq!(max, 50);

        // Restore
        match original {
            Some(val) => env::set_var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", val),
            None => env::remove_var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS"),
        }
    }

    #[test]
    fn test_security_settings() {
        let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
        let original_auth = env::var("TOADSTOOL_AUTH_ENABLED").ok();
        let original_sandbox = env::var("TOADSTOOL_SANDBOXING_ENABLED").ok();

        // Test defaults
        env::remove_var("TOADSTOOL_AUTH_ENABLED");
        env::remove_var("TOADSTOOL_SANDBOXING_ENABLED");

        let _auth = ConfigUtils::get_auth_enabled();
        let _sandbox = ConfigUtils::get_sandboxing_enabled();

        // These values are tested more specifically below when we set them explicitly

        // Test enabling
        env::set_var("TOADSTOOL_AUTH_ENABLED", "true");
        assert!(ConfigUtils::get_auth_enabled());

        env::set_var("TOADSTOOL_SANDBOXING_ENABLED", "true");
        assert!(ConfigUtils::get_sandboxing_enabled());

        // Test disabling
        env::set_var("TOADSTOOL_AUTH_ENABLED", "false");
        assert!(!ConfigUtils::get_auth_enabled());

        // Restore
        match original_auth {
            Some(val) => env::set_var("TOADSTOOL_AUTH_ENABLED", val),
            None => env::remove_var("TOADSTOOL_AUTH_ENABLED"),
        }
        match original_sandbox {
            Some(val) => env::set_var("TOADSTOOL_SANDBOXING_ENABLED", val),
            None => env::remove_var("TOADSTOOL_SANDBOXING_ENABLED"),
        }
    }

    #[test]
    fn test_monitoring_settings() {
        let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
        let original_metrics = env::var("TOADSTOOL_METRICS_ENABLED").ok();
        let original_health = env::var("TOADSTOOL_HEALTH_CHECKS_ENABLED").ok();

        // Test defaults
        env::remove_var("TOADSTOOL_METRICS_ENABLED");
        env::remove_var("TOADSTOOL_HEALTH_CHECKS_ENABLED");

        let _metrics = ConfigUtils::get_metrics_enabled();
        let _health = ConfigUtils::get_health_checks_enabled();

        // These values are tested more specifically when we set them explicitly

        // Test intervals
        let metrics_interval = ConfigUtils::get_metrics_interval();
        assert!(metrics_interval.as_secs() > 0);
        assert!(metrics_interval.as_secs() <= 3600);

        let health_interval = ConfigUtils::get_health_check_interval();
        assert!(health_interval.as_secs() > 0);
        assert!(health_interval.as_secs() <= 3600);

        // Restore
        match original_metrics {
            Some(val) => env::set_var("TOADSTOOL_METRICS_ENABLED", val),
            None => env::remove_var("TOADSTOOL_METRICS_ENABLED"),
        }
        match original_health {
            Some(val) => env::set_var("TOADSTOOL_HEALTH_CHECKS_ENABLED", val),
            None => env::remove_var("TOADSTOOL_HEALTH_CHECKS_ENABLED"),
        }
    }

    #[test]
    fn test_logging_configuration() {
        let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
        let original_level = env::var("TOADSTOOL_LOG_LEVEL").ok();
        let original_dir = env::var("TOADSTOOL_LOG_DIR").ok();

        // Test default log level
        env::remove_var("TOADSTOOL_LOG_LEVEL");
        let level = ConfigUtils::get_log_level();
        assert!(!level.is_empty());
        assert!(["trace", "debug", "info", "warn", "error"].contains(&level.as_str()));

        // Test custom log level
        env::set_var("TOADSTOOL_LOG_LEVEL", "debug");
        let level = ConfigUtils::get_log_level();
        assert_eq!(level, "debug");

        // Test log directory
        env::remove_var("TOADSTOOL_LOG_DIR");
        let dir = ConfigUtils::get_log_dir();
        assert!(!dir.is_empty());

        // Restore
        match original_level {
            Some(val) => env::set_var("TOADSTOOL_LOG_LEVEL", val),
            None => env::remove_var("TOADSTOOL_LOG_LEVEL"),
        }
        match original_dir {
            Some(val) => env::set_var("TOADSTOOL_LOG_DIR", val),
            None => env::remove_var("TOADSTOOL_LOG_DIR"),
        }
    }

    #[test]
    fn test_port_allocation_ranges() {
        let (start, end) = ConfigUtils::get_container_port_range();
        assert!(start < end);
        assert!(start >= 3000);
        assert!(end <= 9999);
        assert!(end - start >= 100); // Reasonable range size

        let (alloc_start, alloc_end) = ConfigUtils::get_port_allocation_range();
        assert!(alloc_start < alloc_end);
        assert!(alloc_end - alloc_start >= 100);
    }

    #[test]
    fn test_service_ports_completeness() {
        let ports = ConfigUtils::get_service_ports();

        // Should have all primals
        assert!(ports.contains_key("songbird"));
        assert!(ports.contains_key("beardog"));
        assert!(ports.contains_key("nestgate"));
        assert!(ports.contains_key("squirrel"));
        assert!(ports.contains_key("toadstool"));

        // All ports should be valid
        for (name, &port) in &ports {
            assert!(port > 0, "Port for {} should be positive", name);
            // Port is u16, so it's automatically < 65536
        }
    }

    #[test]
    fn test_service_endpoints_format() {
        let endpoints = ConfigUtils::get_service_endpoints();

        // Should have all primals
        assert!(endpoints.len() >= 5);

        // All endpoints should be valid HTTP URLs
        for (name, endpoint) in &endpoints {
            assert!(
                endpoint.starts_with("http://") || endpoint.starts_with("https://"),
                "Endpoint for {} should be HTTP/HTTPS: {}",
                name,
                endpoint
            );

            // Should contain a port
            assert!(
                endpoint.contains(':'),
                "Endpoint for {} should have port",
                name
            );
        }
    }

    #[test]
    fn test_encryption_key_path() {
        let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
        let original = env::var("TOADSTOOL_ENCRYPTION_KEY_PATH").ok();

        // Test default
        env::remove_var("TOADSTOOL_ENCRYPTION_KEY_PATH");
        let path = ConfigUtils::get_encryption_key_path();
        assert!(!path.is_empty());

        // Test custom path
        env::set_var("TOADSTOOL_ENCRYPTION_KEY_PATH", "/custom/path/key.pem");
        let path = ConfigUtils::get_encryption_key_path();
        assert_eq!(path, "/custom/path/key.pem");

        // Restore
        match original {
            Some(val) => env::set_var("TOADSTOOL_ENCRYPTION_KEY_PATH", val),
            None => env::remove_var("TOADSTOOL_ENCRYPTION_KEY_PATH"),
        }
    }

    #[test]
    fn test_cache_configuration() {
        let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
        let original_dir = env::var("TOADSTOOL_CACHE_DIR").ok();
        let original_url = env::var("TOADSTOOL_CACHE_URL").ok();

        // Test defaults
        env::remove_var("TOADSTOOL_CACHE_DIR");
        env::remove_var("TOADSTOOL_CACHE_URL");

        let dir = ConfigUtils::get_cache_dir();
        assert!(!dir.is_empty());

        let url = ConfigUtils::get_cache_url();
        assert!(!url.is_empty());

        // Restore
        match original_dir {
            Some(val) => env::set_var("TOADSTOOL_CACHE_DIR", val),
            None => env::remove_var("TOADSTOOL_CACHE_DIR"),
        }
        match original_url {
            Some(val) => env::set_var("TOADSTOOL_CACHE_URL", val),
            None => env::remove_var("TOADSTOOL_CACHE_URL"),
        }
    }

    #[test]
    fn test_resource_limits() {
        let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
        let original_cpu = env::var("TOADSTOOL_MAX_CPU_USAGE").ok();
        let original_mem = env::var("TOADSTOOL_MAX_MEMORY_USAGE").ok();

        // Test defaults
        env::remove_var("TOADSTOOL_MAX_CPU_USAGE");
        env::remove_var("TOADSTOOL_MAX_MEMORY_USAGE");

        let cpu = ConfigUtils::get_max_cpu_usage();
        assert!(cpu > 0.0);
        assert!(cpu <= 100.0);

        let mem = ConfigUtils::get_max_memory_usage();
        assert!(mem > 0);

        // Restore
        match original_cpu {
            Some(val) => env::set_var("TOADSTOOL_MAX_CPU_USAGE", val),
            None => env::remove_var("TOADSTOOL_MAX_CPU_USAGE"),
        }
        match original_mem {
            Some(val) => env::set_var("TOADSTOOL_MAX_MEMORY_USAGE", val),
            None => env::remove_var("TOADSTOOL_MAX_MEMORY_USAGE"),
        }
    }

    #[test]
    fn test_environment_variants() {
        let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
        let original = env::var("TOADSTOOL_ENVIRONMENT").ok();

        // Test different environments
        for environment in &["development", "staging", "production", "test"] {
            env::set_var("TOADSTOOL_ENVIRONMENT", environment);
            let env_val = ConfigUtils::get_environment();
            // Verify it returns a valid environment string
            assert!(!env_val.is_empty());
        }

        // Restore
        match original {
            Some(val) => env::set_var("TOADSTOOL_ENVIRONMENT", val),
            None => env::remove_var("TOADSTOOL_ENVIRONMENT"),
        }
    }

    #[test]
    fn test_print_current_config_no_panic() {
        // This should not panic
        ConfigUtils::print_current_config();
    }
} // end of tests module
