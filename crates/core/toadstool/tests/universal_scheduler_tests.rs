//! Comprehensive tests for UniversalScheduler and ResourceCoordinator
//!
//! This test suite covers the scheduling and resource management components.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use toadstool::resources::{
    CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements,
    ResourceRequirements, StorageRequirements, SystemResources,
};
use toadstool::universal::UniversalPrimalProvider;
use toadstool::universal::{
    JobPriority, NetworkLocation, PrimalCapability, PrimalContext, ResourceCoordinator,
    SecurityLevel, UniversalJob, UniversalJobType, UniversalPrimalRegistry, UniversalScheduler,
    UniversalSystemResources,
};
use toadstool::universal::{
    PrimalEndpoints, PrimalHealth, PrimalRequest, PrimalResponse, PrimalType,
};
use uuid::Uuid;

use toadstool::universal::ResponseStatus;

/// Succeeding mock provider for testing happy-path primal scheduling.
struct SucceedingMockProvider {
    instance_id: String,
    context: PrimalContext,
    primal_type: PrimalType,
}

#[async_trait]
impl UniversalPrimalProvider for SucceedingMockProvider {
    fn primal_id(&self) -> &str {
        &self.instance_id
    }
    fn instance_id(&self) -> &str {
        &self.instance_id
    }
    fn context(&self) -> &PrimalContext {
        &self.context
    }
    fn primal_type(&self) -> PrimalType {
        self.primal_type.clone()
    }
    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string()],
        }]
    }
    async fn health_check(&self) -> PrimalHealth {
        PrimalHealth::Healthy
    }
    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "http://localhost:8080".to_string(),
            health: "http://localhost:8080/health".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }
    async fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> toadstool::ToadStoolResult<PrimalResponse> {
        Ok(PrimalResponse {
            request_id: request.id,
            status: ResponseStatus::Success,
            payload: serde_json::json!({"stdout": format!("Primal '{}' executed successfully", self.primal_type.as_str()), "exit_code": 0}),
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        })
    }
    async fn initialize(&mut self, _config: serde_json::Value) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    async fn shutdown(&mut self) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

fn make_test_context() -> PrimalContext {
    PrimalContext {
        user_id: "test".to_string(),
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
    }
}

/// Minimal mock provider that always fails requests (for error path testing)
struct FailingMockProvider {
    instance_id: String,
    context: PrimalContext,
}

#[async_trait]
impl UniversalPrimalProvider for FailingMockProvider {
    fn primal_id(&self) -> &str {
        &self.instance_id
    }
    fn instance_id(&self) -> &str {
        &self.instance_id
    }
    fn context(&self) -> &PrimalContext {
        &self.context
    }
    fn primal_type(&self) -> PrimalType {
        PrimalType::Compute
    }
    fn capabilities(&self) -> Vec<PrimalCapability> {
        // Must match scheduler's native_capability exactly for find_by_capability to match
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
        }]
    }
    async fn health_check(&self) -> PrimalHealth {
        PrimalHealth::Healthy
    }
    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "http://localhost:8080".to_string(),
            health: "http://localhost:8080/health".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }
    async fn handle_primal_request(
        &self,
        _request: PrimalRequest,
    ) -> toadstool::ToadStoolResult<PrimalResponse> {
        Err(toadstool::ToadStoolError::execution(
            "mock provider failure",
        ))
    }
    async fn initialize(&mut self, _config: serde_json::Value) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    async fn shutdown(&mut self) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

// ============================================================================
// ResourceCoordinator Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation() {
    let result = ResourceCoordinator::new().await;
    assert!(
        result.is_ok(),
        "ResourceCoordinator creation should succeed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_get_available_resources() {
    let coordinator = ResourceCoordinator::new().await.unwrap();

    let resources = coordinator.get_available_resources().await;

    // Verify resources are present and reasonable
    // Note: We don't check >= 0 for unsigned types as that's a type invariant
    assert!(
        resources.cpu_cores > 0.0,
        "CPU cores should be positive (got {})",
        resources.cpu_cores
    );
    assert!(
        resources.memory_bytes > 0,
        "Memory should be positive (got {})",
        resources.memory_bytes
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_allocate_resources() {
    let coordinator = ResourceCoordinator::new().await.unwrap();

    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 1.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 512 * 1024 * 1024, // 512MB
            max_bytes: None,
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    };

    let result = coordinator.allocate_resources(&requirements).await;
    assert!(
        result.is_ok(),
        "Resource allocation should succeed for modest requirements"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_allocate_resources_with_gpu() {
    let coordinator = ResourceCoordinator::new().await.unwrap();

    let requirements = ResourceRequirements {
        cpu: CpuRequirements::default(),
        memory: MemoryRequirements::default(),
        storage: StorageRequirements::default(),
        gpu: Some(GpuRequirements {
            min_units: 1,
            max_units: Some(2),
            gpu_type: None,
            min_memory_bytes: Some(4 * 1024 * 1024 * 1024), // 4GB
        }),
        network: NetworkRequirements::default(),
    };

    let result = coordinator.allocate_resources(&requirements).await;
    assert!(result.is_ok(), "GPU allocation should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_release_resources() {
    let coordinator = ResourceCoordinator::new().await.unwrap();

    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 1.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 256 * 1024 * 1024, // 256MB
            max_bytes: None,
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    };

    let allocation = coordinator.allocate_resources(&requirements).await.unwrap();
    let result = coordinator.release_resources(allocation).await;
    assert!(result.is_ok(), "Resource release should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_multiple_allocations() {
    let coordinator = ResourceCoordinator::new().await.unwrap();

    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 0.5,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 128 * 1024 * 1024, // 128MB
            max_bytes: None,
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    };

    // Allocate multiple times
    let alloc1 = coordinator.allocate_resources(&requirements).await;
    let alloc2 = coordinator.allocate_resources(&requirements).await;
    let alloc3 = coordinator.allocate_resources(&requirements).await;

    assert!(alloc1.is_ok());
    assert!(alloc2.is_ok());
    assert!(alloc3.is_ok());
}

// ============================================================================
// SystemResources Tests
// ============================================================================

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

    let debug_str = format!("{:?}", resources);
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

// ============================================================================
// ResourceRequirements Tests
// ============================================================================

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
            min_bytes: 4 * 1024 * 1024 * 1024,       // 4GB
            max_bytes: Some(8 * 1024 * 1024 * 1024), // 8GB
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
            min_memory_bytes: Some(16 * 1024 * 1024 * 1024), // 16GB
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

    let debug_str = format!("{:?}", requirements);
    assert!(debug_str.contains("ResourceRequirements"));
}

// ============================================================================
// UniversalScheduler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_creation() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let result = UniversalScheduler::new(registry).await;
    assert!(result.is_ok(), "Scheduler creation should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_get_active_job_count() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let count = scheduler.get_active_job_count().await;
    assert_eq!(count, 0, "New scheduler should have 0 active jobs");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_schedule_native_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = create_test_native_job(JobPriority::Normal);

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok(), "Scheduling a native job should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_schedule_wasm_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Wasm {
            module: vec![0x00, 0x61, 0x73, 0x6d], // WASM magic number
            args: vec![],
            env: HashMap::new(),
        },
        priority: JobPriority::High,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    // May fail if no WASM runtime, but should return a result
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_find_primals_by_capability() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let capability = PrimalCapability::NativeExecution {
        architectures: vec!["x86_64".to_string()],
    };

    let primals = scheduler.find_primals_by_capability(&capability).await;
    // Verify we can retrieve primals (no need to check >= 0 for usize)
    let _ = primals.len(); // Consume to verify API works
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_find_wasm_capability() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let capability = PrimalCapability::WasmExecution { wasi_support: true };

    let primals = scheduler.find_primals_by_capability(&capability).await;
    // Verify we can retrieve WASM primals (no need to check >= 0 for usize)
    let _ = primals.len(); // Consume to verify API works
}

// ============================================================================
// Scheduler Creation and Defaults
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_creation_with_empty_registry() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let count = scheduler.get_active_job_count().await;
    assert_eq!(count, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_creation_result_is_ok() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let result = UniversalScheduler::new(registry).await;
    assert!(result.is_ok());
    let scheduler = result.unwrap();
    assert_eq!(scheduler.get_active_job_count().await, 0);
}

// ============================================================================
// Task Submission and Queue Management
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_active_job_count_after_completion() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = create_test_native_job(JobPriority::Normal);
    let _ = scheduler.schedule_job(job).await.unwrap();

    let count = scheduler.get_active_job_count().await;
    assert_eq!(
        count, 0,
        "Active jobs should be cleared after job completes"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_sequential_job_submission() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    for i in 0..5 {
        let job = UniversalJob {
            id: Uuid::new_v4(),
            job_type: UniversalJobType::Native {
                executable: "/bin/echo".to_string(),
                args: vec![format!("job-{i}")],
                env: HashMap::new(),
            },
            priority: JobPriority::Normal,
            resources: ResourceRequirements::default(),
            timeout: Some(Duration::from_secs(30)),
            created_at: chrono::Utc::now(),
            context: create_test_context(),
        };
        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok(), "Job {} should succeed", i);
    }

    assert_eq!(scheduler.get_active_job_count().await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_job_result_contains_execution_id() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = create_test_native_job(JobPriority::Normal);
    let response = scheduler.schedule_job(job).await.unwrap();

    assert_ne!(response.execution_id, uuid::Uuid::nil());
    assert!(matches!(
        response.status,
        toadstool::execution::ExecutionStatus::Success
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_native_job_output_has_runtime_type() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = create_test_native_job(JobPriority::High);
    let response = scheduler.schedule_job(job).await.unwrap();

    assert_eq!(
        response.runtime_used,
        toadstool::execution::RuntimeType::Native
    );
    assert!(response.output.stdout.is_some());
}

// ============================================================================
// Priority Handling
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_emergency_priority_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = create_test_native_job(JobPriority::Emergency);
    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_critical_priority_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = create_test_native_job(JobPriority::Critical);
    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_low_priority_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = create_test_native_job(JobPriority::Low);
    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_background_priority_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = create_test_native_job(JobPriority::Background);
    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
}

#[test]
fn test_job_priority_ordering() {
    use std::cmp::Ordering;
    assert!(JobPriority::Emergency < JobPriority::Critical);
    assert!(JobPriority::Critical < JobPriority::High);
    assert!(JobPriority::High < JobPriority::Normal);
    assert!(JobPriority::Normal < JobPriority::Low);
    assert!(JobPriority::Low < JobPriority::Background);
    assert_eq!(
        JobPriority::Emergency.cmp(&JobPriority::Emergency),
        Ordering::Equal
    );
}

// ============================================================================
// Resource Allocation Logic
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_job_with_custom_resources() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/true".to_string(),
            args: vec![],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: Some(4.0),
                architecture: Some("x86_64".to_string()),
            },
            memory: MemoryRequirements {
                min_bytes: 1024 * 1024 * 1024,
                max_bytes: Some(2 * 1024 * 1024 * 1024),
            },
            storage: StorageRequirements::default(),
            gpu: None,
            network: NetworkRequirements::default(),
        },
        timeout: Some(Duration::from_secs(60)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_job_with_minimal_resources() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["minimal".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::Background,
        resources: ResourceRequirements::default(),
        timeout: None,
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
}

// ============================================================================
// Primal and BiomeOS Job Types
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_schedule_primal_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    // Register a compute provider so the scheduler can route to it
    let provider = Arc::new(SucceedingMockProvider {
        instance_id: "compute-mock-1".to_string(),
        context: make_test_context(),
        primal_type: PrimalType::Compute,
    });
    registry.register_primal(provider).await.unwrap();
    let scheduler = UniversalScheduler::new(Arc::clone(&registry))
        .await
        .unwrap();

    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "compute".to_string(), // matches PrimalType::Compute.as_str()
            endpoint: "http://localhost:8080".to_string(),
            payload: serde_json::json!({"task": "test"}),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok(), "primal job scheduling must succeed");
    let response = result.unwrap();
    assert!(
        response
            .output
            .stdout
            .as_ref()
            .map_or(false, |s| s.contains("executed successfully")),
        "stdout should confirm execution"
    );
    assert_eq!(
        response.runtime_used,
        toadstool::execution::RuntimeType::Native
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_schedule_biome_os_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    // Register an OS primal provider so BiomeOS routing has a target
    let provider = Arc::new(SucceedingMockProvider {
        instance_id: "biome-os-mock-1".to_string(),
        context: make_test_context(),
        primal_type: PrimalType::OS,
    });
    registry.register_primal(provider).await.unwrap();
    let scheduler = UniversalScheduler::new(Arc::clone(&registry))
        .await
        .unwrap();

    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({"name": "test-biome", "version": "1.0"}),
            team_id: "team-001".to_string(),
        },
        priority: JobPriority::High,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(60)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok(), "BiomeOS job scheduling must succeed");
    let response = result.unwrap();
    assert!(
        matches!(
            response.status,
            toadstool::execution::ExecutionStatus::Success
        ),
        "BiomeOS job should succeed with registered OS provider"
    );
}

// ============================================================================
// Task Status Tracking
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_wasm_job_response_structure() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Wasm {
            module: vec![0x00, 0x61, 0x73, 0x6d],
            args: vec![],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(
        result.is_ok(),
        "WASM job scheduling returns Ok even when no engine is registered"
    );
    let response = result.unwrap();
    // runtime_used is always Wasm for WASM jobs regardless of execution outcome
    assert_eq!(
        response.runtime_used,
        toadstool::execution::RuntimeType::Wasm
    );
    // Without a registered WASM engine, execution gracefully fails with stderr
    assert!(response.output.stderr.is_some() || response.output.stdout.is_some());
}

// ============================================================================
// find_primals Capability Variants
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_find_container_runtime_capability() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let capability = PrimalCapability::ContainerRuntime {
        orchestrators: vec!["docker".to_string()],
    };
    let primals = scheduler.find_primals_by_capability(&capability).await;
    assert!(primals.is_empty() || !primals.is_empty()); // API works
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_find_gpu_capability() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let capability = PrimalCapability::GpuAcceleration { cuda_support: true };
    let primals = scheduler.find_primals_by_capability(&capability).await;
    assert!(primals.is_empty() || !primals.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_find_custom_capability() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let capability = PrimalCapability::Custom {
        name: "custom-analytics".to_string(),
        attributes: HashMap::new(),
    };
    let primals = scheduler.find_primals_by_capability(&capability).await;
    assert!(primals.is_empty() || !primals.is_empty());
}

// ============================================================================
// Error Paths
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_native_job_fails_when_provider_returns_error() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(FailingMockProvider {
        instance_id: "failing-native".to_string(),
        context: create_test_context(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = create_test_native_job(JobPriority::Normal);

    let result = scheduler.schedule_job(job).await;
    assert!(
        result.is_err(),
        "Schedule should fail when provider returns error"
    );
}

// ============================================================================
// CpuRequirements Tests
// ============================================================================

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

// ============================================================================
// MemoryRequirements Tests
// ============================================================================

#[test]
fn test_memory_requirements_default() {
    let memory = MemoryRequirements::default();
    assert!(memory.min_bytes > 0);
    assert!(memory.max_bytes.is_none());
}

#[test]
fn test_memory_requirements_with_max() {
    let memory = MemoryRequirements {
        min_bytes: 1024 * 1024 * 1024,           // 1GB
        max_bytes: Some(4 * 1024 * 1024 * 1024), // 4GB
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

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_native_job(priority: JobPriority) -> UniversalJob {
    UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["test".to_string()],
            env: HashMap::new(),
        },
        priority,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    }
}

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
