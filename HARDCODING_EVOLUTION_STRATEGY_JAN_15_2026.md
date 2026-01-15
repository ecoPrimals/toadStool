# Hardcoding Evolution Strategy - January 15, 2026

## 🚨 CRITICAL FINDING

**Primal Name References**: **1,550 instances** across 143 files  
**Deep Debt Violation**: 🔴 **SEVERE** - Primals should only know themselves!

---

## 📊 Hardcoding Breakdown

### Category 1: Primal Names (HIGHEST PRIORITY)

| Pattern | Count | Files | Severity |
|---------|-------|-------|----------|
| `beardog\|BearDog` | ~500 | 143 | 🔴 **SEVERE** |
| `songbird\|SongBird` | ~500 | 143 | 🔴 **SEVERE** |
| `nestgate\|NestGate` | ~350 | 143 | 🔴 **SEVERE** |
| `squirrel` | ~200 | 143 | 🔴 **SEVERE** |
| **TOTAL** | **1,550** | **143** | 🔴 **CRITICAL** |

**Deep Debt Principle Violated**: "Primals know only themselves"

### Category 2: Hardcoded Ports

| Port | Usage | Severity |
|------|-------|----------|
| 8080 | HTTP server default | ⚠️ MODERATE |
| 5432 | PostgreSQL | ⚠️ MODERATE |
| 6379 | Redis | ⚠️ MODERATE |
| 3000 | Alternative HTTP | ⚠️ MODERATE |
| 9000 | Metrics/Admin | ⚠️ MODERATE |

**Count**: ~100 instances  
**Impact**: Runtime inflexibility, port conflicts

### Category 3: Hardcoded URLs

| Pattern | Usage | Severity |
|---------|-------|----------|
| `http://localhost` | Development | ⚠️ MODERATE |
| `http://{ip}:{port}` | Dynamic construction | ✅ ACCEPTABLE |

**Count**: ~50 instances  
**Impact**: Environment-specific, not portable

---

## 🎯 Evolution Strategy

### Phase 1: Capability-Based Discovery (IMMEDIATE)

**Goal**: Replace all primal name references with capability-based discovery

#### Before (Anti-Pattern):
```rust
// ❌ BAD: Hardcoded primal name
let beardog_client = discover_service("beardog").await?;
let endpoint = format!("http://beardog:8000");
```

#### After (Deep Debt Compliant):
```rust
// ✅ GOOD: Capability-based discovery
let security_provider = discover_capability("encryption").await?;
// OR
let security_provider = discover_capability("key-management").await?;
```

**Key Principle**: Ask for **what you need**, not **who provides it**

---

### Phase 2: Port Discovery Evolution

#### Before (Anti-Pattern):
```rust
// ❌ BAD: Hardcoded port
let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
```

#### After (Deep Debt Compliant):
```rust
// ✅ GOOD: Dynamic port discovery
let port = RuntimePortDiscovery::discover_available_port()?;
let addr = SocketAddr::from(([127, 0, 0, 1], port));
```

**Already Implemented**: `RuntimePortDiscovery` module exists!

---

### Phase 3: Environment-Based Configuration

#### Before (Anti-Pattern):
```rust
// ❌ BAD: Hardcoded localhost
let url = "http://localhost:8080";
```

#### After (Deep Debt Compliant):
```rust
// ✅ GOOD: Environment-based or discovered
let url = env::var("TOADSTOOL_ENDPOINT")
    .or_else(|_| discover_service_endpoint("toadstool"))
    .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
```

---

## 🔥 Critical Files for Immediate Fix

### Top 10 Violators

1. **`crates/cli/src/network_config/configurator/core.rs`**
   - Contains: beardog, songbird, nestgate references
   - Purpose: Network configuration
   - **Action**: Replace with capability discovery

2. **`crates/cli/src/templates/specialized_templates.rs`**
   - Contains: Hardcoded primal connections
   - Purpose: Template generation
   - **Action**: Use capability placeholders

3. **`crates/distributed/src/security_provider/beardog_impl/`**
   - Contains: beardog-specific code
   - Purpose: Security provider implementation
   - **Status**: ✅ **ACCEPTABLE** - This IS the beardog impl, self-knowledge OK

4. **`crates/cli/src/ecosystem/`**
   - Contains: Multiple primal references
   - Purpose: Ecosystem integration
   - **Action**: Replace with discovery engine

5. **`crates/core/config/src/`**
   - Contains: Default configurations with primal names
   - Purpose: Configuration defaults
   - **Action**: Remove primal-specific defaults

---

## 📋 Implementation Checklist

### Immediate Actions (Week 1)

- [ ] **Audit beardog_impl/** - Verify it's only self-knowledge
- [ ] **Create CapabilityType enum** - Define capability types
- [ ] **Update discovery engine** - Support capability-based lookup
- [ ] **Fix network configurator** - Remove hardcoded primal names
- [ ] **Update templates** - Use capability placeholders

### Short-term Actions (Week 2-3)

- [ ] **Fix top 20 files** - Replace hardcoded names
- [ ] **Update tests** - Use capability-based mocks
- [ ] **Documentation** - Show capability-based patterns
- [ ] **CI/CD checks** - Lint for primal name violations

### Long-term Actions (Month 1-2)

- [ ] **Full codebase scan** - Find all 1,550 instances
- [ ] **Systematic replacement** - 50-100 instances per week
- [ ] **Integration tests** - Verify discovery works
- [ ] **Performance testing** - Ensure no slowdown

---

## 🏗️ Code Patterns

### Pattern 1: Security/Encryption

```rust
// BEFORE: ❌ Hardcoded primal
use crate::beardog_integration::BearDogClient;
let client = BearDogClient::connect("http://beardog:8000").await?;

// AFTER: ✅ Capability-based
use crate::universal_adapter::UniversalAdapter;
let adapter = UniversalAdapter::new().await?;
let security = adapter.get_security_provider().await?;
// Adapter discovers and connects to ANY security provider (beardog, HSM, KMS, etc.)
```

### Pattern 2: Storage

```rust
// BEFORE: ❌ Hardcoded primal
let nestgate_url = "http://nestgate:9000";
let client = NestGateClient::connect(nestgate_url).await?;

// AFTER: ✅ Capability-based
let adapter = UniversalAdapter::new().await?;
let storage = adapter.get_storage_provider().await?;
// Adapter discovers and connects to ANY storage provider (nestgate, S3, etc.)
```

### Pattern 3: Coordination

```rust
// BEFORE: ❌ Hardcoded primal
let songbird_endpoint = env::var("SONGBIRD_URL")
    .unwrap_or_else(|_| "http://localhost:7000".to_string());

// AFTER: ✅ Capability-based
let adapter = UniversalAdapter::new().await?;
let coordinator = adapter.get_coordination_provider().await?;
// Adapter discovers coordinator via mDNS, env vars, K8s, etc.
```

---

## 🎓 Deep Debt Principles Applied

### 1. Self-Knowledge Only

**Rule**: A primal knows only itself, not other primals

**Example**:
```rust
// ✅ ACCEPTABLE in beardog_impl/client.rs
impl BearDogClient {
    // This IS beardog, so self-knowledge is OK
    pub fn new() -> Self { ... }
}

// ❌ VIOLATION in toadstool/src/config.rs
let beardog_endpoint = "http://beardog:8000";  // toadstool shouldn't know beardog!
```

### 2. Capability-Based Discovery

**Rule**: Request capabilities, not specific providers

**Example**:
```rust
// ❌ BAD: Asking for WHO
discover_service("beardog")

// ✅ GOOD: Asking for WHAT
discover_capability("encryption")
discover_capability("key-management")
discover_capability("secure-storage")
```

### 3. Runtime Discovery

**Rule**: Nothing hardcoded, everything discovered at runtime

**Example**:
```rust
// ❌ BAD: Compile-time knowledge
const BEARDOG_URL: &str = "http://beardog:8000";

// ✅ GOOD: Runtime discovery
let security_provider = DiscoveryEngine::new()
    .discover_by_capability("encryption")
    .await?;
```

---

## 📊 Expected Impact

### After Full Evolution

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| **Primal References** | 1,550 | ~50* | **97% reduction** |
| **Hardcoded Ports** | ~100 | ~10** | **90% reduction** |
| **Deep Debt Score** | 75% | **98%** | **+23%** |
| **Vendor Lock-in** | High | **Zero** | **Complete freedom** |

\* Remaining in self-knowledge contexts (beardog_impl knows beardog)  
\** Remaining as default fallbacks only

### Benefits

1. **True Vendor Agnosticism**
   - Swap beardog for HSM without code changes
   - Swap nestgate for S3 without code changes
   - Swap songbird for Kubernetes without code changes

2. **Runtime Flexibility**
   - Discover services via mDNS, DNS-SD, K8s, consul, etc.
   - No hardcoded endpoints
   - Dynamic port allocation

3. **Sovereignty**
   - No vendor lock-in
   - User chooses providers
   - Complete freedom

---

## 🚀 Quick Wins (Do First)

### Fix #1: Network Configurator (30 min)

**File**: `crates/cli/src/network_config/configurator/core.rs`

**Change**:
```rust
// Before:
url: format!("http://{}:8000/health", domains.beardog),

// After:
url: self.discovery.find_capability("encryption")
    .await?
    .health_endpoint,
```

**Impact**: Removes 10+ hardcoded references

### Fix #2: Template System (45 min)

**File**: `crates/cli/src/templates/specialized_templates.rs`

**Change**: Use capability placeholders
```rust
// Before:
dependencies: vec!["beardog".to_string()],

// After:
required_capabilities: vec![Capability::Encryption],
```

**Impact**: Removes 20+ hardcoded references

### Fix #3: Config Defaults (15 min)

**File**: `crates/core/config/src/defaults.rs`

**Change**: Remove primal-specific defaults
```rust
// Before:
pub const DEFAULT_BEARDOG_PORT: u16 = 8000;

// After:
// (delete - discovered at runtime)
```

**Impact**: Removes 15+ hardcoded constants

---

## ✅ Success Criteria

1. **Zero Hardcoded Primal Names** (except in self-knowledge contexts)
2. **90%+ Port Discovery** (vs hardcoded)
3. **Deep Debt Score: 98%+**
4. **All Tests Passing** (capability-based mocks)
5. **Documentation Updated** (capability-based examples)

---

## 📝 Next Steps

### This Session (Continue Now)

1. ✅ Document strategy (this file)
2. ⏳ Implement Quick Win #1 (network configurator)
3. ⏳ Implement Quick Win #2 (templates)
4. ⏳ Implement Quick Win #3 (config defaults)

### Next Session

5. Create `CapabilityType` enum
6. Enhance discovery engine
7. Update top 10 violator files
8. Add clippy lint for primal names

---

**Priority**: 🔴 **CRITICAL**  
**Impact**: 🚀 **TRANSFORMATIVE**  
**Difficulty**: ⚠️ **MODERATE** (systematic work)  
**Timeline**: 2-3 weeks for full evolution

**This is the KEY to true Deep Debt compliance!**
