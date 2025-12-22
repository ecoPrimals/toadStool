//! Basic usage example for ToadStool
//!
//! This example demonstrates the core functionality of ToadStool including:
//! - Configuration loading
//! - Security context creation
//! - Resource requirement definition
//! - Workload specification
//! - Execution request formation
//! - Runtime orchestration

use std::collections::HashMap;
use std::time::Duration;

use toadstool::{
    execution::{ExecutionRequest, RuntimeType},
    resources::ResourceRequirements,
    security::{IsolationLevel, SecurityContext},
    workload::{ExecutableSource, WorkloadSpec},
    ToadStoolResult,
};

use toadstool::UniversalComputePlatform;
use uuid::Uuid;

#[tokio::main]
async fn main() -> ToadStoolResult<()> {
    println!("🍄 ToadStool Basic Usage Demo");
    println!("Demonstrating core functionality");

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize ToadStool Universal Compute Platform
    let _platform = UniversalComputePlatform::new().await?;
    println!("Initialized ToadStool Universal Compute Platform");

    // Create security context with standard isolation
    let security_context = SecurityContext::for_isolation_level(IsolationLevel::Standard);
    println!("Created security context");

    // Define resource requirements
    let resource_requirements = ResourceRequirements::default();

    // Create workload specification for a simple echo command
    let workload_spec = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: "/bin/echo".into(),
        },
        args: Some(vec!["Hello".to_string(), "ToadStool!".to_string()]),
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };

    // Create execution request
    let execution_request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: workload_spec,
        runtime_hint: Some(RuntimeType::Native),
        resources: resource_requirements,
        security_context,
        timeout: Some(Duration::from_secs(30)),
        environment: HashMap::new(),
        input_data: Default::default(),
        callback_config: None,
        encryption_config: None,
    };

    println!(
        "Created execution request: {:?}",
        execution_request.execution_id
    );
    println!(
        "Workload type: {:?}",
        execution_request.workload.workload_type()
    );
    println!(
        "Security isolation: {:?}",
        execution_request.security_context.isolation_level
    );

    // Note: Actual execution would require registered runtime engines
    // This example demonstrates the API structure and configuration
    println!("✅ Basic usage example completed successfully!");
    println!("📝 This demonstrates the ToadStool API structure");
    println!("🔧 To actually execute workloads, register runtime engines with the orchestrator");

    Ok(())
}
