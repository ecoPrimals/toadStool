# Hardcoded Primal Names - Evolution Plan

**Date**: January 4, 2026  
**Scope**: 182 files with hardcoded primal names  
**Goal**: Evolve to capability-based discovery (zero hardcoding)

---

## 🎯 Executive Summary

**Problem**: 182 files contain hardcoded primal names ("BearDog", "Songbird", "NestGate")  
**Solution**: Evolve to capability-based discovery using `BiomeOSClient`  
**Status**: BiomeOSClient ready ✅ | Evolution in progress 🔄

---

## 📊 Hardcoded Name Inventory

### grep Results (182 files)

```bash
grep -r "BearDog\|Songbird\|NestGate" crates/ --include="*.rs" | wc -l
# Result: 182 files
```

### Breakdown by Category

| Category | Files | Priority | Effort |
|----------|-------|----------|--------|
| **Executor** | 5-10 | 🔴 Critical | 2-3h |
| **Integration** | 50-60 | 🟡 High | 4-5h |
| **Tests** | 80-90 | 🟢 Medium | 3-4h |
| **Documentation** | 30-40 | 🟢 Low | 1-2h |
| **Total** | **182** | - | **10-14h** |

---

## 🔄 Evolution Strategy

### Phase 1: Core Executor (Critical) 🔴

**Files to Update** (~10 files):
1. `crates/cli/src/executor/executor_impl.rs`
2. `crates/cli/src/ecosystem/integrator_impl.rs`
3. `crates/distributed/src/core/coordinator.rs`
4. `crates/distributed/src/beardog_integration/client.rs`
5. `crates/distributed/src/songbird_integration/discovery.rs`

**Pattern**:
```rust
// ❌ OLD (hardcoded):
let beardog = BearDogClient::new("http://localhost:8081");

// ✅ NEW (capability-based):
let biomeos = BiomeOSClient::connect().await?;
let security = biomeos.get_security_provider().await?;
let beardog = BearDogClient::new(&security.endpoint);
```

**Effort**: 2-3 hours  
**Impact**: 🔴 Critical - Unblocks all other work

---

### Phase 2: Integration Layer (High) 🟡

**Files to Update** (~60 files):
- All files in `crates/distributed/src/beardog_integration/`
- All files in `crates/distributed/src/songbird_integration/`
- All files in `crates/integration/nestgate/`

**Pattern**:
```rust
// ❌ OLD (hardcoded string):
const BEARDOG_ENDPOINT: &str = "http://localhost:8081";

// ✅ NEW (discovered):
// Use BiomeOSClient.get_security_provider() instead
```

**Effort**: 4-5 hours  
**Impact**: 🟡 High - Production code paths

---

### Phase 3: Tests (Medium) 🟢

**Files to Update** (~90 files):
- Integration tests
- Unit tests with hardcoded endpoints

**Pattern**:
```rust
// ❌ OLD (hardcoded in test):
#[tokio::test]
async fn test_beardog_integration() {
    let client = BearDogClient::new("http://localhost:8081");
    // ...
}

// ✅ NEW (mock or real discovery):
#[tokio::test]
async fn test_beardog_integration() {
    // Option 1: Mock BiomeOSClient
    let mock_biomeos = MockBiomeOSClient::new();
    mock_biomeos.register_provider(
        Capability::Authentication(AuthCapability::CryptoOperations),
        PrimalInfo { endpoint: "http://localhost:8081", ... }
    );
    
    // Option 2: Use test fixture
    let security = biomeos.get_security_provider().await?;
    let client = BearDogClient::new(&security.endpoint);
    // ...
}
```

**Effort**: 3-4 hours  
**Impact**: 🟢 Medium - Test quality

---

### Phase 4: Documentation (Low) 🟢

**Files to Update** (~40 files):
- README files
- Code examples
- Integration guides

**Pattern**:
```markdown
<!-- ❌ OLD (hardcoded example): -->
```bash
export BEARDOG_ENDPOINT=http://localhost:8081
```

<!-- ✅ NEW (capability-based): -->
```bash
# No hardcoding needed! ToadStool discovers BearDog via biomeOS:
toadstool run biome.yaml
# (ToadStool queries biomeOS for Security provider)
```
```

**Effort**: 1-2 hours  
**Impact**: 🟢 Low - Documentation only

---

## 🛠️ Implementation Plan

### Step 1: Audit & Categorize (30 min)

```bash
# Generate full inventory
grep -rn "BearDog\|Songbird\|NestGate" crates/ --include="*.rs" > hardcoded_inventory.txt

# Categorize by directory
grep -rn "BearDog\|Songbird\|NestGate" crates/cli/src/executor/ --include="*.rs"
grep -rn "BearDog\|Songbird\|NestGate" crates/distributed/ --include="*.rs"
grep -rn "BearDog\|Songbird\|NestGate" crates/integration/ --include="*.rs"
```

### Step 2: Update Executor (2-3h)

**Priority Files**:
1. `executor_impl.rs` - Add `BiomeOSClient` field to `BiomeExecutor`
2. `integrator_impl.rs` - Update registration to use BiomeOSClient
3. `coordinator.rs` - Evolve primal discovery

**Validation**:
```bash
cargo check --package toadstool-cli
cargo test --package toadstool-cli -- executor
```

### Step 3: Update Integration Layer (4-5h)

**Batch Update**:
- BearDog integration: 20-25 files
- Songbird integration: 20-25 files
- NestGate integration: 10-15 files

**Validation**:
```bash
cargo check --package toadstool-distributed
cargo check --package toadstool-integration-nestgate
cargo test --package toadstool-distributed
```

### Step 4: Update Tests (3-4h)

**Strategy**: Create test helpers

```rust
// NEW: Test helper in crates/testing/src/lib.rs
pub struct MockBiomeOSClient {
    providers: HashMap<String, PrimalInfo>,
}

impl MockBiomeOSClient {
    pub fn with_defaults() -> Self {
        let mut mock = Self::new();
        mock.register_provider(
            Capability::Authentication(AuthCapability::CryptoOperations),
            PrimalInfo {
                name: "beardog".to_string(),
                endpoint: "http://localhost:8081".to_string(),
                // ...
            },
        );
        // ... Songbird, NestGate
        mock
    }
}
```

**Validation**:
```bash
cargo test --workspace
```

### Step 5: Update Documentation (1-2h)

**Files to Update**:
- README.md (root)
- docs/guides/integration-guide.md
- crates/*/README.md
- examples/*.rs

**Validation**: Manual review

---

## 📋 Detailed File List (Top 50 Priority)

### Executor & Core (🔴 Critical)

1. `crates/cli/src/executor/executor_impl.rs` - Main executor (line 369-386, 388-412)
2. `crates/cli/src/ecosystem/integrator_impl.rs` - Registration (line 182-220)
3. `crates/cli/src/main.rs` - CLI entry point
4. `crates/cli/src/lib.rs` - Exports

### Distributed Coordination (🔴 Critical)

5. `crates/distributed/src/core/coordinator.rs` - Primal coordination
6. `crates/distributed/src/beardog_integration/client.rs` - BearDog client
7. `crates/distributed/src/songbird_integration/discovery.rs` - Songbird discovery
8. `crates/distributed/src/primal_capabilities/adapters.rs` - Capability adapters

### Configuration (🟡 High)

9. `crates/core/config/src/config_utils.rs` - Config utils
10. `crates/core/config/src/lib.rs` - Config exports
11. `crates/core/config/src/env_config.rs` - Environment config
12. `crates/core/config/src/defaults.rs` - Default endpoints

### Integration (🟡 High)

13-50. All files in:
- `crates/integration/nestgate/` (10 files)
- `crates/distributed/src/beardog_integration/` (5 files)
- `crates/distributed/src/songbird_integration/` (10 files)
- `crates/integration/protocols/` (5 files)

---

## 🧪 Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_biomeos_client_discovery() {
        let mock = MockBiomeOSClient::with_defaults();
        
        // Test Security provider discovery
        let security = mock.get_security_provider().await.unwrap();
        assert_eq!(security.name, "beardog");
        
        // Test Discovery provider discovery
        let discovery = mock.get_discovery_provider().await.unwrap();
        assert_eq!(discovery.name, "songbird");
    }
}
```

### Integration Tests

```rust
#[tokio::test]
#[ignore = "Requires biomeOS running"]
async fn test_real_biomeos_discovery() {
    // Requires: biomeOS + BearDog + Songbird running
    let biomeos = BiomeOSClient::connect().await.unwrap();
    
    let security = biomeos.get_security_provider().await.unwrap();
    assert!(security.endpoint.contains("beardog"));
    
    let discovery = biomeos.get_discovery_provider().await.unwrap();
    assert!(discovery.endpoint.contains("songbird"));
}
```

---

## 📊 Progress Tracking

### Phase 1: Core Executor

- [ ] Add `BiomeOSClient` to `BiomeExecutor`
- [ ] Update `start_primal()` to use BiomeOSClient
- [ ] Update `integrator_impl.rs` registration
- [ ] Tests pass
- [ ] Commit & push

**Estimated**: 2-3 hours

### Phase 2: Integration Layer

- [ ] BearDog integration (20 files)
- [ ] Songbird integration (20 files)
- [ ] NestGate integration (10 files)
- [ ] Tests pass
- [ ] Commit & push

**Estimated**: 4-5 hours

### Phase 3: Tests

- [ ] Create `MockBiomeOSClient`
- [ ] Update executor tests (20 files)
- [ ] Update integration tests (40 files)
- [ ] Update comprehensive tests (30 files)
- [ ] All tests pass
- [ ] Commit & push

**Estimated**: 3-4 hours

### Phase 4: Documentation

- [ ] Update root README.md
- [ ] Update integration guides
- [ ] Update code examples
- [ ] Update crate READMEs
- [ ] Review & commit

**Estimated**: 1-2 hours

---

## 🎯 Success Criteria

### Functional

- ✅ Zero hardcoded primal names in production code
- ✅ All services discovered by capability
- ✅ Graceful degradation (works without biomeOS)
- ✅ All tests pass

### Quality

- ✅ No compilation warnings
- ✅ Linter clean (cargo clippy)
- ✅ Documentation updated
- ✅ Examples functional

### Performance

- ✅ No performance regression
- ✅ Discovery cached (fast lookups)
- ✅ Lazy connection (no overhead)

---

## 🚀 Timeline

| Phase | Duration | Cumulative |
|-------|----------|------------|
| 1. Core Executor | 2-3h | 2-3h |
| 2. Integration | 4-5h | 6-8h |
| 3. Tests | 3-4h | 9-12h |
| 4. Documentation | 1-2h | 10-14h |
| **Total** | **10-14h** | **~2 days** |

---

**Status**: ✅ BiomeOSClient ready | 🔄 Evolution in progress  
**Next**: Phase 1 - Update core executor (2-3 hours)  
**Goal**: Zero hardcoded primal names by January 5, 2026 🎯

