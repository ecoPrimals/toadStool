// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_security_sandbox::*;

#[test]
fn test_resource_usage_default() {
    let usage = ResourceUsage::default();

    assert_eq!(usage.memory_bytes, 0);
    assert!(usage.cpu_percent.abs() < f64::EPSILON);
    assert_eq!(usage.file_descriptors, 0);
    assert_eq!(usage.processes, 0);
}

#[test]
fn test_resource_usage_memory() {
    let usage = ResourceUsage {
        memory_bytes: 256 * 1024 * 1024, // 256MB
        ..ResourceUsage::default()
    };

    assert_eq!(usage.memory_bytes, 256 * 1024 * 1024);
}

#[test]
fn test_resource_usage_cpu() {
    let usage = ResourceUsage {
        cpu_percent: 45.5,
        ..ResourceUsage::default()
    };

    assert!((usage.cpu_percent - 45.5).abs() < f64::EPSILON);
}

#[test]
fn test_resource_usage_file_descriptors() {
    let usage = ResourceUsage {
        file_descriptors: 250,
        ..ResourceUsage::default()
    };

    assert_eq!(usage.file_descriptors, 250);
}

#[test]
fn test_resource_usage_processes() {
    let usage = ResourceUsage {
        processes: 15,
        ..ResourceUsage::default()
    };

    assert_eq!(usage.processes, 15);
}
