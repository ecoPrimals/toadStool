# 🔥 Deep Debt Elimination - Execution Plan

**Date**: January 29, 2026  
**Status**: IN PROGRESS  
**Approach**: Architectural evolution, not patches

---

## 🎯 Philosophy

> "Deep debt solutions evolve the architecture, not just fix symptoms"

### Principles

1. **Capability-Based, Not Hardcoded**: Primals discover each other at runtime
2. **Modern Idiomatic Rust**: Embrace Result, Cow, references
3. **Pure Rust Evolution**: Replace C dependencies with Rust equivalents
4. **Smart Refactoring**: Improve architecture, not just split files
5. **Safe AND Fast**: Evolve unsafe to safe alternatives where possible
6. **Complete Implementations**: No mocks in production

---

## ✅ Phase 1: Validation & Analysis (COMPLETE)

### ecoBin Validation Results

**Dependency Analysis**:
```bash
✅ NO openssl-sys
✅ NO ring
✅ NO aws-lc-sys
✅ NO native-tls
✅ NO zstd-sys (compression)
```

**Infrastructure C (Acceptable)**:
- `linux-raw-sys` - Linux syscall wrappers (unavoidable)
- `inotify-sys` - File system events (OS interface)
- `seccomp-sys` - Sandboxing (security interface)
- `sysinfo` - System information (OS queries)

**Status**: ✅ **ToadStool is ecoBin compliant!**
- Pure Rust application code ✅
- Infrastructure-only C dependencies ✅
- Cross-compilation supported ✅

### Hardcoded Values Analysis

Found **50+ hardcoded primal names** that should use capability-based discovery:

**High Priority**:
1. `crates/core/toadstool/src/ipc_helpers.rs` - Songbird socket path
2. `crates/core/toadstool/src/biomeos_integration/*.rs` - All backends (auth, storage, agent)
3. `crates/integration/beardog/src/discovery.rs` - BearDog service name
4. `crates/distributed/src/*/client.rs` - Client implementations

**Pattern**: Already have capability-based discovery infrastructure, need to use it everywhere!

### Error Handling Analysis

**Production unwrap() counts**:
- `crates/server/src/`: 10 instances (low!)
- `crates/neuromorphic/`: 17 instances (examples/benchmarks)
- Most are in tests/examples ✅

**Focus**: Server and critical paths

---

## 🚀 Phase 2: Architectural Evolution (IN PROGRESS)

### 2.1 Capability-Based Discovery System

**Problem**: Hardcoded primal names violate "self-knowledge only" principle

**Current (Anti-Pattern)**:
```rust
// ❌ Hardcoded primal name
let socket = get_socket_path_for_service("beardog");
let response = call_rpc(&socket, "beardog.generate_key", params);
```

**Evolution Target**:
```rust
// ✅ Capability-based discovery
let provider = discover_by_capability(Capability::Crypto(CryptoCapability::KeyGeneration)).await?;
let response = provider.call("crypto.generate_keypair", params).await?;
```

**Implementation Plan**:
1. ✅ Enhance `ServiceDiscovery` to query by capability
2. Create `CapabilityProvider` abstraction
3. Migrate backends to use capability discovery
4. Remove hardcoded primal names

### 2.2 Error Handling Evolution

**Problem**: Production unwrap() breaks at runtime

**Current (Anti-Pattern)**:
```rust
// ❌ Production unwrap
let value = config.get("key").unwrap();
let data = serde_json::from_str(&json).unwrap();
```

**Evolution Target**:
```rust
// ✅ Proper error handling
let value = config.get("key")
    .ok_or(ConfigError::MissingKey("key"))?;
let data = serde_json::from_str(&json)
    .map_err(|e| ParseError::InvalidJson(e))?;
```

**Implementation Plan**:
1. Create custom error types for each module
2. Implement `From` traits for error conversion
3. Replace unwrap() with `?` operator
4. Add context to errors

### 2.3 HTTP → JSON-RPC Migration

**Problem**: HTTP/REST violates wateringHole IPC protocol

**Current Architecture**:
```
CLI → HTTP/REST → Server
       (TCP/8080)
```

**Evolution Target**:
```
CLI → JSON-RPC → Server
      (Unix socket /primal/toadstool)
```

**Implementation Plan**:
1. Create `UnixJsonRpcServer` (already exists!)
2. Migrate HTTP handlers to JSON-RPC methods
3. Update CLI to use Unix socket client
4. Remove HTTP dependencies (axum, hyper)
5. Keep tarpc as primary protocol

### 2.4 Zero-Copy Optimization

**Problem**: Unnecessary clones in hot paths

**Current (Anti-Pattern)**:
```rust
// ❌ Unnecessary clone
let caps = runtime.capabilities().clone();
let submission = workload.clone();
```

**Evolution Target**:
```rust
// ✅ Reference or Cow
let caps = &runtime.capabilities();
let submission: Cow<Workload> = Cow::Borrowed(workload);
```

**Implementation Plan**:
1. Identify hot paths (RPC handlers, composition engine)
2. Replace clones with references where possible
3. Use `Cow<str>` for strings that may or may not be owned
4. Use `bytes::Bytes` for network payloads

---

## 📋 Phase 3: Systematic Execution

### Module 1: Capability-Based Discovery ⏳

**Files to Create**:
- `crates/core/common/src/capability_discovery.rs`
- `crates/core/common/src/capability_provider.rs`

**Files to Evolve**:
- `crates/core/toadstool/src/ipc_helpers.rs`
- `crates/core/toadstool/src/biomeos_integration/auth_backend.rs`
- `crates/core/toadstool/src/biomeos_integration/storage_backend.rs`
- `crates/core/toadstool/src/biomeos_integration/agent_backend.rs`
- `crates/integration/beardog/src/discovery.rs`
- `crates/distributed/src/beardog_integration/client.rs`

**Migration Pattern**:
```rust
// Step 1: Create capability enum
pub enum SecurityCapability {
    KeyGeneration,
    Encryption,
    Signing,
    TokenManagement,
}

// Step 2: Create provider abstraction
pub struct CapabilityProvider {
    service_name: String,
    socket_path: PathBuf,
    capabilities: Vec<Capability>,
}

impl CapabilityProvider {
    pub async fn discover(cap: Capability) -> Result<Self> {
        // Query Songbird for services with this capability
        let services = query_songbird("ipc.find_capability", cap).await?;
        // Return first available provider
        services.into_iter().next()
            .ok_or(Error::NoProviderFound(cap))
    }
    
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        // Call via JSON-RPC over Unix socket
        unix_jsonrpc_call(&self.socket_path, method, params).await
    }
}

// Step 3: Use in code
let provider = CapabilityProvider::discover(
    Capability::Security(SecurityCapability::KeyGeneration)
).await?;
let result = provider.call("crypto.generate_keypair", params).await?;
```

### Module 2: Error Handling Evolution ⏳

**Priority Files** (production unwrap/expect):
1. `crates/server/src/manual_jsonrpc.rs` - JSON parsing
2. `crates/server/src/resource_validator.rs` - Validation
3. `crates/server/src/resource_optimizer.rs` - Optimization
4. `crates/neuromorphic/akida-reservoir-research/src/` - Research code (lower priority)

**Pattern**:
```rust
// Create module-specific error type
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    
    #[error("Missing configuration key: {0}")]
    MissingKey(String),
    
    #[error("Resource validation failed: {0}")]
    ValidationFailed(String),
}

// Replace unwrap with ?
// Before:
let value: WorkloadSubmission = serde_json::from_value(params).unwrap();

// After:
let value: WorkloadSubmission = serde_json::from_value(params)
    .map_err(ServerError::InvalidJson)?;
```

### Module 3: HTTP → JSON-RPC Migration ⏳

**Files to Remove** (after migration):
- `crates/cli/src/daemon/http_server.rs`
- `crates/api/src/handlers/*.rs`
- `crates/server/src/handlers.rs`

**Files to Enhance**:
- `crates/server/src/manual_jsonrpc.rs` - Already exists!
- `crates/server/src/pure_jsonrpc.rs` - Already exists!
- `crates/core/common/src/unix_jsonrpc_client.rs` - Already exists!

**Migration Steps**:
1. Map HTTP endpoints to JSON-RPC methods
2. Update CLI to use Unix socket client
3. Remove HTTP server from daemon
4. Remove axum/hyper dependencies
5. Update tests

**Endpoint Mapping**:
```rust
// HTTP → JSON-RPC mapping
HTTP POST /workloads/submit → JSON-RPC "workload.submit"
HTTP GET  /workloads/status → JSON-RPC "workload.status"
HTTP GET  /health          → JSON-RPC "system.health"
HTTP GET  /capabilities    → JSON-RPC "system.capabilities"
```

### Module 4: Zero-Copy Optimization ⏳

**Hot Path Files**:
- `crates/server/src/tarpc_server.rs:209` - `submission.clone()`
- `crates/core/toadstool/src/composition_engine.rs:89` - `capabilities().clone()`
- `crates/core/toadstool/src/multi_workload_compositor.rs:125` - `requests.clone()`

**Pattern**:
```rust
// Before: Clone in tarpc handler
async fn submit_workload(&self, submission: WorkloadSubmission) -> Result<WorkloadId> {
    let cloned = submission.clone();  // ❌ Unnecessary
    self.executor.execute(cloned).await
}

// After: Use reference or Cow
async fn submit_workload(&self, submission: WorkloadSubmission) -> Result<WorkloadId> {
    // submission is moved, no clone needed
    self.executor.execute(submission).await
}

// For shared data:
pub fn capabilities(&self) -> Cow<'_, Capabilities> {
    // Return borrowed if no modification needed
    Cow::Borrowed(&self.caps)
}
```

---

## 📊 Progress Tracking

| Module | Status | Files | LOC Changed | Tests Added |
|--------|--------|-------|-------------|-------------|
| **Capability Discovery** | 🔄 In Progress | 0/6 | 0/500 | 0/20 |
| **Error Handling** | ⏳ Pending | 0/4 | 0/200 | 0/10 |
| **HTTP Migration** | ⏳ Pending | 0/8 | 0/1000 | 0/30 |
| **Zero-Copy** | ⏳ Pending | 0/3 | 0/100 | 0/5 |
| **Test Coverage** | ⏳ Pending | - | - | TBD |

---

## 🎯 Success Criteria

### Capability Discovery
- [ ] Zero hardcoded primal names in production (except self-identification)
- [ ] All backends use `CapabilityProvider`
- [ ] Discovery falls back to Songbird query
- [ ] 100% test coverage for discovery

### Error Handling
- [ ] Zero unwrap() in production code
- [ ] Zero expect() in hot paths
- [ ] All errors have context
- [ ] Error types are domain-specific

### HTTP Migration
- [ ] Zero HTTP/REST for primal-to-primal communication
- [ ] All CLI commands use Unix sockets
- [ ] tarpc over Unix sockets (primary)
- [ ] JSON-RPC over Unix sockets (universal)

### Zero-Copy
- [ ] <50 clones in hot paths (down from 2000+)
- [ ] Cow<str> used for string parameters
- [ ] bytes::Bytes used for network payloads
- [ ] Benchmarks show performance improvement

### Overall
- [ ] ecoBin validated ✅
- [ ] 90% test coverage
- [ ] All clippy pedantic warnings fixed
- [ ] All TODOs converted to GitHub issues
- [ ] Documentation updated

---

## 🔄 Next Actions

### Immediate (This Session)
1. Complete musl build validation
2. Create `CapabilityProvider` abstraction
3. Migrate one backend (auth) to capability-based discovery
4. Fix unwrap() in `crates/server/src/manual_jsonrpc.rs`

### Short-Term (Next Session)
1. Migrate remaining backends to capability discovery
2. Create HTTP → JSON-RPC mapping document
3. Start HTTP daemon migration
4. Add zero-copy to composition engine

---

**Philosophy**: Each change makes the architecture MORE capable, not just "fixed"

🦀🧬✨ **Deep Debt Elimination - Building Excellence!** ✨🧬🦀
