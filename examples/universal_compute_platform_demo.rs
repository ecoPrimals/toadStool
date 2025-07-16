//! # Universal Compute Platform Demo
//!
//! This demo showcases ToadStool's Universal Compute Platform capabilities:
//! - Universal scheduling across diverse substrates
//! - Recursive hosting (ToadStool hosting ToadStool)
//! - Ecosystem calling (invoking other ecosystem services)
//! - OS-layer compatibility (acting as an OS when needed)
//! - Songbird network effects (massive distribution)
//! - Universal orchestration (coordinating complex workflows)

use std::collections::HashMap;
use std::time::Duration;

use chrono::Utc;
use tokio::time::sleep;
use tracing::info;
use uuid::Uuid;

use toadstool::{
    ExecutionInput, ExecutionRequest, ResourceRequirements, RuntimeType, SecurityContext,
    ToadStoolResult, WorkloadSpec,
};
use toadstool_distributed::{
    types::jobs::LoadBalancingStrategy, types::resources::ResourceLimits, CompatibilityMode,
    ExecutionTarget, JobPriority, RetryConfig, ToadStoolHostingConfig, UniversalJob,
    UniversalJobQueue, UniversalJobType,
};

use toadstool_distributed::universal::{
    FaultToleranceConfig, NetworkEffectsConfig, NetworkLoadBalancing, RecursiveHostingConfig,
    ResourceSharingConfig, SchedulingAlgorithm, UniversalScheduler, UniversalSchedulerConfig,
};

use toadstool_distributed::universal::scheduler::{OSLayerConfig, SongbirdIntegrationConfig};

/// Universal ToadStool Platform - the complete compute orchestrator
pub struct UniversalToadStoolPlatform {
    /// Universal scheduler for job distribution
    scheduler: UniversalScheduler,
    /// Job queue for managing execution
    job_queue: UniversalJobQueue,
    /// Platform metadata
    platform_metadata: PlatformMetadata,
}

#[derive(Debug, Clone)]
pub struct PlatformMetadata {
    pub platform_id: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

#[tokio::main]
async fn main() -> ToadStoolResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Universal Compute Platform Demo");
    info!("🌟 Demonstrating ToadStool's universal computing capabilities");

    // Initialize the universal platform
    let platform = initialize_universal_platform().await?;

    info!("🎯 Platform initialized successfully!");
    info!("📋 Running comprehensive demonstrations...");

    // Run all demonstrations
    demonstrate_universal_scheduling(&platform).await?;
    demonstrate_recursive_hosting(&platform).await?;
    demonstrate_ecosystem_calling(&platform).await?;
    demonstrate_os_layer_compatibility(&platform).await?;
    demonstrate_songbird_network_effects(&platform).await?;
    demonstrate_universal_orchestration(&platform).await?;

    info!("✅ Universal Compute Platform Demo completed successfully!");
    info!("🌟 ToadStool demonstrates true universal computing:");
    info!("  • Any workload, anywhere, anytime");
    info!("  • Self-hosting and recursive capabilities");
    info!("  • Ecosystem integration and collaboration");
    info!("  • OS-layer compatibility when needed");
    info!("  • Network effects through Songbird");
    info!("  • Universal orchestration across all substrates");

    Ok(())
}

/// Initialize the Universal ToadStool Platform
async fn initialize_universal_platform() -> ToadStoolResult<UniversalToadStoolPlatform> {
    info!("🔧 Initializing Universal ToadStool Platform...");

    // Create scheduler configuration for universal compute
    let scheduler_config = UniversalSchedulerConfig {
        scheduling_algorithms: vec![
            SchedulingAlgorithm::Priority,
            SchedulingAlgorithm::ResourceAware,
            SchedulingAlgorithm::NetworkAware,
        ],
        network_effects: NetworkEffectsConfig {
            enabled: true,
            load_balancing: NetworkLoadBalancing {
                strategy: LoadBalancingStrategy::ResourceAware,
                health_check_interval_ms: 5000,
                failover_threshold: 3,
            },
            resource_sharing: ResourceSharingConfig {
                enabled: true,
                sharing_ratio: 0.8,
                priority_boost: 1.2,
            },
            fault_tolerance: FaultToleranceConfig {
                enabled: true,
                max_retries: 3,
                circuit_breaker_threshold: 5,
            },
        },
        songbird_integration: SongbirdIntegrationConfig {
            enabled: true,
            endpoint: "http://songbird:8080".to_string(),
            auth_token: Some("demo-token".to_string()),
        },
        recursive_hosting: RecursiveHostingConfig::default(),
        os_layer: OSLayerConfig::default(),
    };

    // Initialize the scheduler
    let scheduler = UniversalScheduler::new(scheduler_config).await?;

    // Initialize job queue
    let job_queue = UniversalJobQueue::new();

    // Create platform metadata
    let platform_metadata = PlatformMetadata {
        platform_id: "universal-toadstool-platform".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![
            "universal-scheduling".to_string(),
            "recursive-hosting".to_string(),
            "ecosystem-calling".to_string(),
            "os-layer-compatibility".to_string(),
            "songbird-network-effects".to_string(),
            "universal-orchestration".to_string(),
        ],
    };

    let platform = UniversalToadStoolPlatform {
        scheduler,
        job_queue,
        platform_metadata,
    };

    info!("✅ Universal ToadStool Platform initialized");
    info!(
        "📊 Platform capabilities: {:?}",
        platform.platform_metadata.capabilities
    );

    Ok(platform)
}

/// Demonstrate Universal Scheduling
async fn demonstrate_universal_scheduling(
    platform: &UniversalToadStoolPlatform,
) -> ToadStoolResult<()> {
    info!("📅 Demonstrating Universal Scheduling...");

    // Create diverse job types to showcase universal scheduling
    let jobs = vec![
        create_universal_job(
            UniversalJobType::Local,
            JobPriority::High,
            "High-priority local computation",
        ),
        create_universal_job(
            UniversalJobType::ComputeIntensive,
            JobPriority::Normal,
            "CPU-intensive mathematical computation",
        ),
        create_universal_job(
            UniversalJobType::DataProcessing,
            JobPriority::Normal,
            "Large dataset processing task",
        ),
        create_universal_job(
            UniversalJobType::MachineLearning,
            JobPriority::High,
            "AI model training workload",
        ),
        create_universal_job(
            UniversalJobType::Simulation,
            JobPriority::Low,
            "Scientific simulation task",
        ),
        create_universal_job(
            UniversalJobType::Container,
            JobPriority::Normal,
            "Containerized application execution",
        ),
        create_universal_job(
            UniversalJobType::WASM,
            JobPriority::Normal,
            "WebAssembly module execution",
        ),
        create_universal_job(
            UniversalJobType::GPU,
            JobPriority::High,
            "GPU-accelerated computation",
        ),
    ];

    // Schedule all jobs through the universal scheduler
    for job in jobs {
        let job_description = format!("{:?}", job.job_type);
        platform.scheduler.schedule_job(job).await?;
        info!("📋 Scheduled: {}", job_description);
        sleep(Duration::from_millis(100)).await;
    }

    info!("🎯 Universal scheduling demonstrates ToadStool's ability to:");
    info!("  • Handle any workload type intelligently");
    info!("  • Prioritize jobs based on business requirements");
    info!("  • Distribute work across available resources");
    info!("  • Adapt to different computational needs");

    Ok(())
}

/// Demonstrate Recursive Hosting
async fn demonstrate_recursive_hosting(
    platform: &UniversalToadStoolPlatform,
) -> ToadStoolResult<()> {
    info!("🔄 Demonstrating Recursive Hosting...");

    // Create ToadStool hosting configuration
    let hosting_config = ToadStoolHostingConfig {
        enabled: true,
        mode: "recursive".to_string(),
        resource_limits: {
            let mut limits = HashMap::new();
            limits.insert("cpu_cores".to_string(), 2);
            limits.insert("memory_gb".to_string(), 4);
            limits.insert("storage_gb".to_string(), 20);
            limits
        },
        security_settings: {
            let mut settings = HashMap::new();
            settings.insert("isolation_level".to_string(), "high".to_string());
            settings.insert("network_access".to_string(), "restricted".to_string());
            settings
        },
        resource_allocation: None,
    };

    // Create recursive hosting jobs
    let recursive_jobs = vec![
        UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: Some(UniversalJobType::RecursiveHosting {
                toadstool_config: hosting_config.clone(),
            }),
            execution_request: create_execution_request("Host child ToadStool instance #1"),
            target: ExecutionTarget::Local,
            priority: JobPriority::High,
            dependencies: Vec::new(),
            resource_requirements: toadstool_distributed::ResourceRequirements::default(),
            retry_config: RetryConfig::default(),
            created_at: Utc::now(),
        },
        UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: Some(UniversalJobType::RecursiveHosting {
                toadstool_config: hosting_config.clone(),
            }),
            execution_request: create_execution_request("Host child ToadStool instance #2"),
            target: ExecutionTarget::Local,
            priority: JobPriority::Normal,
            dependencies: Vec::new(),
            resource_requirements: toadstool_distributed::ResourceRequirements::default(),
            retry_config: RetryConfig::default(),
            created_at: Utc::now(),
        },
    ];

    for (i, job) in recursive_jobs.into_iter().enumerate() {
        platform.scheduler.schedule_job(job).await?;
        info!("🔄 Hosted child ToadStool instance #{}", i + 1);
        sleep(Duration::from_millis(200)).await;
    }

    info!("🏗️ Recursive hosting demonstrates ToadStool's ability to:");
    info!("  • Host other ToadStool instances within itself");
    info!("  • Create isolated execution environments");
    info!("  • Manage resource allocation for children");
    info!("  • Enable fractal computing architectures");

    Ok(())
}

/// Demonstrate Ecosystem Calling
async fn demonstrate_ecosystem_calling(
    platform: &UniversalToadStoolPlatform,
) -> ToadStoolResult<()> {
    info!("🌐 Demonstrating Ecosystem Calling...");

    // Create ecosystem service jobs
    let ecosystem_services = vec![
        (
            "nestgate",
            "http://nestgate:8081",
            "Data pipeline orchestration",
        ),
        ("beardog", "http://beardog:8082", "Cryptographic operations"),
        ("songbird", "http://songbird:8083", "Network coordination"),
        ("primals", "http://primals:8084", "Primitive service calls"),
    ];

    for (service_name, endpoint, description) in ecosystem_services {
        let ecosystem_job = UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: Some(UniversalJobType::EcosystemTool {
                tool_name: service_name.to_string(),
                endpoint: endpoint.to_string(),
            }),
            execution_request: create_execution_request(description),
            target: ExecutionTarget::EcosystemService {
                service_name: service_name.to_string(),
                endpoint: endpoint.to_string(),
            },
            priority: JobPriority::Normal,
            dependencies: Vec::new(),
            resource_requirements: toadstool_distributed::ResourceRequirements::default(),
            retry_config: RetryConfig::default(),
            created_at: Utc::now(),
        };

        platform.scheduler.schedule_job(ecosystem_job).await?;
        info!(
            "🌐 Called ecosystem service: {} ({})",
            service_name, description
        );
        sleep(Duration::from_millis(150)).await;
    }

    info!("🤝 Ecosystem calling demonstrates ToadStool's ability to:");
    info!("  • Integrate with other ecosystem services");
    info!("  • Orchestrate complex multi-service workflows");
    info!("  • Maintain service discovery and routing");
    info!("  • Enable collaborative computing");

    Ok(())
}

/// Demonstrate OS-Layer Compatibility
async fn demonstrate_os_layer_compatibility(
    _platform: &UniversalToadStoolPlatform,
) -> ToadStoolResult<()> {
    info!("🖥️ Demonstrating OS-Layer Compatibility...");

    let scheduler_config = UniversalSchedulerConfig {
        scheduling_algorithms: vec![SchedulingAlgorithm::Priority],
        network_effects: NetworkEffectsConfig {
            enabled: false,
            load_balancing: NetworkLoadBalancing {
                strategy: LoadBalancingStrategy::RoundRobin,
                health_check_interval_ms: 60000,
                failover_threshold: 3,
            },
            resource_sharing: ResourceSharingConfig {
                enabled: false,
                sharing_ratio: 0.0,
                priority_boost: 1.0,
            },
            fault_tolerance: FaultToleranceConfig {
                enabled: true,
                max_retries: 3,
                circuit_breaker_threshold: 5,
            },
        },
        songbird_integration: SongbirdIntegrationConfig {
            enabled: false,
            endpoint: "http://localhost:8080".to_string(),
            auth_token: None,
        },
        recursive_hosting: RecursiveHostingConfig::default(),
        os_layer: OSLayerConfig {
            virtual_filesystem_enabled: true,
            process_virtualization_enabled: true,
            network_virtualization_enabled: true,
            compatibility_modes: vec![
                CompatibilityMode::LinuxCompat,
                CompatibilityMode::WindowsCompat,
                CompatibilityMode::MacOSCompat,
                CompatibilityMode::ContainerCompat,
                CompatibilityMode::LegacyCompat {
                    system_type: "unix_legacy".to_string(),
                },
            ],
            os_layer_resource_limits: ResourceLimits::default(),
        },
    };

    let scheduler = UniversalScheduler::new(scheduler_config).await?;

    // Demonstrate different compatibility modes
    let compatibility_scenarios = vec![
        (
            CompatibilityMode::LinuxCompat,
            "Linux system compatibility layer",
        ),
        (
            CompatibilityMode::WindowsCompat,
            "Windows system compatibility layer",
        ),
        (
            CompatibilityMode::MacOSCompat,
            "macOS system compatibility layer",
        ),
        (
            CompatibilityMode::ContainerCompat,
            "Container runtime compatibility",
        ),
        (
            CompatibilityMode::LegacyCompat {
                system_type: "unix_legacy".to_string(),
            },
            "Legacy UNIX system compatibility",
        ),
    ];

    for (compat_mode, description) in compatibility_scenarios {
        let compat_job = UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: Some(UniversalJobType::OSLayerCompatibility {
                compatibility_mode: compat_mode.clone(),
            }),
            execution_request: create_execution_request(description),
            target: ExecutionTarget::Local,
            priority: JobPriority::Normal,
            dependencies: Vec::new(),
            resource_requirements: toadstool_distributed::ResourceRequirements::default(),
            retry_config: RetryConfig::default(),
            created_at: Utc::now(),
        };

        scheduler.schedule_job(compat_job).await?;
        info!(
            "🖥️ Executed with compatibility mode {:?}: {}",
            compat_mode, description
        );
        sleep(Duration::from_millis(300)).await;
    }

    info!("🛡️ OS-Layer compatibility demonstrates ToadStool's ability to:");
    info!("  • Act as an OS when local environment is incompatible");
    info!("  • Provide Linux compatibility on non-Linux systems");
    info!("  • Emulate Windows environments when needed");
    info!("  • Support macOS-specific workloads universally");
    info!("  • Handle legacy system requirements");

    Ok(())
}

/// Demonstrate Songbird Network Effects
async fn demonstrate_songbird_network_effects(
    _platform: &UniversalToadStoolPlatform,
) -> ToadStoolResult<()> {
    info!("🎼 Demonstrating Songbird Network Effects...");

    let scheduler_config = UniversalSchedulerConfig {
        scheduling_algorithms: vec![
            SchedulingAlgorithm::ResourceAware,
            SchedulingAlgorithm::NetworkAware,
        ],
        network_effects: NetworkEffectsConfig {
            enabled: true,
            load_balancing: NetworkLoadBalancing {
                strategy: LoadBalancingStrategy::ResourceAware,
                health_check_interval_ms: 5000,
                failover_threshold: 2,
            },
            resource_sharing: ResourceSharingConfig {
                enabled: true,
                sharing_ratio: 0.9,
                priority_boost: 1.5,
            },
            fault_tolerance: FaultToleranceConfig {
                enabled: true,
                max_retries: 5,
                circuit_breaker_threshold: 3,
            },
        },
        songbird_integration: SongbirdIntegrationConfig {
            enabled: true,
            endpoint: "http://songbird:8080".to_string(),
            auth_token: Some("network-demo-token".to_string()),
        },
        recursive_hosting: RecursiveHostingConfig::default(),
        os_layer: OSLayerConfig {
            virtual_filesystem_enabled: false,
            process_virtualization_enabled: false,
            network_virtualization_enabled: true,
            compatibility_modes: Vec::new(),
            os_layer_resource_limits: ResourceLimits::default(),
        },
    };

    let scheduler = UniversalScheduler::new(scheduler_config).await?;

    // Create jobs that benefit from network effects
    let network_jobs = vec![
        create_universal_job(
            UniversalJobType::Local,
            JobPriority::High,
            "High-load computation requiring network distribution",
        ),
        create_universal_job(
            UniversalJobType::EcosystemTool {
                tool_name: "songbird".to_string(),
                endpoint: "http://songbird:8080".to_string(),
            },
            JobPriority::Normal,
            "Service discovery with load balancing",
        ),
        create_universal_job(
            UniversalJobType::RemoteToadStool {
                endpoint: "http://network-toadstool:8082".to_string(),
            },
            JobPriority::Normal,
            "Cross-network ToadStool execution",
        ),
    ];

    for job in network_jobs {
        let job_description = format!("{:?}", job.job_type);
        scheduler.schedule_job(job).await?;
        info!("🎼 Network job scheduled: {}", job_description);
        sleep(Duration::from_millis(200)).await;
    }

    info!("🌐 Songbird network effects demonstrate ToadStool's ability to:");
    info!("  • Leverage network resources for massive distribution");
    info!("  • Coordinate with other ToadStool instances");
    info!("  • Implement intelligent load balancing");
    info!("  • Provide fault tolerance across the network");

    Ok(())
}

/// Demonstrate Universal Orchestration
async fn demonstrate_universal_orchestration(
    _platform: &UniversalToadStoolPlatform,
) -> ToadStoolResult<()> {
    info!("🎭 Demonstrating Universal Orchestration...");

    let scheduler_config = UniversalSchedulerConfig {
        scheduling_algorithms: vec![
            SchedulingAlgorithm::Priority,
            SchedulingAlgorithm::ResourceAware,
            SchedulingAlgorithm::NetworkAware,
        ],
        network_effects: NetworkEffectsConfig {
            enabled: true,
            load_balancing: NetworkLoadBalancing {
                strategy: LoadBalancingStrategy::ResourceAware,
                health_check_interval_ms: 3000,
                failover_threshold: 2,
            },
            resource_sharing: ResourceSharingConfig {
                enabled: true,
                sharing_ratio: 0.85,
                priority_boost: 1.3,
            },
            fault_tolerance: FaultToleranceConfig {
                enabled: true,
                max_retries: 4,
                circuit_breaker_threshold: 4,
            },
        },
        songbird_integration: SongbirdIntegrationConfig {
            enabled: true,
            endpoint: "http://songbird:8080".to_string(),
            auth_token: Some("orchestration-token".to_string()),
        },
        recursive_hosting: RecursiveHostingConfig::default(),
        os_layer: OSLayerConfig::default(),
    };

    let scheduler = UniversalScheduler::new(scheduler_config).await?;

    // Create a complex orchestrated workflow
    let ai_job_id = Uuid::new_v4();
    let ai_job = UniversalJob {
        job_id: ai_job_id,
        job_type: Some(UniversalJobType::EcosystemTool {
            tool_name: "ai-training-service".to_string(),
            endpoint: "http://ai-service:8088".to_string(),
        }),
        execution_request: create_execution_request("Train AI model for data analysis"),
        target: ExecutionTarget::EcosystemService {
            service_name: "ai-training-service".to_string(),
            endpoint: "http://ai-service:8088".to_string(),
        },
        priority: JobPriority::High,
        dependencies: Vec::new(),
        resource_requirements: toadstool_distributed::ResourceRequirements::default(),
        retry_config: RetryConfig::default(),
        created_at: Utc::now(),
    };

    scheduler.schedule_job(ai_job).await?;
    info!("🤖 Scheduled AI Model Training: {:?}", ai_job_id);

    let data_job_id = Uuid::new_v4();
    let data_processing_job = UniversalJob {
        job_id: data_job_id,
        job_type: Some(UniversalJobType::OSLayerCompatibility {
            compatibility_mode: CompatibilityMode::LinuxCompat,
        }),
        execution_request: create_execution_request("Process large dataset with Linux tools"),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![ai_job_id], // Depends on AI training
        resource_requirements: toadstool_distributed::ResourceRequirements::default(),
        retry_config: RetryConfig::default(),
        created_at: Utc::now(),
    };

    scheduler.schedule_job(data_processing_job).await?;
    info!(
        "📊 Scheduled Data Processing (depends on AI training): {:?}",
        data_job_id
    );

    // Create hosting configuration with default implementation
    let hosting_config = ToadStoolHostingConfig {
        enabled: true,
        mode: "recursive".to_string(),
        resource_limits: HashMap::new(),
        security_settings: HashMap::new(),
        resource_allocation: None,
    };

    let recursive_job_id = Uuid::new_v4();
    let recursive_hosting_job = UniversalJob {
        job_id: recursive_job_id,
        job_type: Some(UniversalJobType::RecursiveHosting {
            toadstool_config: hosting_config,
        }),
        execution_request: create_execution_request("Host distributed computation instance"),
        target: ExecutionTarget::Local,
        priority: JobPriority::Low,
        dependencies: Vec::new(),
        resource_requirements: toadstool_distributed::ResourceRequirements::default(),
        retry_config: RetryConfig::default(),
        created_at: Utc::now(),
    };

    scheduler.schedule_job(recursive_hosting_job).await?;
    info!(
        "🔄 Scheduled Recursive Distribution: {:?}",
        recursive_job_id
    );

    let legacy_job_id = Uuid::new_v4();
    let legacy_integration_job = UniversalJob {
        job_id: legacy_job_id,
        job_type: Some(UniversalJobType::OSLayerCompatibility {
            compatibility_mode: CompatibilityMode::LegacyCompat {
                system_type: "mainframe_cobol".to_string(),
            },
        }),
        execution_request: create_execution_request("Integrate with legacy mainframe system"),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![data_job_id], // Depends on data processing
        resource_requirements: toadstool_distributed::ResourceRequirements::default(),
        retry_config: RetryConfig::default(),
        created_at: Utc::now(),
    };

    scheduler.schedule_job(legacy_integration_job).await?;
    info!(
        "🏛️ Scheduled Legacy Integration (depends on data processing): {:?}",
        legacy_job_id
    );

    info!("🎭 Universal orchestration demonstrates ToadStool's ability to:");
    info!("  • Coordinate complex multi-step workflows");
    info!("  • Handle dependencies between different job types");
    info!("  • Integrate AI, data processing, hosting, and legacy systems");
    info!("  • Provide end-to-end workflow management");

    Ok(())
}

/// Create a universal job for demonstration
fn create_universal_job(
    job_type: UniversalJobType,
    priority: JobPriority,
    description: &str,
) -> UniversalJob {
    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(job_type),
        execution_request: create_execution_request(description),
        target: ExecutionTarget::Local,
        priority,
        dependencies: Vec::new(),
        resource_requirements: toadstool_distributed::ResourceRequirements::default(),
        retry_config: RetryConfig::default(),
        created_at: Utc::now(),
    }
}

/// Create an execution request for demonstration
fn create_execution_request(description: &str) -> ExecutionRequest {
    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::default(),
        runtime_hint: Some(RuntimeType::Native),
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: Some(Duration::from_secs(300)),
        environment: HashMap::new(),
        input_data: ExecutionInput {
            data: description.as_bytes().to_vec(),
            format: Some("text/plain".to_string()),
            metadata: HashMap::new(),
        },
        callback_config: None,
    }
}
