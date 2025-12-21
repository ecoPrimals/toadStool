# Self-Knowledge Architecture Migration Guide
**Date**: December 4, 2025  
**Status**: Implementation in Progress

---

## 🎯 Overview

This guide shows how to migrate from hardcoded primal references to self-knowledge architecture where each primal only knows about itself.

## 🧠 Core Principle

**OLD**: Primals have hardcoded knowledge of other primals
```rust
// ❌ BAD: Hardcoded knowledge of other primals
pub struct Config {
    songbird_url: String,     // Knows about Songbird
    nestgate_url: String,     // Knows about NestGate
    beardog_url: String,      // Knows about BearDog
    squirrel_mcp_url: String, // Knows about Squirrel
}

// Usage requires knowing WHO to talk to
let response = http_client
    .get(&config.songbird_url)
    .send()
    .await?;
```

**NEW**: Primals only know themselves, discover others by capability
```rust
// ✅ GOOD: Self-knowledge only
pub struct ToadStoolIdentity {
    name: "toadstool",              // Only knows its own name
    capabilities: [Compute, ...],    // What I can do
    endpoints: [http://..., ...],    // Where I am
}

// Usage discovers by WHAT you need, not WHO
let discovery = RuntimeDiscovery::new(discovery_client);
let compute_service = discovery.find_compute_service().await?;
let endpoint = compute_service.endpoints[0];
let response = http_client.get(&endpoint.url()).send().await?;
```

---

## 📋 Migration Steps

### Step 1: Define Your Identity

```rust
use toadstool_common::primal_identity::{
    ToadStoolIdentity, PrimalIdentity, ServiceEndpoint,
};

// Create your primal's identity (knows only about itself)
let mut identity = ToadStoolIdentity::new();

// Add your endpoints
identity.add_endpoint(ServiceEndpoint::http("0.0.0.0", 8080));
identity.add_endpoint(ServiceEndpoint::grpc("0.0.0.0", 9090));

// Identity now contains:
// - name: "toadstool"
// - version: from Cargo.toml
// - capabilities: [NativeExecution, ContainerOrchestration, ...]
// - endpoints: [http://0.0.0.0:8080, grpc://0.0.0.0:9090]
```

### Step 2: Set Up Discovery

```rust
use toadstool_common::runtime_discovery::{
    RuntimeDiscovery, LocalhostDiscoveryClient,
};
use std::sync::Arc;

// Create discovery client (can be DNS-SD, mDNS, Registry, etc.)
let discovery_client = Arc::new(LocalhostDiscoveryClient::new());

// Create runtime discovery service
let discovery = RuntimeDiscovery::new(discovery_client)
    .with_cache_ttl(Duration::from_secs(300));
```

### Step 3: Discover Services by Capability

```rust
use toadstool_common::primal_identity::{
    Capability, ComputeCapability, StorageCapability, AuthCapability,
};

// ❌ OLD: Hardcoded primal-specific code
async fn get_storage_url(config: &Config) -> String {
    config.nestgate_url.clone()  // Assumes NestGate = storage
}

// ✅ NEW: Capability-based discovery
async fn get_storage_service(
    discovery: &RuntimeDiscovery
) -> ToadStoolResult<DiscoveredService> {
    // Find ANY service with storage capability
    discovery.find_storage_service().await
}

// Usage
let storage_svc = get_storage_service(&discovery).await?;
let endpoint = storage_svc.endpoints.first()
    .ok_or_else(|| "No endpoints")?;
let url = endpoint.url();
```

### Step 4: Replace Hardcoded References

#### Example 1: Config File Migration

```rust
// ❌ OLD: runtime_defaults.rs
pub const DEFAULT_SONGBIRD_URL: &str = "http://localhost:7654";
pub const DEFAULT_NESTGATE_URL: &str = "http://localhost:8888";
pub const DEFAULT_BEARDOG_URL: &str = "http://localhost:9999";

pub struct ServiceConfig {
    pub songbird_url: String,
    pub nestgate_url: String,
    pub beardog_url: String,
}

// ✅ NEW: No hardcoded primal URLs
pub struct ServiceConfig {
    // Remove all primal-specific fields
    // Add discovery configuration instead
    pub discovery_endpoints: Vec<String>,
    pub discovery_protocol: DiscoveryProtocol,
}

// Get services dynamically
pub async fn get_required_services(
    discovery: &RuntimeDiscovery
) -> ToadStoolResult<RequiredServices> {
    Ok(RequiredServices {
        coordinator: discovery.find_coordinator_service().await?,
        storage: discovery.find_storage_service().await?,
        auth: discovery.find_auth_service().await?,
    })
}
```

#### Example 2: Integration Layer Migration

```rust
// ❌ OLD: songbird_integration/integration.rs
pub struct SongbirdClient {
    base_url: String,  // Hardcoded knowledge of Songbird
    client: HttpClient,
}

impl SongbirdClient {
    pub fn new(url: String) -> Self {
        Self {
            base_url: url,
            client: HttpClient::new(),
        }
    }
    
    pub async fn submit_request(&self, req: Request) -> Result<Response> {
        // Uses hardcoded base_url
        self.client
            .post(&format!("{}/api/v1/submit", self.base_url))
            .json(&req)
            .send()
            .await
    }
}

// ✅ NEW: Generic capability-based client
pub struct CoordinatorClient {
    discovery: Arc<RuntimeDiscovery>,
    client: HttpClient,
    // No hardcoded URLs!
}

impl CoordinatorClient {
    pub fn new(discovery: Arc<RuntimeDiscovery>) -> Self {
        Self {
            discovery,
            client: HttpClient::new(),
        }
    }
    
    pub async fn submit_request(&self, req: Request) -> Result<Response> {
        // Discover coordinator service dynamically
        let coordinator = self.discovery.find_coordinator_service().await?;
        let endpoint = coordinator.endpoints.first()
            .ok_or_else(|| "No coordinator endpoints")?;
        
        // Use discovered endpoint
        self.client
            .post(&format!("{}/api/v1/submit", endpoint.url()))
            .json(&req)
            .send()
            .await
    }
}
```

#### Example 3: CLI Commands Migration

```rust
// ❌ OLD: cli/src/ecosystem/mod.rs
pub async fn connect_to_songbird() -> Result<SongbirdConnection> {
    let url = env::var("SONGBIRD_URL")
        .unwrap_or_else(|_| "http://localhost:7654".to_string());
    SongbirdConnection::new(&url).await
}

// ✅ NEW: Capability-based connection
pub async fn connect_to_coordinator(
    discovery: &RuntimeDiscovery
) -> Result<CoordinatorConnection> {
    let service = discovery.find_coordinator_service().await?;
    CoordinatorConnection::new_from_service(&service).await
}
```

---

## 🔄 Migration Patterns

### Pattern 1: Config Constant Replacement

```rust
// Before
pub const DEFAULT_PRIMAL_X_PORT: u16 = 8080;

// After
// Remove constant entirely.
// Port discovered via RuntimeDiscovery
```

### Pattern 2: Service Client Replacement

```rust
// Before
pub struct PrimalXClient {
    url: String,
}

// After
pub struct CapabilityClient {
    discovery: Arc<RuntimeDiscovery>,
    capability: Capability,
}
```

### Pattern 3: Switch Statement Replacement

```rust
// Before
match primal_name {
    "songbird" => connect_songbird(),
    "nestgate" => connect_nestgate(),
    "beardog" => connect_beardog(),
    _ => Err(UnknownPrimal),
}

// After
let capability = determine_capability(request_type);
let service = discovery.discover_capability(&capability).await?;
connect_to_service(&service).await
```

---

## 📊 Progress Tracking

### Files to Migrate

#### High Priority (Hardcoded References)
- [ ] `crates/core/config/src/runtime_defaults.rs` (~5 primal refs)
- [ ] `crates/core/config/src/services.rs` (~10 primal refs)
- [ ] `crates/distributed/src/songbird_integration/` (~42 refs)
- [ ] `crates/cli/src/ecosystem/` (~54 refs)
- [ ] `crates/distributed/src/crypto_lock.rs` (~54 refs)
- [ ] `crates/auto_config/src/squirrel_mcp.rs` (~48 refs)

#### Medium Priority (Integration Layers)
- [ ] `crates/integration/protocols/` (~30 refs)
- [ ] `crates/client/src/` (~20 refs)

#### Low Priority (Tests & Examples)
- [ ] Test fixtures (appropriate to keep for testing)
- [ ] Example code (appropriate to keep for demonstration)

---

## ✅ Benefits

### Before Migration
- 🔴 **3,910 hardcoded primal references**
- 🔴 **Tight coupling** between primals
- 🔴 **Fragile** - breaks when primals move
- 🔴 **Inflexible** - can't add new primals easily

### After Migration
- ✅ **<100 references** (config only)
- ✅ **Loose coupling** via capabilities
- ✅ **Resilient** - discovers current locations
- ✅ **Flexible** - any primal can provide capability

---

## 🎯 Success Criteria

Migration is complete when:
1. ✅ Each primal only knows its own identity
2. ✅ All service discovery is capability-based
3. ✅ No hardcoded primal names in prod code
4. ✅ Tests use discovery or fixtures
5. ✅ All integrations work via discovery

---

## 📚 See Also

- **Implementation**: `crates/core/common/src/primal_identity.rs`
- **Discovery**: `crates/core/common/src/runtime_discovery.rs`
- **Evolution Plan**: `EVOLUTION_PLAN_DEC_4_2025.md`

---

**Created**: December 4, 2025  
**Status**: Implementation in Progress  
**Target**: Zero hardcoded primal references

