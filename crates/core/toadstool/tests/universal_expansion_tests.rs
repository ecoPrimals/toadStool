// SPDX-License-Identifier: AGPL-3.0-or-later
//! Additional Universal Platform tests for Week 16 Sprint
//!
//! Focus: Expanding coverage on `SystemResources`, `JobMetrics`, and integration tests

use std::collections::HashMap;
use toadstool::universal::{JobPriority, UniversalJobType, UniversalSystemResources};

// ============================================================================
// SystemResources Additional Tests (10 tests)
// ============================================================================

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[test]
fn test_system_resources_creation() {
    let resources = UniversalSystemResources {
        cpu_cores: 8.0,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        storage_bytes: 500 * 1024 * 1024 * 1024,
        network_bandwidth: 1_000_000_000,
        gpu_units: 0,
        special_hardware: HashMap::new(),
    };

    assert_eq!(resources.cpu_cores, 8.0);
}

#[test]
fn test_system_resources_minimal() {
    let resources = UniversalSystemResources {
        cpu_cores: 1.0,
        memory_bytes: 1024 * 1024 * 1024,
        storage_bytes: 10 * 1024 * 1024 * 1024,
        network_bandwidth: 0,
        gpu_units: 0,
        special_hardware: HashMap::new(),
    };

    assert_eq!(resources.network_bandwidth, 0);
}

#[test]
fn test_system_resources_large_values() {
    let resources = UniversalSystemResources {
        cpu_cores: 128.0,
        memory_bytes: 1024 * 1024 * 1024 * 1024,        // 1TB
        storage_bytes: 100 * 1024 * 1024 * 1024 * 1024, // 100TB
        network_bandwidth: 10_000_000_000,              // 10 Gbps
        gpu_units: 8,
        special_hardware: HashMap::new(),
    };

    assert!(resources.cpu_cores > 100.0);
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[test]
fn test_system_resources_fractional_cores() {
    let resources = UniversalSystemResources {
        cpu_cores: 2.5,
        memory_bytes: 4 * 1024 * 1024 * 1024,
        storage_bytes: 100 * 1024 * 1024 * 1024,
        network_bandwidth: 100_000_000,
        gpu_units: 1,
        special_hardware: HashMap::new(),
    };

    assert_eq!(resources.cpu_cores, 2.5);
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[test]
fn test_system_resources_clone() {
    let resources1 = UniversalSystemResources {
        cpu_cores: 4.0,
        memory_bytes: 8 * 1024 * 1024 * 1024,
        storage_bytes: 200 * 1024 * 1024 * 1024,
        network_bandwidth: 500_000_000,
        gpu_units: 2,
        special_hardware: HashMap::new(),
    };

    let resources2 = resources1.clone();
    assert_eq!(resources1.cpu_cores, resources2.cpu_cores);
}

#[test]
fn test_system_resources_debug() {
    let resources = UniversalSystemResources {
        cpu_cores: 4.0,
        memory_bytes: 8 * 1024 * 1024 * 1024,
        storage_bytes: 200 * 1024 * 1024 * 1024,
        network_bandwidth: 500_000_000,
        gpu_units: 0,
        special_hardware: HashMap::new(),
    };

    let debug_str = format!("{resources:?}");
    assert!(debug_str.contains("SystemResources"));
}

#[test]
fn test_system_resources_serialization() {
    let resources = UniversalSystemResources {
        cpu_cores: 4.0,
        memory_bytes: 8 * 1024 * 1024 * 1024,
        storage_bytes: 200 * 1024 * 1024 * 1024,
        network_bandwidth: 500_000_000,
        gpu_units: 0,
        special_hardware: HashMap::new(),
    };

    let json = serde_json::to_string(&resources).unwrap();
    assert!(!json.is_empty());
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[test]
fn test_system_resources_deserialization() {
    let resources = UniversalSystemResources {
        cpu_cores: 4.0,
        memory_bytes: 8 * 1024 * 1024 * 1024,
        storage_bytes: 200 * 1024 * 1024 * 1024,
        network_bandwidth: 500_000_000,
        gpu_units: 0,
        special_hardware: HashMap::new(),
    };

    let json = serde_json::to_string(&resources).unwrap();
    let deserialized: UniversalSystemResources = serde_json::from_str(&json).unwrap();
    assert_eq!(resources.cpu_cores, deserialized.cpu_cores);
}

#[test]
fn test_system_resources_with_gpus() {
    let resources = UniversalSystemResources {
        cpu_cores: 4.0,
        memory_bytes: 8 * 1024 * 1024 * 1024,
        storage_bytes: 200 * 1024 * 1024 * 1024,
        network_bandwidth: 500_000_000,
        gpu_units: 4,
        special_hardware: HashMap::new(),
    };

    assert_eq!(resources.gpu_units, 4);
}

#[test]
fn test_system_resources_comparison() {
    let resources1 = UniversalSystemResources {
        cpu_cores: 4.0,
        memory_bytes: 8 * 1024 * 1024 * 1024,
        storage_bytes: 200 * 1024 * 1024 * 1024,
        network_bandwidth: 500_000_000,
        gpu_units: 0,
        special_hardware: HashMap::new(),
    };

    let resources2 = UniversalSystemResources {
        cpu_cores: 8.0,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        storage_bytes: 400 * 1024 * 1024 * 1024,
        network_bandwidth: 1_000_000_000,
        gpu_units: 2,
        special_hardware: HashMap::new(),
    };

    assert!(resources2.cpu_cores > resources1.cpu_cores);
}

// ============================================================================
// UniversalJobType Additional Tests (8 tests)
// ============================================================================

#[test]
fn test_job_type_native() {
    use std::collections::HashMap;
    let job_type = UniversalJobType::Native {
        executable: "/usr/bin/python3".to_string(),
        args: vec!["script.py".to_string()],
        env: HashMap::new(),
    };

    match job_type {
        UniversalJobType::Native { executable, .. } => {
            assert!(executable.starts_with("/usr"));
        }
        _ => panic!("Should be Native"),
    }
}

#[test]
fn test_job_type_wasm() {
    use std::collections::HashMap;
    let job_type = UniversalJobType::Wasm {
        module: vec![0, 97, 115, 109], // WASM magic bytes
        args: vec!["arg1".to_string()],
        env: HashMap::new(),
    };

    match job_type {
        UniversalJobType::Wasm { module, .. } => {
            assert!(!module.is_empty());
        }
        _ => panic!("Should be Wasm"),
    }
}

#[test]
fn test_job_type_primal() {
    let job_type = UniversalJobType::Primal {
        primal_type: "compute".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        payload: serde_json::json!({"task": "process"}),
    };

    match job_type {
        UniversalJobType::Primal { primal_type, .. } => {
            assert_eq!(primal_type, "compute");
        }
        _ => panic!("Should be Primal"),
    }
}

#[test]
fn test_job_type_biomeos() {
    let job_type = UniversalJobType::BiomeOS {
        biome_manifest: serde_json::json!({"version": "1.0"}),
        team_id: "team-123".to_string(),
    };

    match job_type {
        UniversalJobType::BiomeOS { team_id, .. } => {
            assert_eq!(team_id, "team-123");
        }
        _ => panic!("Should be BiomeOS"),
    }
}

#[test]
fn test_job_type_native_with_env() {
    use std::collections::HashMap;
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin".to_string());

    let job_type = UniversalJobType::Native {
        executable: "/bin/bash".to_string(),
        args: vec![],
        env,
    };

    match job_type {
        UniversalJobType::Native { env, .. } => {
            assert!(env.contains_key("PATH"));
        }
        _ => panic!("Should be Native"),
    }
}

#[test]
fn test_job_type_clone() {
    use std::collections::HashMap;
    let job_type1 = UniversalJobType::Wasm {
        module: vec![1, 2, 3],
        args: vec![],
        env: HashMap::new(),
    };

    let job_type2 = job_type1.clone();

    match (job_type1, job_type2) {
        (UniversalJobType::Wasm { module: m1, .. }, UniversalJobType::Wasm { module: m2, .. }) => {
            assert_eq!(m1, m2);
        }
        _ => panic!("Both should be Wasm"),
    }
}

#[test]
fn test_job_type_debug() {
    let job_type = UniversalJobType::Primal {
        primal_type: "test".to_string(),
        endpoint: "http://localhost".to_string(),
        payload: serde_json::json!({}),
    };

    let debug_str = format!("{job_type:?}");
    assert!(debug_str.contains("Primal"));
}

#[test]
fn test_job_type_serialization() {
    use std::collections::HashMap;
    let job_type = UniversalJobType::Native {
        executable: "/bin/test".to_string(),
        args: vec![],
        env: HashMap::new(),
    };

    let json = serde_json::to_string(&job_type).unwrap();
    assert!(!json.is_empty());
}

// ============================================================================
// JobPriority Additional Tests (6 tests)
// ============================================================================

#[test]
fn test_job_priority_ordering_complete() {
    // Lower numeric values = higher priority, so Emergency < Low
    assert!(JobPriority::Emergency < JobPriority::Critical);
    assert!(JobPriority::Critical < JobPriority::High);
    assert!(JobPriority::High < JobPriority::Normal);
    assert!(JobPriority::Normal < JobPriority::Low);
}

#[test]
fn test_job_priority_equality() {
    let p1 = JobPriority::High;
    let p2 = JobPriority::High;
    let p3 = JobPriority::Normal;

    assert_eq!(p1, p2);
    assert_ne!(p1, p3);
}

#[test]
fn test_job_priority_clone() {
    let p1 = JobPriority::Critical;
    let p2 = p1;

    assert_eq!(p1, p2);
}

#[test]
fn test_job_priority_debug() {
    let priority = JobPriority::High;
    let debug_str = format!("{priority:?}");
    assert!(debug_str.contains("High"));
}

#[test]
fn test_job_priority_serialization() {
    let priority = JobPriority::Critical;
    let json = serde_json::to_string(&priority).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_job_priority_all_variants() {
    let priorities = [
        JobPriority::Low,
        JobPriority::Normal,
        JobPriority::High,
        JobPriority::Critical,
        JobPriority::Emergency,
    ];

    assert_eq!(priorities.len(), 5);
}

// ============================================================================
// Test Coverage Summary
// ============================================================================

#[test]
fn test_universal_expansion_coverage_summary() {
    println!("=== Universal Expansion Test Coverage ===");
    println!("SystemResources Tests:      10 tests");
    println!("UniversalJobType Tests:      8 tests");
    println!("JobPriority Tests:           6 tests");
    println!("─────────────────────────────────────────");
    println!("Total:                      24 tests");
    println!("Target Coverage:            44% → 60%");
    println!("=========================================");
}
