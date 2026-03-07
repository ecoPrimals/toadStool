// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resource type tests — `SystemResources`, `ResourceRequirements`, `CpuRequirements`, `MemoryRequirements`.

use std::collections::HashMap;

use toadstool::resources::{
    CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements,
    ResourceRequirements, StorageRequirements, SystemResources,
};
use toadstool::universal::UniversalSystemResources;

// ── SystemResources ──────────────────────────────────────────────────────────

#[test]
fn test_system_resources_creation() {
    let resources = SystemResources {
        available_cpu_cores: 8.0,
        available_memory_bytes: 16_000_000_000,
        available_storage_bytes: 500_000_000_000,
        available_network_bandwidth: Some(1_000_000_000),
        available_gpu_units: 2,
        ..Default::default()
    };
    assert_eq!(resources.available_cpu_cores, 8.0);
    assert_eq!(resources.available_memory_bytes, 16_000_000_000);
    assert_eq!(resources.available_gpu_units, 2);
}

#[test]
fn test_system_resources_clone() {
    let original = SystemResources {
        available_cpu_cores: 4.0,
        available_memory_bytes: 8_000_000_000,
        available_storage_bytes: 250_000_000_000,
        available_network_bandwidth: Some(500_000_000),
        available_gpu_units: 1,
        ..Default::default()
    };
    let cloned = original.clone();
    assert_eq!(original.available_cpu_cores, cloned.available_cpu_cores);
    assert_eq!(
        original.available_memory_bytes,
        cloned.available_memory_bytes
    );
    assert_eq!(original.available_gpu_units, cloned.available_gpu_units);
}

#[test]
fn test_system_resources_debug() {
    let resources = SystemResources {
        available_cpu_cores: 16.0,
        available_memory_bytes: 32_000_000_000,
        available_storage_bytes: 1_000_000_000_000,
        available_network_bandwidth: Some(10_000_000_000),
        available_gpu_units: 4,
        ..Default::default()
    };
    let debug_str = format!("{resources:?}");
    assert!(debug_str.contains("SystemResources"));
    assert!(debug_str.contains("16"));
}

#[test]
fn test_system_resources_with_special_hardware() {
    let mut special = HashMap::new();
    special.insert("tpu".to_string(), 8);
    special.insert("fpga".to_string(), 2);
    let resources = UniversalSystemResources {
        cpu_cores: 32.0,
        memory_bytes: 64_000_000_000,
        storage_bytes: 2_000_000_000_000,
        network_bandwidth: 100_000_000_000,
        gpu_units: 8,
        special_hardware: special,
    };
    assert_eq!(resources.special_hardware.get("tpu"), Some(&8));
    assert_eq!(resources.special_hardware.get("fpga"), Some(&2));
}

// ── ResourceRequirements ─────────────────────────────────────────────────────

#[test]
fn test_resource_requirements_default() {
    let requirements = ResourceRequirements::default();
    assert_eq!(requirements.cpu.min_cores, 1.0);
    assert!(requirements.gpu.is_none());
}

#[test]
fn test_resource_requirements_with_cpu() {
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 4.0,
            max_cores: Some(8.0),
            architecture: Some("x86_64".to_string()),
        },
        ..Default::default()
    };
    assert_eq!(requirements.cpu.min_cores, 4.0);
    assert_eq!(requirements.cpu.max_cores, Some(8.0));
    assert_eq!(requirements.cpu.architecture, Some("x86_64".to_string()));
}

#[test]
fn test_resource_requirements_with_memory() {
    let requirements = ResourceRequirements {
        memory: MemoryRequirements {
            min_bytes: 4 * 1024 * 1024 * 1024,
            max_bytes: Some(8 * 1024 * 1024 * 1024),
        },
        ..Default::default()
    };
    assert_eq!(requirements.memory.min_bytes, 4 * 1024 * 1024 * 1024);
    assert_eq!(requirements.memory.max_bytes, Some(8 * 1024 * 1024 * 1024));
}

#[test]
fn test_resource_requirements_with_gpu() {
    let requirements = ResourceRequirements {
        gpu: Some(GpuRequirements {
            min_units: 2,
            max_units: Some(4),
            gpu_type: Some("nvidia-a100".to_string()),
            min_memory_bytes: Some(16 * 1024 * 1024 * 1024),
        }),
        ..Default::default()
    };
    assert!(requirements.gpu.is_some());
    let gpu = requirements.gpu.unwrap();
    assert_eq!(gpu.min_units, 2);
    assert_eq!(gpu.min_memory_bytes, Some(16 * 1024 * 1024 * 1024));
}

#[test]
fn test_resource_requirements_clone() {
    let original = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 2.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 2 * 1024 * 1024 * 1024,
            max_bytes: None,
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    };
    let cloned = original.clone();
    assert_eq!(original.cpu.min_cores, cloned.cpu.min_cores);
    assert_eq!(original.memory.min_bytes, cloned.memory.min_bytes);
}

#[test]
fn test_resource_requirements_debug() {
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 8.0,
            max_cores: None,
            architecture: None,
        },
        ..Default::default()
    };
    let debug_str = format!("{requirements:?}");
    assert!(debug_str.contains("ResourceRequirements"));
}

// ── CpuRequirements ──────────────────────────────────────────────────────────

#[test]
fn test_cpu_requirements_default() {
    let cpu = CpuRequirements::default();
    assert_eq!(cpu.min_cores, 1.0);
    assert!(cpu.max_cores.is_none());
    assert!(cpu.architecture.is_none());
}

#[test]
fn test_cpu_requirements_with_max_cores() {
    let cpu = CpuRequirements {
        min_cores: 2.0,
        max_cores: Some(8.0),
        architecture: None,
    };
    assert_eq!(cpu.min_cores, 2.0);
    assert_eq!(cpu.max_cores, Some(8.0));
}

#[test]
fn test_cpu_requirements_with_architecture() {
    let cpu = CpuRequirements {
        min_cores: 1.0,
        max_cores: None,
        architecture: Some("aarch64".to_string()),
    };
    assert_eq!(cpu.architecture, Some("aarch64".to_string()));
}

#[test]
fn test_cpu_requirements_clone() {
    let original = CpuRequirements {
        min_cores: 4.0,
        max_cores: Some(16.0),
        architecture: Some("x86_64".to_string()),
    };
    let cloned = original.clone();
    assert_eq!(original.min_cores, cloned.min_cores);
    assert_eq!(original.max_cores, cloned.max_cores);
    assert_eq!(original.architecture, cloned.architecture);
}

// ── MemoryRequirements ───────────────────────────────────────────────────────

#[test]
fn test_memory_requirements_default() {
    let memory = MemoryRequirements::default();
    assert!(memory.min_bytes > 0);
    assert!(memory.max_bytes.is_none());
}

#[test]
fn test_memory_requirements_with_max() {
    let memory = MemoryRequirements {
        min_bytes: 1024 * 1024 * 1024,
        max_bytes: Some(4 * 1024 * 1024 * 1024),
    };
    assert_eq!(memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(memory.max_bytes, Some(4 * 1024 * 1024 * 1024));
}

#[test]
fn test_memory_requirements_clone() {
    let original = MemoryRequirements {
        min_bytes: 2 * 1024 * 1024 * 1024,
        max_bytes: Some(8 * 1024 * 1024 * 1024),
    };
    let cloned = original.clone();
    assert_eq!(original.min_bytes, cloned.min_bytes);
    assert_eq!(original.max_bytes, cloned.max_bytes);
}
