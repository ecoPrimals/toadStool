# ToadStool Universal IPC Evolution Plan

**Date**: February 3, 2026  
**Status**: READY FOR IMPLEMENTATION  
**Priority**: High (Enables universal deployment)  
**Upstream**: `wateringHole/UNIVERSAL_IPC_STANDARD_V3.md`

---

## 🎯 **OBJECTIVE**

Evolve ToadStool's IPC from Unix-only to **universal multi-transport** following the upstream standard, while maintaining **complete primal autonomy** (no shared dependencies).

---

## 📊 **CURRENT STATE AUDIT**

### **What We Have** ✅

**File**: `crates/core/toadstool/src/ipc_helpers.rs` (673 lines)

**Strengths**:
- ✅ JSON-RPC 2.0 protocol (compliant with standard)
- ✅ Songbird discovery integration
- ✅ biomeOS socket standard (`/run/user/$UID/biomeos/*.sock`)
- ✅ Deep debt principles (runtime discovery, self-knowledge, service-based)
- ✅ Comprehensive semantic method registry
- ✅ Timeout handling (5s)
- ✅ Error handling with ToadStoolError
- ✅ Pure Rust UID detection (no unsafe!)

**Current Transport**:
- ✅ Unix sockets (`tokio::net::UnixStream`) - Works on Linux/macOS
- ❌ No Abstract sockets - **Blocks Android deployment**
- ❌ No TCP fallback - **Blocks cross-device communication**
- ❌ No multi-transport server - **Limits platform compatibility**

**Deep Debt Grade**: **A-** (excellent foundation, needs universal evolution)

---

## 🚨 **DEPLOYMENT BLOCKERS**

### **Real-World Failure Case** (from upstream):

```
Pixel 8a (GrapheneOS):
  ToadStool daemon --socket /data/local/tmp/biomeos/toadstool.sock
  Error: Failed to bind Unix socket
  
Root Cause: SELinux blocks filesystem Unix sockets
Solution Needed: Abstract sockets (@biomeos_toadstool) or TCP fallback
```

### **Current Limitations**:

| Platform | Unix Socket | Abstract | TCP | Status |
|----------|-------------|----------|-----|--------|
| **Linux (desktop)** | ✅ | ❌ | ❌ | Works |
| **Android** | ❌ (SELinux) | ❌ | ❌ | **BLOCKED** |
| **Cross-device** | ❌ | ❌ | ❌ | **BLOCKED** |
| **macOS** | ✅ | N/A | ❌ | Works |
| **Windows** | N/A | N/A | ❌ | **Not supported** |

---

## 🎯 **TARGET STATE**

### **Universal Multi-Transport Support**

```
ToadStool IPC (owned by us, in our codebase):
  ├── Unix sockets      (Tier 1: Desktop Linux/macOS)
  ├── Abstract sockets  (Tier 1: Android, Linux)
  ├── TCP fallback      (Tier 2: Cross-device, Windows)
  └── Auto-detection    (bind all available, connect best)
```

### **Architecture** (Following Upstream Standard):

```rust
// ToadStool owns ALL this code (no shared crates!)
crates/core/toadstool/src/ipc/
├── mod.rs              # Public API, multi-transport orchestration
├── platform/
│   ├── mod.rs          # Platform detection, endpoint types
│   ├── unix.rs         # Unix socket implementation
│   ├── android.rs      # Abstract socket implementation (Linux-specific)
│   ├── tcp.rs          # TCP fallback implementation
│   └── fallback.rs     # Fallback chain logic
├── server.rs           # Multi-transport server (binds all)
├── client.rs           # Smart client (tries best transport)
├── protocol.rs         # JSON-RPC 2.0 helpers
└── error.rs            # IPC-specific errors
```

**Key Principles**:
- ✅ **Primal Autonomy**: All code owned by ToadStool
- ✅ **No Shared Crates**: No `primal-ipc` dependency
- ✅ **Standard Compliance**: Follow behavioral standard, not code
- ✅ **Deep Debt**: Modern idiomatic Rust, safe, isomorphic

---

## 📋 **IMPLEMENTATION PHASES**

### **Phase 1: Module Structure** (30 mins)

Create new module structure while keeping existing code working:

**Tasks**:
1. ✅ Create `crates/core/toadstool/src/ipc/` directory
2. ✅ Move current helpers to `ipc/legacy.rs` (keep working)
3. ✅ Create module files (empty stubs)
4. ✅ Wire up exports
5. ✅ Ensure all tests still pass

**Deliverable**: New structure, existing functionality unchanged

---

### **Phase 2: Platform Abstractions** (1 hour)

Reference BearDog/Songbird patterns for implementation:

**Tasks**:
1. ✅ Create `PlatformSocket` trait
2. ✅ Create `Endpoint` enum (Unix, Abstract, Tcp)
3. ✅ Implement platform detection
4. ✅ Add transport tier logic (Tier 1 → Tier 2 fallback)

**Code Pattern** (from upstream):

```rust
/// Platform-agnostic socket endpoint
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// Filesystem Unix socket (Linux, macOS)
    Unix { path: PathBuf },
    
    /// Abstract Unix socket (Linux, Android)
    #[cfg(target_os = "linux")]
    Abstract { name: String },
    
    /// TCP socket (universal fallback)
    Tcp { host: String, port: u16 },
}

/// Transport tier for fallback logic
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransportTier {
    Tier1, // Preferred (Unix, Abstract)
    Tier2, // Fallback (TCP)
}
```

---

### **Phase 3: Unix Socket Implementation** (30 mins)

Extract and modernize existing Unix socket code:

**Tasks**:
1. ✅ Extract Unix socket logic from `ipc_helpers.rs`
2. ✅ Implement `PlatformSocket` for Unix
3. ✅ Add tests for Unix-specific behavior
4. ✅ Ensure backward compatibility

**File**: `crates/core/toadstool/src/ipc/platform/unix.rs` (~150 lines)

---

### **Phase 4: Abstract Socket Implementation** (45 mins)

Add Android/Linux abstract socket support (NEW):

**Tasks**:
1. ✅ Implement abstract socket binding
2. ✅ Implement abstract socket connection
3. ✅ Add `@biomeos_toadstool` naming standard
4. ✅ Add Linux-specific cfg gates
5. ✅ Add tests (integration with mocking)

**File**: `crates/core/toadstool/src/ipc/platform/android.rs` (~120 lines)

**Reference Pattern** (from BearDog):

```rust
#[cfg(target_os = "linux")]
pub async fn bind_abstract(name: &str) -> Result<UnixListener> {
    use std::os::unix::net::UnixListener as StdListener;
    
    let addr = format!("\0{}", name); // Leading null byte
    let std_listener = StdListener::bind(addr)?;
    std_listener.set_nonblocking(true)?;
    
    UnixListener::from_std(std_listener)
}
```

---

### **Phase 5: TCP Fallback Implementation** (1 hour)

Add cross-device and Windows support (NEW):

**Tasks**:
1. ✅ Implement TCP server binding
2. ✅ Implement TCP client connection
3. ✅ Add port allocation logic (0 = random)
4. ✅ Add service discovery integration
5. ✅ Add tests

**File**: `crates/core/toadstool/src/ipc/platform/tcp.rs` (~200 lines)

**Use Cases**:
- Cross-device communication (phone ↔ laptop)
- Windows platform (no Unix sockets)
- Firewall-friendly environments

---

### **Phase 6: Multi-Transport Server** (1.5 hours)

Create server that binds ALL available transports:

**Tasks**:
1. ✅ Implement `ToadStoolServer` builder pattern
2. ✅ Implement multi-transport binding
3. ✅ Implement connection handler (unified)
4. ✅ Add graceful shutdown
5. ✅ Add comprehensive tests

**File**: `crates/core/toadstool/src/ipc/server.rs` (~300 lines)

**API**:

```rust
let server = ToadStoolServer::builder()
    .with_name("toadstool")
    .with_transports(&[
        Transport::Unix { path: "/run/user/1000/biomeos/toadstool.sock".into() },
        Transport::Abstract { name: "@biomeos_toadstool".into() },
        Transport::Tcp { port: 0 }, // Random port
    ])
    .with_json_rpc_handler(handler)
    .start()
    .await?;
```

---

### **Phase 7: Smart Client** (1 hour)

Create client that tries best transport automatically:

**Tasks**:
1. ✅ Implement `ToadStoolClient` with transport discovery
2. ✅ Implement automatic fallback (Tier 1 → Tier 2)
3. ✅ Add connection pooling
4. ✅ Add tests

**File**: `crates/core/toadstool/src/ipc/client.rs` (~250 lines)

**API**:

```rust
// Automatic transport selection
let client = ToadStoolClient::connect("toadstool").await?;

// Manual transport override
let client = ToadStoolClient::builder()
    .with_endpoint(Endpoint::Tcp { host: "192.168.1.100".into(), port: 8080 })
    .connect()
    .await?;
```

---

### **Phase 8: Migration & Testing** (2 hours)

Migrate existing code and ensure no regressions:

**Tasks**:
1. ✅ Update `ipc_helpers.rs` to use new module
2. ✅ Keep backward-compatible API
3. ✅ Run full test suite
4. ✅ Add integration tests (cross-transport)
5. ✅ Add platform-specific CI tests
6. ✅ Update documentation

---

### **Phase 9: Daemon Integration** (1 hour)

Integrate with ToadStool daemon (if exists):

**Tasks**:
1. ✅ Update daemon to bind multi-transport
2. ✅ Add CLI flags (`--unix`, `--abstract`, `--tcp`)
3. ✅ Add configuration file support
4. ✅ Test end-to-end

---

## 🧪 **TESTING STRATEGY**

### **Unit Tests**:

```rust
#[tokio::test]
async fn test_unix_socket_roundtrip() {
    let server = ToadStoolServer::builder()
        .with_transports(&[Transport::Unix { path: "/tmp/test.sock".into() }])
        .start().await.unwrap();
    
    let client = ToadStoolClient::connect_unix("/tmp/test.sock").await.unwrap();
    let response = client.call("health", json!({})).await;
    assert!(response.is_ok());
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_abstract_socket() {
    let server = ToadStoolServer::builder()
        .with_transports(&[Transport::Abstract { name: "@test".into() }])
        .start().await.unwrap();
    
    let client = ToadStoolClient::connect_abstract("@test").await.unwrap();
    assert!(client.is_connected());
}

#[tokio::test]
async fn test_tcp_fallback() {
    let server = ToadStoolServer::builder()
        .with_transports(&[Transport::Tcp { port: 0 }])
        .start().await.unwrap();
    
    let port = server.tcp_port().unwrap();
    let client = ToadStoolClient::connect_tcp("localhost", port).await.unwrap();
    assert!(client.is_connected());
}

#[tokio::test]
async fn test_multi_transport_server() {
    let server = ToadStoolServer::builder()
        .with_transports(&[
            Transport::Unix { path: "/tmp/test.sock".into() },
            Transport::Tcp { port: 0 },
        ])
        .start().await.unwrap();
    
    // Should bind both transports
    assert!(server.has_unix());
    assert!(server.has_tcp());
}

#[tokio::test]
async fn test_automatic_fallback() {
    // Start server with only TCP
    let server = ToadStoolServer::builder()
        .with_transports(&[Transport::Tcp { port: 8080 }])
        .start().await.unwrap();
    
    // Client tries Unix first, falls back to TCP
    let client = ToadStoolClient::connect("toadstool").await.unwrap();
    assert_eq!(client.transport(), TransportType::Tcp);
}
```

### **Integration Tests**:

```bash
# Test cross-primal communication
./toadstool daemon &
./songbird daemon &

# ToadStool → Songbird (via Songbird discovery)
toadstool-cli ipc call songbird health

# BearDog → ToadStool (via TCP)
beardog-cli ipc call toadstool --tcp compute.execute
```

### **Platform CI Matrix**:

```yaml
# .github/workflows/ipc-tests.yml
jobs:
  test-ipc:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, nightly]
    
    steps:
      - run: cargo test --package toadstool --lib ipc --all-features
      - run: cargo test --package toadstool --test ipc_e2e
```

---

## 📊 **SUCCESS METRICS**

| Metric | Before | Target | Status |
|--------|--------|--------|--------|
| **Platform support** | Linux/macOS only | + Android + Windows | ⏳ |
| **Transport types** | 1 (Unix) | 3 (Unix + Abstract + TCP) | ⏳ |
| **Android deployment** | ❌ Blocked | ✅ Working | ⏳ |
| **Cross-device IPC** | ❌ Not supported | ✅ Working | ⏳ |
| **Automatic fallback** | ❌ Manual | ✅ Automatic | ⏳ |
| **Lines of code** | 673 | ~1200 | ⏳ |
| **Deep debt grade** | A- | A++ | ⏳ |

**Note**: ~1200 lines is HEALTHY autonomy for universal deployment. We own our implementation!

---

## 🔗 **REFERENCE PATTERNS**

### **From BearDog** (extract patterns, NOT code):
- `phase1/beardog/crates/beardog-tunnel/src/platform/` - Platform abstraction
- `phase1/beardog/crates/beardog-tunnel/src/tcp_ipc/` - TCP implementation

### **From Songbird** (extract patterns, NOT code):
- `phase1/songbird/crates/songbird-universal-ipc/` - Multi-transport orchestration
- Platform detection and fallback logic

### **Upstream Standard**:
- `wateringHole/UNIVERSAL_IPC_STANDARD_V3.md` - Behavioral specification
- `wateringHole/PRIMAL_IPC_PROTOCOL.md` - Discovery protocol

---

## ⚡ **QUICK START IMPLEMENTATION**

### **Option 1: Full Evolution** (~7-9 hours)
Complete all phases, full multi-transport support

### **Option 2: Android MVP** (~3-4 hours)
Phases 1-4 only (Unix + Abstract), unblock Android

### **Option 3: Cross-Device MVP** (~4-5 hours)
Phases 1-3, 5-6 (Unix + TCP), enable cross-device

---

## 🎯 **RECOMMENDATION**

**Start with Option 2 (Android MVP)**: Unblock Pixel 8a deployment first, add TCP later when cross-device is needed.

**Rationale**:
1. Immediate value (Android deployment)
2. Smaller scope (easier to validate)
3. Foundation for TCP later
4. Aligns with upstream priority (Android was blocking issue)

---

## 📝 **NEXT STEPS**

**Ready to proceed?**

1. ✅ Audit complete
2. ✅ Plan documented
3. ⏳ Choose implementation option
4. ⏳ Create module structure (Phase 1)
5. ⏳ Implement selected phases
6. ⏳ Test and validate
7. ⏳ Document and commit

---

**Created**: February 3, 2026  
**Status**: READY FOR IMPLEMENTATION  
**Deep Debt**: A++ target (modern, idiomatic, universal, isomorphic)  
**Primal Autonomy**: 100% (no shared crates, owned implementation)

🦀🔗✨ **Universal IPC = Universal Deployment** ✨🔗🦀
