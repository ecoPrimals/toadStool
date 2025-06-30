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
use std::path::PathBuf;
use std::time::Duration;

use toadstool::{
    execution::{ExecutionRequest, ExecutionInput, RuntimeType},
    resources::ResourceRequirements,
    security::{SecurityContext, IsolationLevel, Capability},
    workload::{WorkloadSpec, ExecutableSource},
    runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy},
    error::ToadStoolResult,
};
use toadstool_config::{load_config, ToadStoolConfig};
use uuid::Uuid;

#[tokio::main]
async fn main() -> ToadStoolResult<()> {
    // Initialize logging
    env_logger::init();
    
    // Load configuration
    let config = load_config().unwrap_or_else(|_| {
        println!("Using default configuration");
        ToadStoolConfig::default()
    });
    println!("Loaded configuration successfully");
    
    // Create security context with standard isolation
    let security_context = SecurityContext::for_isolation_level(IsolationLevel::Standard)
        .with_capability(Capability::Execute)
        .with_capability(Capability::Read);
    
    // Define resource requirements
    let resource_requirements = ResourceRequirements::default();
    
    // Create workload specification for a simple echo command
    let workload_spec = WorkloadSpec::Native {
        executable: ExecutableSource::File { 
            path: PathBuf::from("/bin/echo")
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
        input_data: ExecutionInput::default(),
        callback_config: None,
    };
    
    // Create runtime orchestrator
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    
    println!("Created execution request: {:?}", execution_request.execution_id);
    println!("Workload type: {:?}", execution_request.workload.workload_type());
    println!("Security isolation: {:?}", execution_request.security_context.isolation_level);
    
    // Note: Actual execution would require registered runtime engines
    // This example demonstrates the API structure and configuration
    
    Ok(())
} 