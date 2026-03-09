// SPDX-License-Identifier: AGPL-3.0-only
// ============================================================================
// Resource Limits Tests
// ============================================================================

#[test]
fn test_resource_limits_default() {
    let limits = ResourceLimits::default();

    assert_eq!(limits.max_memory_bytes, Some(512 * 1024 * 1024)); // 512MB
    assert_eq!(limits.max_cpu_percent, Some(80.0));
    assert_eq!(limits.max_file_descriptors, Some(1024));
    assert_eq!(limits.max_processes, Some(100));
    assert_eq!(limits.max_disk_bytes, Some(1024 * 1024 * 1024)); // 1GB
}

#[test]
fn test_resource_limits_custom_memory() {
    let limits = ResourceLimits {
        max_memory_bytes: Some(256 * 1024 * 1024), // 256MB
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_memory_bytes, Some(256 * 1024 * 1024));
}

#[test]
fn test_resource_limits_custom_cpu() {
    let limits = ResourceLimits {
        max_cpu_percent: Some(50.0),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_cpu_percent, Some(50.0));
}

#[test]
fn test_resource_limits_custom_file_descriptors() {
    let limits = ResourceLimits {
        max_file_descriptors: Some(512),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_file_descriptors, Some(512));
}

#[test]
fn test_resource_limits_custom_processes() {
    let limits = ResourceLimits {
        max_processes: Some(50),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_processes, Some(50));
}

#[test]
fn test_resource_limits_unlimited() {
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
    assert!(limits.max_execution_time.is_none());
}

#[test]
fn test_resource_limits_with_timeout() {
    let limits = ResourceLimits {
        max_execution_time: Some(Duration::from_secs(300)),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_execution_time, Some(Duration::from_secs(300)));
}

