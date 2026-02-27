//! Modern ToadStool Integration Example
//!
//! This example demonstrates how to use the new capability-based discovery
//! and environment-aware configuration systems together.
//!
//! # Philosophy
//! - Each primal knows only itself
//! - Discover services by capability, not name
//! - Configure via environment, not hardcoding
//!
//! # Run
//! ```bash
//! cargo run --example modern_toadstool_integration
//! ```

use std::time::Duration;
use toadstool_common::infant_discovery::capabilities::capabilities::*;
use toadstool_common::runtime_discovery::{CapabilityMatcher, ServiceRegistry};
use toadstool_common::self_identity::{
    DiscoveredService, HealthStatus, SelfIdentity, ServiceEndpoint,
};
use toadstool_config::network_config::{EndpointBuilder, NetworkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🍄 ToadStool - Modern Integration Example\n");

    // ═══════════════════════════════════════════════════════════════════════
    // PART 1: Self-Identity (Know Yourself)
    // ═══════════════════════════════════════════════════════════════════════

    println!("📋 PART 1: Defining Our Identity");
    println!("─────────────────────────────────────");

    // Define who WE are (ToadStool only knows about itself)
    let self_identity = SelfIdentity::new(
        "ToadStool Universal Runtime",
        uuid::Uuid::new_v4().to_string(),
        [
            "compute:execution",
            "compute:native",
            "compute:wasm",
            "compute:container",
            "compute:python",
        ],
    )
    .with_version(semver::Version::new(0, 7, 0))
    .with_metadata("region", "us-west-2")
    .with_metadata("environment", "development");

    println!("✅ Identity: {}", self_identity.display_name());
    println!("✅ Instance: {}", self_identity.instance_id());
    println!("✅ Capabilities: {:?}", self_identity.capabilities());
    println!();

    // ═══════════════════════════════════════════════════════════════════════
    // PART 2: Network Configuration (Environment-Aware)
    // ═══════════════════════════════════════════════════════════════════════

    println!("🌐 PART 2: Network Configuration");
    println!("─────────────────────────────────────");

    // Load configuration from environment (no hardcoding!)
    let network_config = NetworkConfig::from_env();

    println!("✅ Service Port: {}", network_config.service_port);
    println!("✅ API Port: {}", network_config.api_port);
    println!("✅ Metrics Port: {}", network_config.metrics_port);
    println!("✅ Listen Address: {}", network_config.listen_address);

    // Build endpoint URLs
    let endpoint_builder = EndpointBuilder::new(network_config.clone());
    println!("\n📍 Endpoints:");
    println!("   Service: {}", endpoint_builder.service_url());
    println!("   API: {}", endpoint_builder.api_url());
    println!("   Metrics: {}", endpoint_builder.metrics_url());
    println!("   Health: {}", endpoint_builder.health_url());
    println!();

    // ═══════════════════════════════════════════════════════════════════════
    // PART 3: Service Discovery Registry
    // ═══════════════════════════════════════════════════════════════════════

    println!("🔍 PART 3: Runtime Service Discovery");
    println!("─────────────────────────────────────");

    // Create discovery registry (knows only ourselves)
    let registry = ServiceRegistry::new(self_identity);
    println!("✅ Registry initialized with our identity");

    // ═══════════════════════════════════════════════════════════════════════
    // PART 4: Simulating Service Discovery
    // ═══════════════════════════════════════════════════════════════════════

    println!("\n🎭 PART 4: Discovering Services by Capability");
    println!("─────────────────────────────────────");

    // Register some mock discovered services
    register_mock_services(&registry).await?;

    // Discovery 1: Find PKI service (traditionally BearDog)
    println!("\n🔐 Discovering PKI capability...");
    match registry
        .discover_one(CapabilityMatcher::requires(PKI))
        .await
    {
        Ok(service) => {
            println!("   ✅ Found: {}", service.display_name);
            println!("   📍 Endpoint: {}", service.endpoints[0].uri);
            println!("   💚 Health: {:?}", service.health);
            println!("   🎯 Capabilities: {:?}", service.capabilities);
        }
        Err(e) => println!("   ❌ Not found: {}", e),
    }

    // Discovery 2: Find orchestration service (traditionally Songbird)
    println!("\n🎵 Discovering orchestration capability...");
    match registry
        .discover_one(CapabilityMatcher::requires(ORCHESTRATION))
        .await
    {
        Ok(service) => {
            println!("   ✅ Found: {}", service.display_name);
            println!("   📍 Endpoint: {}", service.endpoints[0].uri);
            println!("   💚 Health: {:?}", service.health);
            println!("   🎯 Capabilities: {:?}", service.capabilities);
        }
        Err(e) => println!("   ❌ Not found: {}", e),
    }

    // Discovery 3: Find storage service (traditionally NestGate)
    println!("\n💾 Discovering storage capability...");
    match registry
        .discover_one(CapabilityMatcher::requires(STORAGE))
        .await
    {
        Ok(service) => {
            println!("   ✅ Found: {}", service.display_name);
            println!("   📍 Endpoint: {}", service.endpoints[0].uri);
            println!("   💚 Health: {:?}", service.health);
            println!("   🎯 Capabilities: {:?}", service.capabilities);
        }
        Err(e) => println!("   ❌ Not found: {}", e),
    }

    // Discovery 4: Find service with multiple capabilities
    println!("\n🎯 Discovering with optional capabilities...");
    let matcher = CapabilityMatcher::requires(ORCHESTRATION)
        .with_optional([LOAD_BALANCING, SERVICE_MESH]);

    match registry.discover(matcher).await {
        Ok(services) => {
            println!("   ✅ Found {} matching services", services.len());
            for service in services {
                println!("      - {}: {:?}", service.display_name, service.health);
            }
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }

    // ═══════════════════════════════════════════════════════════════════════
    // PART 5: Health Monitoring
    // ═══════════════════════════════════════════════════════════════════════

    println!("\n💚 PART 5: Health Monitoring");
    println!("─────────────────────────────────────");

    let stats = registry.health_stats().await;
    println!("Health Statistics:");
    for (status, count) in stats {
        println!("   {:?}: {} services", status, count);
    }

    // List all discovered services
    println!("\n📋 All Discovered Services:");
    let all_services = registry.list_all().await;
    for service in &all_services {
        println!(
            "   - {} [{:?}] @ {}",
            service.display_name, service.health, service.endpoints[0].uri
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // PART 6: The Philosophy in Action
    // ═══════════════════════════════════════════════════════════════════════

    println!("\n🎓 PART 6: What We Just Demonstrated");
    println!("─────────────────────────────────────");
    println!("✅ Self-Knowledge: ToadStool knows only itself");
    println!("✅ Capability-Based: Services discovered by what they do");
    println!("✅ Runtime Discovery: No hardcoded primal names");
    println!("✅ Environment Config: No hardcoded ports/addresses");
    println!("✅ Health-Aware: Automatic health filtering");
    println!("✅ Protocol-Agnostic: Not tied to specific implementations");

    println!("\n📈 Benefits:");
    println!("   • Add new services without code changes");
    println!("   • Multiple providers of same capability");
    println!("   • Automatic failover to healthy services");
    println!("   • Environment-specific configuration");
    println!("   • No vendor lock-in");

    println!("\n🎊 Modern ToadStool Integration Complete!\n");

    Ok(())
}

/// Register mock services for demonstration
async fn register_mock_services(registry: &ServiceRegistry) -> Result<(), Box<dyn std::error::Error>> {
    // Mock PKI service (simulating BearDog)
    let pki_service = DiscoveredService {
        display_name: "PKI Service".to_string(),
        instance_id: "pki-001".to_string(),
        capabilities: vec![PKI.to_string(), SECRETS.to_string(), AUTHENTICATION.to_string()]
            .into_iter()
            .collect(),
        endpoints: vec![ServiceEndpoint {
            protocol: "https".to_string(),
            uri: "https://pki-service.local:8443".to_string(),
            priority: 0,
        }],
        version: semver::Version::new(1, 0, 0),
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
        health: HealthStatus::Healthy,
    };
    registry.register(pki_service).await?;

    // Mock orchestration service (simulating Songbird)
    let orchestration_service = DiscoveredService {
        display_name: "Orchestration Service".to_string(),
        instance_id: "orch-001".to_string(),
        capabilities: vec![
            ORCHESTRATION.to_string(),
            SERVICE_MESH.to_string(),
            LOAD_BALANCING.to_string(),
        ]
        .into_iter()
        .collect(),
        endpoints: vec![ServiceEndpoint {
            protocol: "grpc".to_string(),
            uri: "grpc://orchestration.local:9090".to_string(),
            priority: 0,
        }],
        version: semver::Version::new(2, 1, 0),
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
        health: HealthStatus::Healthy,
    };
    registry.register(orchestration_service).await?;

    // Mock storage service (simulating NestGate)
    let storage_service = DiscoveredService {
        display_name: "Storage Service".to_string(),
        instance_id: "storage-001".to_string(),
        capabilities: vec![
            STORAGE.to_string(),
            KEY_VALUE_STORE.to_string(),
            CACHE.to_string(),
        ]
        .into_iter()
        .collect(),
        endpoints: vec![ServiceEndpoint {
            protocol: "http".to_string(),
            uri: "http://storage.local:8081".to_string(),
            priority: 0,
        }],
        version: semver::Version::new(1, 5, 2),
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
        health: HealthStatus::Healthy,
    };
    registry.register(storage_service).await?;

    // Mock AI service (simulating Squirrel) - degraded
    let ai_service = DiscoveredService {
        display_name: "AI Processing Service".to_string(),
        instance_id: "ai-001".to_string(),
        capabilities: vec![AI_PROCESSING.to_string(), NLP.to_string()]
            .into_iter()
            .collect(),
        endpoints: vec![ServiceEndpoint {
            protocol: "http".to_string(),
            uri: "http://ai.local:8888".to_string(),
            priority: 0,
        }],
        version: semver::Version::new(0, 9, 0),
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
        health: HealthStatus::Degraded, // Note: degraded but still usable
    };
    registry.register(ai_service).await?;

    Ok(())
}

