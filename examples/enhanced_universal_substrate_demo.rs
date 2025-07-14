//! Enhanced Universal Substrate Demo - Simplified Version
//!
//! This demo shows ToadStool's universal compute capabilities
//! with a focus on core functionality and simplified examples.

use std::time::Duration;
use uuid::Uuid;
use chrono::Utc;

use toadstool::error::ToadStoolResult;
use toadstool::universal::{
    UniversalComputePlatform, UniversalJob, UniversalJobType, JobPriority,
    UniversalPlatformConfig, SystemResources, PrimalContext, NetworkLocation, SecurityLevel,
};
use toadstool::resources::{
    ResourceRequirements, CpuRequirements, MemoryRequirements, 
    NetworkRequirements, StorageRequirements,
};

#[tokio::main]
async fn main() -> ToadStoolResult<()> {
    println!("🍄 Enhanced Universal Substrate Demo");
    println!("Demonstrating ToadStool's universal compute capabilities");
    println!("{}", "=".repeat(60));

    // Initialize the universal compute platform
    let platform = UniversalComputePlatform::new().await?;
    
    // Demonstrate basic job execution
    demonstrate_basic_execution(&platform).await?;
    
    // Demonstrate different job types
    demonstrate_job_types(&platform).await?;
    
    // Demonstrate resource management
    demonstrate_resource_management(&platform).await?;
    
    println!("\n✅ Enhanced Universal Substrate Demo completed successfully!");
    Ok(())
}

async fn demonstrate_basic_execution(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n🔧 Demonstrating Basic Execution");
    println!("{}", "-".repeat(40));
    
    let context = create_demo_context();
    
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "echo".to_string(),
            args: vec!["Hello from Universal Substrate!".to_string()],
            env: std::collections::HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: Utc::now(),
        context,
    };
    
    println!("  📋 Executing basic job...");
    let response = platform.execute_universal_job(job).await?;
    
    println!("  ✅ Job completed with status: {:?}", response.status);
    if let Some(stdout) = response.output.stdout {
        println!("  📤 Output: {}", stdout);
    }
    
    Ok(())
}

async fn demonstrate_job_types(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n🎯 Demonstrating Different Job Types");
    println!("{}", "-".repeat(40));
    
    let context = create_demo_context();
    
    // Native job
    let native_job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "ls".to_string(),
            args: vec!["-la".to_string()],
            env: std::collections::HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(10)),
        created_at: Utc::now(),
        context: context.clone(),
    };
    
    println!("  🖥️  Executing native job...");
    let _response = platform.execute_universal_job(native_job).await?;
    println!("  ✅ Native job completed");
    
    // WASM job
    let wasm_job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Wasm {
            module: b"mock_wasm_module".to_vec(),
            args: vec!["arg1".to_string()],
            env: std::collections::HashMap::new(),
        },
        priority: JobPriority::High,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(15)),
        created_at: Utc::now(),
        context,
    };
    
    println!("  🔧 Executing WASM job...");
    let _response = platform.execute_universal_job(wasm_job).await?;
    println!("  ✅ WASM job completed");
    
    Ok(())
}

async fn demonstrate_resource_management(platform: &UniversalComputePlatform) -> ToadStoolResult<()> {
    println!("\n📊 Demonstrating Resource Management");
    println!("{}", "-".repeat(40));
    
    let context = create_demo_context();
    
    // Create a job with specific resource requirements
    let resource_job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "sleep".to_string(),
            args: vec!["1".to_string()],
            env: std::collections::HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 4.0,
                max_cores: Some(8.0),
                architecture: Some("x86_64".to_string()),
            },
            memory: MemoryRequirements {
                min_bytes: 8 * 1024 * 1024 * 1024, // 8GB
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 100 * 1024 * 1024 * 1024, // 100GB
                max_bytes: None,
                storage_type: Some("ssd".to_string()),
            },
            network: NetworkRequirements {
                min_bandwidth: Some(10),
                max_bandwidth: None,
                max_latency_ms: None,
            },
            gpu: None,
        },
        timeout: Some(Duration::from_secs(5)),
        created_at: Utc::now(),
        context,
    };
    
    println!("  📋 Executing job with resource requirements...");
    println!("    - CPU: 2.0 cores");
    println!("    - Memory: 1GB");
    println!("    - Storage: 100MB");
    println!("    - Network: 10Mbps");
    
    let _response = platform.execute_universal_job(resource_job).await?;
    println!("  ✅ Resource-managed job completed");
    
    Ok(())
}

fn create_demo_context() -> PrimalContext {
    PrimalContext {
        user_id: "demo_user".to_string(),
        device_id: "demo_device".to_string(),
        session_id: Uuid::new_v4().to_string(),
        network_location: NetworkLocation {
            ip_address: "127.0.0.1".to_string(),
            subnet: None,
            network_id: None,
            geo_location: None,
        },
        security_level: SecurityLevel::Standard,
        metadata: std::collections::HashMap::new(),
    }
}
