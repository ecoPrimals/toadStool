//! Level 0 Demo: Native Execution
//! Demonstrates basic native execution using ToadStool's UniversalComputePlatform

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
    println!("🍄 ToadStool Level 0: Native Runtime Execution");
    println!("════════════════════════════════════════════════════════\n");

    println!("📌 DEMO OBJECTIVE:");
    println!("   Demonstrate ToadStool's ability to execute native binaries");
    println!("   with proper resource management and security.\n");

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
    println!("✅ Context created for local execution\n");

    // Step 3: Define Native Workload
    println!("━━━ Step 3: Define Native Workload ━━━");
    println!("   Runtime: Native (direct OS execution)");
    println!("   Command: /bin/bash -c 'echo Hello from ToadStool! && date'");
    println!("   Resources: 0.1 CPU, 32MB RAM");
    println!("   Timeout: 10 seconds\n");

    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/bash".to_string(),
            args: vec![
                "-c".to_string(),
                "echo '🍄 Hello from ToadStool Native Runtime!' && echo 'Current time:' && date && echo 'Process ID:' && echo $$".to_string(),
            ],
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
                min_bytes: 32 * 1024 * 1024, // 32 MB
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

    // Step 4: Execute Job
    println!("━━━ Step 4: Execute Job ━━━");
    println!("🚀 Submitting job {} to ToadStool...", job.id);

    let start = std::time::Instant::now();
    let response = platform.execute_universal_job(job.clone()).await?;
    let duration = start.elapsed();

    println!("✅ Job completed in {:.2}s\n", duration.as_secs_f64());

    // Step 5: Display Results
    println!("━━━ Step 5: Execution Results ━━━");
    println!("Job ID: {}", job.id);
    println!("Status: {:?}", response.status);
    println!("Duration: {:.3}s", duration.as_secs_f64());

    if let Some(stdout) = response.output.stdout {
        println!("\n📤 Standard Output:");
        println!("┌────────────────────────────────────┐");
        for line in stdout.lines() {
            println!("│ {:<34} │", line);
        }
        println!("└────────────────────────────────────┘");
    }

    if let Some(stderr) = response.output.stderr {
        if !stderr.is_empty() {
            println!("\n⚠️  Standard Error:");
            println!("{}", stderr);
        }
    }

    println!("\n━━━ Architecture Visualization ━━━");
    println!(
        "
   ┌───────────────────────────┐
   │       User / Demo         │
   │  (UniversalComputePlatform)│
   └───────────┬───────────────┘
               │ Submit Job
               ↓
   ┌───────────────────────────┐
   │     🍄 ToadStool Core     │
   │  (Job Scheduler)          │
   └───────────┬───────────────┘
               │ Route to Runtime
               ↓
   ┌───────────────────────────┐
   │   Native Runtime Engine   │
   │  (Direct OS Execution)    │
   └───────────┬───────────────┘
               │ Execute
               ↓
   ┌───────────────────────────┐
   │     Operating System      │
   │  (/bin/bash process)      │
   └───────────────────────────┘
"
    );

    println!("\n━━━ Key Takeaways ━━━");
    println!("✅ Native runtime provides:");
    println!("   • Maximum performance (no overhead)");
    println!("   • Direct system access");
    println!("   • Full compatibility with system tools");
    println!("   • Fast startup time");
    println!("\n⚠️  Trade-offs:");
    println!("   • Less security isolation");
    println!("   • Platform-specific (not portable)");
    println!("   • Requires trust in workload");

    println!("\n════════════════════════════════════════════════════════");
    println!("✅ DEMO COMPLETE - Native Execution Successful!");
    println!("════════════════════════════════════════════════════════");

    Ok(())
}
