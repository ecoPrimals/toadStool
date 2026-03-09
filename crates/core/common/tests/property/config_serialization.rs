// SPDX-License-Identifier: AGPL-3.0-only
//! Property-based tests for config serialization
//!
//! Tests that config structures can be serialized and deserialized
//! without losing information (round-trip property).

use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use toadstool_common::config_bases::{TimeoutConfig, HealthCheckConfig};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_timeout_config_round_trip(
        conn_secs in 1u64..3600,
        req_secs in 1u64..3600,
        read_secs in 1u64..3600,
        write_secs in 1u64..3600,
    ) {
        use std::time::Duration;
        
        let config = TimeoutConfig {
            connection_timeout: Duration::from_secs(conn_secs),
            request_timeout: Duration::from_secs(req_secs),
            read_timeout: Duration::from_secs(read_secs),
            write_timeout: Duration::from_secs(write_secs),
        };
        
        // Serialize to JSON
        let json = serde_json::to_string(&config).unwrap();
        
        // Deserialize back
        let deserialized: TimeoutConfig = serde_json::from_str(&json).unwrap();
        
        // Should be equal (round-trip property)
        prop_assert_eq!(config.connection_timeout, deserialized.connection_timeout);
        prop_assert_eq!(config.request_timeout, deserialized.request_timeout);
        prop_assert_eq!(config.read_timeout, deserialized.read_timeout);
        prop_assert_eq!(config.write_timeout, deserialized.write_timeout);
    }

    #[test]
    fn prop_health_check_config_round_trip(
        interval_secs in 1u64..3600,
        timeout_secs in 1u64..3600,
        retries in 0u32..10,
    ) {
        use std::time::Duration;
        
        let config = HealthCheckConfig {
            interval: Duration::from_secs(interval_secs),
            timeout: Duration::from_secs(timeout_secs),
            retries,
            path: "/health".to_string(),
        };
        
        // Serialize to JSON
        let json = serde_json::to_string(&config).unwrap();
        
        // Deserialize back
        let deserialized: HealthCheckConfig = serde_json::from_str(&json).unwrap();
        
        // Should be equal (round-trip property)
        prop_assert_eq!(config.interval, deserialized.interval);
        prop_assert_eq!(config.timeout, deserialized.timeout);
        prop_assert_eq!(config.retries, deserialized.retries);
        prop_assert_eq!(config.path, deserialized.path);
    }
}
