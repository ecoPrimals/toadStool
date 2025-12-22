# 🍄 ToadStool - Priority Action Plan
**Date**: December 20, 2025  
**Current Grade**: A- (90.6/100)  
**Target Grade**: A+ (98/100)  
**Timeline**: 3-4 weeks

---

## 🔴 CRITICAL PRIORITIES (This Week)

### 1. Establish Performance Baseline ⚡
**Priority**: 🔴 **CRITICAL**  
**Effort**: 1-2 days  
**Impact**: HIGH

**Current State**:
- Benchmarks defined but not executed
- No baseline metrics documented
- No performance regression detection

**Action Items**:
```bash
# 1. Run benchmarks
cd /home/eastgate/Development/ecoPrimals/toadstool
cargo bench --bench hot_paths > benchmarks_baseline_$(date +%Y%m%d).txt
cargo bench --bench baseline_benchmarks >> benchmarks_baseline_$(date +%Y%m%d).txt

# 2. Document results
cat benchmarks_baseline_*.txt >> docs/PERFORMANCE_BASELINE.md

# 3. Add to CI
# Add cargo bench to .github/workflows/ (if using GitHub Actions)
```

**Success Criteria**:
- ✅ Baseline metrics documented
- ✅ Performance regression tests in CI
- ✅ Optimization targets identified

---

### 2. Update Security Dependencies 🔒
**Priority**: 🔴 **CRITICAL**  
**Effort**: 1 hour  
**Impact**: MEDIUM

**Current State**:
- 1 LOW-severity vulnerability: `idna 0.4.0`
- Affects `validator 0.16.1` → `toadstool-api`, `toadstool`

**Action Items**:
```bash
# 1. Update validator (pulls in idna >= 1.0.0)
cargo update -p validator

# 2. Verify fix
cargo audit

# 3. Test
cargo test --workspace

# 4. Commit
git add Cargo.lock
git commit -m "security: update validator to fix idna vulnerability (RUSTSEC-2024-0421)"
```

**Success Criteria**:
- ✅ `cargo audit` shows 0 vulnerabilities
- ✅ All tests still pass
- ✅ Cargo.lock updated

---

### 3. Add 15-20 Targeted Unit Tests 🧪
**Priority**: 🔴 **CRITICAL**  
**Effort**: 2-3 days  
**Impact**: HIGH

**Current State**:
- 44.77% line coverage (need 90%)
- API handlers: 0% coverage
- Some core modules: 30-40% coverage

**Target Modules** (from llvm-cov analysis):

#### 3a. API Handlers (0% → 60%)
```rust
// crates/api/src/handlers/cluster.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_list_clusters() { /* ... */ }
    
    #[tokio::test]
    async fn test_create_cluster() { /* ... */ }
    
    #[tokio::test]
    async fn test_cluster_error_handling() { /* ... */ }
}
```

Target files:
- `api/src/handlers/cluster.rs` (+3 tests)
- `api/src/handlers/execution.rs` (+3 tests)
- `api/src/handlers/health.rs` (+2 tests)
- `api/src/handlers/metrics.rs` (+3 tests)

#### 3b. Runtime Modules (35% → 60%)
```rust
// crates/runtime/gpu/src/backends/cuda_impl.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_cuda_device_detection() { /* ... */ }
    
    #[test]
    fn test_cuda_memory_allocation() { /* ... */ }
    
    #[test]
    fn test_cuda_error_handling() { /* ... */ }
}
```

Target files:
- `runtime/gpu/src/backends/cuda_impl.rs` (+3 tests)
- `runtime/wasm/src/component_model.rs` (+3 tests)
- `runtime/wasm/src/cache.rs` (+2 tests)

**Success Criteria**:
- ✅ 15-20 new tests added
- ✅ All tests pass
- ✅ Coverage: 45% → 55%

---

## 🟡 HIGH PRIORITIES (Next 2 Weeks)

### 4. Test Coverage Campaign 📊
**Priority**: 🟡 **HIGH**  
**Effort**: 2 weeks  
**Impact**: HIGH

**Week 1 Goals** (45% → 65%):

**Day 1-2**: Core Modules
- `crates/core/common/src/error.rs` (current: ~45%, target: 75%)
- `crates/core/config/src/lib.rs` (current: ~57%, target: 80%)
- Add 20-30 tests

**Day 3-4**: Runtime Engines
- `crates/runtime/gpu/src/engine.rs` (current: ~40%, target: 70%)
- `crates/runtime/wasm/src/engine.rs` (current: ~45%, target: 75%)
- Add 15-20 tests

**Day 5**: Integration Tests
- Add 5-10 cross-module integration tests
- Focus on error paths and edge cases

**Week 2 Goals** (65% → 80%):

**Day 1-2**: API Layer
- All handlers (0% → 60%)
- Middleware (0% → 50%)
- WebSocket (1.55% → 50%)
- Add 30-40 tests

**Day 3-4**: Distributed System
- Coordinator (current: ~50%, target: 80%)
- Tower management (current: ~40%, target: 75%)
- Add 15-20 tests

**Day 5**: Property-Based Tests
- Add 5-10 property tests for critical modules
- Use existing property test infrastructure

**Success Criteria**:
- ✅ Coverage: 45% → 80%
- ✅ All new tests pass
- ✅ No regression in existing tests

---

### 5. Inter-Primal Showcase Polish 🎭
**Priority**: 🟡 **HIGH**  
**Effort**: 3-5 days  
**Impact**: MEDIUM

**Current State**:
- Basic showcases exist
- Need polish and documentation
- Missing some integration scenarios

**Action Items**:

**Day 1**: BearDog Integration
```bash
# Polish encrypted workload demo
cd showcase/inter-primal/beardog/
# Add comprehensive README
# Add error handling demonstrations
# Add performance metrics
```

**Day 2**: NestGate Integration
```bash
# Polish persistent storage demo
cd showcase/inter-primal/nestgate/
# Add data lifecycle demonstrations
# Add recovery scenarios
# Add scaling examples
```

**Day 3**: Songbird Coordination
```bash
# Polish distributed coordination demo
cd showcase/inter-primal/songbird/
# Add multi-tower scenarios
# Add failover demonstrations
# Add load balancing examples
```

**Day 4-5**: Documentation & Video
- Write comprehensive README for each showcase
- Create architecture diagrams
- Record 5-minute demo video for each
- Add troubleshooting sections

**Success Criteria**:
- ✅ 3 polished inter-primal demos
- ✅ Complete documentation for each
- ✅ Video walkthroughs
- ✅ Working out-of-the-box

---

## 🟢 MEDIUM PRIORITIES (Weeks 3-4)

### 6. Performance Optimization ⚡
**Priority**: 🟢 **MEDIUM**  
**Effort**: 1 week  
**Impact**: MEDIUM

**Optimization Targets** (from analysis):

1. **String Allocations** (5-10% improvement)
   - 13,896 `to_string()` calls
   - Use `Cow<str>` pattern where appropriate
   - Use `&str` instead of `String` in hot paths

2. **Clone Operations** (memory efficiency)
   - 2,343 clone calls
   - ~200 potentially avoidable
   - Use `Arc<T>` for shared data in async contexts

3. **Network Buffers** (10-15% throughput)
   - Zero-copy serialization opportunities
   - Use `bytes` crate for buffer management
   - Avoid intermediate allocations

**Action Plan**:

**Day 1-2**: Profile & Identify
```bash
# Profile hot paths
cargo flamegraph --bench hot_paths

# Identify allocation hotspots
cargo build --release
perf record -g target/release/toadstool-server
perf report
```

**Day 3-4**: Optimize Strings
```rust
// Before
fn process(s: String) -> String {
    s.to_uppercase()
}

// After
fn process(s: &str) -> Cow<str> {
    if needs_allocation(s) {
        Cow::Owned(s.to_uppercase())
    } else {
        Cow::Borrowed(s)
    }
}
```

**Day 5**: Optimize Clones
```rust
// Before
let data = expensive_data.clone();
tokio::spawn(async move {
    process(data).await;
});

// After
let data = Arc::new(expensive_data);
let data_ref = Arc::clone(&data);
tokio::spawn(async move {
    process(&data_ref).await;
});
```

**Success Criteria**:
- ✅ 5-10% performance improvement
- ✅ Reduced allocations in hot paths
- ✅ Benchmarks show improvement
- ✅ No functionality regressions

---

### 7. Phase 4 Discovery (mDNS/DNS-SD) 🔍
**Priority**: 🟢 **MEDIUM**  
**Effort**: 1 week  
**Impact**: MEDIUM

**Current State**:
- Fallback ports in `crates/core/config/src/ports.rs`
- Violates self-knowledge principle
- Environment variables work but not ideal

**Action Plan**:

**Day 1-2**: mDNS Implementation
```rust
// crates/core/config/src/discovery/mdns.rs
use mdns_sd::{ServiceDaemon, ServiceInfo};

pub async fn discover_primal(
    capability: &str,
) -> Result<Endpoint, DiscoveryError> {
    let mdns = ServiceDaemon::new()?;
    
    // Discover service by capability
    let services = mdns
        .browse(&format!("_{}._tcp.local.", capability))?
        .timeout(Duration::from_secs(5))
        .collect::<Vec<_>>()
        .await;
    
    // Return first available service
    services.first()
        .map(|s| Endpoint::from_service(s))
        .ok_or(DiscoveryError::NotFound)
}
```

**Day 3**: DNS-SD Support
```rust
// Add DNS-SD fallback
pub async fn discover_via_dns_sd(
    capability: &str,
) -> Result<Endpoint, DiscoveryError> {
    // Query DNS for SRV records
    // Return endpoint if found
}
```

**Day 4**: Integration
```rust
// Update discovery.rs
pub async fn discover(capability: &str) -> Result<Endpoint> {
    // 1. Try environment variable
    if let Some(endpoint) = env_override(capability) {
        return Ok(endpoint);
    }
    
    // 2. Try mDNS
    if let Ok(endpoint) = discover_via_mdns(capability).await {
        return Ok(endpoint);
    }
    
    // 3. Try DNS-SD
    if let Ok(endpoint) = discover_via_dns_sd(capability).await {
        return Ok(endpoint);
    }
    
    // 4. ERROR (no fallback!)
    Err(DiscoveryError::NotFound(capability))
}
```

**Day 5**: Remove Fallbacks
```rust
// Delete crates/core/config/src/ports.rs::fallback module
// Update all references to use discovery
// Update documentation
```

**Success Criteria**:
- ✅ mDNS discovery working
- ✅ DNS-SD support added
- ✅ Fallback ports removed
- ✅ 100% self-knowledge achieved
- ✅ All tests pass

---

### 8. Security Hardening 🔐
**Priority**: 🟢 **MEDIUM**  
**Effort**: 1 week  
**Impact**: MEDIUM

**Action Items**:

**Day 1**: Dependency Policy
```toml
# Add deny.toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"
unsound = "deny"

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
]

[bans]
multiple-versions = "warn"
```

**Day 2-3**: Security Testing
```bash
# Penetration testing
# - SQL injection (if applicable)
# - Command injection
# - Path traversal
# - Resource exhaustion
# - Authentication bypass

# Add security tests to test suite
```

**Day 4**: Documentation
```markdown
# docs/SECURITY.md
- Vulnerability reporting process
- Security update policy
- Supported versions
- Known issues and mitigations
```

**Day 5**: CI Integration
```yaml
# .github/workflows/security.yml
- cargo audit
- cargo deny check
- dependency review action
```

**Success Criteria**:
- ✅ `cargo deny` configured
- ✅ Security tests added
- ✅ SECURITY.md complete
- ✅ CI pipeline includes security checks

---

## 📈 WEEK-BY-WEEK ROADMAP

### Week 1: Foundation
**Goal**: A- → A (92/100)

- ✅ Day 1: Performance baseline + Security update
- ✅ Day 2-4: Add 15-20 unit tests (45% → 55%)
- ✅ Day 5: Documentation updates

**Expected Outcome**: A- maintained, foundation solid

---

### Week 2: Coverage Push
**Goal**: A (92/100) → A (94/100)

- ✅ Day 1-4: Test coverage campaign (55% → 75%)
- ✅ Day 5: Inter-primal showcase polish (start)

**Expected Outcome**: A grade achieved

---

### Week 3: Integration & Polish
**Goal**: A (94/100) → A (96/100)

- ✅ Day 1-3: Inter-primal showcases (complete)
- ✅ Day 4-5: Performance optimization

**Expected Outcome**: A grade solidified

---

### Week 4: Final Push
**Goal**: A (96/100) → A+ (98/100)

- ✅ Day 1-3: Phase 4 discovery implementation
- ✅ Day 4-5: Security hardening + final polish

**Expected Outcome**: A+ achieved

---

## 🎯 SUCCESS METRICS

### Technical Metrics

| Metric | Current | Week 1 | Week 2 | Week 3 | Week 4 |
|--------|---------|--------|--------|--------|--------|
| **Overall Grade** | A- (90/100) | A- (92/100) | A (94/100) | A (96/100) | A+ (98/100) |
| **Test Coverage** | 45% | 55% | 75% | 85% | 90% |
| **Clippy Warnings** | 0 | 0 | 0 | 0 | 0 |
| **Security Vulns** | 1 LOW | 0 | 0 | 0 | 0 |
| **Performance** | ? | Baseline | Optimized | Optimized | Tuned |

### Quality Gates

**Week 1 Gate**:
- [ ] Performance baseline established ✓
- [ ] Security vulnerabilities = 0 ✓
- [ ] Test coverage >= 55% ✓

**Week 2 Gate**:
- [ ] Test coverage >= 75% ✓
- [ ] API handlers >= 60% coverage ✓
- [ ] All clippy checks pass ✓

**Week 3 Gate**:
- [ ] Test coverage >= 85% ✓
- [ ] All showcases polished ✓
- [ ] Performance improvements measured ✓

**Week 4 Gate**:
- [ ] Test coverage >= 90% ✓
- [ ] Phase 4 discovery working ✓
- [ ] Security hardening complete ✓
- [ ] A+ grade achieved ✓

---

## 🚨 BLOCKERS & RISKS

### Potential Blockers

1. **API Handler Testing**
   - Issue: Requires async test setup
   - Mitigation: Use `tokio::test` macro
   - Workaround: Test utilities in `crates/testing/`

2. **mDNS Dependencies**
   - Issue: Platform-specific networking
   - Mitigation: Test on multiple platforms
   - Workaround: Keep env var fallback during transition

3. **Performance Regression**
   - Issue: Optimizations might introduce bugs
   - Mitigation: Comprehensive benchmarks
   - Workaround: Feature flag optimizations

### Risk Management

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Coverage target too aggressive | MEDIUM | LOW | Aim for 85% if 90% unrealistic |
| Performance optimization breaks tests | HIGH | MEDIUM | Benchmark before/after each change |
| mDNS platform incompatibility | MEDIUM | MEDIUM | Keep env var fallback permanently |
| Time estimate too optimistic | LOW | MEDIUM | Prioritize ruthlessly |

---

## 📞 DAILY CHECKLIST

### Every Day
```bash
# Start of day
git pull origin main
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings

# End of day
cargo test --workspace
cargo llvm-cov --workspace --lib --summary-only
git add .
git commit -m "..."
git push origin feature-branch
```

### Every Week
```bash
# Monday morning
# Review progress
# Adjust priorities
# Update STATUS.md

# Friday afternoon
# Run full test suite
# Update documentation
# Prepare weekly report
```

---

## 🎉 COMPLETION CRITERIA

### A+ Grade (98/100) Achieved When:

- [x] All critical priorities complete
- [x] Test coverage >= 90%
- [x] 0 security vulnerabilities
- [x] Performance baseline established
- [x] Phase 4 discovery working
- [x] All showcases polished
- [x] Documentation complete
- [x] CI/CD includes all checks
- [x] 0 clippy warnings
- [x] All tests pass

---

**For questions or clarification, refer to**:
- Full Audit: `COMPREHENSIVE_AUDIT_REPORT_DEC_20_2025_EXTENDED.md`
- Quick Summary: `AUDIT_QUICK_SUMMARY_DEC_20_2025.md`
- Current Status: `STATUS.md`

🍄 **Let's achieve A+ together!**

