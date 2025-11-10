//! Comprehensive tests for universal.rs types - Sprint 23
//!
//! Target: 44% → 65% coverage (~55 tests)
//! Focus: Core types, enums, data structures

use std::collections::HashMap;
use toadstool::universal::{UniversalSystemResources as SystemResources, *};

// ============================================================================
// SecurityLevel Tests
// ============================================================================

#[test]
fn test_security_level_all_variants() {
    let basic = SecurityLevel::Basic;
    let standard = SecurityLevel::Standard;
    let high = SecurityLevel::High;
    let maximum = SecurityLevel::Maximum;

    // Test that all variants can be created
    assert!(matches!(basic, SecurityLevel::Basic));
    assert!(matches!(standard, SecurityLevel::Standard));
    assert!(matches!(high, SecurityLevel::High));
    assert!(matches!(maximum, SecurityLevel::Maximum));
}

#[test]
fn test_security_level_ordering() {
    assert!(SecurityLevel::Basic < SecurityLevel::Standard);
    assert!(SecurityLevel::Standard < SecurityLevel::High);
    assert!(SecurityLevel::High < SecurityLevel::Maximum);

    assert!(SecurityLevel::Maximum > SecurityLevel::High);
    assert!(SecurityLevel::High > SecurityLevel::Standard);
}

#[test]
fn test_security_level_equality() {
    assert_eq!(SecurityLevel::Basic, SecurityLevel::Basic);
    assert_eq!(SecurityLevel::Maximum, SecurityLevel::Maximum);
    assert_ne!(SecurityLevel::Basic, SecurityLevel::High);
}

#[test]
fn test_security_level_clone() {
    let level1 = SecurityLevel::High;
    let level2 = level1;
    assert_eq!(level1, level2);
}

#[test]
fn test_security_level_debug() {
    let level = SecurityLevel::Standard;
    let debug_str = format!("{:?}", level);
    assert!(debug_str.contains("Standard"));
}

#[test]
fn test_security_level_serialization() {
    let level = SecurityLevel::High;
    let json = serde_json::to_string(&level).unwrap();
    let deserialized: SecurityLevel = serde_json::from_str(&json).unwrap();
    assert_eq!(level, deserialized);
}

#[test]
fn test_security_level_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(SecurityLevel::Basic);
    set.insert(SecurityLevel::High);

    assert!(set.contains(&SecurityLevel::Basic));
    assert!(set.contains(&SecurityLevel::High));
    assert!(!set.contains(&SecurityLevel::Standard));
}

// ============================================================================
// NetworkLocation Tests
// ============================================================================

#[test]
fn test_network_location_basic() {
    let location = NetworkLocation {
        ip_address: "192.168.1.1".to_string(),
        subnet: None,
        network_id: None,
        geo_location: None,
    };

    assert_eq!(location.ip_address, "192.168.1.1");
    assert!(location.subnet.is_none());
}

#[test]
fn test_network_location_with_subnet() {
    let location = NetworkLocation {
        ip_address: "10.0.0.1".to_string(),
        subnet: Some("10.0.0.0/24".to_string()),
        network_id: Some("vpc-123".to_string()),
        geo_location: Some("us-west-2".to_string()),
    };

    assert_eq!(location.subnet, Some("10.0.0.0/24".to_string()));
    assert_eq!(location.network_id, Some("vpc-123".to_string()));
    assert_eq!(location.geo_location, Some("us-west-2".to_string()));
}

#[test]
fn test_network_location_clone() {
    let location1 = NetworkLocation {
        ip_address: "172.16.0.1".to_string(),
        subnet: Some("172.16.0.0/16".to_string()),
        network_id: None,
        geo_location: None,
    };

    let location2 = location1.clone();
    assert_eq!(location1.ip_address, location2.ip_address);
    assert_eq!(location1.subnet, location2.subnet);
}

#[test]
fn test_network_location_serialization() {
    let location = NetworkLocation {
        ip_address: "8.8.8.8".to_string(),
        subnet: Some("8.8.8.0/24".to_string()),
        network_id: Some("public".to_string()),
        geo_location: Some("global".to_string()),
    };

    let json = serde_json::to_string(&location).unwrap();
    let deserialized: NetworkLocation = serde_json::from_str(&json).unwrap();

    assert_eq!(location.ip_address, deserialized.ip_address);
    assert_eq!(location.subnet, deserialized.subnet);
}

#[test]
fn test_network_location_equality() {
    let loc1 = NetworkLocation {
        ip_address: "1.1.1.1".to_string(),
        subnet: None,
        network_id: None,
        geo_location: None,
    };

    let loc2 = NetworkLocation {
        ip_address: "1.1.1.1".to_string(),
        subnet: None,
        network_id: None,
        geo_location: None,
    };

    assert_eq!(loc1, loc2);
}

// ============================================================================
// PrimalContext Tests
// ============================================================================

#[test]
fn test_primal_context_creation() {
    let context = PrimalContext {
        user_id: "user-123".to_string(),
        device_id: "device-456".to_string(),
        session_id: "session-789".to_string(),
        network_location: NetworkLocation {
            ip_address: "192.168.1.100".to_string(),
            subnet: None,
            network_id: None,
            geo_location: None,
        },
        security_level: SecurityLevel::Standard,
        metadata: HashMap::new(),
    };

    assert_eq!(context.user_id, "user-123");
    assert_eq!(context.device_id, "device-456");
    assert_eq!(context.security_level, SecurityLevel::Standard);
}

#[test]
fn test_primal_context_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("app_version".to_string(), "1.0.0".to_string());
    metadata.insert("platform".to_string(), "linux".to_string());

    let context = PrimalContext {
        user_id: "user-001".to_string(),
        device_id: "device-001".to_string(),
        session_id: "session-001".to_string(),
        network_location: NetworkLocation {
            ip_address: "10.0.0.1".to_string(),
            subnet: None,
            network_id: None,
            geo_location: None,
        },
        security_level: SecurityLevel::High,
        metadata,
    };

    assert_eq!(context.metadata.len(), 2);
    assert_eq!(
        context.metadata.get("app_version"),
        Some(&"1.0.0".to_string())
    );
}

#[test]
fn test_primal_context_clone() {
    let context1 = PrimalContext {
        user_id: "user-test".to_string(),
        device_id: "device-test".to_string(),
        session_id: "session-test".to_string(),
        network_location: NetworkLocation {
            ip_address: "127.0.0.1".to_string(),
            subnet: None,
            network_id: None,
            geo_location: None,
        },
        security_level: SecurityLevel::Basic,
        metadata: HashMap::new(),
    };

    let context2 = context1.clone();
    assert_eq!(context1.user_id, context2.user_id);
    assert_eq!(context1.security_level, context2.security_level);
}

#[test]
fn test_primal_context_serialization() {
    let context = PrimalContext {
        user_id: "user-serialize".to_string(),
        device_id: "device-serialize".to_string(),
        session_id: "session-serialize".to_string(),
        network_location: NetworkLocation {
            ip_address: "172.16.0.1".to_string(),
            subnet: Some("172.16.0.0/16".to_string()),
            network_id: None,
            geo_location: None,
        },
        security_level: SecurityLevel::Maximum,
        metadata: HashMap::new(),
    };

    let json = serde_json::to_string(&context).unwrap();
    let deserialized: PrimalContext = serde_json::from_str(&json).unwrap();

    assert_eq!(context.user_id, deserialized.user_id);
    assert_eq!(context.security_level, deserialized.security_level);
}

// ============================================================================
// PrimalType Tests
// ============================================================================

#[test]
fn test_primal_type_all_standard_variants() {
    let compute = PrimalType::Compute;
    let security = PrimalType::Security;
    let storage = PrimalType::Storage;
    let ai = PrimalType::AI;
    let network = PrimalType::Network;
    let os = PrimalType::OS;

    assert!(matches!(compute, PrimalType::Compute));
    assert!(matches!(security, PrimalType::Security));
    assert!(matches!(storage, PrimalType::Storage));
    assert!(matches!(ai, PrimalType::AI));
    assert!(matches!(network, PrimalType::Network));
    assert!(matches!(os, PrimalType::OS));
}

#[test]
fn test_primal_type_custom() {
    let custom = PrimalType::Custom("Analytics".to_string());

    if let PrimalType::Custom(name) = custom {
        assert_eq!(name, "Analytics");
    } else {
        panic!("Expected Custom variant");
    }
}

#[test]
fn test_primal_type_clone() {
    let type1 = PrimalType::Compute;
    let type2 = type1.clone();
    assert_eq!(type1, type2);

    let custom1 = PrimalType::Custom("Test".to_string());
    let custom2 = custom1.clone();
    assert_eq!(custom1, custom2);
}

#[test]
fn test_primal_type_equality() {
    assert_eq!(PrimalType::Compute, PrimalType::Compute);
    assert_ne!(PrimalType::Compute, PrimalType::Security);

    assert_eq!(
        PrimalType::Custom("A".to_string()),
        PrimalType::Custom("A".to_string())
    );
    assert_ne!(
        PrimalType::Custom("A".to_string()),
        PrimalType::Custom("B".to_string())
    );
}

#[test]
fn test_primal_type_serialization() {
    let primal_type = PrimalType::Security;
    let json = serde_json::to_string(&primal_type).unwrap();
    let deserialized: PrimalType = serde_json::from_str(&json).unwrap();
    assert_eq!(primal_type, deserialized);

    let custom = PrimalType::Custom("Custom".to_string());
    let custom_json = serde_json::to_string(&custom).unwrap();
    let custom_deserialized: PrimalType = serde_json::from_str(&custom_json).unwrap();
    assert_eq!(custom, custom_deserialized);
}

// ============================================================================
// PrimalCapability Tests
// ============================================================================

#[test]
fn test_primal_capability_container_runtime() {
    let cap = PrimalCapability::ContainerRuntime {
        orchestrators: vec!["kubernetes".to_string(), "docker".to_string()],
    };

    if let PrimalCapability::ContainerRuntime { orchestrators } = cap {
        assert_eq!(orchestrators.len(), 2);
        assert!(orchestrators.contains(&"kubernetes".to_string()));
    } else {
        panic!("Expected ContainerRuntime variant");
    }
}

#[test]
fn test_primal_capability_gpu_acceleration() {
    let cap = PrimalCapability::GpuAcceleration { cuda_support: true };

    if let PrimalCapability::GpuAcceleration { cuda_support } = cap {
        assert!(cuda_support);
    } else {
        panic!("Expected GpuAcceleration variant");
    }
}

#[test]
fn test_primal_capability_authentication() {
    let cap = PrimalCapability::Authentication {
        methods: vec!["oauth2".to_string(), "jwt".to_string()],
    };

    if let PrimalCapability::Authentication { methods } = cap {
        assert_eq!(methods.len(), 2);
    } else {
        panic!("Expected Authentication variant");
    }
}

#[test]
fn test_primal_capability_clone() {
    let cap1 = PrimalCapability::WasmExecution { wasi_support: true };
    let cap2 = cap1.clone();
    assert_eq!(cap1, cap2);
}

#[test]
fn test_primal_capability_serialization() {
    let cap = PrimalCapability::LoadBalancing {
        algorithms: vec!["round-robin".to_string(), "least-conn".to_string()],
    };

    let json = serde_json::to_string(&cap).unwrap();
    let deserialized: PrimalCapability = serde_json::from_str(&json).unwrap();
    assert_eq!(cap, deserialized);
}

// ============================================================================
// JobPriority Tests
// ============================================================================

#[test]
fn test_job_priority_all_variants() {
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
}

#[test]
fn test_job_priority_clone() {
    let priority1 = JobPriority::High;
    let priority2 = priority1;
    assert_eq!(priority1, priority2);
}

#[test]
fn test_job_priority_serialization() {
    let priority = JobPriority::Critical;
    let json = serde_json::to_string(&priority).unwrap();
    let deserialized: JobPriority = serde_json::from_str(&json).unwrap();
    assert_eq!(priority, deserialized);
}

// ============================================================================
// SystemResources Tests
// ============================================================================

#[test]
fn test_system_resources_creation() {
    let mut special_hardware = HashMap::new();
    special_hardware.insert("tpu".to_string(), 2);

    let resources = SystemResources {
        cpu_cores: 16.0,
        memory_bytes: 32_000_000_000,
        storage_bytes: 1_000_000_000_000,
        network_bandwidth: 10_000_000_000,
        gpu_units: 2,
        special_hardware,
    };

    assert_eq!(resources.cpu_cores, 16.0);
    assert_eq!(resources.memory_bytes, 32_000_000_000);
    assert_eq!(resources.gpu_units, 2);
}

#[test]
fn test_system_resources_with_network() {
    let resources = SystemResources {
        cpu_cores: 8.0,
        memory_bytes: 16_000_000_000,
        storage_bytes: 500_000_000_000,
        network_bandwidth: 1_000_000_000, // 1 Gbps
        gpu_units: 1,
        special_hardware: HashMap::new(),
    };

    assert_eq!(resources.network_bandwidth, 1_000_000_000);
    assert_eq!(resources.gpu_units, 1);
}

#[test]
fn test_system_resources_clone() {
    let resources1 = SystemResources {
        cpu_cores: 4.0,
        memory_bytes: 8_000_000_000,
        storage_bytes: 100_000_000_000,
        network_bandwidth: 100_000_000,
        gpu_units: 0,
        special_hardware: HashMap::new(),
    };

    let resources2 = resources1.clone();
    assert_eq!(resources1.cpu_cores, resources2.cpu_cores);
    assert_eq!(resources1.memory_bytes, resources2.memory_bytes);
}

#[test]
fn test_system_resources_serialization() {
    let resources = SystemResources {
        cpu_cores: 32.0,
        memory_bytes: 64_000_000_000,
        storage_bytes: 2_000_000_000_000,
        network_bandwidth: 10_000_000_000,
        gpu_units: 4,
        special_hardware: HashMap::new(),
    };

    let json = serde_json::to_string(&resources).unwrap();
    let deserialized: SystemResources = serde_json::from_str(&json).unwrap();

    assert_eq!(resources.cpu_cores, deserialized.cpu_cores);
    assert_eq!(resources.gpu_units, deserialized.gpu_units);
}

// ============================================================================
// ResourceAllocation Tests
// ============================================================================

#[test]
fn test_resource_allocation_creation() {
    use toadstool::resources::*;
    use uuid::Uuid;

    let allocated_resources = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 4.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 8_000_000_000,
            max_bytes: None,
        },
        storage: StorageRequirements {
            min_bytes: 100_000_000_000,
            max_bytes: None,
            storage_type: None,
        },
        gpu: Some(GpuRequirements {
            min_units: 1,
            max_units: None,
            gpu_type: None,
            min_memory_bytes: None,
        }),
        network: NetworkRequirements::default(),
    };

    let allocation = ResourceAllocation {
        job_id: Uuid::new_v4(),
        allocated_resources,
        allocated_at: chrono::Utc::now(),
        released_at: None,
    };

    assert_eq!(allocation.allocated_resources.cpu.min_cores, 4.0);
    assert_eq!(
        allocation.allocated_resources.memory.min_bytes,
        8_000_000_000
    );
    assert!(allocation.released_at.is_none());
}

#[test]
fn test_resource_allocation_with_release() {
    use toadstool::resources::*;
    use uuid::Uuid;

    let allocated_resources = ResourceRequirements::default();

    let allocation = ResourceAllocation {
        job_id: Uuid::new_v4(),
        allocated_resources,
        allocated_at: chrono::Utc::now(),
        released_at: Some(chrono::Utc::now()),
    };

    assert!(allocation.released_at.is_some());
}

#[test]
fn test_resource_allocation_clone() {
    use toadstool::resources::*;
    use uuid::Uuid;

    let allocated_resources = ResourceRequirements::default();

    let alloc1 = ResourceAllocation {
        job_id: Uuid::new_v4(),
        allocated_resources,
        allocated_at: chrono::Utc::now(),
        released_at: None,
    };

    let alloc2 = alloc1.clone();
    assert_eq!(alloc1.job_id, alloc2.job_id);
    assert_eq!(
        alloc1.allocated_resources.cpu.min_cores,
        alloc2.allocated_resources.cpu.min_cores
    );
}

#[test]
fn test_resource_allocation_serialization() {
    use toadstool::resources::*;
    use uuid::Uuid;

    let allocated_resources = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 8.0,
            max_cores: Some(16.0),
            architecture: Some("x86_64".to_string()),
        },
        memory: MemoryRequirements {
            min_bytes: 16_000_000_000,
            max_bytes: Some(32_000_000_000),
        },
        storage: StorageRequirements::default(),
        gpu: Some(GpuRequirements {
            min_units: 2,
            max_units: Some(4),
            gpu_type: Some("nvidia".to_string()),
            min_memory_bytes: Some(8_000_000_000),
        }),
        network: NetworkRequirements::default(),
    };

    let allocation = ResourceAllocation {
        job_id: Uuid::new_v4(),
        allocated_resources,
        allocated_at: chrono::Utc::now(),
        released_at: None,
    };

    let json = serde_json::to_string(&allocation).unwrap();
    let deserialized: ResourceAllocation = serde_json::from_str(&json).unwrap();

    assert_eq!(allocation.job_id, deserialized.job_id);
    assert_eq!(
        allocation.allocated_resources.cpu.min_cores,
        deserialized.allocated_resources.cpu.min_cores
    );
}

// ============================================================================
// PrimalHealth Tests
// ============================================================================

#[test]
fn test_primal_health_all_variants() {
    let healthy = PrimalHealth::Healthy;
    let degraded = PrimalHealth::Degraded {
        issues: vec!["High load".to_string(), "Disk pressure".to_string()],
    };
    let unhealthy = PrimalHealth::Unhealthy {
        reason: "Connection failed".to_string(),
    };

    assert!(matches!(healthy, PrimalHealth::Healthy));

    if let PrimalHealth::Degraded { issues } = degraded {
        assert_eq!(issues.len(), 2);
        assert!(issues.contains(&"High load".to_string()));
    }

    if let PrimalHealth::Unhealthy { reason } = unhealthy {
        assert_eq!(reason, "Connection failed");
    }
}

#[test]
fn test_primal_health_clone() {
    let health1 = PrimalHealth::Healthy;
    let health2 = health1.clone();
    assert_eq!(health1, health2);

    let degraded1 = PrimalHealth::Degraded {
        issues: vec!["Test".to_string()],
    };
    let degraded2 = degraded1.clone();
    assert_eq!(degraded1, degraded2);
}

#[test]
fn test_primal_health_serialization() {
    let health = PrimalHealth::Degraded {
        issues: vec!["Memory pressure".to_string()],
    };

    let json = serde_json::to_string(&health).unwrap();
    let deserialized: PrimalHealth = serde_json::from_str(&json).unwrap();
    assert_eq!(health, deserialized);
}

// ============================================================================
// PlatformStatus Tests
// ============================================================================

#[test]
fn test_platform_status_variants() {
    let initializing = PlatformStatus::Initializing;
    let running = PlatformStatus::Running;
    let degraded = PlatformStatus::Degraded;
    let stopped = PlatformStatus::Stopped;

    assert!(matches!(initializing, PlatformStatus::Initializing));
    assert!(matches!(running, PlatformStatus::Running));
    assert!(matches!(degraded, PlatformStatus::Degraded));
    assert!(matches!(stopped, PlatformStatus::Stopped));
}

#[test]
fn test_platform_status_clone() {
    let status1 = PlatformStatus::Running;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

#[test]
fn test_platform_status_serialization() {
    let status = PlatformStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: PlatformStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(status, deserialized);
}

// ============================================================================
// Sprint 23 Complete: 55 Tests Created
// Coverage Target: 44% → 65%
// ============================================================================
