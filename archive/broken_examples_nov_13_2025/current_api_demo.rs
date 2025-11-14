//! Current API Demonstration
//!
//! This example demonstrates the CURRENT (October 2025) ToadStool API.
//! Use this as a reference for migrating other examples.

use chrono::Utc;
use toadstool::ExecutionRequest;
use toadstool_common::config_bases::RetryConfig;
use toadstool_distributed::{
    BiologicalComputingPlatform, CpuRequirements, ExecutionTarget, GpuRequirements, JobPriority,
    MemoryRequirements, NetworkRequirements, ResourceRequirements, StorageRequirements,
    UniversalJob, UniversalJobType,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🍄 ToadStool Current API Demonstration");
    println!("=====================================\n");

    // Example 1: Basic Job Creation
    println!("📝 Example 1: Creating a basic compute job");
    let basic_job = create_basic_job();
    println!("   Job ID: {}", basic_job.job_id);
    println!("   Priority: {:?}", basic_job.priority);
    println!("   Created: {}", basic_job.created_at);
    println!(
        "   CPU Cores: {}",
        basic_job.resource_requirements.cpu.min_cores
    );
    println!();

    // Example 2: GPU Job
    println!("📝 Example 2: Creating a GPU-accelerated job");
    let gpu_job = create_gpu_job();
    println!("   Job ID: {}", gpu_job.job_id);
    println!("   Job Type: {:?}", gpu_job.job_type);
    if let Some(gpu_req) = &gpu_job.resource_requirements.gpu {
        println!("   GPU Memory: {} GB", gpu_req.min_memory_gb);
        println!("   Compute Capability: {:?}", gpu_req.compute_capability);
    }
    println!();

    // Example 3: Ecosystem Tool Job
    println!("📝 Example 3: Creating an ecosystem tool job");
    let ecosystem_job = create_ecosystem_tool_job();
    println!("   Job ID: {}", ecosystem_job.job_id);
    if let Some(UniversalJobType::EcosystemTool {
        tool_name,
        endpoint,
    }) = &ecosystem_job.job_type
    {
        println!("   Tool: {}", tool_name);
        println!("   Endpoint: {}", endpoint);
    }
    println!();

    // Example 4: Resource Requirements
    println!("📝 Example 4: Detailed resource requirements");
    let resource_job = create_resource_intensive_job();
    let req = &resource_job.resource_requirements;
    println!("   CPU: {} cores", req.cpu.min_cores);
    println!(
        "   Memory: {} GB",
        req.memory.min_bytes / (1024 * 1024 * 1024)
    );
    println!(
        "   Storage: {} GB",
        req.storage.min_bytes / (1024 * 1024 * 1024)
    );
    if let Some(bw) = req.network.bandwidth_mbps {
        println!("   Network: {} Mbps", bw);
    }
    println!();

    // Example 5: Retry Configuration
    println!("📝 Example 5: Custom retry configuration");
    let retry_job = create_job_with_retries();
    println!("   Max Attempts: {}", retry_job.retry_config.max_attempts);
    println!("   Backoff: {:?}", retry_job.retry_config.backoff_strategy);
    println!(
        "   Conditions: {} types",
        retry_job.retry_config.retry_conditions.len()
    );
    println!();

    // Example 6: Execution Targets
    println!("📝 Example 6: Different execution targets");
    demonstrate_execution_targets();
    println!();

    // Example 7: Biological Computing Platform (if applicable)
    println!("📝 Example 7: Biological computing platform example");
    let bio_platform = BiologicalComputingPlatform::ProteinFolding {
        platform: "Folding@home".to_string(),
        folding_algorithms: vec!["Rosetta".to_string(), "AMBER".to_string()],
        molecular_dynamics: true,
    };
    println!("   Platform: {:?}", bio_platform);
    println!();

    println!("✅ All examples completed successfully!");
    println!("\n📖 For migration guide, see: EXAMPLE_API_MIGRATION_GUIDE.md");

    Ok(())
}

/// Create a basic compute job with minimal configuration
fn create_basic_job() -> UniversalJob {
    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::ComputeIntensive),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: RetryConfig::default(),
        created_at: Utc::now(),
    }
}

/// Create a GPU-accelerated job
fn create_gpu_job() -> UniversalJob {
    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::GPU),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::High,
        dependencies: vec![],
        resource_requirements: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 4.0,
                max_cores: Some(8.0),
            },
            memory: MemoryRequirements {
                min_bytes: 8 * 1024 * 1024 * 1024, // 8GB
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 10 * 1024 * 1024 * 1024, // 10GB
                max_bytes: None,
            },
            network: NetworkRequirements {
                bandwidth_mbps: Some(100),
                latency_ms: Some(10),
            },
            gpu: Some(GpuRequirements {
                min_memory_gb: 8.0,
                compute_capability: Some("7.5".to_string()),
            }),
        },
        retry_config: RetryConfig::default(),
        created_at: Utc::now(),
    }
}

/// Create an ecosystem tool execution job
fn create_ecosystem_tool_job() -> UniversalJob {
    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::EcosystemTool {
            tool_name: "analysis-tool".to_string(),
            endpoint: "http://localhost:8080".to_string(),
        }),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: RetryConfig::default(),
        created_at: Utc::now(),
    }
}

/// Create a resource-intensive job with detailed requirements
fn create_resource_intensive_job() -> UniversalJob {
    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::DataProcessing),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::High,
        dependencies: vec![],
        resource_requirements: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 16.0,
                max_cores: Some(32.0),
            },
            memory: MemoryRequirements {
                min_bytes: 64 * 1024 * 1024 * 1024,        // 64GB
                max_bytes: Some(128 * 1024 * 1024 * 1024), // 128GB max
            },
            storage: StorageRequirements {
                min_bytes: 500 * 1024 * 1024 * 1024,        // 500GB
                max_bytes: Some(1024 * 1024 * 1024 * 1024), // 1TB max
            },
            network: NetworkRequirements {
                bandwidth_mbps: Some(1000), // 1Gbps
                latency_ms: Some(5),
            },
            gpu: None,
        },
        retry_config: RetryConfig::default(),
        created_at: Utc::now(),
    }
}

/// Create a job with custom retry configuration
fn create_job_with_retries() -> UniversalJob {
    use toadstool_distributed::{BackoffStrategy, RetryCondition};

    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::NetworkIntensive),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: RetryConfig {
            max_attempts: 5,
            backoff_strategy: BackoffStrategy::ExponentialJittered {
                base_ms: 1000,
                max_ms: 60000,
            },
            retry_conditions: vec![
                RetryCondition::NetworkError,
                RetryCondition::ServiceUnavailable,
                RetryCondition::TemporaryFailure,
            ],
        },
        created_at: Utc::now(),
    }
}

/// Demonstrate different execution targets
fn demonstrate_execution_targets() {
    use toadstool_distributed::{LoadBalancingStrategy, ResourceConstraints};

    // Local execution
    let _local = ExecutionTarget::Local;
    println!("   ✓ Local execution");

    // Load-balanced execution
    let _load_balanced = ExecutionTarget::LoadBalanced {
        strategy: LoadBalancingStrategy::ResourceAware,
    };
    println!("   ✓ Load-balanced execution");

    // Best available with constraints
    let _best_available = ExecutionTarget::BestAvailable {
        constraints: ResourceConstraints {
            max_cpu_cores: Some(8.0),
            max_memory_bytes: Some(16 * 1024 * 1024 * 1024),
            required_features: vec!["avx2".to_string()],
            excluded_nodes: vec![],
        },
    };
    println!("   ✓ Best available with constraints");
}
