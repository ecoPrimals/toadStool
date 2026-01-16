# Unix Socket Infrastructure - VERIFIED ✅

**Discovery**: We already have comprehensive unix socket infrastructure!  
**Impact**: Migration is 50% easier - just remove HTTP, use existing sockets!  
**Timeline**: 4-6 hours (down from 8 hours)

---

## ✅ EXISTING INFRASTRUCTURE

### **1. Server Uses Unix Sockets** (`crates/server/src/main.rs`)

**Evidence**:
```rust
// Line 110: Server serves on unix socket
server.serve_unix(&socket_path).await

// Line 87-99: JSON-RPC also on unix socket
let jsonrpc_socket = socket_path.with_extension("jsonrpc.sock");
let jsonrpc_server = ManualJsonRpcServer::new(Arc::clone(&executor), version.clone());
jsonrpc_server.serve(jsonrpc_socket_clone).await
```

**Protocols**:
- tarpc (binary RPC) - PRIMARY
- JSON-RPC 2.0 - UNIVERSAL compatibility

---

### **2. Socket Path Discovery** (`crates/server/src/main.rs`)

**Evidence**: Lines 148-190 - Complete socket path resolution

**Priority Order**:
1. `TOADSTOOL_SOCKET` env var (primal-specific, highest priority)
2. `BIOMEOS_SOCKET_PATH` env var (orchestrator-provided)
3. XDG runtime directory (`/run/user/<uid>/toadstool-<family>.sock`)
4. `/tmp` fallback (`/tmp/toadstool-<family>.sock`)

**Result**: Already implements TRUE PRIMAL discovery!

---

### **3. Songbird Client Uses Unix Sockets** (`crates/server/src/songbird_client.rs`)

**Evidence**:
```rust
// Line 33: ServiceLocation type
location_type: "unix-socket"

// Line 83, 96: Unix socket URLs
return Ok(format!("unix://{}", socket));
```

**Socket Discovery**:
- `SONGBIRD_SOCKET` env var
- Runtime directory: `{runtime_dir}/songbird-{family}.sock`
- TRUE PRIMAL pattern!

---

### **4. Manual JSON-RPC Over Unix** (`crates/server/src/manual_jsonrpc.rs`)

**Evidence**:
```rust
// Line 49: Unix socket imports
use tokio::net::{UnixListener, UnixStream};

// Line 186: Connection handler
async fn handle_connection(&self, stream: UnixStream)
```

**Result**: Already have JSON-RPC over unix sockets working!

---

### **5. Registration Uses Unix Sockets** (`crates/server/src/main.rs`)

**Evidence**: Line 319-320
```rust
ServiceLocation {
    location_type: "unix-socket".to_string(),
    path: socket_path.to_string_lossy().to_string(),
    protocol: "tarpc".to_string(),
}
```

**Result**: Already registers unix socket location with Songbird!

---

## 🎯 WHAT THIS MEANS

### **Migration is MUCH Simpler**

**Before Assessment**: 
- "Need to build unix socket infrastructure" ❌
- Estimated: 8 hours

**After Discovery**:
- "Already have unix socket infrastructure!" ✅
- Estimated: 4-6 hours (just remove HTTP!)

---

### **Pattern to Follow**

**We ALREADY do this for Songbird**:
```rust
// From songbird_client.rs
pub async fn get_songbird_socket() -> Result<String> {
    // Method 1: Direct socket path
    if let Ok(socket) = std::env::var("SONGBIRD_SOCKET") {
        return Ok(format!("unix://{}", socket));
    }
    
    // Method 2: Runtime directory
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        let username = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
        format!("/tmp/toadstool-runtime-{}", username)
    });
    
    let socket = format!("{}/songbird-{}.sock", runtime_dir, family);
    return Ok(format!("unix://{}", socket));
}
```

**Apply Same Pattern to BearDog, etc.**:
```rust
pub async fn get_beardog_socket() -> Result<String> {
    if let Ok(socket) = std::env::var("BEARDOG_SOCKET") {
        return Ok(format!("unix://{}", socket));
    }
    
    let runtime_dir = get_runtime_dir();
    let socket = format!("{}/beardog-{}.sock", runtime_dir, family);
    Ok(format!("unix://{}", socket))
}
```

---

## 🚀 SIMPLIFIED MIGRATION PLAN

### **Step 1: Add Socket Discovery Functions** (1 hour)

Create helpers for each primal:
- `get_beardog_socket() -> Result<String>`
- `get_nestgate_socket() -> Result<String>`
- Already have: `get_songbird_socket()`

---

### **Step 2: Replace HTTP Clients** (2 hours)

**Pattern for Each File**:
```rust
// REMOVE this:
let client = reqwest::Client::new();
let response = client.get("http://beardog:8080/api").send().await?;

// ADD this:
use tokio::net::UnixStream;
let socket_path = get_beardog_socket().await?;
let stream = UnixStream::connect(socket_path).await?;
// Use existing tarpc or JSON-RPC client
```

---

### **Step 3: Remove reqwest** (1 hour)

- Remove from 9 Cargo.toml files
- Clean rebuild
- Verify no ring!

---

### **Step 4: Test** (1-2 hours)

- All tests pass
- ARM cross-compilation works
- No ring in cargo tree

---

## 📋 FILES TO CHANGE

### **Already Using Unix Sockets** ✅

- `crates/server/src/main.rs` - Server
- `crates/server/src/songbird_client.rs` - Songbird
- `crates/server/src/manual_jsonrpc.rs` - JSON-RPC

**NO CHANGES NEEDED!**

---

### **Need to Convert to Unix Sockets** (14+ files)

**BearDog Integration**:
- `crates/distributed/src/beardog_integration/client.rs`

**Songbird Integration** (already mostly done?):
- `crates/distributed/src/songbird_integration/*.rs` (7 files)
  - Check if already using sockets!

**Ecosystem**:
- `crates/core/toadstool/src/ecosystem/types.rs`
- `crates/core/toadstool/src/ecosystem/communication.rs`

**Discovery**:
- `crates/core/common/src/infant_discovery/sources.rs`
- `crates/core/common/src/infant_discovery/detectors.rs`

**BiomeOS** (check if already using sockets):
- `crates/core/toadstool/src/biomeos_integration/*.rs` (3 files)

---

## 🎊 CONFIDENCE LEVEL

**VERY HIGH** ✅

**Why**:
1. Infrastructure exists ✅
2. Patterns established ✅  
3. Already using for Songbird ✅
4. Just need to extend pattern ✅

**Estimate**: 
- Optimistic: 4 hours
- Realistic: 5-6 hours
- Conservative: 6 hours

**Much better than 8 hours!**

---

**Status**: Infrastructure verified, ready to execute  
**Timeline**: 4-6 hours (50% faster than estimated!)  
**Confidence**: Very High ✅

