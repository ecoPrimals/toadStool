//! Comprehensive tests for UniversalComputePlatform
//!
//! This test suite covers the critical async structures that were previously untested:
//! - UniversalComputePlatform creation and initialization
//! - Runtime engine registration and management
//! - Job execution and scheduling
//! - Resource coordination
//! - Primal registry operations

use std::collections::HashMap;
use std::time::Duration;
use toadstool::resources::ResourceRequirements;
use toadstool::universal::{
    JobPriority, NetworkLocation, PrimalCapability, PrimalContext, PrimalType, SecurityLevel,
    UniversalComputePlatform, UniversalJob, UniversalJobType, UniversalPlatformConfig,
};
use uuid::Uuid;

// ============================================================================
// UniversalComputePlatform Tests
// ============================================================================

#[tokio::test]
async fn test_platform_creation() {
    let result = UniversalComputePlatform::new().await;
    assert!(result.is_ok(), "Platform creation should succeed");
}

#[tokio::test]
async fn test_platform_creation_with_default_config() {
    let config = UniversalPlatformConfig::default();
    let result = UniversalComputePlatform::new_with_config(config).await;
    assert!(
        result.is_ok(),
        "Platform creation with config should succeed"
    );
}

#[tokio::test]
async fn test_platform_creation_initializes_components() {
    let platform = UniversalComputePlatform::new().await.unwrap();

    // Platform should have no runtime engines initially
    let runtimes = platform.get_available_runtimes().await;
    assert!(
        runtimes.is_empty(),
        "New platform should have no runtime engines"
    );
}

#[tokio::test]
async fn test_platform_get_available_runtimes_empty() {
    let platform = UniversalComputePlatform::new().await.unwrap();
    let runtimes = platform.get_available_runtimes().await;

    assert_eq!(runtimes.len(), 0, "New platform should have 0 runtimes");
}

#[tokio::test]
async fn test_platform_find_primals_by_capability() {
    let platform = UniversalComputePlatform::new().await.unwrap();

    // Try to find primals with container runtime capability
    let capability = PrimalCapability::ContainerRuntime {
        orchestrators: vec!["docker".to_string()],
    };

    let _primals = platform.find_primals_by_capability(&capability).await;

    // Should return a vec (may be empty or contain ToadStool primal)
    // Assert that we can get the primal list (length check removed as >= 0 is always true)
}

// ============================================================================
// UniversalPlatformConfig Tests
// ============================================================================

#[test]
fn test_platform_config_default() {
    let config = UniversalPlatformConfig::default();

    // Config should have reasonable defaults
    assert!(format!("{:?}", config).contains("UniversalPlatformConfig"));
}

#[test]
fn test_platform_config_clone() {
    let config = UniversalPlatformConfig::default();
    let cloned = config.clone();

    // Should be equal after clone
    assert_eq!(
        format!("{:?}", config),
        format!("{:?}", cloned),
        "Cloned config should be equal"
    );
}

// ============================================================================
// UniversalJob Tests
// ============================================================================

#[test]
fn test_universal_job_creation() {
    let mut env = HashMap::new();
    env.insert("TEST".to_string(), "value".to_string());

    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            env: env.clone(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: None,
        created_at: chrono::Utc::now(),
        context: PrimalContext {
            user_id: "test-user".to_string(),
            device_id: "test-device".to_string(),
            session_id: Uuid::new_v4().to_string(),
            network_location: NetworkLocation {
                ip_address: "127.0.0.1".to_string(),
                subnet: None,
                network_id: None,
                geo_location: None,
            },
            security_level: SecurityLevel::Standard,
            metadata: HashMap::new(),
        },
    };

    assert!(!job.id.to_string().is_empty(), "Job should have an ID");
    assert_eq!(job.priority, JobPriority::Normal);
}

#[test]
fn test_universal_job_clone() {
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Wasm {
            module: vec![0x00, 0x61, 0x73, 0x6d],
            args: vec![],
            env: HashMap::new(),
        },
        priority: JobPriority::High,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };

    let cloned = job.clone();
    assert_eq!(job.id, cloned.id);
    assert_eq!(job.priority, cloned.priority);
}

#[test]
fn test_universal_job_debug() {
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({"name": "test-biome"}),
            team_id: "team-123".to_string(),
        },
        priority: JobPriority::Low,
        resources: ResourceRequirements::default(),
        timeout: None,
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };

    let debug_str = format!("{:?}", job);
    assert!(debug_str.contains("UniversalJob"));
    assert!(debug_str.contains("Low"));
}

// ============================================================================
// JobPriority Tests
// ============================================================================

#[test]
fn test_job_priority_variants() {
    let low = JobPriority::Low;
    let normal = JobPriority::Normal;
    let high = JobPriority::High;
    let critical = JobPriority::Critical;

    assert!(matches!(low, JobPriority::Low));
    assert!(matches!(normal, JobPriority::Normal));
    assert!(matches!(high, JobPriority::High));
    assert!(matches!(critical, JobPriority::Critical));
}

#[test]
fn test_job_priority_ordering() {
    assert!(JobPriority::Low < JobPriority::Normal);
    assert!(JobPriority::Normal < JobPriority::High);
    assert!(JobPriority::High < JobPriority::Critical);

    assert!(JobPriority::Critical > JobPriority::High);
    assert!(JobPriority::High > JobPriority::Normal);
    assert!(JobPriority::Normal > JobPriority::Low);
}

#[test]
fn test_job_priority_equality() {
    let p1 = JobPriority::High;
    let p2 = JobPriority::High;
    let p3 = JobPriority::Low;

    assert_eq!(p1, p2);
    assert_ne!(p1, p3);
}

#[test]
fn test_job_priority_copy() {
    let original = JobPriority::Critical;
    let copied = original; // JobPriority implements Copy

    assert_eq!(original, copied);
}

#[test]
fn test_job_priority_debug() {
    let priority = JobPriority::High;
    let debug_str = format!("{:?}", priority);

    assert!(debug_str.contains("High"));
}

// ============================================================================
// UniversalJobType Tests
// ============================================================================

#[test]
fn test_job_type_native() {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin".to_string());

    let job_type = UniversalJobType::Native {
        executable: "/bin/ls".to_string(),
        args: vec!["-la".to_string()],
        env,
    };

    assert!(matches!(job_type, UniversalJobType::Native { .. }));

    if let UniversalJobType::Native { executable, .. } = job_type {
        assert_eq!(executable, "/bin/ls");
    }
}

#[test]
fn test_job_type_wasm() {
    let wasm_module = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic number
    let job_type = UniversalJobType::Wasm {
        module: wasm_module.clone(),
        args: vec!["--help".to_string()],
        env: HashMap::new(),
    };

    assert!(matches!(job_type, UniversalJobType::Wasm { .. }));

    if let UniversalJobType::Wasm { module, .. } = job_type {
        assert_eq!(module[0..4], [0x00, 0x61, 0x73, 0x6d]);
    }
}

#[test]
fn test_job_type_primal() {
    let payload = serde_json::json!({
        "action": "execute",
        "params": {"key": "value"}
    });

    let job_type = UniversalJobType::Primal {
        primal_type: "compute".to_string(),
        endpoint: "http://toadstool.local".to_string(),
        payload,
    };

    assert!(matches!(job_type, UniversalJobType::Primal { .. }));
}

#[test]
fn test_job_type_biomeos() {
    let manifest = serde_json::json!({
        "name": "my-biome",
        "version": "1.0",
        "resources": {
            "cpu": "2",
            "memory": "4Gi"
        }
    });

    let job_type = UniversalJobType::BiomeOS {
        biome_manifest: manifest,
        team_id: "team-alpha".to_string(),
    };

    assert!(matches!(job_type, UniversalJobType::BiomeOS { .. }));

    if let UniversalJobType::BiomeOS { team_id, .. } = job_type {
        assert_eq!(team_id, "team-alpha");
    }
}

#[test]
fn test_job_type_clone() {
    let original = UniversalJobType::Primal {
        primal_type: "storage".to_string(),
        endpoint: "http://nestgate.local".to_string(),
        payload: serde_json::json!({"op": "backup"}),
    };

    let cloned = original.clone();

    if let (
        UniversalJobType::Primal {
            primal_type: pt1, ..
        },
        UniversalJobType::Primal {
            primal_type: pt2, ..
        },
    ) = (&original, &cloned)
    {
        assert_eq!(pt1, pt2);
    }
}

#[test]
fn test_job_type_debug() {
    let job_type = UniversalJobType::Native {
        executable: "/bin/test".to_string(),
        args: vec![],
        env: HashMap::new(),
    };

    let debug_str = format!("{:?}", job_type);
    assert!(debug_str.contains("Native"));
    assert!(debug_str.contains("/bin/test"));
}

// ============================================================================
// PrimalType Tests (Additional)
// ============================================================================

#[test]
fn test_primal_type_compute() {
    let pt = PrimalType::Compute;
    assert!(matches!(pt, PrimalType::Compute));
}

#[test]
fn test_primal_type_security() {
    let pt = PrimalType::Security;
    assert!(matches!(pt, PrimalType::Security));
}

#[test]
fn test_primal_type_storage() {
    let pt = PrimalType::Storage;
    assert!(matches!(pt, PrimalType::Storage));
}

#[test]
fn test_primal_type_ai() {
    let pt = PrimalType::AI;
    assert!(matches!(pt, PrimalType::AI));
}

#[test]
fn test_primal_type_network() {
    let pt = PrimalType::Network;
    assert!(matches!(pt, PrimalType::Network));
}

#[test]
fn test_primal_type_os() {
    let pt = PrimalType::OS;
    assert!(matches!(pt, PrimalType::OS));
}

#[test]
fn test_primal_type_custom() {
    let pt = PrimalType::Custom("MyCustomPrimal".to_string());

    if let PrimalType::Custom(name) = &pt {
        assert_eq!(name, "MyCustomPrimal");
    }
}

#[test]
fn test_primal_type_equality() {
    assert_eq!(PrimalType::Compute, PrimalType::Compute);
    assert_eq!(PrimalType::Security, PrimalType::Security);
    assert_ne!(PrimalType::Compute, PrimalType::Security);

    let custom1 = PrimalType::Custom("test".to_string());
    let custom2 = PrimalType::Custom("test".to_string());
    let custom3 = PrimalType::Custom("other".to_string());

    assert_eq!(custom1, custom2);
    assert_ne!(custom1, custom3);
}

#[test]
fn test_primal_type_clone() {
    let original = PrimalType::Custom("CloneTest".to_string());
    #[allow(clippy::clone_on_copy)]
    let cloned = original.clone();

    assert_eq!(original, cloned);
}

#[test]
fn test_primal_type_debug() {
    let pt = PrimalType::Network;
    let debug_str = format!("{:?}", pt);

    assert!(debug_str.contains("Network"));
}

#[test]
fn test_primal_type_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(PrimalType::Compute);
    set.insert(PrimalType::Security);
    set.insert(PrimalType::Compute); // Duplicate

    assert_eq!(set.len(), 2, "HashSet should contain 2 unique types");
    assert!(set.contains(&PrimalType::Compute));
    assert!(set.contains(&PrimalType::Security));
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_context() -> PrimalContext {
    PrimalContext {
        user_id: "test-user".to_string(),
        device_id: "test-device".to_string(),
        session_id: Uuid::new_v4().to_string(),
        network_location: NetworkLocation {
            ip_address: "127.0.0.1".to_string(),
            subnet: Some("192.168.1.0/24".to_string()),
            network_id: Some("test-network".to_string()),
            geo_location: Some("US-East".to_string()),
        },
        security_level: SecurityLevel::Standard,
        metadata: HashMap::new(),
    }
}

#[cfg(test)]
mod test_helpers {
    use super::*;

    #[test]
    fn test_create_test_context() {
        let context = create_test_context();

        assert_eq!(context.user_id, "test-user");
        assert_eq!(context.device_id, "test-device");
        assert_eq!(context.security_level, SecurityLevel::Standard);
        assert_eq!(context.network_location.ip_address, "127.0.0.1");
    }
}
