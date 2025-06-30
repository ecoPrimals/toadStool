//! # ToadStool Runtime Engines Integration Demo
//!
//! Comprehensive demonstration of all three runtime engines:
//! - WebAssembly Runtime with WASI support
//! - Container Runtime with Docker integration
//! - GPU Runtime foundation with device detection
//!
//! This example shows end-to-end execution workflows, runtime selection,
//! security contexts, and resource management.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

use toadstool::{
    execution::*,
    resources::*,
    security::*,
    workload::*,
    runtime::RuntimeOrchestrator,
    config::ToadStoolConfig,
    init,
};

use toadstool_runtime_wasm::WasmRuntimeEngine;
use toadstool_runtime_container::ContainerRuntimeEngine;
use toadstool_runtime_gpu::GpuRuntimeEngine;
use toadstool_runtime_native::NativeRuntimeEngine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize ToadStool with tracing
    init()?;
    
    println!("🍄 ToadStool Runtime Engines Integration Demo");
    println!("=" .repeat(60));
    
    // Load configuration
    let config = ToadStoolConfig::load_from_env()
        .unwrap_or_else(|_| ToadStoolConfig::default());
    
    // Create runtime orchestrator
    let mut orchestrator = RuntimeOrchestrator::new();
    
    // Initialize and register all runtime engines
    println!("\n📦 Initializing Runtime Engines...");
    
    // 1. Native Runtime Engine (always available)
    match NativeRuntimeEngine::new() {
        Ok(mut native_engine) => {
            native_engine.initialize(config.runtime.clone()).await?;
            println!("✅ Native Runtime Engine initialized");
            orchestrator.register_runtime(RuntimeType::Native, Box::new(native_engine));
        }
        Err(e) => println!("❌ Native Runtime Engine failed: {}", e),
    }
    
    // 2. WebAssembly Runtime Engine
    match WasmRuntimeEngine::new() {
        Ok(mut wasm_engine) => {
            wasm_engine.initialize(config.runtime.clone()).await?;
            println!("✅ WebAssembly Runtime Engine initialized");
            orchestrator.register_runtime(RuntimeType::Wasm, Box::new(wasm_engine));
        }
        Err(e) => println!("❌ WebAssembly Runtime Engine failed: {}", e),
    }
    
    // 3. Container Runtime Engine (may fail if Docker not available)
    match ContainerRuntimeEngine::new() {
        Ok(mut container_engine) => {
            match container_engine.initialize(config.runtime.clone()).await {
                Ok(()) => {
                    println!("✅ Container Runtime Engine initialized");
                    orchestrator.register_runtime(RuntimeType::Container, Box::new(container_engine));
                }
                Err(e) => println!("⚠️  Container Runtime Engine initialization failed: {}", e),
            }
        }
        Err(e) => println!("⚠️  Container Runtime Engine creation failed: {}", e),
    }
    
    // 4. GPU Runtime Foundation
    match GpuRuntimeEngine::new() {
        Ok(mut gpu_engine) => {
            gpu_engine.initialize(config.runtime.clone()).await?;
            println!("✅ GPU Runtime Foundation initialized");
            
            // Show detected GPU devices
            let devices = gpu_engine.get_available_devices();
            if devices.is_empty() {
                println!("   📊 No GPU devices detected");
            } else {
                println!("   📊 Detected {} GPU device(s):", devices.len());
                for device in devices {
                    println!("      - {} ({:?}) - {} compute units", 
                        device.name, device.framework, device.compute_units);
                }
            }
            
            orchestrator.register_runtime(RuntimeType::Gpu, Box::new(gpu_engine));
        }
        Err(e) => println!("❌ GPU Runtime Foundation failed: {}", e),
    }
    
    println!("\n🎯 Runtime Orchestrator Status:");
    println!("   Available Runtimes: {:?}", orchestrator.get_available_runtimes());
    
    // Demo 1: Native Process Execution
    println!("\n" + "=" .repeat(60));
    println!("🔧 Demo 1: Native Process Execution");
    println!("=" .repeat(60));
    
    let native_request = create_native_request()?;
    match orchestrator.execute(native_request).await {
        Ok(response) => {
            println!("✅ Native execution completed successfully");
            println!("   Runtime: {:?}", response.runtime_used);
            println!("   Duration: {:?}", response.duration);
            if let Some(stdout) = &response.output.stdout {
                println!("   Output: {}", stdout.trim());
            }
            if let Some(exit_code) = response.output.exit_code {
                println!("   Exit Code: {}", exit_code);
            }
        }
        Err(e) => println!("❌ Native execution failed: {}", e),
    }
    
    // Demo 2: WebAssembly Module Execution
    println!("\n" + "=" .repeat(60));
    println!("🕸️  Demo 2: WebAssembly Module Execution");
    println!("=" .repeat(60));
    
    let wasm_request = create_wasm_request()?;
    match orchestrator.execute(wasm_request).await {
        Ok(response) => {
            println!("✅ WASM execution completed successfully");
            println!("   Runtime: {:?}", response.runtime_used);
            println!("   Duration: {:?}", response.duration);
            println!("   Status: {:?}", response.status);
            if !response.warnings.is_empty() {
                println!("   Warnings: {:?}", response.warnings);
            }
        }
        Err(e) => println!("❌ WASM execution failed: {}", e),
    }
    
    // Demo 3: Container Execution
    println!("\n" + "=" .repeat(60));
    println!("🐳 Demo 3: Container Execution");
    println!("=" .repeat(60));
    
    let container_request = create_container_request()?;
    match orchestrator.execute(container_request).await {
        Ok(response) => {
            println!("✅ Container execution completed successfully");
            println!("   Runtime: {:?}", response.runtime_used);
            println!("   Duration: {:?}", response.duration);
            if let Some(stdout) = &response.output.stdout {
                println!("   Output: {}", stdout.trim());
            }
            if let Some(exit_code) = response.output.exit_code {
                println!("   Exit Code: {}", exit_code);
            }
        }
        Err(e) => println!("⚠️  Container execution failed (expected if Docker unavailable): {}", e),
    }
    
    // Demo 4: GPU Foundation Check
    println!("\n" + "=" .repeat(60));
    println!("⚡ Demo 4: GPU Foundation Capabilities");
    println!("=" .repeat(60));
    
    let gpu_request = create_gpu_request()?;
    match orchestrator.execute(gpu_request).await {
        Ok(response) => {
            println!("✅ GPU foundation check completed");
            println!("   Runtime: {:?}", response.runtime_used);
            println!("   Duration: {:?}", response.duration);
            if let Some(stdout) = &response.output.stdout {
                println!("   Message: {}", stdout);
            }
            if !response.warnings.is_empty() {
                println!("   Note: {}", response.warnings[0]);
            }
            
            // Display GPU metrics
            if let Some(devices) = response.output.result.get("available_devices") {
                println!("   Available Devices: {}", devices);
            }
        }
        Err(e) => println!("❌ GPU foundation check failed: {}", e),
    }
    
    // Demo 5: Runtime Selection and Capabilities
    println!("\n" + "=" .repeat(60));
    println!("🎛️  Demo 5: Runtime Capabilities Analysis");
    println!("=" .repeat(60));
    
    for runtime_type in orchestrator.get_available_runtimes() {
        if let Some(engine) = orchestrator.get_runtime(&runtime_type) {
            let capabilities = engine.get_capabilities();
            println!("\n📋 {} Runtime Capabilities:", format!("{:?}", runtime_type));
            println!("   Supported Workloads: {:?}", capabilities.supported_workloads);
            println!("   Max Concurrent: {:?}", capabilities.max_concurrent_executions);
            println!("   Architectures: {:?}", capabilities.supported_architectures);
            println!("   Features:");
            for (feature, enabled) in &capabilities.platform_features {
                println!("     - {}: {}", feature, if *enabled { "✅" } else { "❌" });
            }
            
            // Get runtime metrics
            match engine.get_metrics().await {
                Ok(metrics) => {
                    println!("   Metrics:");
                    if !metrics.custom_metrics.is_empty() {
                        for (key, value) in &metrics.custom_metrics {
                            println!("     - {}: {}", key, value);
                        }
                    }
                }
                Err(e) => println!("   Metrics unavailable: {}", e),
            }
        }
    }
    
    // Demo 6: Security Context Testing
    println!("\n" + "=" .repeat(60));
    println!("🔒 Demo 6: Security Context Validation");
    println!("=" .repeat(60));
    
    // Test different isolation levels
    let isolation_levels = [
        IsolationLevel::None,
        IsolationLevel::Basic,
        IsolationLevel::Standard,
        IsolationLevel::Enhanced,
    ];
    
    for isolation_level in &isolation_levels {
        println!("\n🔐 Testing {:?} Isolation Level:", isolation_level);
        
        let security_request = create_security_test_request(isolation_level.clone())?;
        match orchestrator.execute(security_request).await {
            Ok(response) => {
                println!("   ✅ Security validation passed");
                println!("   Runtime: {:?}", response.runtime_used);
                println!("   Duration: {:?}", response.duration);
            }
            Err(e) => {
                println!("   ⚠️  Security validation: {}", e);
            }
        }
    }
    
    // Demo 7: Resource Limit Testing
    println!("\n" + "=" .repeat(60));
    println!("📊 Demo 7: Resource Limit Validation");
    println!("=" .repeat(60));
    
    // Test resource limits
    let resource_tests = [
        ("Normal Memory", 64), // 64 MB
        ("High Memory", 512),  // 512 MB 
        ("Excessive Memory", 8192), // 8 GB (likely to fail)
    ];
    
    for (test_name, memory_mb) in &resource_tests {
        println!("\n💾 Testing {}: {} MB", test_name, memory_mb);
        
        let resource_request = create_resource_test_request(*memory_mb)?;
        match orchestrator.execute(resource_request).await {
            Ok(response) => {
                println!("   ✅ Resource validation passed");
                println!("   Runtime: {:?}", response.runtime_used);
                println!("   Memory Usage: {} bytes", response.metrics.memory_usage_bytes);
            }
            Err(e) => {
                println!("   ⚠️  Resource validation failed: {}", e);
            }
        }
    }
    
    println!("\n" + "=" .repeat(60));
    println!("🎉 ToadStool Runtime Engines Demo Completed!");
    println!("   Successfully demonstrated all three runtime engines");
    println!("   with comprehensive integration testing.");
    println!("=" .repeat(60));
    
    Ok(())
}

/// Create a native process execution request
fn create_native_request() -> anyhow::Result<ExecutionRequest> {
    Ok(ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from("/bin/echo"),
            },
            args: Some(vec!["Hello from Native Runtime!".to_string()]),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        runtime_hint: Some(RuntimeType::Native),
        resources: ResourceRequirements {
            memory_mb: Some(32),
            cpu_cores: Some(0.1),
            storage_mb: None,
            network_mbps: None,
            gpu_memory_mb: None,
        },
        security_context: SecurityContext::new(IsolationLevel::Basic),
        timeout: Some(Duration::from_secs(10)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
    })
}

/// Create a WebAssembly module execution request
fn create_wasm_request() -> anyhow::Result<ExecutionRequest> {
    // Create a minimal WASM module for testing
    let minimal_wasm = vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version
    ];
    
    Ok(ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Wasm {
            module_source: WasmModuleSource::Bytes {
                data: minimal_wasm,
            },
            wasi_config: Some(WasiConfig {
                env_vars: HashMap::new(),
                args: vec!["wasm_module".to_string()],
                stdin: None,
                dir_mappings: Vec::new(),
            }),
            host_functions: Vec::new(),
            memory_limit: Some(64 * 1024 * 1024), // 64 MB
        },
        runtime_hint: Some(RuntimeType::Wasm),
        resources: ResourceRequirements {
            memory_mb: Some(64),
            cpu_cores: Some(0.2),
            storage_mb: None,
            network_mbps: None,
            gpu_memory_mb: None,
        },
        security_context: SecurityContext::new(IsolationLevel::Standard),
        timeout: Some(Duration::from_secs(30)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
    })
}

/// Create a container execution request
fn create_container_request() -> anyhow::Result<ExecutionRequest> {
    Ok(ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Container {
            image: "hello-world".to_string(),
            command: None,
            args: None,
            working_dir: None,
            user: None,
            volumes: Vec::new(),
            ports: Vec::new(),
            registry_auth: None,
        },
        runtime_hint: Some(RuntimeType::Container),
        resources: ResourceRequirements {
            memory_mb: Some(128),
            cpu_cores: Some(0.1),
            storage_mb: None,
            network_mbps: None,
            gpu_memory_mb: None,
        },
        security_context: SecurityContext::new(IsolationLevel::Enhanced),
        timeout: Some(Duration::from_secs(60)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
    })
}

/// Create a GPU foundation test request
fn create_gpu_request() -> anyhow::Result<ExecutionRequest> {
    Ok(ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Gpu {
            kernel_source: GpuKernelSource::Source {
                code: "// GPU kernel placeholder".to_string(),
            },
            framework: GpuFramework::OpenCl,
            device_requirements: GpuDeviceRequirements {
                min_compute_capability: None,
                min_memory_mb: Some(256),
                device_count: None,
                device_ids: None,
            },
            compute_params: HashMap::new(),
        },
        runtime_hint: Some(RuntimeType::Gpu),
        resources: ResourceRequirements {
            memory_mb: Some(256),
            cpu_cores: Some(0.1),
            storage_mb: None,
            network_mbps: None,
            gpu_memory_mb: Some(256),
        },
        security_context: SecurityContext::new(IsolationLevel::Basic),
        timeout: Some(Duration::from_secs(10)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
    })
}

/// Create a security context test request
fn create_security_test_request(isolation_level: IsolationLevel) -> anyhow::Result<ExecutionRequest> {
    let mut security_context = SecurityContext::new(isolation_level);
    
    // Add some capabilities based on isolation level
    match isolation_level {
        IsolationLevel::None => {
            security_context.add_capability(Capability::Read);
            security_context.add_capability(Capability::Write);
            security_context.add_capability(Capability::Execute);
            security_context.add_capability(Capability::NetworkClient);
        }
        IsolationLevel::Basic => {
            security_context.add_capability(Capability::Read);
            security_context.add_capability(Capability::Execute);
        }
        IsolationLevel::Standard => {
            security_context.add_capability(Capability::Read);
        }
        _ => {
            // Enhanced and Maximum have minimal capabilities
        }
    }
    
    Ok(ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from("/bin/echo"),
            },
            args: Some(vec!["Security test".to_string()]),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        runtime_hint: Some(RuntimeType::Native),
        resources: ResourceRequirements {
            memory_mb: Some(32),
            cpu_cores: Some(0.1),
            storage_mb: None,
            network_mbps: None,
            gpu_memory_mb: None,
        },
        security_context,
        timeout: Some(Duration::from_secs(5)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
    })
}

/// Create a resource limit test request
fn create_resource_test_request(memory_mb: u64) -> anyhow::Result<ExecutionRequest> {
    Ok(ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from("/bin/echo"),
            },
            args: Some(vec!["Resource test".to_string()]),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        runtime_hint: Some(RuntimeType::Native),
        resources: ResourceRequirements {
            memory_mb: Some(memory_mb),
            cpu_cores: Some(0.1),
            storage_mb: None,
            network_mbps: None,
            gpu_memory_mb: None,
        },
        security_context: SecurityContext::new(IsolationLevel::Basic),
        timeout: Some(Duration::from_secs(5)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
    })
} 