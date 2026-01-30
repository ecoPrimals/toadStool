# 📊 Test Coverage Expansion Plan

**Date**: January 29, 2026  
**Status**: Ready for Implementation  
**Estimated Effort**: 2-3 hours  
**Priority**: Medium (Quality assurance, non-blocking)  
**Current Status**: llvm-cov installed, compilation fixes applied

---

## 📈 Coverage Goal

**Target**: 90% line coverage across workspace  
**Strategy**: Focus on high-value, production code first  
**Tools**: `cargo-llvm-cov` for measurement, `criterion` for benchmarks

---

## 🎯 Current State

### Tools Installed
- ✅ `cargo-llvm-cov` v0.8.2 - Coverage measurement
- ✅ `criterion` - Benchmarking framework
- ✅ Test framework in place (tokio-test, assert_cmd, etc.)

### Compilation Status
- ✅ Fixed: `CryptoCapability` import in tests
- ✅ Fixed: `f32::EPSILON` vs `f64::EPSILON` in akida tests
- ⏳ Next: Run full coverage measurement

---

## 📊 Coverage Measurement Strategy

### Step 1: Baseline Measurement (15 minutes)

```bash
# Generate HTML coverage report
cargo llvm-cov --workspace --html --ignore-filename-regex '(tests|benches|examples)/'

# Open report
open target/llvm-cov/html/index.html  # macOS
xdg-open target/llvm-cov/html/index.html  # Linux

# Generate summary
cargo llvm-cov --workspace --summary-only
```

### Step 2: Identify Gaps (30 minutes)

**Priority Areas** (Production Code):
1. **Core Modules** (High Priority)
   - `crates/core/common/src/` - Shared utilities
   - `crates/core/toadstool/src/` - Main application logic
   - `crates/cli/src/daemon/` - Daemon server

2. **Integration Modules** (High Priority)
   - `crates/core/toadstool/src/biomeos_integration/` - BiomeOS backends
   - `crates/distributed/src/` - Distributed coordination
   - `crates/core/common/src/capability_provider.rs` - New capability system

3. **Runtime Modules** (Medium Priority)
   - `crates/runtime/wasm/src/` - WASM execution
   - `crates/runtime/gpu/src/` - GPU operations
   - `crates/runtime/secure_enclave/src/` - Security

4. **Neuromorphic** (Medium Priority)
   - `crates/neuromorphic/akida-driver/` - Hardware driver
   - `crates/neuromorphic/akida-models/` - Model parsing

**Exclude from Coverage**:
- Examples (`examples/`, `*/examples/`)
- Benchmarks (`benches/`, `*/benches/`)
- Tests themselves (`tests/`, `*/tests/`)
- Generated code (`target/`)

### Step 3: Write Missing Tests (60-90 minutes)

**Test Categories to Add**:

#### 3.1 Unit Tests (Core Logic)

```rust
// Example: Test new CapabilityProvider
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_capability_discovery_integration() {
        // Mock discovery service
        let discovery = MockDiscoveryService::new();
        discovery.register_service("security", vec![
            Capability::Crypto(CryptoCapability::Encryption)
        ]);
        
        // Discover by capability
        let provider = CapabilityProvider::discover(
            Capability::Crypto(CryptoCapability::Encryption)
        ).await.unwrap();
        
        assert_eq!(provider.service_name(), "security");
    }
    
    #[tokio::test]
    async fn test_capability_provider_caching() {
        // Test that subsequent calls use cache
        let provider1 = CapabilityProvider::discover(capability).await?;
        let provider2 = CapabilityProvider::discover(capability).await?;
        assert_eq!(provider1.service_name(), provider2.service_name());
    }
    
    #[tokio::test]
    async fn test_capability_provider_error_handling() {
        // Test error paths
        let result = CapabilityProvider::discover(
            Capability::Custom { name: "nonexistent".into(), version: "1.0".into() }
        ).await;
        assert!(result.is_err());
    }
}
```

#### 3.2 Integration Tests (Cross-Module)

```rust
// tests/integration/capability_discovery.rs
#[tokio::test]
async fn test_end_to_end_capability_discovery() {
    // Start mock services
    let _security = start_mock_security_service().await;
    let _storage = start_mock_storage_service().await;
    let _discovery = start_mock_discovery_service().await;
    
    // Test evolved backends can find services
    let auth_backend = AuthBackendEvolved::new();
    auth_backend.initialize().await.unwrap();
    
    let storage_backend = StorageBackendEvolved::new();
    storage_backend.initialize().await.unwrap();
    
    // Verify they work
    let token = auth_backend.request_token(...).await.unwrap();
    let volume = storage_backend.provision_volume(...).await.unwrap();
    
    assert!(!token.is_empty());
    assert_eq!(volume.status, VolumeStatus::Available);
}
```

#### 3.3 Error Path Tests

```rust
#[tokio::test]
async fn test_capability_provider_network_failure() {
    // Simulate network failure
    let result = CapabilityProvider::discover_with_timeout(
        capability,
        Duration::from_millis(100)
    ).await;
    
    assert!(matches!(result, Err(CapabilityError::Timeout)));
}

#[tokio::test]
async fn test_json_rpc_invalid_response() {
    // Test invalid JSON response handling
    let client = UnixJsonRpcClient::new("/tmp/invalid.sock");
    let result = client.call("method", json!({})).await;
    
    assert!(result.is_err());
}
```

#### 3.4 Edge Case Tests

```rust
#[test]
fn test_capability_serialization_edge_cases() {
    // Empty capability names
    let cap = Capability::Custom { name: "".into(), version: "1.0".into() };
    let s = capability_to_string(&cap);
    assert_eq!(s, "");
    
    // Unicode in capability names
    let cap = Capability::Custom { name: "🦀".into(), version: "1.0".into() };
    let s = capability_to_string(&cap);
    assert_eq!(s, "🦀");
    
    // Very long names
    let long_name = "a".repeat(1000);
    let cap = Capability::Custom { name: long_name.clone(), version: "1.0".into() };
    let s = capability_to_string(&cap);
    assert_eq!(s.len(), 1000);
}
```

### Step 4: Add Property-Based Tests (30 minutes)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_capability_serialization_roundtrip(
        name in "[a-z]{1,20}",
        version in "[0-9]\\.[0-9]"
    ) {
        let cap = Capability::Custom { name: name.clone(), version };
        let serialized = capability_to_string(&cap);
        let deserialized = string_to_capability(&serialized);
        
        assert_eq!(capability_to_string(&deserialized), name);
    }
}
```

---

## 📋 Coverage Checklist by Module

### Core Common (`crates/core/common/`)

**Current Coverage**: Unknown (Baseline needed)  
**Target**: 90%

**Priority Tests to Add**:
- [ ] `capability_provider.rs` - Full discovery flow with mocks
- [ ] `primal_identity.rs` - Identity management tests
- [ ] `service_discovery.rs` - Service registration/lookup
- [ ] `unix_jsonrpc_client.rs` - RPC client error paths
- [ ] `auth.rs` - Credential handling edge cases

### Core ToadStool (`crates/core/toadstool/`)

**Current Coverage**: Unknown  
**Target**: 85% (some legacy code may be skipped)

**Priority Tests to Add**:
- [ ] `biomeos_integration/auth_backend_evolved.rs` - Integration tests
- [ ] `biomeos_integration/storage_backend_evolved.rs` - Integration tests
- [ ] `biomeos_integration/agent_backend_evolved.rs` - Integration tests
- [ ] `composition_engine.rs` - Composition logic
- [ ] `multi_workload_compositor.rs` - Multi-workload scenarios
- [ ] `resources.rs` - Resource allocation

### CLI Daemon (`crates/cli/src/daemon/`)

**Current Coverage**: Unknown  
**Target**: 80% (integration-heavy, some test complexity)

**Priority Tests to Add**:
- [ ] `jsonrpc_server.rs` - RPC method handlers
- [ ] `server.rs` - Daemon lifecycle
- [ ] `http_server.rs` - Backward compatibility (if kept)
- [ ] Integration test: Full daemon startup/shutdown

### Distributed (`crates/distributed/`)

**Current Coverage**: Unknown  
**Target**: 85%

**Priority Tests to Add**:
- [ ] `beardog_integration/client_evolved.rs` - Client discovery
- [ ] `crypto_integration/client.rs` - Crypto operations
- [ ] Fallback/retry logic tests
- [ ] Multi-provider scenarios

### Runtime Modules

**Current Coverage**: Likely 60-70% (good test coverage already)  
**Target**: 85%

**Priority Tests to Add**:
- [ ] `wasm/` - Error path tests
- [ ] `gpu/` - Hardware abstraction tests
- [ ] `secure_enclave/` - Security boundary tests

### Neuromorphic

**Current Coverage**: Good (Phase 4 complete)  
**Target**: 90%

**Priority Tests to Add**:
- [ ] `akida-driver/` - Error recovery tests
- [ ] `akida-models/` - Malformed model handling

---

## 🚀 Implementation Plan

### Phase 1: Baseline & Gaps (30 minutes)

```bash
# 1. Run coverage measurement
cargo llvm-cov --workspace --html \
    --ignore-filename-regex '(tests|benches|examples)/'

# 2. Generate summary report
cargo llvm-cov --workspace --summary-only > coverage_baseline.txt

# 3. Identify low-coverage files
grep -E "^[^[:space:]].*[0-9]+\.[0-9]+%" coverage_baseline.txt | \
    awk '{if ($NF < 80.0) print}' | \
    sort -k2 -n
```

### Phase 2: Write High-Priority Tests (60 minutes)

**Focus Order**:
1. `capability_provider.rs` - NEW, zero coverage
2. `*_evolved.rs` backends - NEW, zero coverage
3. `jsonrpc_server.rs` - NEW, zero coverage
4. Core integration flows

**Test Template**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Happy path
    #[tokio::test]
    async fn test_feature_works() { /* ... */ }
    
    // Error paths
    #[tokio::test]
    async fn test_feature_handles_error() { /* ... */ }
    
    // Edge cases
    #[test]
    fn test_feature_edge_cases() { /* ... */ }
    
    // Integration
    #[tokio::test]
    async fn test_feature_integration() { /* ... */ }
}
```

### Phase 3: Medium-Priority Tests (45 minutes)

**Focus**: Bring existing modules from 70% → 90%

1. Add error path tests for existing modules
2. Add edge case tests
3. Add integration scenarios

### Phase 4: Verify & Document (15 minutes)

```bash
# 1. Re-run coverage
cargo llvm-cov --workspace --html \
    --ignore-filename-regex '(tests|benches|examples)/'

# 2. Compare before/after
diff coverage_baseline.txt coverage_after.txt

# 3. Document results
cat > COVERAGE_RESULTS_JAN29_2026.md <<EOF
# Test Coverage Results

## Baseline
- Total Coverage: X%
- Core Modules: Y%
- New Code: 0%

## After Expansion
- Total Coverage: X+N%
- Core Modules: Y+M%
- New Code: Z%

## Remaining Gaps
- [List modules < 90%]

## Next Steps
- [Prioritized list]
EOF
```

---

## 📊 Expected Results

### Coverage Targets by Module

| Module | Before | After | Target |
|--------|--------|-------|--------|
| **core/common** | Unknown | 85% | 90% |
| **core/toadstool** | Unknown | 80% | 85% |
| **cli/daemon** | Unknown | 75% | 80% |
| **distributed** | Unknown | 80% | 85% |
| **runtime** | ~70% | 85% | 85% |
| **neuromorphic** | ~85% | 90% | 90% |
| **Overall** | ~65% | 85% | 90% |

### Test Count Increase

| Category | Before | After | Increase |
|----------|--------|-------|----------|
| **Unit Tests** | ~500 | ~750 | +50% |
| **Integration Tests** | ~50 | ~100 | +100% |
| **Property Tests** | 0 | ~20 | NEW! |
| **Total** | ~550 | ~870 | +58% |

---

## 🎓 Testing Best Practices

### 1. Test Organization

```rust
// tests/
//   unit/           - Pure unit tests
//   integration/    - Cross-module tests
//   e2e/            - End-to-end scenarios
//   common/         - Test utilities
//     mod.rs        - Shared test helpers
//     mocks.rs      - Mock implementations
```

### 2. Mock Strategy

```rust
// Use dependency injection for testability
#[async_trait]
pub trait StorageBackend {
    async fn provision_volume(&self, config: &VolumeConfig) -> Result<VolumeInfo>;
}

// Production
pub struct NestGateBackend { /* ... */ }

// Testing
pub struct MockStorageBackend {
    volumes: Arc<Mutex<HashMap<String, VolumeInfo>>>,
}
```

### 3. Test Data Builders

```rust
// tests/common/builders.rs
pub struct VolumeConfigBuilder {
    name: String,
    size: usize,
    // ...
}

impl VolumeConfigBuilder {
    pub fn new() -> Self {
        Self {
            name: "test-volume".into(),
            size: 1024 * 1024 * 1024,  // 1 GB default
        }
    }
    
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
    
    pub fn size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }
    
    pub fn build(self) -> VolumeConfig {
        VolumeConfig { /* ... */ }
    }
}

// Usage
let config = VolumeConfigBuilder::new()
    .name("my-volume")
    .size(5 * GB)
    .build();
```

### 4. Async Test Utilities

```rust
// tests/common/test_utils.rs
pub async fn with_timeout<F, T>(duration: Duration, fut: F) -> Result<T>
where
    F: Future<Output = T>,
{
    tokio::time::timeout(duration, fut)
        .await
        .map_err(|_| anyhow!("Test timeout"))
}

// Usage
#[tokio::test]
async fn test_slow_operation() {
    let result = with_timeout(Duration::from_secs(5), async {
        slow_operation().await
    }).await.unwrap();
}
```

---

## 🚧 Known Challenges

### 1. Integration Test Complexity

**Challenge**: Testing capability discovery requires running multiple services  
**Solution**: Use mock services with in-memory transport

```rust
// Start lightweight mock services
let security_mock = MockService::builder()
    .name("security")
    .capability(Capability::Crypto(CryptoCapability::Encryption))
    .method("security.request_token", |params| {
        Ok(json!({"token": "test-token"}))
    })
    .start().await?;
```

### 2. Async Test Flakiness

**Challenge**: Timing-dependent tests can be flaky  
**Solution**: Use deterministic test execution

```rust
// Bad: Real time delays
tokio::time::sleep(Duration::from_millis(100)).await;

// Good: Manual time control
let mut time = tokio::time::pause();
time.advance(Duration::from_millis(100)).await;
```

### 3. Hardware-Dependent Tests

**Challenge**: GPU/NPU tests require specific hardware  
**Solution**: Feature-gated with fallbacks

```rust
#[cfg(feature = "gpu-tests")]
#[tokio::test]
async fn test_gpu_operation() {
    if !has_gpu() {
        eprintln!("Skipping GPU test: No GPU available");
        return;
    }
    // ... actual test
}
```

---

## 📝 Documentation Updates

After achieving 90% coverage:

1. **Update STATUS.md**
   - Add coverage badge
   - Document test strategy

2. **Update TESTING.md**
   - Add coverage instructions
   - Document test organization

3. **Update CI/CD**
   - Add coverage gates
   - Generate coverage reports on PRs

---

## 🎯 Success Criteria

| Metric | Target | Stretch |
|--------|--------|---------|
| **Overall Coverage** | 85% | 90% |
| **Core Modules** | 90% | 95% |
| **New Code (Evolved)** | 90% | 95% |
| **Integration Tests** | +50 tests | +100 tests |
| **Test Execution Time** | < 5 min | < 3 min |
| **All Tests Pass** | ✅ | ✅ |

---

## 📖 References

- [cargo-llvm-cov Documentation](https://github.com/taiki-e/cargo-llvm-cov)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [tokio Testing Guide](https://tokio.rs/tokio/topics/testing)
- [proptest Documentation](https://docs.rs/proptest/)

---

**Status**: ✅ **PLAN COMPLETE**  
**Ready for**: Implementation (2-3 hours)  
**Priority**: Medium (Quality assurance)  
**Blocking**: No (production-ready as-is)

**Current State**: llvm-cov installed, compilation fixes applied  
**Next Step**: Run baseline coverage measurement

🦀🧬 **ToadStool - Test Coverage Plan Ready!** 🧬🦀
