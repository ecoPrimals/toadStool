//! Songbird Discovery Demo
//!
//! Demonstrates capability-based discovery - ToadStool discovers orchestration
//! services without knowing "Songbird" exists!

use toadstool::discovery::{discover_orchestration, OrchestrationClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🍄 ToadStool - Capability-Based Discovery Demo\n");

    // Method 1: Simple helper function
    println!("📍 Method 1: Simple Discovery");
    println!("   Discovering orchestration service by capability...");
    
    match discover_orchestration().await {
        Ok(endpoint) => {
            println!("   ✅ Discovered orchestration service at: {}", endpoint);
            println!("   📝 Note: Could be Songbird, or ANY service with those capabilities!\n");
        }
        Err(e) => {
            println!("   ❌ No orchestration service found: {}", e);
            println!("   💡 Tip: Set SONGBIRD_ENDPOINT env var or run Songbird locally\n");
        }
    }

    // Method 2: Using OrchestrationClient directly
    println!("📍 Method 2: OrchestrationClient");
    let client = OrchestrationClient::new();
    
    println!("   Trying specific capabilities...");
    
    // Try service-discovery
    match client.discover_service_discovery().await {
        Ok(endpoint) => {
            println!("   ✅ Found service-discovery capability at: {}", endpoint);
        }
        Err(_) => {
            println!("   ❌ No service-discovery capability found");
        }
    }
    
    // Try load-balancing
    match client.discover_load_balancing().await {
        Ok(endpoint) => {
            println!("   ✅ Found load-balancing capability at: {}", endpoint);
        }
        Err(_) => {
            println!("   ❌ No load-balancing capability found");
        }
    }
    
    // Try job-routing
    match client.discover_job_routing().await {
        Ok(endpoint) => {
            println!("   ✅ Found job-routing capability at: {}", endpoint);
        }
        Err(_) => {
            println!("   ❌ No job-routing capability found");
        }
    }

    println!("\n🎯 Key Principle:");
    println!("   ToadStool NEVER mentions 'Songbird' in code!");
    println!("   It discovers services by CAPABILITY at runtime.");
    
    println!("\n💡 How to Provide Services:");
    println!("   1. Environment variable: SONGBIRD_ENDPOINT=http://localhost:8082");
    println!("   2. Run Songbird locally (auto-discovered via mDNS)");
    println!("   3. Configure in primal-capabilities.toml");
    
    Ok(())
}

