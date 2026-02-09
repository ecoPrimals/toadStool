//! Level 0 Demo: WASM Execution
//! Demonstrates WebAssembly execution using ToadStool's UniversalComputePlatform

use std::time::Duration;
use uuid::Uuid;

use toadstool::error::ToadStoolResult;
use toadstool::resources::ResourceRequirements;
use toadstool::universal::{
    JobPriority, NetworkLocation, PrimalContext, SecurityLevel, UniversalComputePlatform,
    UniversalJob, UniversalJobType,
};

#[tokio::main]
async fn main() -> ToadStoolResult<()> {
    println!("════════════════════════════════════════════════════════");
    println!("🍄 ToadStool Level 0: WASM Runtime Execution");
    println!("════════════════════════════════════════════════════════\n");

    println!("📌 DEMO OBJECTIVE:");
    println!("   Demonstrate ToadStool's ability to execute WebAssembly modules");
    println!("   with security sandboxing and portability.\n");

    // Step 1: Initialize Platform
    println!("━━━ Step 1: Initialize ToadStool Platform ━━━");
    let platform = UniversalComputePlatform::new().await?;
    println!("✅ Platform initialized successfully\n");

    // Step 2: Create Execution Context
    println!("━━━ Step 2: Create Execution Context ━━━");
    let context = PrimalContext {
        user_id: "local_user".to_string(),
        device_id: "local_device".to_string(),
        session_id: Uuid::new_v4().to_string(),
        network_location: NetworkLocation {
            ip_address: "127.0.0.1".to_string(),
            subnet: Some("127.0.0.0/24".to_string()),
            network_id: Some("local".to_string()),
            geo_location: None,
        },
        security_level: SecurityLevel::Standard,
        metadata: std::collections::HashMap::new(),
    };
    println!("✅ Context created for sandboxed execution\n");

    // Step 3: Create a simple WASM module
    println!("━━━ Step 3: Create WASM Module ━━━");
    println!("   Creating a simple WASM module (WAT format)");
    println!("   Module: Adds two numbers");

    // Simple WAT (WebAssembly Text) module that exports an add function
    let wat = r#"
        (module
            (func (export "add") (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.add
            )
        )
    "#;

    let wasm_bytes = wat::parse_str(wat).map_err(|e| {
        toadstool::error::ToadStoolError::runtime(format!("Failed to parse WAT: {}", e))
    })?;

    println!("✅ WASM module compiled ({} bytes)\n", wasm_bytes.len());

    // Step 4: Define WASM Workload
    println!("━━━ Step 4: Define WASM Workload ━━━");
    println!("   Runtime: WASM (sandboxed execution)");
    println!("   Module Size: {} bytes", wasm_bytes.len());
    println!("   Resources: 0.1 CPU, 64MB RAM");
    println!("   Timeout: 10 seconds");
    println!("   Security: Sandboxed (no system access)\n");

    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Wasm {
            module: wasm_bytes.clone(),
            args: vec![],
            env: std::collections::HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements {
            cpu: toadstool::resources::CpuRequirements {
                min_cores: 0.1,
                max_cores: None,
                architecture: None,
            },
            memory: toadstool::resources::MemoryRequirements {
                min_bytes: 64 * 1024 * 1024, // 64 MB
                max_bytes: None,
            },
            storage: toadstool::resources::StorageRequirements {
                min_bytes: 0,
                max_bytes: None,
                storage_type: None,
            },
            gpu: None,
            network: toadstool::resources::NetworkRequirements {
                min_bandwidth: None,
                max_bandwidth: None,
                max_latency_ms: None,
            },
        },
        timeout: Some(Duration::from_secs(10)),
        created_at: chrono::Utc::now(),
        context,
    };

    // Step 5: Execute Job
    println!("━━━ Step 5: Execute WASM Job ━━━");
    println!("🚀 Submitting job {} to ToadStool...", job.id);

    let start = std::time::Instant::now();
    let response = platform.execute_universal_job(job.clone()).await?;
    let duration = start.elapsed();

    println!("✅ Job completed in {:.2}s\n", duration.as_secs_f64());

    // Step 6: Display Results
    println!("━━━ Step 6: Execution Results ━━━");
    println!("Job ID: {}", job.id);
    println!("Status: {:?}", response.status);
    println!("Duration: {:.3}s", duration.as_secs_f64());
    println!("WASM Module Size: {} bytes", wasm_bytes.len());

    if let Some(stdout) = response.output.stdout {
        println!("\n📤 Standard Output:");
        println!("┌────────────────────────────────────┐");
        for line in stdout.lines() {
            println!("│ {:<34} │", line);
        }
        println!("└────────────────────────────────────┘");
    }

    println!("\n━━━ Architecture Visualization ━━━");
    println!(
        "
   ┌───────────────────────────┐
   │       User / Demo         │
   │  (UniversalComputePlatform)│
   └───────────┬───────────────┘
               │ Submit WASM Job
               ↓
   ┌───────────────────────────┐
   │     🍄 ToadStool Core     │
   │  (Job Scheduler)          │
   └───────────┬───────────────┘
               │ Route to WASM Runtime
               ↓
   ┌───────────────────────────┐
   │   WASM Runtime Engine     │
   │  (Wasmtime/Wasmer)        │
   │  🔒 SANDBOXED             │
   └───────────┬───────────────┘
               │ Execute in Sandbox
               ↓
   ┌───────────────────────────┐
   │     WASM Module           │
   │  (add function)           │
   │  ✅ NO SYSTEM ACCESS      │
   └───────────────────────────┘
"
    );

    println!("\n━━━ Key Takeaways ━━━");
    println!("✅ WASM runtime provides:");
    println!("   • Strong security isolation (sandboxed)");
    println!("   • Platform independence (portable)");
    println!("   • Near-native performance");
    println!("   • No system access (secure by default)");
    println!("   • Deterministic execution");
    println!("\n⚠️  Trade-offs:");
    println!("   • Slightly slower than native (5-10% overhead)");
    println!("   • Limited system interaction");
    println!("   • Module size overhead");
    println!("   • Requires WASM compilation toolchain");

    println!("\n════════════════════════════════════════════════════════");
    println!("✅ DEMO COMPLETE - WASM Execution Successful!");
    println!("════════════════════════════════════════════════════════");

    Ok(())
}
