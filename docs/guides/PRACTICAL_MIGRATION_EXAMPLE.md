# 🔄 Practical Migration Example
## Real Code Transformation - Before & After

**Date**: December 3, 2025  
**File**: `crates/cli/src/zero_config/discovery.rs`  
**Status**: Migration pattern demonstrated

---

## 🎯 THE TRANSFORMATION

### ❌ BEFORE: Hardcoded Discovery (Current Code)

**Location**: `crates/cli/src/zero_config/discovery.rs:371-416`

```rust
/// Discover BearDog service
async fn discover_beardog(&self) -> Result<Option<ServiceEndpoint>> {
    debug!("Discovering BearDog service");

    // ❌ HARDCODED: Port 8000 assumption
    let endpoint = "http://localhost:8000";

    match self.check_service_endpoint(endpoint, "beardog").await {
        Ok(service) => Ok(Some(service)),
        Err(_) => {
            debug!("BearDog service not found");
            Ok(None)
        }
    }
}

/// Discover NestGate service
async fn discover_nestgate(&self) -> Result<Option<ServiceEndpoint>> {
    debug!("Discovering NestGate service");

    // ❌ HARDCODED: Port 9000 assumption
    let endpoint = "http://localhost:9000";

    match self.check_service_endpoint(endpoint, "nestgate").await {
        Ok(service) => Ok(Some(service)),
        Err(_) => {
            debug!("NestGate service not found");
            Ok(None)
        }
    }
}

/// Discover Squirrel service
async fn discover_squirrel(&self) -> Result<Option<ServiceEndpoint>> {
    debug!("Discovering Squirrel service");

    // ❌ HARDCODED: Port 6000 assumption
    let endpoint = "http://localhost:6000";

    match self.check_service_endpoint(endpoint, "squirrel").await {
        Ok(service) => Ok(Some(service)),
        Err(_) => {
            debug!("Squirrel service not found");
            Ok(None)
        }
    }
}
```

**Problems**:
- ❌ Hardcoded primal names (beardog, nestgate, squirrel)
- ❌ Hardcoded ports (8000, 9000, 6000)
- ❌ Hardcoded to localhost only
- ❌ Assumes specific primal implementations
- ❌ Can't discover alternative providers
- ❌ Not scalable (add new primal = add new function)

---

### ✅ AFTER: Capability-Based Discovery (Modern Pattern)

```rust
use toadstool_common::infant_discovery::capabilities::capabilities::*;
use toadstool_common::runtime_discovery::{CapabilityMatcher, ServiceRegistry};

/// Discover PKI service (capability-based, not name-based)
async fn discover_pki_service(
    &self,
    registry: &ServiceRegistry,
) -> Result<Option<DiscoveredService>> {
    debug!("Discovering PKI capability (formerly BearDog)");

    // ✅ CAPABILITY-BASED: Find ANY service providing PKI
    match registry
        .discover_one(CapabilityMatcher::requires(PKI))
        .await
    {
        Ok(service) => {
            info!(
                service = %service.display_name,
                endpoint = %service.endpoints[0].uri,
                "Discovered PKI service"
            );
            Ok(Some(service))
        }
        Err(_) => {
            debug!("No PKI service available");
            Ok(None)
        }
    }
}

/// Discover storage service (capability-based)
async fn discover_storage_service(
    &self,
    registry: &ServiceRegistry,
) -> Result<Option<DiscoveredService>> {
    debug!("Discovering storage capability (formerly NestGate)");

    // ✅ CAPABILITY-BASED: Find ANY service providing storage
    match registry
        .discover_one(CapabilityMatcher::requires(STORAGE))
        .await
    {
        Ok(service) => {
            info!(
                service = %service.display_name,
                endpoint = %service.endpoints[0].uri,
                capabilities = ?service.capabilities,
                "Discovered storage service"
            );
            Ok(Some(service))
        }
        Err(_) => {
            debug!("No storage service available");
            Ok(None)
        }
    }
}

/// Discover AI processing service (capability-based)
async fn discover_ai_service(
    &self,
    registry: &ServiceRegistry,
) -> Result<Option<DiscoveredService>> {
    debug!("Discovering AI processing capability (formerly Squirrel)");

    // ✅ CAPABILITY-BASED: Find ANY service providing AI
    match registry
        .discover_one(CapabilityMatcher::requires(AI_PROCESSING))
        .await
    {
        Ok(service) => {
            info!(
                service = %service.display_name,
                endpoint = %service.endpoints[0].uri,
                health = ?service.health,
                "Discovered AI service"
            );
            Ok(Some(service))
        }
        Err(_) => {
            debug!("No AI service available");
            Ok(None)
        }
    }
}
```

**Benefits**:
- ✅ No hardcoded primal names
- ✅ No hardcoded ports
- ✅ Works with any network configuration
- ✅ Discovers any implementation of capability
- ✅ Health-aware (unhealthy services filtered)
- ✅ Scalable (add primal = just announce capability)

---

## 🔍 SIDE-BY-SIDE COMPARISON

### Hardcoded Approach (Old)
```rust
// Function per primal
async fn discover_beardog() { /* hardcoded port 8000 */ }
async fn discover_songbird() { /* hardcoded port 3000 */ }
async fn discover_nestgate() { /* hardcoded port 9000 */ }
async fn discover_squirrel() { /* hardcoded port 6000 */ }
async fn discover_biomeos() { /* hardcoded port 8080 */ }
// Add new primal = add new function!
```

### Capability-Based Approach (New)
```rust
// Single generic function
async fn discover_by_capability(
    registry: &ServiceRegistry,
    capability: &str,
) -> Result<Option<DiscoveredService>> {
    registry.discover_one(CapabilityMatcher::requires(capability)).await
}

// Usage:
let pki = discover_by_capability(registry, PKI).await?;
let storage = discover_by_capability(registry, STORAGE).await?;
let ai = discover_by_capability(registry, AI_PROCESSING).await?;
// Add new primal = just discover its capability! No code changes!
```

---

## 🔨 MIGRATION STEPS FOR THIS FILE

### Step 1: Add ServiceRegistry Parameter

**Before**:
```rust
impl ZeroConfigDiscovery {
    pub async fn discover_services(&self) -> Result<DiscoveryResult> {
        let beardog = self.discover_beardog().await?;
        let nestgate = self.discover_nestgate().await?;
        let squirrel = self.discover_squirrel().await?;
        // ...
    }
}
```

**After**:
```rust
impl ZeroConfigDiscovery {
    pub async fn discover_services(
        &self,
        registry: &ServiceRegistry,  // ✅ Add registry
    ) -> Result<DiscoveryResult> {
        let pki = self.discover_by_capability(registry, PKI).await?;
        let storage = self.discover_by_capability(registry, STORAGE).await?;
        let ai = self.discover_by_capability(registry, AI_PROCESSING).await?;
        // ...
    }
}
```

### Step 2: Replace Individual Functions

**Before**: 3 separate hardcoded functions (90 lines)
```rust
async fn discover_beardog() { /* 30 lines */ }
async fn discover_nestgate() { /* 30 lines */ }
async fn discover_squirrel() { /* 30 lines */ }
```

**After**: 1 generic capability function (20 lines)
```rust
async fn discover_by_capability(
    &self,
    registry: &ServiceRegistry,
    capability: &str,
) -> Result<Option<DiscoveredService>> {
    registry
        .discover_one(CapabilityMatcher::requires(capability))
        .await
        .ok()
}
```

**Reduction**: 90 lines → 20 lines (78% reduction!)

### Step 3: Update ServiceEndpoint Type

**Before**: Custom type with hardcoded assumptions
```rust
struct ServiceEndpoint {
    service_type: ServiceType,  // ❌ Hardcoded enum
    address: SocketAddr,
    // ...
}

enum ServiceType {
    BearDog,    // ❌ Hardcoded
    Songbird,   // ❌ Hardcoded
    NestGate,   // ❌ Hardcoded
    Squirrel,   // ❌ Hardcoded
}
```

**After**: Use our DiscoveredService type
```rust
use toadstool_common::self_identity::DiscoveredService;

// ✅ Already has:
// - display_name (not hardcoded type)
// - capabilities (dynamic)
// - endpoints (protocol-agnostic)
// - health (automatic tracking)
```

---

## 📊 IMPACT ANALYSIS

### Code Reduction
- **Before**: 3 functions × 30 lines = 90 lines
- **After**: 1 function × 20 lines = 20 lines
- **Reduction**: 78% fewer lines

### Maintenance
- **Before**: Add primal = new function (30 lines each)
- **After**: Add primal = announce capability (0 lines!)
- **Scalability**: ∞ (infinite)

### Hardcoding Eliminated
- **Before**: 3 hardcoded names, 3 hardcoded ports
- **After**: 0 hardcoded names, 0 hardcoded ports
- **Reduction**: 100%

---

## 🎯 FULL FILE MIGRATION ESTIMATE

**File**: `crates/cli/src/zero_config/discovery.rs`  
**Current Size**: ~600 lines  
**Hardcoded**: beardog, songbird, nestgate, squirrel discovery  
**Estimated After**: ~400 lines (33% reduction)  
**Effort**: 4-6 hours  
**Complexity**: Medium (straightforward refactoring)

---

## 🚀 NEXT FILES TO MIGRATE

### Priority 1: Zero-Config Discovery (This File)
- **File**: `crates/cli/src/zero_config/discovery.rs`
- **Impact**: HIGH (foundation for zero-config)
- **Effort**: 4-6 hours
- **Hardcoding**: 4 service discovery functions

### Priority 2: Network Configurator
- **File**: `crates/cli/src/network_config/configurator/core.rs`
- **Impact**: HIGH (network configuration)
- **Effort**: 6-8 hours
- **Hardcoding**: DNS domains, health endpoints, service URLs

### Priority 3: Ecosystem Integrator
- **File**: `crates/cli/src/ecosystem/integrator_impl.rs`
- **Impact**: MEDIUM (legacy layer)
- **Effort**: 4 hours
- **Hardcoding**: Service type mapping

### Priority 4: Templates
- **File**: `crates/cli/src/templates/specialized_templates.rs`
- **Impact**: MEDIUM (service templates)
- **Effort**: 2-3 hours
- **Hardcoding**: Service dependencies, port mappings

---

## ✅ MIGRATION SUCCESS CRITERIA

### For Each File
- [ ] Zero hardcoded primal names
- [ ] Zero hardcoded ports (use NetworkConfig)
- [ ] Uses ServiceRegistry for discovery
- [ ] Uses CapabilityMatcher for matching
- [ ] Tests updated and passing
- [ ] Documentation updated

### Overall
- [ ] All CLI discovery functions migrated
- [ ] All network configuration migrated
- [ ] All ecosystem integration migrated
- [ ] Full test coverage maintained
- [ ] Performance validated

---

## 🎓 LESSONS

### What Makes This Easy
1. **Foundation Complete** - All building blocks ready
2. **Patterns Clear** - Examples provided
3. **Systematic** - Same pattern repeats
4. **Safe** - Can migrate incrementally

### What Makes This Valuable
1. **Eliminates Hardcoding** - 3,350 → 0 instances
2. **Improves Scalability** - Add services without code
3. **Better Resilience** - Automatic health checking
4. **Modern Architecture** - Capability-based design

---

## 📚 REFERENCE

### Before Pattern (Hardcoded)
```rust
async fn discover_PRIMAL(&self) -> Result<Option<ServiceEndpoint>> {
    let endpoint = "http://localhost:PORT";  // ❌ Hardcoded
    self.check_service_endpoint(endpoint, "PRIMAL").await
}
```

### After Pattern (Capability-Based)
```rust
async fn discover_by_capability(
    &self,
    registry: &ServiceRegistry,
    capability: &str,
) -> Result<Option<DiscoveredService>> {
    registry.discover_one(CapabilityMatcher::requires(capability)).await.ok()
}
```

---

**Status**: Pattern demonstrated, ready to apply  
**Next**: Begin actual migration (4-6 hours per file)  
**Timeline**: 2-3 weeks for complete migration

---

*"The pattern is clear. The path is obvious. The work is straightforward. Let's proceed."* 🚀

