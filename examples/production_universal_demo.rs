#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::useless_format)]
#![allow(clippy::redundant_pattern_matching)]
//! # Production Universal Architecture Demo
//!
//! This example demonstrates the complete ToadStool Universal Architecture
//! capabilities in a production-ready scenario.
//!
//! ## Features Demonstrated
//!
//! - Universal job execution (Native, WASM, Primal, BiomeOS)
//! - Capability-based primal discovery
//! - Resource allocation and management
//! - Inter-primal communication
//! - Security level enforcement
//! - Context-aware routing
//! - Platform status monitoring
//! - Ecosystem integration
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example production_universal_demo
//! ```

use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use toadstool::universal::{
    JobPriority, NetworkLocation, PrimalCapability, PrimalContext, PrimalRequest, SecurityLevel,
    UniversalComputePlatform, UniversalJob, UniversalJobType,
};
use toadstool::{
    init, CpuRequirements, MemoryRequirements, ResourceRequirements, ToadStoolError,
    ToadStoolResult,
};

#[tokio::main]
async fn main() -> ToadStoolResult<()> {
    // Initialize ToadStool
    init().map_err(|e| ToadStoolError::execution(e.to_string()))?;

    println!("🍄 ToadStool Production Universal Architecture Demo");
    println!("{}", "=".repeat(60));

    // Create universal compute platform
    let platform = UniversalComputePlatform::new().await?;
    println!("✅ Universal compute platform initialized");

    // Demonstrate platform capabilities
    demo_platform_status(&platform).await?;
    demo_capability_discovery(&platform).await?;
    demo_native_execution(&platform).await?;
    demo_wasm_execution(&platform).await?;
    demo_primal_routing(&platform).await?;
    demo_biomeos_integration(&platform).await?;
    demo_resource_management(&platform).await?;
    demo_security_levels(&platform).await?;
    demo_ecosystem_integration(&platform).await?;

    println!("\n🎉 Production Universal Architecture Demo Complete!");
    println!("All capabilities demonstrated successfully.");

    Ok(())
}

/// Demonstrate platform status monitoring
async fn demo_platform_status(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n📊 Platform Status Monitoring");
    println!("{}", "-".repeat(40));

    // Get platform configuration
    let config = platform.get_config();
    println!("🔧 Configuration:");
    println!("  • Recursive hosting: {}", config.recursive_hosting);
    println!(
        "  • Ecosystem integration: {}",
        config.ecosystem_integration
    );
    println!("  • BiomeOS integration: {}", config.biomeos_integration);
    println!("  • Max concurrent jobs: {}", config.max_concurrent_jobs);

    // Get available runtimes
    let runtimes = platform.get_available_runtimes().await;
    println!("🏃 Available runtimes: {runtimes:?}");

    Ok(())
}

/// Demonstrate capability-based primal discovery
async fn demo_capability_discovery(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n🔍 Capability-Based Primal Discovery");
    println!("{}", "-".repeat(40));

    // Discover native execution capabilities
    let native_providers = platform
        .find_primals_by_capability(&PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string()],
        })
        .await;
    println!("🖥️  Native execution providers: {}", native_providers.len());

    // Discover WASM capabilities
    let wasm_providers = platform
        .find_primals_by_capability(&PrimalCapability::WasmExecution { wasi_support: true })
        .await;
    println!("🕸️  WASM execution providers: {}", wasm_providers.len());

    // Discover container capabilities
    let container_providers = platform
        .find_primals_by_capability(&PrimalCapability::ContainerRuntime {
            orchestrators: vec!["docker".to_string()],
        })
        .await;
    println!(
        "📦 Container runtime providers: {}",
        container_providers.len()
    );

    // Discover load balancing capabilities
    let lb_providers = platform
        .find_primals_by_capability(&PrimalCapability::LoadBalancing {
            algorithms: vec!["round_robin".to_string()],
        })
        .await;
    println!("⚖️  Load balancing providers: {}", lb_providers.len());

    Ok(())
}

/// Demonstrate native execution
async fn demo_native_execution(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n🖥️  Native Execution Demo");
    println!("{}", "-".repeat(40));

    let context = create_demo_context("native_demo", SecurityLevel::Standard);

    // Create native job
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["Hello from ToadStool Universal Architecture!".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context,
    };

    println!("🚀 Executing native job: {}", job.id);
    let response = platform.execute_universal_job(job).await?;

    println!("✅ Native execution completed:");
    println!("  • Status: {:?}", response.status);
    println!("  • Duration: {:?}", response.duration);
    println!("  • Runtime: {:?}", response.runtime_used);
    if let Some(stdout) = &response.output.stdout {
        println!("  • Output: {stdout}");
    }

    Ok(())
}

/// Demonstrate WASM execution
async fn demo_wasm_execution(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n🕸️  WASM Execution Demo");
    println!("{}", "-".repeat(40));

    let context = create_demo_context("wasm_demo", SecurityLevel::High);

    // Create simple WASM module (mock)
    let wasm_module = create_mock_wasm_module();

    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Wasm {
            module: wasm_module,
            args: vec!["arg1".to_string(), "arg2".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::High,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context,
    };

    println!("🚀 Executing WASM job: {}", job.id);
    let response = platform.execute_universal_job(job).await?;

    println!("✅ WASM execution completed:");
    println!("  • Status: {:?}", response.status);
    println!("  • Duration: {:?}", response.duration);
    println!("  • Runtime: {:?}", response.runtime_used);
    if let Some(stdout) = &response.output.stdout {
        println!("  • Output: {stdout}");
    }

    Ok(())
}

/// Demonstrate primal routing
async fn demo_primal_routing(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n🎯 Primal Routing Demo");
    println!("{}", "-".repeat(40));

    let context = create_demo_context("primal_demo", SecurityLevel::Standard);

    // Create primal request
    let request = PrimalRequest {
        id: Uuid::new_v4(),
        source: "toadstool-demo".to_string(),
        target: "toadstool".to_string(),
        request_type: "compute_request".to_string(),
        payload: serde_json::json!({
            "operation": "status_check",
            "parameters": {
                "include_metrics": true
            }
        }),
        context: context.clone(),
        metadata: HashMap::new(),
        timestamp: chrono::Utc::now(),
    };

    println!("🚀 Routing primal request: {}", request.id);
    let response = platform.route_primal_request(request).await?;

    println!("✅ Primal routing completed:");
    println!("  • Status: {:?}", response.status);
    println!("  • Response: {}", response.payload);

    // Create primal job
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "compute".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            payload: serde_json::json!({
                "task": "process_data",
                "data": "sample_data"
            }),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context,
    };

    println!("🚀 Executing primal job: {}", job.id);
    let response = platform.execute_universal_job(job).await?;

    println!("✅ Primal execution completed:");
    println!("  • Status: {:?}", response.status);
    println!("  • Duration: {:?}", response.duration);

    Ok(())
}

/// Demonstrate BiomeOS integration
async fn demo_biomeos_integration(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n🌱 BiomeOS Integration Demo");
    println!("{}", "-".repeat(40));

    let context = create_demo_context("biomeos_demo", SecurityLevel::Maximum);

    // Create BiomeOS job
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({
                "version": "1.0",
                "team": "demo-team",
                "services": {
                    "web-service": {
                        "image": "nginx:latest",
                        "ports": ["80:8080"],
                        "environment": {
                            "ENV": "production"
                        }
                    }
                },
                "resources": {
                    "cpu": "2",
                    "memory": "4Gi"
                }
            }),
            team_id: "demo-team".to_string(),
        },
        priority: JobPriority::High,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(60)),
        created_at: chrono::Utc::now(),
        context,
    };

    println!("🚀 Executing BiomeOS job: {}", job.id);
    let response = platform.execute_universal_job(job).await?;

    println!("✅ BiomeOS execution completed:");
    println!("  • Status: {:?}", response.status);
    println!("  • Duration: {:?}", response.duration);
    println!("  • Team: demo-team");
    if let Some(stdout) = &response.output.stdout {
        println!("  • Output: {stdout}");
    }

    Ok(())
}

/// Demonstrate resource management
async fn demo_resource_management(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n💾 Resource Management Demo");
    println!("{}", "-".repeat(40));

    let context = create_demo_context("resource_demo", SecurityLevel::Standard);

    // Create resource-intensive job
    let mut resources = ResourceRequirements::default();
    resources.cpu = CpuRequirements {
        min_cores: 2.0,
        max_cores: Some(2.0),
        architecture: None,
    };
    resources.memory = MemoryRequirements {
        min_bytes: 1024 * 1024 * 1024, // 1GB
        max_bytes: Some(1024 * 1024 * 1024),
    };

    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/sleep".to_string(),
            args: vec!["2".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources,
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context,
    };

    println!("🚀 Executing resource-intensive job: {}", job.id);
    println!("  • CPU cores: {}", job.resources.cpu.min_cores);
    println!(
        "  • Memory: {} MB",
        job.resources.memory.min_bytes / (1024 * 1024)
    );

    let response = platform.execute_universal_job(job).await?;

    println!("✅ Resource management completed:");
    println!("  • Status: {:?}", response.status);
    println!("  • Duration: {:?}", response.duration);
    println!("  • CPU usage: {:.2}%", response.metrics.cpu.usage_percent);
    println!(
        "  • Memory usage: {} MB",
        response.metrics.memory.used_bytes / (1024 * 1024)
    );

    Ok(())
}

/// Demonstrate security levels
async fn demo_security_levels(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n🔒 Security Levels Demo");
    println!("{}", "-".repeat(40));

    let security_levels = vec![
        SecurityLevel::Basic,
        SecurityLevel::Standard,
        SecurityLevel::High,
        SecurityLevel::Maximum,
    ];

    for level in security_levels {
        let context = create_demo_context(&format!("security_{level:?}"), level);

        let job = UniversalJob {
            id: Uuid::new_v4(),
            job_type: UniversalJobType::Native {
                executable: "/bin/echo".to_string(),
                args: vec![format!("Security level: {:?}", level)],
                env: HashMap::new(),
            },
            priority: JobPriority::Normal,
            resources: ResourceRequirements::default(),
            timeout: Some(Duration::from_secs(30)),
            created_at: chrono::Utc::now(),
            context,
        };

        println!("🚀 Executing job with security level: {level:?}");
        let response = platform.execute_universal_job(job).await?;

        println!("  ✅ Status: {:?}", response.status);
    }

    Ok(())
}

/// Demonstrate ecosystem integration
async fn demo_ecosystem_integration(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n🌐 Ecosystem Integration Demo");
    println!("{}", "-".repeat(40));

    // Discover ecosystem
    println!("🔍 Discovering ecosystem...");
    platform.discover_ecosystem().await?;
    println!("✅ Ecosystem discovery completed");

    // Test different primal types
    let primal_types = vec![
        ("compute", "ToadStool compute primal"),
        ("security", "BearDog security primal"),
        ("storage", "NestGate storage primal"),
        ("ai", "Squirrel AI primal"),
        ("network", "Songbird network primal"),
    ];

    for (primal_type, description) in primal_types {
        println!("🎯 Testing {description} integration...");

        let context =
            create_demo_context(&format!("ecosystem_{primal_type}"), SecurityLevel::Standard);

        let job = UniversalJob {
            id: Uuid::new_v4(),
            job_type: UniversalJobType::Primal {
                primal_type: primal_type.to_string(),
                endpoint: format!("http://localhost:8080/{primal_type}"),
                payload: serde_json::json!({
                    "operation": "health_check",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }),
            },
            priority: JobPriority::Normal,
            resources: ResourceRequirements::default(),
            timeout: Some(Duration::from_secs(30)),
            created_at: chrono::Utc::now(),
            context,
        };

        let response = platform.execute_universal_job(job).await?;
        println!("  ✅ {} integration: {:?}", description, response.status);
    }

    Ok(())
}

/// Create a demo context for testing
fn create_demo_context(demo_name: &str, security_level: SecurityLevel) -> PrimalContext {
    PrimalContext {
        user_id: format!("demo-user-{demo_name}"),
        device_id: "demo-device".to_string(),
        session_id: Uuid::new_v4().to_string(),
        network_location: NetworkLocation {
            ip_address: "127.0.0.1".to_string(),
            subnet: Some("192.168.1.0/24".to_string()),
            network_id: Some("demo-network".to_string()),
            geo_location: Some("localhost".to_string()),
        },
        security_level,
        metadata: {
            let mut metadata = HashMap::new();
            metadata.insert("demo".to_string(), "true".to_string());
            metadata.insert("demo_name".to_string(), demo_name.to_string());
            metadata.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
            metadata
        },
    }
}

/// Create a mock WASM module for demonstration
fn create_mock_wasm_module() -> Vec<u8> {
    // This is a minimal valid WASM module that does nothing
    // In production, this would be actual compiled WASM bytecode
    vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // WASM version
    ]
}
