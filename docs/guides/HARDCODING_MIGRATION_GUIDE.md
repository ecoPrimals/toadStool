# 🔄 Hardcoding Migration Guide
## Evolving from Hardcoded Primals to Capability-Based Discovery

**Date**: December 3, 2025  
**Philosophy**: Each primal knows only itself. Others are discovered at runtime.

---

## 🎯 The Problem: Hardcoded Primal Knowledge

**Before**: Code had hardcoded knowledge of other primals:
```rust
// ❌ BAD: Hardcoded primal names
let beardog_url = "http://localhost:8080";
let beardog_client = BeardogClient::new(beardog_url);

// ❌ BAD: Hardcoded primal detection
match primal_name {
    "beardog" => handle_beardog(),
    "songbird" => handle_songbird(),
    "nestgate" => handle_nestgate(),
    _ => Err("Unknown primal"),
}

// ❌ BAD: Hardcoded capability mapping
if service_name == "beardog" {
    // We know BearDog provides PKI
    use_pki_service();
}
```

**Problems**:
1. **Tight coupling** - Changes to primal names break code
2. **Not scalable** - Adding new primals requires code changes
3. **Not composable** - Can't discover alternate implementations
4. **Not agnostic** - Assumes specific primal ecosystem

---

## ✅ The Solution: Capability-Based Discovery

**After**: Code discovers services by capabilities at runtime:
```rust
// ✅ GOOD: Capability-based discovery
use toadstool_common::infant_discovery::capabilities::PKI;
use toadstool_common::runtime_discovery::{ServiceRegistry, CapabilityMatcher};

// Discover ANY service that provides PKI (could be BearDog, or something else!)
let pki_services = registry
    .discover(CapabilityMatcher::requires(PKI))
    .await?;

let pki_service = pki_services
    .first()
    .ok_or("No PKI service available")?;

// Connect to discovered service (protocol-agnostic)
let client = connect_to(&pki_service.endpoints[0]).await?;
```

**Benefits**:
1. **Loose coupling** - Depends on capabilities, not names
2. **Scalable** - New services auto-discovered
3. **Composable** - Multiple implementations possible
4. **Agnostic** - Works with any ecosystem

---

## 📚 Migration Steps

### Step 1: Initialize Self-Identity

Each primal declares only itself:

```rust
use toadstool_common::self_identity::SelfIdentity;
use toadstool_common::infant_discovery::capabilities::*;

// ✅ ToadStool knows only about itself
let self_identity = SelfIdentity::new(
    "ToadStool Runtime",           // Display name (for humans)
    "toadstool-instance-1",         // Unique instance ID
    [
        COMPUTE_EXECUTION,          // What WE provide
        COMPUTE_NATIVE,
        COMPUTE_WASM,
        COMPUTE_CONTAINER,
    ],
)
.with_version(semver::Version::new(0, 7, 0))
.with_metadata("region", "us-west-2")
.with_metadata("environment", "production");
```

### Step 2: Create Service Registry

Initialize the discovery system:

```rust
use toadstool_common::runtime_discovery::ServiceRegistry;

// Create registry (knows only itself)
let registry = ServiceRegistry::new(self_identity);

// Start discovery (listens for announcements from other services)
tokio::spawn(async move {
    registry.start_discovery().await
});
```

### Step 3: Replace Hardcoded Connections

**BEFORE** (hardcoded):
```rust
// ❌ Hardcoded BearDog connection
let beardog = BeardogClient::new("http://localhost:8080")?;
let cert = beardog.sign_certificate(csr).await?;
```

**AFTER** (capability-based):
```rust
// ✅ Discover PKI service (might be BearDog, might be something else)
use toadstool_common::infant_discovery::capabilities::PKI;

let pki_service = registry
    .discover_one(CapabilityMatcher::requires(PKI))
    .await?;

// Generic PKI client (works with any PKI service)
let pki_client = PkiClient::connect(&pki_service.endpoints[0]).await?;
let cert = pki_client.sign_certificate(csr).await?;
```

### Step 4: Replace Hardcoded Service Names

**BEFORE** (hardcoded):
```rust
// ❌ Hardcoded primal detection
fn get_service_type(name: &str) -> ServiceType {
    match name.to_lowercase().as_str() {
        "beardog" => ServiceType::Security,
        "songbird" => ServiceType::Orchestration,
        "nestgate" => ServiceType::Storage,
        "squirrel" => ServiceType::AI,
        _ => ServiceType::Unknown,
    }
}
```

**AFTER** (capability-based):
```rust
// ✅ Capability-based service categorization
fn get_service_capabilities(service: &DiscoveredService) -> Vec<String> {
    service.capabilities.iter().cloned().collect()
}

// ✅ Discover by what we need, not who provides it
let orchestration_services = registry
    .discover(CapabilityMatcher::requires(ORCHESTRATION))
    .await?;
```

### Step 5: Replace Hardcoded Capability Mapping

**BEFORE** (hardcoded):
```rust
// ❌ Hardcoded capability knowledge
#[allow(deprecated)]
use toadstool_common::primal_capabilities::legacy_primal_to_capabilities;

let caps = legacy_primal_to_capabilities("beardog"); // Returns [PKI, SECRETS, AUTH, ...]
```

**AFTER** (dynamic):
```rust
// ✅ Capabilities announced at runtime by services themselves
let service = registry
    .discover_one(CapabilityMatcher::requires(PKI))
    .await?;

// Service tells us what it can do
println!("Service provides: {:?}", service.capabilities);
```

---

## 🔍 Common Migration Patterns

### Pattern 1: Required + Optional Capabilities

**Use case**: Need PKI, prefer service with secrets management too

```rust
let matcher = CapabilityMatcher::requires(PKI)
    .with_optional([SECRETS, AUTHENTICATION]);

let services = registry.discover(matcher).await?;
// Services with all capabilities score higher
```

### Pattern 2: Excluding Deprecated Services

**Use case**: Avoid legacy implementations

```rust
let matcher = CapabilityMatcher::requires(STORAGE)
    .excluding(["deprecated", "legacy"]);

let services = registry.discover(matcher).await?;
// Only modern implementations returned
```

### Pattern 3: Multi-Capability Services

**Use case**: Need orchestration AND load balancing

```rust
let matcher = CapabilityMatcher::requires_all([
    ORCHESTRATION,
    LOAD_BALANCING,
]);

let services = registry.discover(matcher).await?;
```

### Pattern 4: Health-Aware Discovery

**Use case**: Only connect to healthy services

```rust
// Discovery automatically filters unhealthy services
let services = registry
    .discover(CapabilityMatcher::requires(PKI))
    .await?;

// All returned services are Healthy or Degraded
// Unhealthy services are excluded
```

---

## 📦 Environment Variables & Configuration

Replace hardcoded ports/URLs with environment-based discovery:

**BEFORE** (hardcoded):
```rust
const BEARDOG_URL: &str = "http://localhost:8080";
const SONGBIRD_URL: &str = "http://localhost:3000";
const NESTGATE_URL: &str = "http://localhost:8081";
```

**AFTER** (discovery endpoints):
```rust
// Discovery endpoints (where to announce/discover services)
let discovery_endpoints = env::var("DISCOVERY_ENDPOINTS")
    .unwrap_or_else(|_| "http://localhost:9999".to_string());

// Services announce themselves to discovery endpoints
// Clients discover services from discovery endpoints
// No hardcoded service URLs!
```

---

## 🧪 Testing Strategy

### Testing with Mock Discovery

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_capability_discovery() {
        let self_id = SelfIdentity::new("Test", "test-1", ["compute"]);
        let registry = ServiceRegistry::new(self_id);

        // Register mock service
        let mock_pki = DiscoveredService {
            display_name: "Mock PKI".to_string(),
            instance_id: "mock-pki-1".to_string(),
            capabilities: [PKI].iter().map(|s| s.to_string()).collect(),
            endpoints: vec![/* test endpoints */],
            // ... other fields
        };

        registry.register(mock_pki).await.unwrap();

        // Test discovery
        let found = registry
            .discover_one(CapabilityMatcher::requires(PKI))
            .await
            .unwrap();

        assert_eq!(found.display_name, "Mock PKI");
    }
}
```

---

## 📊 Migration Checklist

### Phase 1: Foundation (Week 1)
- [x] Create `self_identity` module ✅
- [x] Create `runtime_discovery` module ✅
- [ ] Initialize `ServiceRegistry` in main application
- [ ] Set up discovery announcements

### Phase 2: Critical Paths (Week 2-3)
- [ ] Migrate PKI service connections (BearDog)
- [ ] Migrate orchestration connections (Songbird)
- [ ] Migrate storage connections (NestGate)
- [ ] Migrate AI service connections (Squirrel)

### Phase 3: Configuration (Week 3-4)
- [ ] Remove hardcoded primal URLs
- [ ] Remove hardcoded port constants
- [ ] Add environment variable overrides
- [ ] Update deployment documentation

### Phase 4: Testing & Validation (Week 4)
- [ ] Add integration tests for discovery
- [ ] Test multi-provider scenarios
- [ ] Test failover and health checking
- [ ] Performance testing

### Phase 5: Cleanup (Week 5)
- [ ] Remove deprecated `primal_capabilities` module
- [ ] Remove hardcoded primal name constants
- [ ] Update all documentation
- [ ] Archive migration helpers

---

## 🎯 Success Metrics

**Before Migration**:
- 3,350 hardcoded primal name references
- 1,191 hardcoded port/network references
- Zero runtime discovery

**After Migration**:
- 0 hardcoded primal name references ✅
- Minimal hardcoded configuration (discovery endpoints only)
- 100% runtime capability-based discovery ✅

---

## 📖 Additional Resources

- `crates/core/common/src/self_identity.rs` - Self-identity implementation
- `crates/core/common/src/runtime_discovery.rs` - Discovery system
- `crates/core/common/src/infant_discovery/` - Capability definitions
- `crates/core/common/src/primal_capabilities.rs` - Legacy migration helper (deprecated)

---

**Philosophy**: 
> "Know thyself. Discover others by what they can do, not who they are."

**Status**: Foundation complete, ready for migration ✅

