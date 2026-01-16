# Pure Rust Migration Scope Analysis - January 16, 2026

**Discovery**: Larger scope than initially assessed  
**New Estimate**: 12-16 hours (more realistic)  
**Approach**: Phased migration with incremental testing

---

## 📊 ACTUAL SCOPE

### **Files with reqwest Usage**

**BearDog Integration** (1 large file):
- `crates/distributed/src/beardog_integration/client.rs` - **739 lines**
  - Multiple API endpoints (encrypt, decrypt, sign, verify, etc.)
  - All using HTTP + JSON
  - Needs JSON-RPC over unix socket client

**Songbird Integration** (7 files):
- `crates/distributed/src/songbird_integration/*.rs`
  - May already support unix sockets (needs verification)

**BiomeOS Integration** (3 files):
- `crates/core/toadstool/src/biomeos_integration/storage_backend.rs`
- `crates/core/toadstool/src/biomeos_integration/agent_backend.rs`
- `crates/core/toadstool/src/biomeos_integration/auth_backend.rs`
  - HTTP clients for NUCLEUS
  - Needs conversion to unix sockets

**Ecosystem Communication** (2 files):
- `crates/core/toadstool/src/ecosystem/types.rs` - ServiceClient enum
- `crates/core/toadstool/src/ecosystem/communication.rs` - Client creation

**Discovery** (2 files):
- `crates/core/common/src/infant_discovery/sources.rs`
- `crates/core/common/src/infant_discovery/detectors.rs`

**Other Integration** (5+ files):
- Various coordination, capabilities, etc.

---

## 🎯 REVISED STRATEGY

### **Phase 1: Create Unix Socket JSON-RPC Client** (3 hours)

**Goal**: Generic JSON-RPC client over unix sockets

**Pattern**:
```rust
pub struct UnixJsonRpcClient {
    socket_path: PathBuf,
}

impl UnixJsonRpcClient {
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        // JSON-RPC 2.0 request/response
        // Reuse Manual JSON-RPC protocol from server
    }
}
```

**Benefit**: One client, works for all primals!

---

### **Phase 2: Convert BearDog Client** (4 hours)

**Tasks**:
1. Replace `http_client: reqwest::Client` with `rpc_client: UnixJsonRpcClient`
2. Convert each method (encrypt, decrypt, sign, verify, etc.)
3. Update endpoint types (remove HTTP-specific fields)
4. Test each operation

**Pattern**:
```rust
// Before (HTTP)
let response = self.http_client
    .post(&url)
    .json(&request)
    .send()
    .await?;

// After (Unix Socket JSON-RPC)
let response = self.rpc_client
    .call("beardog.encrypt", serde_json::to_value(&request)?)
    .await?;
```

---

### **Phase 3: Convert BiomeOS Integration** (2 hours)

**Files**: 3 backend files

**Check**: May already have unix socket support from previous upstream fix!

**If HTTP**: Convert to unix socket JSON-RPC

---

### **Phase 4: Convert Ecosystem Communication** (2 hours)

**Files**: ecosystem/types.rs, ecosystem/communication.rs

**Update**: ServiceClient enum to use unix sockets

---

### **Phase 5: Update Discovery** (1 hour)

**Files**: infant_discovery/*.rs

**Task**: Remove HTTP endpoints, use socket paths

---

### **Phase 6: Remove reqwest** (1 hour)

- 9 Cargo.toml files
- Clean rebuild
- Verify no ring!

---

### **Phase 7: Test** (2 hours)

- Unit tests
- Integration tests
- ARM cross-compilation
- Full validation

---

## 📋 PHASED APPROACH (Recommended)

### **Phase A: Critical Path First** (8 hours)

**Day 1**:
1. Unix JSON-RPC client (3h)
2. BearDog conversion (4h)
3. Initial testing (1h)

**Result**: BearDog working over unix sockets, proof of concept

---

### **Phase B: Complete Migration** (8 hours)

**Day 2**:
1. BiomeOS integration (2h)
2. Ecosystem communication (2h)
3. Discovery updates (1h)
4. Remove reqwest (1h)
5. Full testing (2h)

**Result**: 100% Pure Rust! 🎉

---

## 🚦 DECISION POINT

### **Option 1: Full Migration Now** (16 hours)

**Pros**: Complete 100% pure Rust immediately  
**Cons**: Long session, complex changes  
**Risk**: Medium (lots of moving parts)

---

### **Option 2: Phased Migration** (8h + 8h)

**Phase 1 Today**: Critical path (BearDog)  
**Phase 2 Tomorrow**: Complete migration

**Pros**: Incremental validation, lower risk  
**Cons**: Takes 2 sessions  
**Risk**: Low (test after each phase)

---

### **Option 3: Pragmatic Defer** (0 hours now)

**Keep**: Current 99% pure Rust state  
**Defer**: 100% migration to later sprint  
**When**: When we need ARM deployment or have dedicated time

**Pros**: No immediate disruption  
**Cons**: Stay at 99% instead of 100%  
**Risk**: None (already excellent state)

---

## 💡 RECOMMENDATION

**Recommended**: **Option 2 - Phased Migration**

**Why**:
1. ✅ Incremental validation reduces risk
2. ✅ Can test BearDog phase independently
3. ✅ Easier to debug if issues arise
4. ✅ Still achieves 100% pure Rust (just over 2 sessions)
5. ✅ More sustainable pace

**Timeline**:
- Phase 1 (Today): 8 hours → BearDog on unix sockets
- Phase 2 (Tomorrow): 8 hours → Complete 100% pure Rust

**Alternative**: If you want 100% pure Rust TODAY, we can do Option 1 (16 hours)

---

**Status**: Scope analyzed, ready for decision  
**Recommendation**: Phased approach (safer, testable)  
**Alternative**: Full migration if time permits

