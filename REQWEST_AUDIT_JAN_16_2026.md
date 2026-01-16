# Reqwest Usage Audit - January 16, 2026

**Goal**: Identify all reqwest usage to enable 100% Pure Rust migration  
**Finding**: 9 Cargo.toml files, 20+ Rust files using reqwest  
**Strategy**: Replace with local IPC/discovery (Songbird handles external HTTP)

---

## 📊 Cargo.toml Dependencies (9 files)

**Workspace**:
1. `Cargo.toml` - Workspace dependency definition

**Production Crates** (7):
2. `crates/api/Cargo.toml`
3. `crates/auto_config/Cargo.toml`
4. `crates/cli/Cargo.toml`
5. `crates/client/Cargo.toml`
6. `crates/distributed/Cargo.toml`
7. `crates/server/Cargo.toml`

**Testing** (1):
8. `crates/testing/Cargo.toml` - Optional, feature-gated

---

## 🔍 Usage Patterns (20+ files)

### **Category 1: BiomeOS Integration** (3 files)

**Pattern**: HTTP client for biomeOS API calls

**Files**:
- `crates/core/toadstool/src/biomeos_integration/storage_backend.rs`
- `crates/core/toadstool/src/biomeos_integration/agent_backend.rs`
- `crates/core/toadstool/src/biomeos_integration/auth_backend.rs`

**Usage**: `reqwest::Client` for storage/agent/auth operations

**Migration**: These are for biomeOS NUCLEUS - likely already have local socket integration! Check for unix socket paths.

---

### **Category 2: BYOB Health Checks** (1 file)

**Pattern**: HTTP health check endpoints

**Files**:
- `crates/core/toadstool/src/byob/health.rs`

**Usage**: `reqwest::Client::builder()` for health monitoring

**Migration**: 
- Keep for actual external BYOB endpoints (Docker, K8s, etc.)
- OR make health checks go through Songbird
- DECISION: This is legitimate external HTTP for BYOB monitoring

---

### **Category 3: Deployment Layer** (1 file)

**Pattern**: AWS metadata service (169.254.169.254)

**Files**:
- `crates/core/toadstool/src/deployment_layer.rs`

**Usage**: `reqwest::get("http://169.254.169.254/latest/meta-data/...")`

**Migration**:
- AWS metadata is legitimate external HTTP
- BUT: Should this go through Songbird?
- OR: Keep as deployment-time-only operation?
- DECISION: Deployment detection can use direct HTTP (one-time, startup only)

---

### **Category 4: Ecosystem/Primal Discovery** (2 files)

**Pattern**: HTTP-based primal discovery

**Files**:
- `crates/core/toadstool/src/ecosystem/types.rs` - `ServiceClient::JsonRpc(reqwest::Client)`
- `crates/core/toadstool/src/ecosystem/communication.rs`

**Usage**: HTTP for primal communication

**Migration**: 
✅ **REPLACE** - Use local discovery mechanisms!
- Unix sockets for local primals
- mDNS for network discovery
- Environment-based configuration
- This is the KEY migration!

---

### **Category 5: Distributed Integration** (12+ files)

**Pattern**: HTTP clients for Songbird, BearDog, etc.

**Files**:
- `crates/core/common/src/infant_discovery/sources.rs`
- `crates/core/common/src/infant_discovery/detectors.rs`
- `crates/distributed/src/coordination_integration/client.rs`
- `crates/distributed/src/primal_capabilities/adapters.rs`
- `crates/distributed/src/crypto_integration/client.rs`
- `crates/distributed/src/songbird_integration/*.rs` (7 files)
- `crates/distributed/src/beardog_integration/client.rs`
- `crates/distributed/src/ecosystem/caller.rs`
- `crates/distributed/src/ecosystem/caller_new.rs`

**Usage**: HTTP clients for primal-to-primal communication

**Migration**:
✅ **REPLACE** - Use local IPC instead!
- BearDog: Unix socket or shared memory
- Songbird: Songbird handles external HTTP, we use local IPC
- Local discovery: mDNS, environment, configuration

---

## 🎯 Migration Strategy

### **Phase 1: Keep Legitimate External HTTP** (2 files)

**Rationale**: Some HTTP is genuinely needed

**Files to Keep reqwest**:
1. `byob/health.rs` - External BYOB endpoint health checks
2. `deployment_layer.rs` - AWS/GCP/Azure metadata services

**Alternative**: Make these optional features or move to Songbird proxy

---

### **Phase 2: Replace Primal Communication** (14+ files)

**Target**: All primal-to-primal HTTP

**Strategy**:
```rust
// Before: HTTP-based
let client = reqwest::Client::new();
let response = client.get("http://beardog:8080/entropy").send().await?;

// After: Unix socket-based
use tokio::net::UnixStream;
let stream = UnixStream::connect("/var/run/beardog/entropy.sock").await?;
// Use tarpc or JSON-RPC over unix socket
```

**Files**:
- All `songbird_integration/*.rs` 
- All `beardog_integration/*.rs`
- `ecosystem/types.rs`, `ecosystem/communication.rs`
- `infant_discovery/*.rs`
- `distributed/coordination_integration/*.rs`

---

### **Phase 3: BiomeOS Integration** (3 files)

**Check**: Do we already have unix socket support?

**Files**:
- `biomeos_integration/storage_backend.rs`
- `biomeos_integration/agent_backend.rs`
- `biomeos_integration/auth_backend.rs`

**Migration**: Likely already have socket paths from previous evolution!

---

## 🚦 Decision Matrix

| Usage | Keep HTTP? | Reason | Alternative |
|-------|-----------|--------|-------------|
| **BYOB Health** | Maybe | External endpoints | Optional feature |
| **AWS Metadata** | Maybe | Deployment detection | Startup-only, optional |
| **Primal Discovery** | ❌ NO | Should be local | Unix sockets, mDNS |
| **Songbird Client** | ❌ NO | Use IPC | tarpc over unix socket |
| **BearDog Client** | ❌ NO | Use IPC | tarpc over unix socket |
| **BiomeOS** | ❌ NO | Already have sockets? | Unix sockets |

---

## 💡 Key Insight

**From Upstream Guidance**:
> "ToadStool = Compute orchestration (internal operations)  
> Songbird = External communication (HTTP/TLS)  
> TRUE PRIMAL architecture = Separation of concerns"

**What This Means**:
- ✅ ToadStool should use **local IPC** for all primal communication
- ✅ Songbird handles all **external HTTP/TLS**
- ✅ This removes reqwest → rustls → ring dependency chain
- ✅ Result: **100% Pure Rust!**

---

## 📋 Recommended Approach

### **Option A: Aggressive (100% Pure Rust)**

**Remove reqwest entirely**:
- All primal communication via unix sockets/tarpc
- BYOB health via Songbird proxy OR remove feature
- AWS metadata via Songbird proxy OR startup-only feature
- Result: **100% Pure Rust** ✅

**Pros**: Complete sovereignty, TRUE PRIMAL architecture  
**Cons**: More refactoring needed, BYOB feature complexity

---

### **Option B: Pragmatic (99.9% Pure Rust)**

**Keep minimal reqwest**:
- Remove from all primal communication (14+ files)
- Keep for BYOB health checks (optional feature)
- Keep for deployment detection (startup-only)
- Result: **99.9% Pure Rust** (reqwest but no ring!)

**How**: Use `reqwest` with `native-tls` feature OFF, `rustls` OFF
- Wait, reqwest without TLS still depends on ring? Need to check!

**Pros**: Less refactoring, keeps BYOB convenience  
**Cons**: Still have some external HTTP

---

### **Option C: Hybrid (Recommended)**

**Aggressive for primal communication, pragmatic for deployment**:
- ✅ Remove ALL primal-to-primal HTTP (14+ files) → **local IPC**
- ✅ Remove biomeOS HTTP (use existing unix sockets)
- ⚠️ Keep deployment detection as optional startup feature
- ⚠️ Keep BYOB health as optional feature OR proxy through Songbird

**Check**: Can reqwest work without pulling in ring?

---

## 🎯 Next Steps

1. **Check**: Does reqwest *always* pull in ring?
2. **Audit**: BiomeOS backends - do we have socket support already?
3. **Design**: Unix socket interface for primal communication
4. **Implement**: Replace HTTP with IPC (14+ files)
5. **Test**: All functionality preserved
6. **Validate**: `cargo tree` shows no ring!

---

**Status**: Audit complete, ready for implementation decision  
**Recommendation**: Option A (Aggressive) for TRUE 100% Pure Rust  
**Effort**: 6-8 hours (most time in primal communication refactor)

