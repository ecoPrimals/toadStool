# Pure Infant Discovery Evolution - Execution Plan

**Date**: January 4, 2026  
**Status**: Removing last vestiges of hardcoded primal knowledge  
**Goal**: A+ (97/100) - Pure infant discovery architecture

---

## 🎯 Philosophy

**"Code starts with ZERO knowledge, discovers like an infant"**

ToadStool at birth knows:
- ✅ **Self**: Own ports (8084, 8085, 8086, 9090)
- ❌ **Others**: Nothing about BearDog, Songbird, NestGate

---

## 📊 Current Status

### Deprecated Constants (To Remove)

**File**: `crates/core/config/src/defaults.rs` (Lines 96-131)

```rust
// ❌ REMOVE THESE:
#[deprecated] pub const SONGBIRD_PORT: u16 = 8080;
#[deprecated] pub const BEARDOG_PORT: u16 = 8081;
#[deprecated] pub const NESTGATE_PORT: u16 = 8082;
#[deprecated] pub const SQUIRREL_PORT: u16 = 8083;
```

**Usage**: 11 matches across 5 files (mostly tests)

### Deprecated Endpoint Helpers (To Remove)

**File**: `crates/core/config/src/defaults.rs` (Lines 365-411)

```rust
// ❌ REMOVE THESE:
#[deprecated]
pub mod endpoints {
    pub fn songbird() -> String { ... }
    pub fn beardog() -> String { ... }
    pub fn nestgate() -> String { ... }
    pub fn squirrel() -> String { ... }
}
```

---

## 📋 Execution Strategy

### Phase 1: Document Pure Discovery Pattern (30 min)

**Create**: `docs/architecture/INFANT_DISCOVERY.md`

**Content**:
```markdown
# Infant Discovery Architecture

## Philosophy
Code deploys with ZERO knowledge of other primals.
Discovers everything at runtime, like an infant learning.

## 3-Layer Discovery
1. biomeOS Registry (family-level)
2. Songbird (universal adapter)
3. mDNS (zero-config local)

## Usage Example
```rust
// ❌ OLD (hardcoded):
let beardog_port = 8081;
let beardog = connect("localhost", beardog_port);

// ✅ NEW (discovered):
let biomeos = BiomeOSClient::connect().await?;
let security = biomeos.get_security_provider().await?;
let beardog = connect(&security.endpoint);
```

### Phase 2: Update Test Infrastructure (1 hour)

**Strategy**: Tests need mock discovery, not hardcoded ports

**Create**: `crates/testing/src/mocks/discovery.rs`

```rust
#[cfg(test)]
pub struct MockDiscoveryService {
    services: HashMap<Capability, PrimalInfo>,
}

#[cfg(test)]
impl MockDiscoveryService {
    pub fn with_defaults() -> Self {
        let mut services = HashMap::new();
        services.insert(
            Capability::Security,
            PrimalInfo {
                name: "beardog".into(),
                endpoint: "http://localhost:8081".into(),
                capabilities: vec![Capability::Security],
                metadata: HashMap::new(),
            },
        );
        // ... Songbird, NestGate, etc.
        Self { services }
    }
}
```

### Phase 3: Update Call Sites (1 hour)

**Target Files** (11 matches):
1. `crates/core/config/src/config_utils.rs` (4 functions - DEPRECATED)
2. `crates/core/config/src/lib.rs` (4 functions - DEPRECATED)
3. `crates/core/config/src/env_config.rs` (1 usage)
4. `crates/integration/protocols/src/client.rs` (1 usage)
5. `crates/core/toadstool/tests/lib_comprehensive_tests.rs` (1 test)

**Strategy**:
- Functions marked deprecated → Keep but add compile warnings
- Tests → Update to use MockDiscoveryService
- Production code → Update to use RuntimeDiscovery

### Phase 4: Remove Deprecated Constants (30 min)

**File**: `crates/core/config/src/defaults.rs`

**Remove**:
- Lines 96-131: Deprecated primal port constants
- Lines 365-411: Deprecated endpoint helper functions
- Lines 586-605: Test code using deprecated ports

**Keep**:
- Lines 92-147: Self-configuration (API_PORT, METRICS_PORT, etc.)

### Phase 5: Verification (30 min)

**Checks**:
1. ✅ No hardcoded primal ports in production
2. ✅ All tests use MockDiscoveryService
3. ✅ RuntimeDiscovery used everywhere
4. ✅ Only self-knowledge constants remain

---

## 🎯 Migration Path (Backward Compatible)

### Current (Deprecated but works)
```rust
#[allow(deprecated)]
use toadstool_config::defaults::network::BEARDOG_PORT;
let port = BEARDOG_PORT; // 8081
```

### Transition (Environment override)
```bash
BEARDOG_PORT=9081 cargo run
```

### Final (Pure discovery)
```rust
let biomeos = BiomeOSClient::connect().await?;
let security = biomeos.get_security_provider().await?;
// security.endpoint = "http://beardog-prod:9081" (discovered!)
```

---

## 📊 Impact Analysis

### Files to Modify: 5
- `crates/core/config/src/defaults.rs` (remove deprecated)
- `crates/core/config/src/config_utils.rs` (deprecate functions)
- `crates/core/config/src/lib.rs` (deprecate functions)
- `crates/core/config/src/env_config.rs` (update to discovery)
- `crates/testing/src/mocks/discovery.rs` (NEW - test infra)

### Tests to Update: ~10
- Replace hardcoded ports with MockDiscoveryService
- Update assertions for discovered endpoints

### Breaking Changes: NONE
- Deprecated functions remain (with warnings)
- Environment variables still work
- Graceful fallback chain intact

---

## 🏆 Success Criteria

### Code Quality
- [ ] No hardcoded primal ports in production
- [ ] All discovery via RuntimeDiscovery/BiomeOSClient
- [ ] Tests use MockDiscoveryService
- [ ] Deprecated constants removed
- [ ] Documentation updated

### Philosophy Validation
- [ ] ToadStool knows only itself ✅
- [ ] Discovers others by capability ✅
- [ ] 3-layer discovery working ✅
- [ ] Universal adapter (Songbird) ✅
- [ ] Zero 2^n connections ✅

### Grade Impact
- Current: A+ (94/100)
- Target: A+ (97/100)
- Improvement: +3 points

---

## ⏱️ Timeline

**Total**: 3.5-4 hours

- Phase 1 (Documentation): 30 min
- Phase 2 (Test Infrastructure): 1 hour
- Phase 3 (Update Call Sites): 1 hour
- Phase 4 (Remove Deprecated): 30 min
- Phase 5 (Verification): 30 min

---

## 🚀 Execution Order

1. **Document first** - Establish pattern
2. **Test infrastructure** - MockDiscoveryService
3. **Update call sites** - Production code
4. **Remove deprecated** - Clean up
5. **Verify** - Ensure pure discovery

---

**Status**: Ready to execute ✅  
**Risk**: Low (backward compatible) ✅  
**Impact**: High (pure infant discovery) ✅

