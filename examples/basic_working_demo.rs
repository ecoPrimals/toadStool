// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tracing::{Level, info};
use uuid::Uuid;

use toadstool::{
    WorkloadType,
    execution::{ExecutionInput, ExecutionRequest, RuntimeConfig, RuntimeEngine, RuntimeType},
    resources::{ResourceMonitor, ResourceRequirements},
    security::{Capability, IsolationLevel, SecurityContext},
    workload::{ExecutableSource, WorkloadSpec},
};

use toadstool_management_monitoring::SystemResourceMonitor;
use toadstool_runtime_native::NativeRuntimeEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🍄 ToadStool Basic Working Demo");

    // Test 1: Resource Monitor
    info!("📊 Testing Resource Monitor...");
    test_resource_monitor().await?;

    // Test 2: Native Runtime Engine
    info!("🚀 Testing Native Runtime Engine...");
    test_native_runtime().await?;

    // Test 3: Integration Test
    info!("🔗 Testing Integration...");
    test_integration().await?;

    info!("✅ All tests completed successfully!");
    Ok(())
}

async fn test_resource_monitor() -> Result<(), Box<dyn std::error::Error>> {
    let monitor = SystemResourceMonitor::new();

    // Test basic functionality
    let workload_id = "test-workload-1";

    // Start monitoring
    monitor.start_monitoring(workload_id)?;
    info!("✓ Started monitoring for {}", workload_id);

    // Get metrics (now properly async)
    let metrics = monitor.get_metrics(workload_id).await?;
    info!(
        "✓ Retrieved metrics: CPU {}%, Memory {} bytes",
        metrics.cpu.usage_percent, metrics.memory.used_bytes
    );

    // Stop monitoring
    monitor.stop_monitoring(workload_id)?;
    info!("✓ Stopped monitoring for {}", workload_id);

    Ok(())
}

async fn test_native_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = NativeRuntimeEngine::new();

    // Initialize the engine
    engine.initialize(RuntimeConfig::default()).await?;
    info!("✓ Native runtime engine initialized");

    // Test capabilities
    let capabilities = engine.get_capabilities();
    info!(
        "✓ Engine supports {} workload types",
        capabilities.supported_workloads.len()
    );
    assert!(engine.supports_workload(&WorkloadType::Native));

    // Test simple execution (platform-specific)
    #[cfg(unix)]
    {
        let request = create_echo_request("Hello ToadStool!");
        let response = engine.execute(request).await?;

        match response.status {
            toadstool::execution::ExecutionStatus::Success => {
                info!("✓ Echo command executed successfully");
                if let Some(stdout) = &response.output.stdout {
                    info!("  Output: {}", stdout.trim());
                }
            }
            _ => {
                return Err(format!("Execution failed: {:?}", response.status).into());
            }
        }
    }

    #[cfg(windows)]
    {
        let request = create_windows_echo_request("Hello ToadStool!");
        let response = engine.execute(request).await?;

        match response.status {
            toadstool::execution::ExecutionStatus::Success => {
                info!("✓ Echo command executed successfully");
                if let Some(stdout) = &response.output.stdout {
                    info!("  Output: {}", stdout.trim());
                }
            }
            _ => {
                return Err(format!("Execution failed: {:?}", response.status).into());
            }
        }
    }

    // Shutdown
    engine.shutdown().await?;
    info!("✓ Native runtime engine shut down");

    Ok(())
}

async fn test_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create resource monitor
    let monitor = Arc::new(SystemResourceMonitor::new());

    // Create native runtime with monitor
    let mut engine = NativeRuntimeEngine::new()
        .with_resource_monitor(monitor.clone() as Arc<dyn ResourceMonitor>);
    engine.initialize(RuntimeConfig::default()).await?;

    info!("✓ Created integrated runtime with monitoring");

    // Execute a simple command with monitoring
    #[cfg(unix)]
    {
        let request = create_echo_request("Integration test!");
        let response = engine.execute(request).await?;

        match response.status {
            toadstool::execution::ExecutionStatus::Success => {
                info!("✓ Integrated execution successful");
                info!("  Duration: {:?}", response.duration);
                info!("  Runtime: {:?}", response.runtime_used);
            }
            _ => {
                return Err(format!("Integrated execution failed: {:?}", response.status).into());
            }
        }
    }

    engine.shutdown().await?;
    info!("✓ Integration test completed");

    Ok(())
}

#[cfg(unix)]
fn create_echo_request(message: &str) -> ExecutionRequest {
    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from("/bin/echo"),
            },
            args: Some(vec![message.to_string()]),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        runtime_hint: Some(RuntimeType::Native),
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::for_isolation_level(IsolationLevel::Basic)
            .with_capability(Capability::Execute)
            .with_capability(Capability::Read),
        timeout: Some(Duration::from_secs(5)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    }
}

#[cfg(windows)]
fn create_windows_echo_request(message: &str) -> ExecutionRequest {
    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from("cmd"),
            },
            args: Some(vec![
                "/C".to_string(),
                "echo".to_string(),
                message.to_string(),
            ]),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        runtime_hint: Some(RuntimeType::Native),
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::for_isolation_level(IsolationLevel::Basic)
            .with_capability(Capability::Execute)
            .with_capability(Capability::Read),
        timeout: Some(Duration::from_secs(5)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    }
}
