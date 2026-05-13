// SPDX-License-Identifier: AGPL-3.0-or-later
//! Additional comprehensive tests for `config_utils`
//!
//! Expanding test coverage for configuration utility functions
//!
//! ✅ MODERNIZED: Uses `temp_env` for thread-safe env var scoping (Rust 2024)

mod tests {
    use toadstool_config::config_utils::ConfigUtils;

    #[test]
    fn test_worker_threads_configuration() {
        temp_env::with_var("TOADSTOOL_WORKER_THREADS", None::<&str>, || {
            let threads = ConfigUtils::get_worker_threads();
            assert!(threads > 0);
            assert!(threads <= 1024);
        });
        temp_env::with_var("TOADSTOOL_WORKER_THREADS", Some("8"), || {
            let threads = ConfigUtils::get_worker_threads();
            assert_eq!(threads, 8);
        });
    }

    #[test]
    fn test_execution_timeout_configuration() {
        temp_env::with_var("TOADSTOOL_EXECUTION_TIMEOUT", None::<&str>, || {
            let timeout = ConfigUtils::get_execution_timeout();
            assert!(timeout.as_secs() > 0);
            assert!(timeout.as_secs() <= 3600);
        });
        temp_env::with_var("TOADSTOOL_EXECUTION_TIMEOUT", Some("600"), || {
            let timeout = ConfigUtils::get_execution_timeout();
            assert!(timeout.as_secs() > 0);
        });
    }

    #[test]
    fn test_max_concurrent_executions() {
        temp_env::with_var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", None::<&str>, || {
            let max = ConfigUtils::get_max_concurrent_executions();
            assert!(max > 0);
            assert!(max <= 10_000);
        });
        temp_env::with_var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", Some("50"), || {
            let max = ConfigUtils::get_max_concurrent_executions();
            assert_eq!(max, 50);
        });
    }

    #[test]
    fn test_security_settings() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_AUTH_ENABLED", None::<&str>),
                ("TOADSTOOL_SANDBOXING_ENABLED", None::<&str>),
            ],
            || {
                let _auth = ConfigUtils::get_auth_enabled();
                let _sandbox = ConfigUtils::get_sandboxing_enabled();
            },
        );
        temp_env::with_var("TOADSTOOL_AUTH_ENABLED", Some("true"), || {
            assert!(ConfigUtils::get_auth_enabled());
        });
        temp_env::with_var("TOADSTOOL_SANDBOXING_ENABLED", Some("true"), || {
            assert!(ConfigUtils::get_sandboxing_enabled());
        });
        temp_env::with_var("TOADSTOOL_AUTH_ENABLED", Some("false"), || {
            assert!(!ConfigUtils::get_auth_enabled());
        });
    }

    #[test]
    fn test_monitoring_settings() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_METRICS_ENABLED", None::<&str>),
                ("TOADSTOOL_HEALTH_CHECKS_ENABLED", None::<&str>),
            ],
            || {
                let _metrics = ConfigUtils::get_metrics_enabled();
                let _health = ConfigUtils::get_health_checks_enabled();
                let metrics_interval = ConfigUtils::get_metrics_interval();
                assert!(metrics_interval.as_secs() > 0);
                assert!(metrics_interval.as_secs() <= 3600);
                let health_interval = ConfigUtils::get_health_check_interval();
                assert!(health_interval.as_secs() > 0);
                assert!(health_interval.as_secs() <= 3600);
            },
        );
    }

    #[test]
    fn test_logging_configuration() {
        temp_env::with_var("TOADSTOOL_LOG_LEVEL", None::<&str>, || {
            let level = ConfigUtils::get_log_level();
            assert!(!level.is_empty());
            assert!(["trace", "debug", "info", "warn", "error"].contains(&level.as_str()));
        });
        temp_env::with_var("TOADSTOOL_LOG_LEVEL", Some("debug"), || {
            let level = ConfigUtils::get_log_level();
            assert_eq!(level, "debug");
        });
        temp_env::with_var("TOADSTOOL_LOG_DIR", None::<&str>, || {
            let dir = ConfigUtils::get_log_dir();
            assert!(!dir.is_empty());
        });
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

        // Self-knowledge: toadstool always knows its own port
        assert!(
            ports.contains_key("toadstool"),
            "Primal must have self-knowledge of its own port"
        );

        // Port 0 = OS-assigned; all in valid u16 range (type guarantees validity)
    }

    #[test]
    fn test_service_endpoints_format() {
        let endpoints = ConfigUtils::get_service_endpoints();

        // At minimum, toadstool knows its own endpoint
        assert!(
            !endpoints.is_empty(),
            "Should have at least the self endpoint"
        );

        // All endpoints should be valid HTTP URLs
        for (name, endpoint) in &endpoints {
            assert!(
                endpoint.starts_with("http://") || endpoint.starts_with("https://"),
                "Endpoint for {name} should be HTTP/HTTPS: {endpoint}"
            );

            assert!(
                endpoint.contains(':'),
                "Endpoint for {name} should have port"
            );
        }
    }

    #[test]
    fn test_encryption_key_path() {
        temp_env::with_var("TOADSTOOL_ENCRYPTION_KEY_PATH", None::<&str>, || {
            let path = ConfigUtils::get_encryption_key_path();
            assert!(!path.is_empty());
        });
        temp_env::with_var(
            "TOADSTOOL_ENCRYPTION_KEY_PATH",
            Some("/custom/path/key.pem"),
            || {
                let path = ConfigUtils::get_encryption_key_path();
                assert_eq!(path, "/custom/path/key.pem");
            },
        );
    }

    #[test]
    fn test_cache_configuration() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_CACHE_DIR", None::<&str>),
                ("TOADSTOOL_CACHE_URL", None::<&str>),
            ],
            || {
                let dir = ConfigUtils::get_cache_dir();
                assert!(!dir.is_empty());
                let url = ConfigUtils::get_cache_url();
                assert!(!url.is_empty());
            },
        );
    }

    #[test]
    fn test_resource_limits() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_MAX_CPU_USAGE", None::<&str>),
                ("TOADSTOOL_MAX_MEMORY_USAGE", None::<&str>),
            ],
            || {
                let cpu = ConfigUtils::get_max_cpu_usage();
                assert!(cpu > 0.0);
                assert!(cpu <= 100.0);
                let mem = ConfigUtils::get_max_memory_usage();
                assert!(mem > 0);
            },
        );
    }

    #[test]
    fn test_environment_variants() {
        for environment in &["development", "staging", "production", "test"] {
            temp_env::with_var("TOADSTOOL_ENVIRONMENT", Some(environment), || {
                let env_val = ConfigUtils::get_environment();
                assert!(!env_val.is_empty());
            });
        }
    }

    #[test]
    fn test_print_current_config_no_panic() {
        // print_current_config() is only available in debug builds
        #[cfg(debug_assertions)]
        ConfigUtils::print_current_config();
    }
} // end of tests module
