# 🏛️ Modern Idiomatic Rust Architecture Examples
## From Hardcoded to Capability-Based

**Date**: December 3, 2025  
**Status**: Foundation Complete - Ready for Integration

---

## 🎯 Core Philosophy

> **Each primal knows only itself. Others are discovered at runtime by capability, not by name.**

---

## Example 1: Service Initialization

### ❌ OLD (Hardcoded)

```rust
// Hardcoded knowledge of ecosystem
const BEARDOG_URL: &str = "http://localhost:8080";
const SONGBIRD_URL: &str = "http://localhost:3000";
const NESTGATE_URL: &str = "http://localhost:8081";

async fn initialize_services() -> Result<Services> {
    let beardog = BeardogClient::new(BEARDOG_URL)?;
    let songbird = SongbirdClient::new(SONGBIRD_URL)?;
    let nestgate = NestgateClient::new(NESTGATE_URL)?;
    
    Ok(Services {
        beardog,
        songbird,
        nestgate,
    })
}
```

###✅ NEW (Capability-Based, Self-Aware)

```rust
use toadstool_common::self_identity::SelfIdentity;
use toadstool_common::runtime_discovery::ServiceRegistry;
use toadstool_common::infant_discovery::capabilities::*;
use toadstool_config::network_config::NetworkConfig;

async fn initialize_toadstool() -> Result<ToadStoolRuntime> {
    // 1. Know ourselves (ONLY ourselves)
    let self_identity = SelfIdentity::new(
        "ToadStool Runtime",
        uuid::Uuid::new_v4().to_string(),
        [COMPUTE_EXECUTION, COMPUTE_NATIVE, COMPUTE_WASM, COMPUTE_CONTAINER],
    )
    .with_version(semver::Version::new(0, 7, 0));

    // 2. Load network config from environment (zero hardcoding)
    let network_config = NetworkConfig::from_env();

    // 3. Create discovery registry (knows only ourselves)
    let registry = ServiceRegistry::new(self_identity);

    // 4. Start announcing ourselves
    tokio::spawn({
        let registry = registry.clone();
        let endpoints = network_config.discovery_endpoints.clone();
        async move {
            ServiceAnnouncer::new(registry, Duration::from_secs(30))
                .run(endpoints)
                .await
        }
    });

    // 5. Start discovering others (by capability, not name!)
    tokio::spawn({
        let registry = registry.clone();
        async move {
            registry.start_discovery_listener().await
        }
    });

    Ok(ToadStoolRuntime {
        registry,
        network_config,
    })
}
```

---

## Example 2: Connecting to PKI Service

### ❌ OLD (Hardcoded BearDog)

```rust
async fn sign_certificate(csr: CertificateRequest) -> Result<Certificate> {
    // Hardcoded assumption: BearDog provides PKI at port 8080
    let beardog = BeardogClient::new("http://localhost:8080")?;
    let cert = beardog.sign_certificate(csr).await?;
    Ok(cert)
}
```

### ✅ NEW (Capability-Based)

```rust
use toadstool_common::infant_discovery::capabilities::PKI;
use toadstool_common::runtime_discovery::CapabilityMatcher;

async fn sign_certificate(
    registry: &ServiceRegistry,
    csr: CertificateRequest,
) -> Result<Certificate> {
    // Discover ANY service that provides PKI (might be BearDog, might not!)
    let pki_service = registry
        .discover_one(CapabilityMatcher::requires(PKI))
        .await?;

    // Connect to discovered service (protocol-agnostic)
    let client = PkiClient::connect(&pki_service.endpoints[0]).await?;
    
    // Use the service (we don't care what it's called)
    let cert = client.sign_certificate(csr).await?;
    Ok(cert)
}
```

---

## Example 3: Multi-Capability Discovery

### ❌ OLD (Hardcoded Songbird)

```rust
async fn setup_orchestration() -> Result<Orchestrator> {
    // Assume Songbird provides orchestration at port 3000
    let songbird = SongbirdClient::new("http://localhost:3000")?;
    
    // Assume it has all these features
    let orchestrator = Orchestrator {
        service_mesh: songbird.clone(),
        load_balancer: songbird.clone(),
        coordinator: songbird,
    };
    
    Ok(orchestrator)
}
```

### ✅ NEW (Capability Matching)

```rust
use toadstool_common::infant_discovery::capabilities::*;

async fn setup_orchestration(registry: &ServiceRegistry) -> Result<Orchestrator> {
    // Discover service with multiple capabilities
    let matcher = CapabilityMatcher::requires(ORCHESTRATION)
        .with_optional([LOAD_BALANCING, SERVICE_MESH]);

    let services = registry.discover(matcher).await?;
    
    // Best match is first (highest score)
    let orchestration_service = services
        .first()
        .ok_or("No orchestration service found")?;

    let client = OrchestrationClient::connect(
        &orchestration_service.endpoints[0]
    ).await?;

    Ok(Orchestrator {
        client,
        capabilities: orchestration_service.capabilities.clone(),
    })
}
```

---

## Example 4: Configuration Loading

### ❌ OLD (Hardcoded Ports)

```rust
#[derive(Debug, Clone)]
struct ServerConfig {
    api_port: u16,
    metrics_port: u16,
    health_port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            api_port: 8080,      // Hardcoded!
            metrics_port: 9090,  // Hardcoded!
            health_port: 8081,   // Hardcoded!
        }
    }
}

async fn start_server() -> Result<()> {
    let config = ServerConfig::default();
    
    // Bind to hardcoded ports
    let api_server = bind_api(config.api_port).await?;
    let metrics_server = bind_metrics(config.metrics_port).await?;
    let health_server = bind_health(config.health_port).await?;
    
    Ok(())
}
```

### ✅ NEW (Environment-Aware)

```rust
use toadstool_config::network_config::{NetworkConfig, EndpointBuilder};

async fn start_server() -> Result<()> {
    // Load from environment with sensible defaults
    let config = NetworkConfig::from_env();
    
    // Or use profile-specific configs
    let config = match std::env::var("ENV").as_deref() {
        Ok("production") => NetworkConfig::production(),
        Ok("test") => NetworkConfig::test(),
        _ => NetworkConfig::development(),
    };
    
    // Bind to configured addresses (not hardcoded!)
    let api_server = bind_api(config.api_addr()).await?;
    let metrics_server = bind_metrics(config.metrics_addr()).await?;
    let health_server = bind_health(config.health_addr()).await?;
    
    // Build URLs from configuration
    let builder = EndpointBuilder::new(config.clone());
    info!("API available at: {}", builder.api_url());
    info!("Metrics available at: {}", builder.metrics_url());
    
    Ok(())
}
```

**Environment Configuration**:
```bash
# Development (default)
cargo run

# Production
ENV=production \
TOADSTOOL_LISTEN_ADDRESS=0.0.0.0 \
TOADSTOOL_SERVICE_PORT=8080 \
TOADSTOOL_DISCOVERY_ENDPOINTS=http://consul:8500,http://etcd:2379 \
cargo run --release

# Testing
ENV=test cargo test
# Uses random available ports automatically!
```

---

## Example 5: Health Checking & Failover

### ❌ OLD (Manual Retry Logic)

```rust
async fn connect_to_storage() -> Result<StorageClient> {
    // Try hardcoded services in order
    if let Ok(client) = NestgateClient::new("http://localhost:8081").await {
        return Ok(StorageClient::Nestgate(client));
    }
    
    if let Ok(client) = BackupStorage::new("http://localhost:8082").await {
        return Ok(StorageClient::Backup(client));
    }
    
    Err("All storage services unavailable")
}
```

### ✅ NEW (Automatic Health-Aware Discovery)

```rust
use toadstool_common::infant_discovery::capabilities::STORAGE;

async fn connect_to_storage(registry: &ServiceRegistry) -> Result<StorageClient> {
    // Discovery automatically:
    // 1. Filters out unhealthy services
    // 2. Returns healthy and degraded only
    // 3. Sorts by match score
    let services = registry
        .discover(CapabilityMatcher::requires(STORAGE))
        .await?;

    // Try services in order (best match first)
    for service in services {
        if let Ok(client) = StorageClient::connect(&service.endpoints[0]).await {
            info!(
                "Connected to storage service: {} (health: {:?})",
                service.display_name,
                service.health
            );
            return Ok(client);
        }
    }

    Err("No available storage service found")
}
```

---

## Example 6: Testing Strategy

### ❌ OLD (Hardcoded Test Ports)

```rust
#[tokio::test]
async fn test_api_endpoints() {
    // Start test server on hardcoded port
    let server = start_test_server(8080).await.unwrap();
    
    // Tests conflict if run in parallel!
    let client = TestClient::new("http://localhost:8080");
    
    // ... tests ...
}
```

### ✅ NEW (Dynamic Test Configuration)

```rust
use toadstool_config::network_config::NetworkConfig;

#[tokio::test]
async fn test_api_endpoints() {
    // Use test configuration (random available ports)
    let config = NetworkConfig::test();
    
    // Start server on OS-assigned port
    let server = start_test_server(config.clone()).await.unwrap();
    let actual_port = server.local_addr().port();
    
    // Build client URL from actual port
    let builder = EndpointBuilder::new(config);
    let client = TestClient::new(&builder.api_url());
    
    // Tests can run in parallel! No port conflicts!
    
    // ... tests ...
}
```

---

## 🏆 Benefits of Modern Architecture

### 1. **Loose Coupling**
- Services discovered by capability, not name
- Can swap implementations without code changes
- Multiple providers of same capability possible

### 2. **Environment Flexibility**
- All configuration via environment variables
- Different configs for dev/prod/test
- No hardcoded values to change

### 3. **Automatic Failover**
- Health checking built-in
- Automatic fallback to healthy services
- Degraded services still usable

### 4. **Scalability**
- Add new services without code changes
- Services announce themselves
- Discovery is automatic

### 5. **Testing**
- No port conflicts in parallel tests
- Mock services register with capabilities
- Test isolation guaranteed

### 6. **Modern Rust Patterns**
- Async/await throughout
- Type-safe configuration
- Builder patterns
- Zero-cost abstractions

---

## 📚 Migration Checklist

For each hardcoded service connection:

- [ ] Identify capability needed (PKI, STORAGE, ORCHESTRATION, etc.)
- [ ] Replace hardcoded URL with discovery call
- [ ] Update error handling for "service not found"
- [ ] Add health check tolerance
- [ ] Update tests to use mock discovery
- [ ] Document capability requirements

For each hardcoded port/address:

- [ ] Move to `NetworkConfig`
- [ ] Add environment variable override
- [ ] Update deployment docs
- [ ] Update tests to use `NetworkConfig::test()`
- [ ] Verify no hardcoded fallbacks remain

---

## 🎯 Success Metrics

**Code Quality**:
- Hardcoded primal names: 3,350 → 0 (100% reduction)
- Hardcoded ports: 1,191 → <10 (99% reduction)
- Capability-based discovery: 0% → 100%

**Operational**:
- Configuration flexibility: ★★★★★
- Service resilience: ★★★★★
- Testing isolation: ★★★★★
- Deployment simplicity: ★★★★★

---

**Status**: Foundation Complete, Examples Documented  
**Next**: Begin integration migration  
**Timeline**: 2-3 weeks for complete migration

---

*"The architecture that adapts is the architecture that survives."*

