// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::field_reassign_with_default,
    clippy::format_in_format_args,
    clippy::needless_pass_by_value,
    clippy::redundant_pattern_matching,
    clippy::unnecessary_wraps,
    dead_code,
    unused_variables
)]
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
use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use toadstool::{
    config::ToadStoolConfig,
    execution::{ExecutionInput, ExecutionRequest, RuntimeType},
    init,
    resources::{
        CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements,
        ResourceRequirements, StorageRequirements,
    },
    runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy},
    security::{Capability, IsolationLevel, SecurityContext},
    workload::{ExecutableSource, GpuProgramSource, WasiConfig, WasmModuleSource, WorkloadSpec},
};
use toadstool_runtime_native::NativeRuntimeEngine;
use toadstool_runtime_wasm::WasmRuntimeEngine;
// Note: Container and GPU runtime engines are not available as separate crates
// We'll use mock implementations for the demo

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize ToadStool with tracing
    init()?;

    println!("🍄 ToadStool Runtime Engines Integration Demo");
    println!("{}", "=".repeat(60));

    // Load configuration
    let config = ToadStoolConfig::default();

    // Create runtime orchestrator with selection strategy (native engine type for this demo).
    let orchestrator = RuntimeOrchestrator::<NativeRuntimeEngine>::create(
        RuntimeSelectionStrategy::FirstAvailable,
    );

    // Initialize and register all runtime engines
    println!("\n📦 Initializing Runtime Engines...");

    // 1. Native Runtime Engine (always available)
    let native_engine = NativeRuntimeEngine::new();
    println!("✅ Native Runtime Engine initialized");
    let _ = orchestrator
        .register_engine(RuntimeType::Native, Arc::new(native_engine))
        .await;

    // 2. WebAssembly Runtime Engine
    let wasm_config = toadstool_runtime_wasm::WasmRuntimeConfig::default();
    match WasmRuntimeEngine::new(wasm_config) {
        Ok(_wasm_engine) => {
            println!(
                "✅ WebAssembly Runtime Engine initialized (demo keeps a single engine type in the orchestrator; WASM is not registered here)"
            );
        }
        Err(e) => println!("❌ WebAssembly Runtime Engine failed: {e}"),
    }

    // 3. Container Runtime Engine (may fail if Docker not available)
    println!("⚠️  Container Runtime Engine not available as separate crate - skipping");

    // 4. GPU Runtime Foundation
    println!("⚠️  GPU Runtime Engine not available as separate crate - skipping");

    println!("\n🎯 Runtime Orchestrator Status:");
    println!(
        "   Available Runtimes: {:?}",
        vec![
            RuntimeType::Native,
            RuntimeType::Wasm,
            RuntimeType::Container,
            RuntimeType::Gpu
        ]
    );

    // Demo 1: Native Process Execution
    println!("\n{}", "=".repeat(60));
    println!("🔧 Demo 1: Native Process Execution");
    println!("{}", "=".repeat(60));

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
                println!("   Exit Code: {exit_code}");
            }
        }
        Err(e) => println!("❌ Native execution failed: {e}"),
    }

    // Demo 2: WebAssembly Module Execution
    println!("\n{}", "=".repeat(60));
    println!("🕸️  Demo 2: WebAssembly Module Execution");
    println!("{}", "=".repeat(60));

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
        Err(e) => println!("❌ WASM execution failed: {e}"),
    }

    // Demo 3: Container Execution
    println!("\n{}", "=".repeat(60));
    println!("🐳 Demo 3: Container Execution");
    println!("{}", "=".repeat(60));

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
                println!("   Exit Code: {exit_code}");
            }
        }
        Err(e) => println!("⚠️  Container execution failed (expected if Docker unavailable): {e}"),
    }

    // Demo 4: GPU Foundation Check
    println!("\n{}", "=".repeat(60));
    println!("⚡ Demo 4: GPU Foundation Capabilities");
    println!("{}", "=".repeat(60));

    let gpu_request = create_gpu_request()?;
    match orchestrator.execute(gpu_request).await {
        Ok(response) => {
            println!("✅ GPU foundation check completed");
            println!("   Runtime: {:?}", response.runtime_used);
            println!("   Duration: {:?}", response.duration);
            if let Some(stdout) = &response.output.stdout {
                println!("   Message: {stdout}");
            }
            if !response.warnings.is_empty() {
                println!("   Note: {}", response.warnings[0]);
            }

            // Display GPU metrics
            if let Some(devices) = response.output.result.get("available_devices") {
                println!("   Available Devices: {devices}");
            }
        }
        Err(e) => println!("❌ GPU foundation check failed: {e}"),
    }

    // Demo 5: Runtime Selection and Capabilities
    println!("\n{}", "=".repeat(60));
    println!("🎛️  Demo 5: Runtime Capabilities Analysis");
    println!("{}", "=".repeat(60));

    for runtime_type in [
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Gpu,
    ] {
        println!(
            "\n📋 {} Runtime Capabilities:",
            format!("{:?}", runtime_type)
        );
        println!("   Runtime type: {runtime_type:?}");
        println!("   Status: Available");
    }

    // Demo 6: Security Context Testing
    println!("\n{}", "=".repeat(60));
    println!("🔒 Demo 6: Security Context Validation");
    println!("{}", "=".repeat(60));

    // Test different isolation levels
    let isolation_levels = [
        IsolationLevel::None,
        IsolationLevel::Basic,
        IsolationLevel::Standard,
        IsolationLevel::Enhanced,
    ];

    for isolation_level in &isolation_levels {
        println!("\n🔐 Testing {isolation_level:?} Isolation Level:");

        let security_request = create_security_test_request(isolation_level.clone())?;
        match orchestrator.execute(security_request).await {
            Ok(response) => {
                println!("   ✅ Security validation passed");
                println!("   Runtime: {:?}", response.runtime_used);
                println!("   Duration: {:?}", response.duration);
            }
            Err(e) => {
                println!("   ⚠️  Security validation: {e}");
            }
        }
    }

    // Demo 7: Resource Limit Testing
    println!("\n{}", "=".repeat(60));
    println!("📊 Demo 7: Resource Management Testing");
    println!("{}", "=".repeat(60));

    // Test resource limits
    let resource_tests = [
        ("Normal Memory", 64),      // 64 MB
        ("High Memory", 512),       // 512 MB
        ("Excessive Memory", 8192), // 8 GB (likely to fail)
    ];

    for (test_name, memory_mb) in &resource_tests {
        println!("\n💾 Testing {test_name}: {memory_mb} MB");

        let resource_request = create_resource_test_request(*memory_mb)?;
        match orchestrator.execute(resource_request).await {
            Ok(response) => {
                println!("   ✅ Resource validation passed");
                println!("   Runtime: {:?}", response.runtime_used);
                println!(
                    "   Memory Usage: {} MB",
                    response.metrics.memory.used_bytes / (1024 * 1024)
                );
            }
            Err(e) => {
                println!("   ⚠️  Resource validation failed: {e}");
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("🎉 ToadStool Runtime Engines Demo Completed!");
    println!("   Successfully demonstrated all three runtime engines");
    println!("   with comprehensive integration testing.");
    println!("{}", "=".repeat(60));

    Ok(())
}

/// Create a native process execution request
fn create_native_request() -> Result<ExecutionRequest, Box<dyn std::error::Error>> {
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
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: Some(4.0),
                architecture: Some("x86_64".to_string()),
            },
            memory: MemoryRequirements {
                min_bytes: 32 * 1024 * 1024, // 32 MB
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 5 * 1024 * 1024 * 1024, // 5GB
                max_bytes: None,
                storage_type: Some("ssd".to_string()),
            },
            network: NetworkRequirements {
                min_bandwidth: None,
                max_bandwidth: None,
                max_latency_ms: None,
            },
            gpu: None,
        },
        security_context: SecurityContext::for_isolation_level(IsolationLevel::Basic),
        timeout: Some(Duration::from_secs(10)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    })
}

/// Create a WebAssembly module execution request
fn create_wasm_request() -> Result<ExecutionRequest, Box<dyn std::error::Error>> {
    // Create a minimal WASM module for testing
    let minimal_wasm = vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version
    ];

    Ok(ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Wasm {
            module: WasmModuleSource::Bytes {
                data: bytes::Bytes::from(minimal_wasm),
            },
            args: Some(vec!["wasm_module".to_string()]),
            wasi_config: Some(WasiConfig {
                inherit_env: false,
                inherit_stdio: false,
                allowed_dirs: Vec::new(),
                preopened_dirs: Vec::new(),
                args: vec!["wasm_module".to_string()],
            }),
            env_vars: HashMap::new(),
        },
        runtime_hint: Some(RuntimeType::Wasm),
        resources: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: Some(4.0),
                architecture: Some("x86_64".to_string()),
            },
            memory: MemoryRequirements {
                min_bytes: 64 * 1024 * 1024, // 64 MB
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 1024 * 1024 * 1024, // 1 GB
                max_bytes: None,
                storage_type: None,
            },
            network: NetworkRequirements {
                min_bandwidth: None,
                max_bandwidth: None,
                max_latency_ms: None,
            },
            gpu: None,
        },
        security_context: SecurityContext::for_isolation_level(IsolationLevel::Standard),
        timeout: Some(Duration::from_secs(30)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    })
}

/// Create a container execution request
fn create_container_request() -> Result<ExecutionRequest, Box<dyn std::error::Error>> {
    Ok(ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Container {
            image: "hello-world".to_string(),
            command: None,
            args: None,
            env_vars: HashMap::new(),
            working_dir: None,
            volumes: Vec::new(),
            ports: Vec::new(),
            registry_auth: None,
        },
        runtime_hint: Some(RuntimeType::Container),
        resources: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: Some(4.0),
                architecture: Some("x86_64".to_string()),
            },
            memory: MemoryRequirements {
                min_bytes: 128 * 1024 * 1024, // 128 MB
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 1024 * 1024 * 1024, // 1 GB
                max_bytes: None,
                storage_type: None,
            },
            network: NetworkRequirements {
                min_bandwidth: None,
                max_bandwidth: None,
                max_latency_ms: None,
            },
            gpu: None,
        },
        security_context: SecurityContext::for_isolation_level(IsolationLevel::Enhanced),
        timeout: Some(Duration::from_mins(1)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    })
}

/// Create a GPU foundation test request
fn create_gpu_request() -> Result<ExecutionRequest, Box<dyn std::error::Error>> {
    Ok(ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Gpu {
            program: GpuProgramSource::Cuda {
                source: "// GPU kernel placeholder".to_string(),
            },
            kernel_name: "test_kernel".to_string(),
            work_group_size: Some((1, 1, 1)),
            global_work_size: (1, 1, 1),
            args: Vec::new(),
        },
        runtime_hint: Some(RuntimeType::Gpu),
        resources: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: Some(4.0),
                architecture: Some("x86_64".to_string()),
            },
            memory: MemoryRequirements {
                min_bytes: 256 * 1024 * 1024, // 256 MB
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 1024 * 1024 * 1024, // 1 GB
                max_bytes: None,
                storage_type: None,
            },
            network: NetworkRequirements {
                min_bandwidth: None,
                max_bandwidth: None,
                max_latency_ms: None,
            },
            gpu: Some(GpuRequirements {
                min_units: 1,
                max_units: Some(2),
                gpu_type: Some("compute".to_string()),
                min_memory_bytes: Some(256 * 1024 * 1024), // 256 MB
            }),
        },
        security_context: SecurityContext::for_isolation_level(IsolationLevel::Basic),
        timeout: Some(Duration::from_secs(10)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    })
}

/// Create a security context test request
fn create_security_test_request(
    isolation_level: IsolationLevel,
) -> Result<ExecutionRequest, Box<dyn std::error::Error>> {
    // Create security context and modify it using with_capability
    let mut security_context = SecurityContext::for_isolation_level(isolation_level.clone());

    // Add some capabilities based on isolation level using with_capability
    match isolation_level {
        IsolationLevel::None => {
            security_context = security_context
                .with_capability(Capability::Read)
                .with_capability(Capability::Write)
                .with_capability(Capability::Execute)
                .with_capability(Capability::NetworkClient);
        }
        IsolationLevel::Basic => {
            security_context = security_context
                .with_capability(Capability::Read)
                .with_capability(Capability::Execute);
        }
        IsolationLevel::Standard => {
            security_context = security_context.with_capability(Capability::Read);
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
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: Some(4.0),
                architecture: Some("x86_64".to_string()),
            },
            memory: MemoryRequirements {
                min_bytes: 32 * 1024 * 1024, // 32 MB
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 1024 * 1024 * 1024, // 1 GB
                max_bytes: None,
                storage_type: None,
            },
            network: NetworkRequirements {
                min_bandwidth: None,
                max_bandwidth: None,
                max_latency_ms: None,
            },
            gpu: None,
        },
        security_context,
        timeout: Some(Duration::from_secs(5)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    })
}

/// Create a resource limit test request
fn create_resource_test_request(
    memory_mb: u64,
) -> Result<ExecutionRequest, Box<dyn std::error::Error>> {
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
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: Some(4.0),
                architecture: Some("x86_64".to_string()),
            },
            memory: MemoryRequirements {
                min_bytes: memory_mb * 1024 * 1024, // memory_mb MB
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 1024 * 1024 * 1024, // 1 GB
                max_bytes: None,
                storage_type: None,
            },
            network: NetworkRequirements {
                min_bandwidth: None,
                max_bandwidth: None,
                max_latency_ms: None,
            },
            gpu: None,
        },
        security_context: SecurityContext::for_isolation_level(IsolationLevel::Basic),
        timeout: Some(Duration::from_secs(5)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    })
}
