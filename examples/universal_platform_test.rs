// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::redundant_closure_for_method_calls)]

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use toadstool::UniversalResourceRequirements;
use toadstool::universal::{
    JobPriority, NetworkLocation, PrimalCapability, PrimalContext, PrimalRequest, SecurityLevel,
    UniversalComputePlatform, UniversalJob, UniversalJobType, get_platform_status,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Rebuilt Universal Compute Platform");

    // Create platform
    let platform = UniversalComputePlatform::new().await?;
    println!("✅ Universal platform created successfully");

    // Test platform status
    let status = get_platform_status().await;
    println!("📊 Platform status: {status:?}");

    // Test job creation
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "echo".to_string(),
            args: vec!["Hello, Universal ToadStool!".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: UniversalResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: SystemTime::now(),
        context: PrimalContext {
            user_id: "test_user".to_string(),
            device_id: "test_device".to_string(),
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
    };

    // Execute job
    println!("🏃 Executing universal job...");
    let result = platform.execute_universal_job(job).await?;
    println!("✅ Job completed successfully!");
    println!("📋 Result: {:?}", result.output.stdout);

    // Test primal capabilities
    let capability = PrimalCapability::NativeExecution {
        architectures: vec!["x86_64".to_string()],
    };
    let providers = platform.find_primals_by_capability(&capability).await;
    println!(
        "🔍 Found {} providers with native execution capability",
        providers.len()
    );

    // Test different job types
    let wasm_job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Wasm {
            module: vec![0x00, 0x61, 0x73, 0x6d], // Basic WASM header
            args: vec!["test".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::High,
        resources: UniversalResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: SystemTime::now(),
        context: PrimalContext {
            user_id: "test_user".to_string(),
            device_id: "test_device".to_string(),
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
    };

    println!("🕸️ Executing WASM job...");
    let wasm_result = platform.execute_universal_job(wasm_job).await?;
    println!("✅ WASM job completed successfully!");
    println!("📋 WASM Result: {:?}", wasm_result.output.stdout);

    // Test BiomeOS job
    let biome_job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({
                "name": "test-app",
                "version": "1.0.0"
            }),
            team_id: "test-team".to_string(),
        },
        priority: JobPriority::Normal,
        resources: UniversalResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: SystemTime::now(),
        context: PrimalContext {
            user_id: "test_user".to_string(),
            device_id: "test_device".to_string(),
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
    };

    println!("🌱 Executing BiomeOS job...");
    let biome_result = platform.execute_universal_job(biome_job).await?;
    println!("✅ BiomeOS job completed successfully!");
    println!("📋 BiomeOS Result: {:?}", biome_result.output.stdout);

    // Test primal request routing
    let primal_request = PrimalRequest {
        id: Uuid::new_v4(),
        source: "test".to_string(),
        target: "toadstool-main".to_string(),
        request_type: "health_check".to_string(),
        payload: serde_json::json!({}),
        context: PrimalContext {
            user_id: "test_user".to_string(),
            device_id: "test_device".to_string(),
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
        timestamp: SystemTime::now(),
    };

    println!("🎯 Testing primal request routing...");
    let primal_response = platform.route_primal_request(primal_request).await?;
    println!("✅ Primal request completed successfully!");
    println!("📋 Primal Response: {:?}", primal_response.status);

    println!("🎉 All tests passed! Universal platform is working correctly.");
    println!("🏗️ Successfully rebuilt ToadStool with clean, modern universal architecture!");

    Ok(())
}
