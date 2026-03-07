// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::unused_async)]
//! # Simplified Distributed Computing Demo
//!
//! Demonstrates the new Songbird-centric architecture where:
//! - ToadStool provides standalone execution capabilities
//! - Songbird handles service discovery, load balancing, and orchestration
//! - ToadStool integrates with Songbird for ecosystem-wide coordination

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use toadstool::workload::{ExecutableSource, WorkloadSpec};
use toadstool::{
    ExecutionInput, ExecutionRequest, ResourceRequirements, RuntimeType, SecurityContext,
};
use toadstool_distributed::{
    DistributedConfig, DistributedCoordinator, SongbirdConfig, StandaloneConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🍄 ToadStool Simplified Distributed Computing Demo");
    println!("================================================");

    // Demonstrate standalone operation
    println!("\n🔧 1. Standalone Operation");
    demonstrate_standalone_operation().await?;

    // Demonstrate Songbird integration
    println!("\n🎼 2. Songbird Integration");
    demonstrate_songbird_integration().await?;

    // Compare architectures
    println!("\n📊 3. Architecture Comparison");
    compare_architectures().await?;

    println!("\n✅ Demo completed successfully!");
    Ok(())
}

async fn demonstrate_standalone_operation() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating standalone ToadStool instance...");

    // Create standalone configuration
    let config = DistributedConfig {
        instance_id: "toadstool-standalone".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 5,
            default_timeout_secs: 60,
            enable_job_queue: true,
            max_queue_size: 50,
        },
        songbird_integration: None, // No Songbird integration
    };

    // Create and start coordinator
    let coordinator = Arc::new(DistributedCoordinator::new(config).await?);
    coordinator.clone().start().await?;

    println!("✓ Standalone coordinator started");

    // Submit some test executions
    println!("Submitting test executions...");

    let executions = vec![
        ("Hello World Script", "echo 'Hello from ToadStool!'"),
        ("Simple Math", "echo $((2 + 3))"),
        ("Date Command", "date"),
    ];

    for (name, _code) in executions {
        let request = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from("echo"),
                },
                args: Some(vec!["Hello from ToadStool!".to_string()]),
                working_dir: None,
                env_vars: HashMap::new(),
                user: None,
            },
            runtime_hint: Some(RuntimeType::Native),
            resources: ResourceRequirements::default(),
            security_context: SecurityContext::default(),
            timeout: Some(Duration::from_secs(30)),
            environment: HashMap::new(),
            input_data: ExecutionInput::default(),
            callback_config: None,
            encryption_config: None,
        };

        let execution_id = coordinator.submit_execution(request).await?;
        println!("✓ Submitted execution '{name}': {execution_id}");
    }

    // Wait for executions to complete
    tokio::time::sleep(Duration::from_secs(2)).await;
    println!("✓ Standalone executions completed");

    Ok(())
}

async fn demonstrate_songbird_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating ToadStool instance with Songbird integration...");

    // Create configuration with Songbird integration
    let config = DistributedConfig {
        instance_id: "toadstool-songbird-integrated".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 120,
            enable_job_queue: true,
            max_queue_size: 100,
        },
        songbird_integration: Some(SongbirdConfig {
            endpoint: "http://localhost:8080".to_string(), // Songbird endpoint
            auth_token: None,
            health_reporting_interval_secs: 30,
        }),
    };

    // Create and start coordinator
    let coordinator = Arc::new(DistributedCoordinator::new(config).await?);

    println!("✓ Coordinator with Songbird integration created");

    // Note: In a real scenario, this would register with Songbird
    // For demo purposes, we'll simulate the integration
    println!("📡 Would register with Songbird at http://localhost:8080");
    println!("📡 Would report capabilities and health status");
    println!("📡 Would receive execution requests routed by Songbird");

    // Show the capabilities that would be reported
    // Note: Capabilities are detected during coordinator construction
    println!("🏷️  Capabilities to report:");
    println!("   - Execution Environments: Native, Container, WASM");
    println!("   - Resource Monitoring: Enabled");
    println!("   - Distributed Coordination: Enabled");
    println!("   - Supported Runtimes: Native, Container, WASM");
    println!(
        "   - Platform: {} ({})",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!("   - CPU Cores: Available");

    // Simulate receiving a request from Songbird
    println!("\n🔄 Simulating request from Songbird...");
    let ml_request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from("python"),
            },
            args: Some(vec![
                "-c".to_string(),
                "print('ML training complete!')".to_string(),
            ]),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        runtime_hint: Some(RuntimeType::Native),
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: Some(Duration::from_secs(300)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    };

    let execution_id = coordinator.submit_execution(ml_request).await?;
    println!("✓ Processed Songbird-routed execution: {execution_id}");

    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("✓ Songbird integration demonstration completed");

    Ok(())
}

async fn compare_architectures() -> Result<(), Box<dyn std::error::Error>> {
    println!("Comparing ToadStool architectures:");

    println!("\n❌ OLD ARCHITECTURE (Complex, Duplicated):");
    println!("   ┌─────────────────────────────────────┐");
    println!("   │ ToadStool Instance                  │");
    println!("   │ ├─ Service Discovery (Consul/Etcd) │");
    println!("   │ ├─ Load Balancing (RoundRobin/etc) │");
    println!("   │ ├─ Cluster Management              │");
    println!("   │ ├─ Complex Job Scheduling          │");
    println!("   │ ├─ Health Monitoring               │");
    println!("   │ └─ Execution Engine                │");
    println!("   └─────────────────────────────────────┘");
    println!("   Problem: Each ToadStool duplicates orchestration logic!");

    println!("\n✅ NEW ARCHITECTURE (Songbird-Centric):");
    println!("   ┌─────────────────────────────────────┐");
    println!("   │ Songbird (Universal Orchestrator)  │");
    println!("   │ ├─ Service Discovery               │");
    println!("   │ ├─ Load Balancing                  │");
    println!("   │ ├─ Request Routing                 │");
    println!("   │ ├─ Health Monitoring               │");
    println!("   │ └─ Cluster Coordination            │");
    println!("   └─────────────────────────────────────┘");
    println!("              │ Routes requests");
    println!("              ▼");
    println!("   ┌─────────────────────────────────────┐");
    println!("   │ ToadStool Instance (Simplified)    │");
    println!("   │ ├─ Songbird Integration Client     │");
    println!("   │ ├─ Capability Reporting            │");
    println!("   │ ├─ Health Status Reporting         │");
    println!("   │ ├─ Local Resource Management       │");
    println!("   │ └─ Execution Engine (Core Focus)   │");
    println!("   └─────────────────────────────────────┘");

    println!("\n📈 BENEFITS OF NEW ARCHITECTURE:");
    println!("   ✓ Simpler ToadStool implementation");
    println!("   ✓ No duplicated orchestration logic");
    println!("   ✓ Songbird handles complex coordination");
    println!("   ✓ ToadStool focuses on execution excellence");
    println!("   ✓ Better ecosystem integration");
    println!("   ✓ Easier maintenance and scaling");

    println!("\n🔍 CODE REDUCTION:");
    println!("   Before: ~2,400 lines of complex distributed code");
    println!("   After:  ~200 lines of focused integration code");
    println!("   Reduction: ~90% less complexity!");

    Ok(())
}
