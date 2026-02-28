//! Simple ToadStool Universal Compute Platform Demo
//!
//! This demonstrates the core functionality of ToadStool without external dependencies

use std::time::{Duration, SystemTime};
use uuid::Uuid;

use toadstool::error::ToadStoolResult;
use toadstool::resources::ResourceRequirements;
use toadstool::universal::{
    JobPriority, NetworkLocation, PrimalContext, SecurityLevel, UniversalComputePlatform,
    UniversalJob, UniversalJobType,
};

#[tokio::main]
async fn main() -> ToadStoolResult<()> {
    println!("🍄 ToadStool Simple Demo");
    println!("Demonstrating universal compute capabilities");

    // Initialize the platform
    let platform = UniversalComputePlatform::new().await?;

    // Create a simple context
    let context = PrimalContext {
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
    };

    // Create a simple job
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "echo".to_string(),
            args: vec!["Hello, Universal Compute!".to_string()],
            env: std::collections::HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: SystemTime::now(),
        context,
    };

    // Execute the job
    println!("Executing universal job...");
    let response = platform.execute_universal_job(job).await?;

    println!("Job completed with status: {:?}", response.status);
    if let Some(stdout) = response.output.stdout {
        println!("Output: {stdout}");
    }

    println!("✅ Demo completed successfully!");
    Ok(())
}
