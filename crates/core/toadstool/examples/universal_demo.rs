// SPDX-License-Identifier: AGPL-3.0-only
//! ToadStool Universal Compute Platform Demo
//! Showcases core functionality without external dependencies

use std::collections::HashMap;
use std::time::Duration;
use toadstool::universal::{
    get_platform_status, NetworkLocation, PrimalCapability, PrimalContext, PrimalRequest,
    SecurityLevel,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> toadstool::ToadStoolResult<()> {
    // Initialize tracing (ignore if already initialized)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    println!("🍄 ToadStool Universal Compute Platform - Working Demo");
    println!("{}", "=".repeat(60));

    // Initialize ToadStool
    println!("\n🚀 Initializing ToadStool...");
    toadstool::init()?;

    // Show core capabilities
    println!("\n✨ Core Capabilities:");
    for capability in toadstool::UNIVERSAL_CAPABILITIES {
        println!("   ✓ {capability}");
    }

    // Create universal compute platform
    println!("\n🏗️  Creating Universal Compute Platform...");
    let platform = toadstool::UniversalComputePlatform::new().await?;
    println!("   ✅ Platform created successfully!");

    // Get platform status
    println!("\n📊 Platform Status:");
    let status = get_platform_status().await;
    println!("   Platform Status: {status:?}");
    println!(
        "   Ecosystem Integration: {}",
        platform.get_config().ecosystem_integration
    );
    println!(
        "   BiomeOS Integration: {}",
        platform.get_config().biomeos_integration
    );
    println!(
        "   Recursive Hosting: {}",
        platform.get_config().recursive_hosting
    );

    // Initialize platform with biomeOS as a primal (through ecosystem)
    let _biomeos_platform = toadstool::init_with_ecosystem().await?;
    println!("   ✅ BiomeOS platform created!");
    let biomeos_status = get_platform_status().await;
    println!("   BiomeOS Platform Status: {biomeos_status:?}");

    // Test universal configuration
    println!("\n⚙️  Testing Universal Configuration...");
    let config = toadstool::UniversalPlatformConfig {
        recursive_hosting: true,
        ecosystem_integration: true,
        biomeos_integration: true,
        max_concurrent_jobs: 10,
        pure_ecosystem: false,
    };
    let _custom_platform = toadstool::UniversalComputePlatform::new_with_config(config).await?;
    println!("   ✅ Custom platform created with universal configuration!");

    // Create universal job
    println!("\n🎯 Creating Universal Job...");
    let context = PrimalContext {
        user_id: "demo-user".to_string(),
        device_id: "demo-device".to_string(),
        session_id: Uuid::new_v4().to_string(),
        network_location: NetworkLocation {
            ip_address: "127.0.0.1".to_string(),
            subnet: Some("192.168.1.0/24".to_string()),
            network_id: Some("local".to_string()),
            geo_location: Some("localhost".to_string()),
        },
        security_level: SecurityLevel::Standard,
        metadata: HashMap::new(),
    };

    let job = toadstool::UniversalJob {
        id: Uuid::new_v4(),
        job_type: toadstool::UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["Hello from ToadStool Universal Platform!".to_string()],
            env: HashMap::new(),
        },
        priority: toadstool::JobPriority::Normal,
        resources: toadstool::ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context,
    };

    println!("   Job ID: {}", job.id);

    // Execute universal job
    println!("   🏃 Executing universal job...");
    let result = platform.execute_universal_job(job).await?;
    println!("   ✅ Job completed successfully!");
    println!("   📋 Output: {:?}", result.output);

    // Test capability discovery
    println!("\n🔍 Testing Universal Capability Discovery...");
    let native_capability = PrimalCapability::NativeExecution {
        architectures: vec!["x86_64".to_string()],
    };
    let providers = platform
        .find_primals_by_capability(&native_capability)
        .await;
    println!(
        "   Found {} providers with native execution capability",
        providers.len()
    );

    // Test primal request routing
    println!("\n🔀 Testing Universal Primal Request Routing...");
    let primal_request = PrimalRequest {
        id: Uuid::new_v4(),
        source: "demo-client".to_string(),
        target: "toadstool".to_string(),
        request_type: "health_check".to_string(),
        payload: serde_json::json!({}),
        context: PrimalContext {
            user_id: "demo-user".to_string(),
            device_id: "demo-device".to_string(),
            session_id: Uuid::new_v4().to_string(),
            network_location: NetworkLocation {
                ip_address: "127.0.0.1".to_string(),
                subnet: None,
                network_id: None,
                geo_location: None,
            },
            security_level: SecurityLevel::Standard,
            metadata: HashMap::new(),
        },
        metadata: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };

    let response = platform.route_primal_request(primal_request).await?;
    println!("   ✅ Primal request routed successfully!");
    println!("   📋 Response status: {:?}", response.status);

    println!("\n🎉 ToadStool Universal Demo Complete!");
    println!("🌟 Key Features Demonstrated:");
    println!("   ✅ Universal compute platform initialization");
    println!("   ✅ BiomeOS integration capabilities");
    println!("   ✅ Universal configuration management");
    println!("   ✅ Universal job creation and execution");
    println!("   ✅ Capability-based primal discovery");
    println!("   ✅ Universal primal request routing");
    println!("   ✅ Modern, clean universal architecture");

    println!("\n🚀 ToadStool Universal Compute Platform is WORKING!");
    println!("💡 Ready to execute workloads on any substrate!");

    Ok(())
}
