# Migration Guide: Hardcoding to Capability-Based Discovery

This guide explains how to migrate from hardcoded primal names and endpoints to the new capability-based discovery architecture.

## Overview

**Old Architecture** (Hardcoded):
- Services know each other by name (e.g., "songbird", "beardog")
- Endpoints are hardcoded or configured statically
- Tight coupling between services

**New Architecture** (Capability-Based):
- Services discover each other by capability, not name
- Endpoints are discovered at runtime
- Loose coupling, better scalability

## Why This Matters

### Problems with Hardcoding
1. **Primal-Specific Knowledge**: ToadStool shouldn't need to know "Songbird exists"
2. **Tight Coupling**: Changes to service names break everything
3. **No Flexibility**: Can't swap implementations or add alternatives
4. **Deployment Complexity**: Must configure every service's location

### Benefits of Capability-Based Discovery
1. **Self-Knowledge Only**: Each primal knows only itself
2. **Loose Coupling**: Services find what they need by capability
3. **Implementation Agnostic**: Any service providing the capability works
4. **Dynamic Discovery**: Services can come and go at runtime

## Architecture Evolution

### Phase 1: Deprecation (Current)
- Legacy endpoint fields marked `#[deprecated]`
- New discovery APIs available alongside legacy
- Tests updated to use new patterns
- Documentation reflects both approaches

### Phase 2: Dual Mode (Next Release)
- Runtime detection: Use discovery if available, fallback to config
- Helper functions for transparent migration
- Metrics to track discovery vs. fallback usage

### Phase 3: Discovery-Only (Future)
- Remove deprecated fields and functions
- Pure capability-based architecture
- No hardcoded knowledge of other primals

## Migration Patterns

### Pattern 1: Finding a Service by Capability

**Old Code** (Hardcoded):
```rust
use toadstool_config::network;

let songbird_url = network::get_songbird_endpoint();
let client = HttpClient::new(&songbird_url);
```

**New Code** (Capability-Based):
```rust
use toadstool_common::primal_identity::Capability;
use toadstool_common::runtime_discovery::RuntimeDiscovery;
use toadstool_config::discovery_integration::create_discovery;

// Create discovery client
let discovery = create_discovery()?;

// Find service by capability (not by name!)
let services = discovery
    .discover_capability(&Capability::Coordination)
    .await?;

if let Some(service) = services.first() {
    let endpoint = &service.endpoints[0];
    let url = format!("{}://{}:{}", 
        endpoint.protocol, endpoint.address, endpoint.port);
    let client = HttpClient::new(&url);
}
```

**Migration Helper** (Transitional):
```rust
use toadstool_config::discovery_integration::discover_or_fallback;
use toadstool_common::primal_identity::Capability;

let discovery = create_discovery()?;

// Discover with fallback to legacy config
let url = discover_or_fallback(
    &discovery,
    &Capability::Coordination,
    &config.network.endpoints.songbird  // Fallback
).await?;

let client = HttpClient::new(&url);
```

### Pattern 2: Self-Knowledge (My Own Endpoints)

**Old Code** (Hardcoded):
```rust
use toadstool_config::network;

let my_port = network::get_toadstool_port();
let my_endpoint = format!("http://localhost:{}", my_port);
```

**New Code** (Self-Knowledge):
```rust
use toadstool_common::primal_identity::{PrimalIdentity, PrimalType};

let identity = PrimalIdentity::new(
    PrimalType::ToadStool,
    "toadstool-compute-1",
)?;

// Register my capabilities
identity.add_capability(Capability::Compute(
    ComputeCapability::NativeExecution
))?;

// Register my endpoint
identity.add_endpoint(
    "http",
    "0.0.0.0",
    50000,
    Some("/api/v1".to_string()),
)?;

// Get my own endpoint
let my_endpoint = identity.get_primary_endpoint()?;
```

### Pattern 3: Finding Multiple Services with Load Balancing

**Old Code** (Single Hardcoded Endpoint):
```rust
let beardog_url = network::get_beardog_endpoint();
let crypto_client = CryptoClient::new(&beardog_url);
```

**New Code** (Discovery with Load Balancing):
```rust
use toadstool_config::discovery_integration::discover_with_load_balancing;
use toadstool_common::primal_identity::Capability;

let discovery = create_discovery()?;

// Discover and select with load balancing
let url = discover_with_load_balancing(
    &discovery,
    &Capability::Crypto,
    "http://localhost:50002"  // Fallback
).await?;

let crypto_client = CryptoClient::new(&url);
```

### Pattern 4: Finding All Services with a Capability

**New Code** (Discovery All):
```rust
use toadstool_config::discovery_integration::discover_all_by_capability;
use toadstool_common::primal_identity::Capability;

let discovery = create_discovery()?;

// Find all storage services
let storage_services = discover_all_by_capability(
    &discovery,
    &Capability::Storage
).await?;

// Use all available storage services (sharding, replication, etc.)
for service in storage_services {
    let endpoint = &service.endpoints[0];
    println!("Found storage service: {}://{} :{}", 
        endpoint.protocol, endpoint.address, endpoint.port);
}
```

## Capability Types

The system supports various capability types. Use these instead of service names:

### Core Capabilities
- `Capability::Compute(ComputeCapability::*)` - Compute/execution services
- `Capability::Storage` - Storage services
- `Capability::Crypto` - Cryptography services
- `Capability::Coordination` - Coordination/mesh services
- `Capability::AI` - AI/ML services

### Compute Sub-Capabilities
- `ComputeCapability::NativeExecution` - Native binary execution
- `ComputeCapability::ContainerOrchestration` - Container management
- `ComputeCapability::WasmExecution` - WebAssembly runtime
- `ComputeCapability::PythonExecution` - Python runtime
- `ComputeCapability::DistributedCompute` - Distributed execution

## Configuration Migration

### Legacy Configuration (Still Supported)

```toml
[network.endpoints]
songbird = "http://localhost:50001"
beardog = "http://localhost:50002"
nestgate = "http://localhost:50003"
squirrel = "http://localhost:50004"
```

These fields are now deprecated but still functional as fallbacks.

### Discovery-Based Configuration (Recommended)

```toml
[discovery]
# Enable automatic service discovery
enabled = true

# Discovery mechanisms (in priority order)
mechanisms = ["mdns", "consul", "static"]

# Cache TTL for discovered services
cache_ttl = "5m"

# Fallback to config endpoints if discovery fails
enable_fallback = true
```

## Testing Strategies

### Unit Tests with Mock Discovery

```rust
use toadstool_config::discovery_integration::MockDiscoveryClient;
use toadstool_common::primal_identity::*;

#[tokio::test]
async fn test_with_mock_discovery() {
    // Create mock discovery client
    let mock_client = Arc::new(MockDiscoveryClient::new());
    let discovery = RuntimeDiscovery::new(mock_client.clone());
    
    // Register test services
    let test_service = DiscoveredService {
        id: Some("test-coord-1".to_string()),
        capabilities: vec![Capability::Coordination],
        endpoints: vec![ServiceEndpoint {
            protocol: "http".to_string(),
            address: "localhost".to_string(),
            port: 9999,
            path: None,
            metadata: HashMap::new(),
        }],
        healthy: true,
        metadata: HashMap::new(),
    };
    
    mock_client.register_service(&test_service).await?;
    
    // Test discovery
    let services = discovery
        .discover_capability(&Capability::Coordination)
        .await?;
    
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].endpoints[0].port, 9999);
}
```

### Integration Tests with Real Discovery

```rust
#[tokio::test]
#[ignore] // Only run with real services available
async fn test_real_discovery() {
    let discovery = create_discovery()?;
    
    // Attempt to discover real coordination service
    let services = discovery
        .discover_capability(&Capability::Coordination)
        .await;
    
    match services {
        Ok(svcs) if !svcs.is_empty() => {
            println!("Found {} coordination service(s)", svcs.len());
        }
        Ok(_) => {
            println!("No coordination services discovered");
        }
        Err(e) => {
            println!("Discovery failed: {}", e);
        }
    }
}
```

## Common Pitfalls

### Pitfall 1: Still Using Service Names

**Wrong**:
```rust
// Don't hardcode service names
let songbird_services = discovery.discover_by_name("songbird").await?;
```

**Right**:
```rust
// Use capabilities instead
let coord_services = discovery
    .discover_capability(&Capability::Coordination)
    .await?;
```

### Pitfall 2: Ignoring Discovery Errors

**Wrong**:
```rust
let services = discovery.discover_capability(&cap).await.unwrap();
```

**Right**:
```rust
let services = match discovery.discover_capability(&cap).await {
    Ok(svcs) if !svcs.is_empty() => svcs,
    Ok(_) => {
        // No services found, use fallback or return error
        return Err(ToadStoolError::Integration(
            IntegrationError::ServiceNotFound("No coordination service".to_string())
        ));
    }
    Err(e) => {
        // Discovery failed, log and use fallback
        tracing::warn!("Discovery failed: {}", e);
        vec![/* fallback service */]
    }
};
```

### Pitfall 3: Not Handling Multiple Services

**Limited**:
```rust
// Only uses first service
let service = services.first().unwrap();
```

**Better**:
```rust
// Consider all available services
for service in &services {
    // Try each service with retry logic
    match try_service(service).await {
        Ok(result) => return Ok(result),
        Err(e) => {
            tracing::warn!("Service failed: {}, trying next", e);
            continue;
        }
    }
}
Err(ToadStoolError::Integration(
    IntegrationError::AllServicesUnavailable
))
```

## Timeline

- **v0.7.0** (Current): Deprecation phase - both approaches work
- **v0.8.0** (Q1 2026): Dual mode - discovery preferred, config fallback
- **v0.9.0** (Q2 2026): Discovery-only for new code
- **v1.0.0** (Q3 2026): Remove all deprecated hardcoded endpoints

## Migration Checklist

- [ ] Audit code for hardcoded service names and endpoints
- [ ] Replace `network::get_*_endpoint()` calls with discovery
- [ ] Update configuration to use discovery settings
- [ ] Add capability-based service registration on startup
- [ ] Implement fallback strategies for discovery failures
- [ ] Update tests to use mock discovery clients
- [ ] Update documentation and API docs
- [ ] Remove `#[allow(deprecated)]` attributes
- [ ] Test in development environment
- [ ] Roll out to production with monitoring

## Additional Resources

- [Self-Knowledge Architecture](./SELF_KNOWLEDGE_MIGRATION.md)
- [Capability System Documentation](../../specs/PRIMAL_CAPABILITY_SYSTEM.md)
- [Runtime Discovery API](../../crates/core/common/src/runtime_discovery.rs)
- [Discovery Integration Helpers](../../crates/core/config/src/discovery_integration.rs)

## Support

For questions or issues during migration:
1. Check the [FAQ](./FAQ.md)
2. Review [examples](../../examples/discovery/)
3. Open an issue with the `migration` label

