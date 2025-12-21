# Chaos Tests Integration Status

**Date**: December 6, 2025  
**Status**: 🟡 **PARTIAL** - Tests exist but need implementation

---

## Current Situation

### What Exists
- ✅ 6 chaos test files in `tests/chaos/`:
  - `fault_injection.rs` (19KB)
  - `network_failures_month2.rs` (8KB)
  - `real_fault_injection.rs` (17KB)
  - `resilience_tests.rs` (18KB)
  - `resource_exhaustion_month2.rs` (11KB)
  - `timeout_scenarios_month2.rs` (11KB)

### Problem
The tests reference helper functions that don't exist:
```rust
setup_test_nodes()
inject_network_partition()
test_system_operation_during_partition()
// ... etc
```

These are **test stubs** - placeholders for actual chaos engineering infrastructure.

---

## Why This Happened

These appear to be **specification tests** - written to define behavior before implementation. This is actually a GOOD practice (TDD), but they were checked in before the infrastructure was built.

---

## What's Needed

### Option 1: Implement Infrastructure (HIGH effort, 2-3 weeks)
Build the actual chaos engineering framework:
- Node simulator
- Network partition injector
- Fault injection framework
- Recovery validators

**Effort**: 40-60 hours  
**Value**: Full chaos testing capability

### Option 2: Comment Out Stub Tests (LOW effort, 5 minutes)
Add `#[ignore]` or move to `tests/chaos_specs/` directory

**Effort**: 5 minutes  
**Value**: Clean test suite, no false failures

### Option 3: Use Existing Chaos Tools (MEDIUM effort, 1 week)
Integrate with tools like `tokio-test`, `chaos-mesh`, or custom lightweight framework

**Effort**: 20-30 hours  
**Value**: Practical chaos testing

---

## Recommendation

**For Now**: Option 2 - Mark as ignored specs
```rust
#[tokio::test]
#[ignore = "Chaos infrastructure not yet implemented"]
async fn test_network_partition_resilience() {
    // ... test code
}
```

**Later**: Option 3 - Implement with existing tools when expanding coverage

---

## Integration Steps (Future)

1. **Create `crates/testing/src/chaos/`** infrastructure
2. **Implement helper functions**:
   - `setup_test_nodes()`
   - `inject_network_partition()`
   - `inject_resource_pressure()`
   - `simulate_service_failure()`
3. **Add chaos dependencies** to `crates/testing/Cargo.toml`
4. **Remove `#[ignore]`** from tests
5. **Run in CI** with appropriate timeouts

---

**Status**: Documented, moved to backlog  
**Priority**: Medium (valuable but not blocking production)  
**Effort**: 1-3 weeks for full implementation

