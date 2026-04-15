// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_security_sandbox::*;
use std::time::Duration;

#[test]
fn test_resource_limits_no_memory_limit() {
    let limits = ResourceLimits {
        max_memory_bytes: None,
        ..ResourceLimits::default()
    };

    assert!(limits.max_memory_bytes.is_none());
}

#[test]
fn test_resource_limits_no_cpu_limit() {
    let limits = ResourceLimits {
        max_cpu_percent: None,
        ..ResourceLimits::default()
    };

    assert!(limits.max_cpu_percent.is_none());
}

#[test]
fn test_resource_limits_very_high_memory() {
    let limits = ResourceLimits {
        max_memory_bytes: Some(64 * 1024 * 1024 * 1024), // 64GB
        ..ResourceLimits::default()
    };

    assert!(limits.max_memory_bytes.unwrap() > 1024 * 1024 * 1024);
}

#[test]
fn test_resource_limits_very_low_cpu() {
    let limits = ResourceLimits {
        max_cpu_percent: Some(10.0),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_cpu_percent, Some(10.0));
}

#[test]
fn test_resource_limits_max_execution_time() {
    let limits = ResourceLimits {
        max_execution_time: Some(Duration::from_secs(600)), // 10 minutes
        ..ResourceLimits::default()
    };

    assert!(limits.max_execution_time.is_some());
}

#[test]
fn test_resource_limits_no_execution_time_limit() {
    let limits = ResourceLimits {
        max_execution_time: None,
        ..ResourceLimits::default()
    };

    assert!(limits.max_execution_time.is_none());
}

#[test]
fn test_resource_limits_network_bandwidth() {
    let limits = ResourceLimits {
        max_network_bps: Some(100 * 1024 * 1024), // 100 MB/s
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_network_bps, Some(100 * 1024 * 1024));
}

#[test]
fn test_resource_limits_all_none() {
    let limits = ResourceLimits {
        max_memory_bytes: None,
        max_cpu_percent: None,
        max_file_descriptors: None,
        max_processes: None,
        max_disk_bytes: None,
        max_network_bps: None,
        max_execution_time: None,
    };

    assert!(limits.max_memory_bytes.is_none());
    assert!(limits.max_cpu_percent.is_none());
    assert!(limits.max_file_descriptors.is_none());
}

#[test]
fn test_resource_limits_serialization() {
    let limits = ResourceLimits::default();
    let json = serde_json::to_string(&limits).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_resource_limits_zero_cpu() {
    let limits = ResourceLimits {
        max_cpu_percent: Some(0.0),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_cpu_percent, Some(0.0));
}

#[test]
fn test_resource_limits_hundred_cpu() {
    let limits = ResourceLimits {
        max_cpu_percent: Some(100.0),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_cpu_percent, Some(100.0));
}

#[test]
fn test_resource_limits_minimal_memory() {
    let limits = ResourceLimits {
        max_memory_bytes: Some(1024 * 1024), // 1MB
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_memory_bytes, Some(1024 * 1024));
}

#[test]
fn test_resource_limits_large_memory() {
    let limits = ResourceLimits {
        max_memory_bytes: Some(16 * 1024 * 1024 * 1024), // 16GB
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_memory_bytes, Some(16 * 1024 * 1024 * 1024));
}

#[test]
fn test_resource_limits_unlimited_execution() {
    let limits = ResourceLimits {
        max_execution_time: None,
        ..ResourceLimits::default()
    };

    assert!(limits.max_execution_time.is_none());
}

#[test]
fn test_resource_limits_short_execution() {
    let limits = ResourceLimits {
        max_execution_time: Some(Duration::from_secs(1)),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_execution_time, Some(Duration::from_secs(1)));
}

#[test]
fn test_resource_limits_unlimited_network() {
    let limits = ResourceLimits {
        max_network_bps: None,
        ..ResourceLimits::default()
    };

    assert!(limits.max_network_bps.is_none());
}

#[test]
fn test_resource_limits_low_bandwidth() {
    let limits = ResourceLimits {
        max_network_bps: Some(128 * 1024), // 128 KB/s
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_network_bps, Some(128 * 1024));
}

#[test]
fn test_resource_limits_high_bandwidth() {
    let limits = ResourceLimits {
        max_network_bps: Some(1024 * 1024 * 1024), // 1 GB/s
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_network_bps, Some(1024 * 1024 * 1024));
}
