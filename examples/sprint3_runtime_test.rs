//! Sprint 3 Runtime Engines Test
//! 
//! This example demonstrates the three runtime engines implemented in Sprint 3:
//! - WebAssembly Runtime Engine
//! - Container Runtime Engine  
//! - GPU Runtime Foundation
//!
//! Tests basic functionality, capabilities, and integration with the RuntimeOrchestrator.

use std::collections::HashMap;
use std::time::Duration;

use tracing::info;
use uuid::Uuid;

use toadstool::{
    execution::{ExecutionRequest, RuntimeType, RuntimeConfig, ExecutionInput, RuntimeEngine},
    resources::ResourceRequirements,
    security::SecurityContext,
    workload::{WorkloadSpec, WasmModuleSource},
    runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy},
    error::ToadStoolResult,
};

// Import our runtime engines
use toadstool_runtime_wasm::{WasmRuntimeEngine, WasmRuntimeConfig};
use toadstool_runtime_container::ContainerRuntimeEngine;
use toadstool_runtime_gpu::GpuRuntimeEngine;

#[tokio::main]
async fn main() -> ToadStoolResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🍄 ToadStool Sprint 3 Runtime Engines Test");
    info!("Testing WASM, Container, and GPU runtime engines");
    
    // Test 1: WebAssembly Runtime Engine
    info!("\n=== Testing WebAssembly Runtime Engine ===");
    test_wasm_runtime().await?;
    
    // Test 2: Container Runtime Engine  
    info!("\n=== Testing Container Runtime Engine ===");
    test_container_runtime().await?;
    
    // Test 3: GPU Runtime Foundation
    info!("\n=== Testing GPU Runtime Foundation ===");
    test_gpu_runtime().await?;
    
    // Test 4: Runtime Orchestrator Integration
    info!("\n=== Testing Runtime Orchestrator Integration ===");
    test_runtime_orchestrator().await?;
    
    info!("\n✅ All Sprint 3 runtime engines tested successfully!");
    Ok(())
}

async fn test_wasm_runtime() -> ToadStoolResult<()> {
    info!("🔧 Creating WASM Runtime Engine...");
    
    // Create WASM runtime config
    let wasm_config = WasmRuntimeConfig::default();
    let mut wasm_engine = WasmRuntimeEngine::new(wasm_config)?;
    
    // Initialize with configuration
    let config = RuntimeConfig::default();
    wasm_engine.initialize(config).await?;
    
    // Check capabilities
    let capabilities = wasm_engine.get_capabilities();
    info!("✓ WASM Engine Capabilities: {} workload types supported", 
          capabilities.supported_workloads.len());
    
    // Test workload support
    let workload_type = toadstool::execution::WorkloadType::Wasm;
    assert!(wasm_engine.supports_workload(&workload_type));
    info!("✓ WASM engine supports WASM workloads");
    
    // Get metrics
    let metrics = wasm_engine.get_metrics().await?;
    info!("✓ WASM engine metrics retrieved: {} MB memory usage", 
          metrics.memory.usage_bytes / (1024 * 1024));
    
    info!("✓ WASM Runtime Engine test completed");
    Ok(())
}

async fn test_container_runtime() -> ToadStoolResult<()> {
    info!("🐳 Creating Container Runtime Engine...");
    
    let mut container_engine = ContainerRuntimeEngine::new()?;
    
    // Initialize with configuration
    let config = RuntimeConfig::default();
    container_engine.initialize(config).await?;
    
    // Check capabilities
    let capabilities = container_engine.get_capabilities();
    info!("✓ Container Engine Capabilities: {} workload types supported", 
          capabilities.supported_workloads.len());
    
    // Test workload support
    let workload_type = toadstool::execution::WorkloadType::Container;
    assert!(container_engine.supports_workload(&workload_type));
    info!("✓ Container engine supports container workloads");
    
    // Get metrics
    let metrics = container_engine.get_metrics().await?;
    info!("✓ Container engine metrics retrieved: {} MB memory usage", 
          metrics.memory.usage_bytes / (1024 * 1024));
    
    info!("✓ Container Runtime Engine test completed");
    Ok(())
}

async fn test_gpu_runtime() -> ToadStoolResult<()> {
    info!("🎮 Creating GPU Runtime Engine...");
    
    let mut gpu_engine = GpuRuntimeEngine::new()?;
    
    // Initialize with configuration
    let config = RuntimeConfig::default();
    gpu_engine.initialize(config).await?;
    
    // Check capabilities
    let capabilities = gpu_engine.get_capabilities();
    info!("✓ GPU Engine Capabilities: {} workload types supported", 
          capabilities.supported_workloads.len());
    
    // Test workload support
    let workload_type = toadstool::execution::WorkloadType::Gpu;
    assert!(gpu_engine.supports_workload(&workload_type));
    info!("✓ GPU engine supports GPU workloads");
    
    // Get metrics
    let metrics = gpu_engine.get_metrics().await?;
    info!("✓ GPU engine metrics retrieved: {} MB memory usage", 
          metrics.memory.usage_bytes / (1024 * 1024));
    
    // Test GPU device detection
    info!("🔍 Testing GPU device detection...");
    let devices = gpu_engine.get_available_devices();
    info!("✅ Detected {} GPU devices", devices.len());
    for device in devices {
        info!("  - Device: {} (Memory: {} MB)", device.name, device.total_memory_bytes / (1024 * 1024));
    }
    
    info!("✓ GPU Runtime Foundation test completed");
    Ok(())
}

async fn test_runtime_orchestrator() -> ToadStoolResult<()> {
    info!("🎭 Testing Runtime Orchestrator Integration...");
    
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    
    // Register all three runtime engines
    let wasm_config = WasmRuntimeConfig::default();
    let wasm_engine = WasmRuntimeEngine::new(wasm_config)?;
    let container_engine = ContainerRuntimeEngine::new()?;
    let gpu_engine = GpuRuntimeEngine::new()?;
    
    orchestrator.register_engine(RuntimeType::Wasm, Box::new(wasm_engine)).await?;
    orchestrator.register_engine(RuntimeType::Container, Box::new(container_engine)).await?;
    orchestrator.register_engine(RuntimeType::Gpu, Box::new(gpu_engine)).await?;
    
    info!("✓ All three runtime engines registered with orchestrator");
    
    // Test execution request creation (basic validation)
    let wasm_workload = WorkloadSpec::Wasm {
        module_source: WasmModuleSource::Bytes { data: vec![] },
        wasi_config: None,
        host_functions: vec![],
        memory_limit: None,
    };
    
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: wasm_workload,
        runtime_hint: Some(RuntimeType::Wasm),
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: Some(Duration::from_secs(10)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
    };
    
    info!("✓ Successfully created execution request for WASM workload");
    
    // Test container workload request
    let container_workload = WorkloadSpec::Container {
        image: "test:latest".to_string(),
        command: None,
        args: None,
        working_dir: None,
        user: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    };
    
    let container_request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: container_workload,
        runtime_hint: Some(RuntimeType::Container),
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: Some(Duration::from_secs(30)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
    };
    
    info!("✓ Successfully created execution request for Container workload");
    
    info!("✓ Runtime Orchestrator integration test completed");
    Ok(())
} 