// SPDX-License-Identifier: AGPL-3.0-or-later
//! Simple wasmi execution test
//!
//! Tests basic WASM module loading and execution with wasmi runtime.

use std::time::Duration;
use toadstool::error::ToadStoolResult;
use toadstool::execution::RuntimeEngine;
use toadstool::execution::{ExecutionRequest, RuntimeType};
use toadstool::resources::ResourceRequirements;
use toadstool::workload::{WasmModuleSource, WorkloadSpec};
use toadstool::SecurityContext;
use toadstool_runtime_wasm::{WasmRuntimeConfig, WasmRuntimeEngine};
use uuid::Uuid;

#[tokio::main]
async fn main() -> ToadStoolResult<()> {
    println!("🦀 Testing Pure Rust WASM Runtime (wasmi)!\n");

    // Create a simple WASM module (add function)
    let wasm_bytes = wat::parse_str(
        r#"
        (module
            (func (export "add") (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.add
            )
            (func (export "_start")
                ;; Simple start function that does nothing
                nop
            )
        )
    "#,
    )
    .map_err(|e| toadstool::error::ToadStoolError::validation(format!("WAT parse error: {e}")))?;

    println!("✅ Compiled WAT to WASM ({} bytes)", wasm_bytes.len());

    // Create wasmi runtime engine
    let config = WasmRuntimeConfig::default();
    let engine = WasmRuntimeEngine::new(config)?;

    println!("✅ Created wasmi runtime engine");

    // Initialize engine
    let mut engine = engine;
    engine
        .initialize(toadstool::execution::RuntimeConfig {
            settings: std::collections::HashMap::new(),
            resource_limits: None,
            security_settings: None,
            logging: None,
        })
        .await?;

    println!("✅ Initialized runtime engine");

    // Get runtime capabilities (synchronous)
    let capabilities = engine.get_capabilities();
    println!("\n📊 Runtime Capabilities:");
    println!("  • Version: {}", capabilities.version);
    println!(
        "  • Max Concurrent: {:?}",
        capabilities.max_concurrent_executions
    );
    println!(
        "  • Supported Workloads: {:?}",
        capabilities.supported_workloads
    );
    println!(
        "  • Architectures: {:?}",
        capabilities.supported_architectures
    );

    // Create execution request
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Wasm {
            module: WasmModuleSource::Bytes {
                data: wasm_bytes.into(),
            },
            args: Some(vec![]),
            wasi_config: None,
            env_vars: std::collections::HashMap::new(),
        },
        runtime_hint: Some(RuntimeType::Wasm),
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: Some(Duration::from_secs(5)),
        environment: std::collections::HashMap::new(),
        input_data: toadstool::execution::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    };

    println!("\n🚀 Executing WASM module...");

    // Execute the module
    let response = engine.execute(request).await?;

    println!("\n✅ Execution completed!");
    println!("  • Status: {:?}", response.status);
    println!("  • Duration: {:?}", response.duration);
    println!("  • Runtime Used: {:?}", response.runtime_used);

    if let Some(output) = response.output.stdout {
        println!("  • Stdout: {output}");
    }

    // Get runtime metrics
    let metrics = engine.get_metrics().await?;
    println!("\n📈 Runtime Metrics:");
    println!("  • Memory Used: {} bytes", metrics.memory.used_bytes);
    println!("  • Memory Peak: {} bytes", metrics.memory.peak_bytes);

    println!("\n🎉 Pure Rust WASM Runtime Test Complete!");
    println!("   100% Pure Rust - No C dependencies!");
    println!("   Cross-compiles to ARM/RISC-V/etc trivially!");

    Ok(())
}
