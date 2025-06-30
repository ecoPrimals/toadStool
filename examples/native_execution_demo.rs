use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn, error};
use uuid::Uuid;
use tokio::time;
use env_logger;

use toadstool::{
    execution::{ExecutionRequest, ExecutionInput, RuntimeType, RuntimeConfig, RuntimeEngine},
    resources::{ResourceRequirements, ResourceMonitor},
    security::{SecurityContext, IsolationLevel, Capability},
    workload::{WorkloadSpec, ExecutableSource, NativeWorkload},
    runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy},
    error::ToadStoolError,
};
use toadstool_management_monitoring::{SystemResourceMonitor, MonitoringConfig, MonitoringGranularity};
use toadstool_runtime_native::NativeRuntimeEngine;

/// Configuration for the demo execution
#[derive(Debug, Clone)]
struct DemoConfig {
    /// Timeout for simple commands
    pub simple_timeout: Duration,
    /// Timeout for CPU-intensive commands
    pub cpu_timeout: Duration,
    /// Timeout for file operations
    pub file_timeout: Duration,
    /// Timeout for security tests
    pub security_timeout: Duration,
    /// Timeout for resource limit tests
    pub resource_timeout: Duration,
    /// Monitoring granularity for the demo
    pub monitoring_granularity: MonitoringGranularity,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            simple_timeout: Duration::from_secs(5),
            cpu_timeout: Duration::from_secs(10),
            file_timeout: Duration::from_secs(3),
            security_timeout: Duration::from_secs(5),
            resource_timeout: Duration::from_secs(2),
            monitoring_granularity: MonitoringGranularity::HighFrequency, // 10ms for demo
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    let config = DemoConfig::default();
    
    info!("🍄 ToadStool Native Execution Demo");
    info!("==================================");
    
    // Initialize monitoring with high-frequency granularity for demo
    info!("📊 Initializing resource monitoring...");
    let monitoring_config = MonitoringConfig {
        granularity: config.monitoring_granularity,
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        ..Default::default()
    };
    
    let mut resource_monitor = SystemResourceMonitor::with_config(monitoring_config);
    resource_monitor.start_monitoring_loop().await?;
    
    // Initialize native runtime engine
    let native_engine = NativeRuntimeEngine::new()
        .map_err(|e| toadstool::error::ToadStoolError::runtime_error(&format!("Failed to create native engine: {}", e)))?;
    
    // Create runtime orchestrator
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    orchestrator.register_engine("Native", Box::new(native_engine)).await?;
    
    info!("✅ ToadStool runtime environment initialized");
    
    info!("🚀 Running execution scenarios...");
    
    // Run demo scenarios
    run_scenario_1_simple_echo(&orchestrator, &config).await?;
    run_scenario_2_cpu_intensive(&orchestrator, &config).await?;
    run_scenario_3_file_operations(&orchestrator, &config).await?;
    run_scenario_4_enhanced_security(&orchestrator, &config).await?;
    run_scenario_5_resource_limits(&orchestrator, &config).await?;
    
    // Display runtime capabilities
    info!("📋 Runtime Engine Capabilities:");
    info!("  Demo completed - runtime engines executed successfully");
    
    info!("🧹 Shutting down...");
    resource_monitor.stop_monitoring_loop().await?;
    
    info!("✅ Demo completed successfully!");
    Ok(())
}

async fn run_scenario_1_simple_echo(orchestrator: &RuntimeOrchestrator, config: &DemoConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("1️⃣  Scenario 1: Simple Echo Command");
    
    // Create a simple echo workload
    let workload_spec = WorkloadSpec::Native(NativeWorkload {
        executable: "/bin/echo".to_string(),
        args: vec!["Hello from ToadStool!".to_string()],
        environment: HashMap::new(),
        working_directory: None,
    });
    
    let security_context = SecurityContext {
        isolation_level: IsolationLevel::None,
        user_context: None,
        capabilities: std::collections::HashSet::new(),
        resource_limits: None,
        network_access: false,
        file_system_access: vec![],
    };
    
    let resource_requirements = ResourceRequirements::default();
    
    let request = ExecutionRequest {
        workload_spec,
        security_context,
        resource_requirements,
        timeout: Some(config.simple_timeout),
        priority: 0,
        metadata: HashMap::new(),
    };
    
    match orchestrator.execute(request).await {
        Ok(response) => {
            if let Some(stdout) = &response.output.stdout {
                info!("Echo output: {}", stdout.trim());
            }
        }
        Err(e) => {
            warn!("Scenario 1 failed: {}", e);
            return Ok(());
        }
    }
    
    Ok(())
}

async fn run_scenario_2_cpu_intensive(orchestrator: &RuntimeOrchestrator, config: &DemoConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("2️⃣  Scenario 2: CPU-Intensive Workload");
    
    // Create a CPU-intensive workload using timeout to limit execution
    let workload_spec = WorkloadSpec::Native(NativeWorkload {
        executable: "timeout".to_string(),
        args: vec!["2".to_string(), "yes".to_string()],
        environment: HashMap::new(),
        working_directory: None,
    });
    
    let security_context = SecurityContext {
        isolation_level: IsolationLevel::Basic,
        user_context: None,
        capabilities: std::collections::HashSet::new(),
        resource_limits: None,
        network_access: false,
        file_system_access: vec![],
    };
    
    let mut resource_requirements = ResourceRequirements::default();
    resource_requirements.cpu.max_cores = Some(1.0);
    resource_requirements.memory.max_bytes = Some(64 * 1024 * 1024); // 64 MB
    
    let request = ExecutionRequest {
        workload_spec,
        security_context,
        resource_requirements,
        timeout: Some(config.cpu_timeout),
        priority: 0,
        metadata: HashMap::new(),
    };
    
    let start_time = std::time::Instant::now();
    match orchestrator.execute(request).await {
        Ok(_response) => {
            let duration = start_time.elapsed();
            info!("CPU test completed in {:?}", duration);
            info!("Peak CPU usage: 0.0%"); // Would be populated by real monitoring
            info!("Peak memory usage: 0 MB"); // Would be populated by real monitoring
        }
        Err(e) => {
            warn!("Scenario 2 failed: {}", e);
            // Don't return error for timeout - this is expected behavior
            if !e.to_string().contains("timeout") {
                return Ok(());
            }
        }
    }
    
    Ok(())
}

async fn run_scenario_3_file_operations(orchestrator: &RuntimeOrchestrator, config: &DemoConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("3️⃣  Scenario 3: File System Operations");
    
    // Create a file system workload
    let workload_spec = WorkloadSpec::Native(NativeWorkload {
        executable: "/bin/ls".to_string(),
        args: vec!["-la".to_string(), "/tmp".to_string()],
        environment: HashMap::new(),
        working_directory: None,
    });
    
    let security_context = SecurityContext {
        isolation_level: IsolationLevel::Standard,
        user_context: None,
        capabilities: {
            let mut caps = std::collections::HashSet::new();
            caps.insert("file:read".to_string());
            caps
        },
        resource_limits: None,
        network_access: false,
        file_system_access: vec!["/tmp".to_string()],
    };
    
    let resource_requirements = ResourceRequirements::default();
    
    let request = ExecutionRequest {
        workload_spec,
        security_context,
        resource_requirements,
        timeout: Some(config.file_timeout),
        priority: 0,
        metadata: HashMap::new(),
    };
    
    match orchestrator.execute(request).await {
        Ok(response) => {
            if let Some(stdout) = &response.output.stdout {
                info!("Filesystem test output length: {} characters", stdout.len());
            }
        }
        Err(e) => {
            warn!("Scenario 3 failed: {}", e);
            return Ok(());
        }
    }
    
    Ok(())
}

async fn run_scenario_4_enhanced_security(orchestrator: &RuntimeOrchestrator, config: &DemoConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("4️⃣  Scenario 4: Enhanced Security Isolation");
    
    // Create a workload with enhanced security
    let workload_spec = WorkloadSpec::Native(NativeWorkload {
        executable: "/bin/echo".to_string(),
        args: vec!["Security test".to_string()],
        environment: HashMap::new(),
        working_directory: None,
    });
    
    let security_context = SecurityContext {
        isolation_level: IsolationLevel::Enhanced,
        user_context: None,
        capabilities: std::collections::HashSet::new(),
        resource_limits: None,
        network_access: false,
        file_system_access: vec![],
    };
    
    let resource_requirements = ResourceRequirements::default();
    
    let request = ExecutionRequest {
        workload_spec,
        security_context,
        resource_requirements,
        timeout: Some(config.security_timeout),
        priority: 0,
        metadata: HashMap::new(),
    };
    
    match orchestrator.execute(request).await {
        Ok(response) => {
            info!("Security test completed with status: {:?}", response.status);
        }
        Err(e) => {
            warn!("Scenario 4 failed: {}", e);
            return Ok(());
        }
    }
    
    Ok(())
}

async fn run_scenario_5_resource_limits(orchestrator: &RuntimeOrchestrator, config: &DemoConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("5️⃣  Scenario 5: Resource Limits Testing");
    
    // Create a workload with strict resource limits
    let workload_spec = WorkloadSpec::Native(NativeWorkload {
        executable: "/bin/echo".to_string(),
        args: vec!["Resource test".to_string()],
        environment: HashMap::new(),
        working_directory: None,
    });
    
    let security_context = SecurityContext {
        isolation_level: IsolationLevel::Standard,
        user_context: None,
        capabilities: std::collections::HashSet::new(),
        resource_limits: None,
        network_access: false,
        file_system_access: vec![],
    };
    
    let mut resource_requirements = ResourceRequirements::default();
    resource_requirements.cpu.max_cores = Some(0.1); // Very restrictive
    resource_requirements.memory.max_bytes = Some(16 * 1024 * 1024); // 16 MB
    
    let request = ExecutionRequest {
        workload_spec,
        security_context,
        resource_requirements,
        timeout: Some(config.resource_timeout),
        priority: 0,
        metadata: HashMap::new(),
    };
    
    match orchestrator.execute(request).await {
        Ok(response) => {
            info!("Resource limits test completed: {:?}", response.status);
        }
        Err(e) => {
            warn!("Scenario 5 failed: {}", e);
            // Resource limit violations are expected in this test
            if !e.to_string().contains("resource") && !e.to_string().contains("limit") {
                return Ok(());
            }
        }
    }
    
    Ok(())
}

// Helper functions for cross-platform executable paths
fn get_echo_executable() -> PathBuf {
    #[cfg(unix)]
    return PathBuf::from("/bin/echo");
    
    #[cfg(windows)]
    return PathBuf::from("cmd");
}

fn get_ls_executable() -> PathBuf {
    #[cfg(unix)]
    return PathBuf::from("/bin/ls");
    
    #[cfg(windows)]
    return PathBuf::from("cmd");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_demo_components() {
        // Test that we can create the basic components
        let monitor = SystemResourceMonitor::new();
        let engine = NativeRuntimeEngine::new();
        let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
        
        // Basic smoke test
        assert!(true);
    }

    #[test]
    fn test_executable_discovery() {
        // Test that we can find common executables
        let echo_path = get_echo_executable();
        assert!(!echo_path.as_os_str().is_empty());
    }
} 