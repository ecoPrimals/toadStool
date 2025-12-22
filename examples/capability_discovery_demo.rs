//! Example: Migration from Hardcoded to Discovery
//!
//! This example shows the evolution from hardcoded service addresses
//! to runtime capability-based discovery.

use std::collections::HashMap;
use toadstool_common::primal_discovery::{DiscoveryConfig, PrimalDiscovery};

/// OLD WAY: Hardcoded service addresses ❌
#[allow(dead_code)]
mod hardcoded_approach {
    pub const SONGBIRD_PORT: u16 = 8080;
    pub const BEARDOG_PORT: u16 = 8081;
    pub const NESTGATE_PORT: u16 = 8082;

    pub fn get_orchestration_url() -> String {
        format!("http://localhost:{}", SONGBIRD_PORT)
    }

    pub fn get_security_url() -> String {
        format!("http://localhost:{}", BEARDOG_PORT)
    }
}

/// NEW WAY: Capability-based discovery ✅
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up discovery with fallbacks for development
    let config = DiscoveryConfig {
        enable_mdns: true, // Try mDNS first
        fallbacks: HashMap::from([
            (
                "orchestration".to_string(),
                "http://localhost:8080".to_string(),
            ),
            ("security".to_string(), "http://localhost:8081".to_string()),
            ("storage".to_string(), "http://localhost:8082".to_string()),
            (
                "ai_coordination".to_string(),
                "http://localhost:8083".to_string(),
            ),
        ]),
        ..Default::default()
    };

    let discovery = PrimalDiscovery::with_config(config).await?;

    // Discover services by capability, not by name!
    println!("🔍 Discovering services by capability...\n");

    // Orchestration (Songbird)
    match discovery.find_capability("orchestration").await {
        Ok(endpoint) => {
            println!("✅ Found orchestration service:");
            println!("   URL: {}", endpoint.url());
            println!("   Discovered via: {:?}", endpoint.discovered_via);
            println!("   Trust level: {:?}", endpoint.trust_level);
        }
        Err(e) => println!("❌ Orchestration not found: {}", e),
    }

    println!();

    // Security (BearDog)
    match discovery.find_capability("security").await {
        Ok(endpoint) => {
            println!("✅ Found security service:");
            println!("   URL: {}", endpoint.url());
            println!("   Discovered via: {:?}", endpoint.discovered_via);
        }
        Err(e) => println!("❌ Security not found: {}", e),
    }

    println!();

    // Storage (NestGate)
    match discovery.find_capability("storage").await {
        Ok(endpoint) => {
            println!("✅ Found storage service:");
            println!("   URL: {}", endpoint.url());
            println!("   Discovered via: {:?}", endpoint.discovered_via);
        }
        Err(e) => println!("❌ Storage not found: {}", e),
    }

    println!("\n🎉 Benefits of discovery-based approach:");
    println!("   • Zero compile-time coupling");
    println!("   • Works in any environment");
    println!("   • Handles dynamic topologies");
    println!("   • Container-friendly");
    println!("   • Auto-scales");

    Ok(())
}
