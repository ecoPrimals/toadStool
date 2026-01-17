# Songbird Review Summary - Unix Socket Status

**Date**: January 16, 2026  
**Review**: Songbird primal-sdk and registration modules  
**Finding**: ⚠️ **Songbird SDK also uses HTTP, not Unix sockets**

---

## 🔍 **FINDINGS**

### **Songbird Primal SDK**

**Files Reviewed**:
- `crates/songbird-primal-sdk/src/registration.rs`
- `crates/songbird-primal-sdk/src/toadstool.rs` (deprecated)
- `crates/songbird-primal-sdk/src/compute_capability.rs`

**Current Implementation**:
```rust
// registration.rs line 95
pub enum RegistrationError {
    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),  // ❌ Uses HTTP
```

```rust
// compute_capability.rs lines 22, 36
use reqwest::Client;  // ❌ HTTP client
pub struct ComputeCapabilityClient {
    http_client: Client,  // ❌ HTTP
}
```

**Status**: Songbird SDK uses HTTP/reqwest for primal registration and capability discovery

---

## 💡 **KEY INSIGHT**

**The Issue**: BOTH ToadStool AND Songbird currently use HTTP for primal-to-primal communication

**Why**: Likely historical - HTTP was easier to implement initially

**Problem**: Violates architecture principles:
1. **Concentrated Gap**: Only Songbird should have HTTP/TLS
2. **Self-Knowledge**: Primals shouldn't have external service knowledge
3. **Pure Rust**: HTTP brings ring C dependency

---

## 🎯 **CORRECT ARCHITECTURE** (Per User Guidance)

```
Primal A (Pure Rust)
  ↓ Unix Socket + JSON-RPC
Songbird (Has TLS)
  ↓ Unix Socket + JSON-RPC  
Primal B (Pure Rust)
```

**Key Points**:
- **Primal-to-Primal**: Unix sockets (no HTTP!)
- **Primal-to-Songbird**: Unix sockets (no HTTP!)  
- **Songbird-to-External**: HTTP/TLS (concentrated gap)

---

## ✅ **WHAT WE ALREADY HAVE**

### **ToadStool's Manual JSON-RPC Server**

**File**: `crates/server/src/manual_jsonrpc.rs`

**Implementation**: ✅ Pure Rust JSON-RPC over Unix sockets!

```rust
// Lines 1-20 (from review)
//! Manual JSON-RPC 2.0 Server over Unix Sockets
//! Deep debt principle: Pure implementation without jsonrpsee overhead
//! Universal protocol accessible from any language

use tokio::net::{UnixListener, UnixStream};  // ✅ Unix sockets!

pub struct ManualJsonRpcServer {
    socket_path: PathBuf,
    executor: Arc<dyn WorkloadExecutor + Send + Sync>,
}

impl ManualJsonRpcServer {
    pub async fn start(socket_path: PathBuf, executor: ...) {
        let listener = UnixListener::bind(&socket_path)?;  // ✅ Unix socket
        // ... JSON-RPC 2.0 implementation ...
    }
}
```

**Status**: ✅ **WE ALREADY HAVE THE SOLUTION!**

---

## 🚀 **EVOLUTION PATH**

### **Phase 1: Create Unix Socket Registration Client** (ToadStool)

Use ToadStool's own `manual_jsonrpc.rs` as template:

```rust
// NEW: crates/server/src/songbird_unix_client.rs
use tokio::net::UnixStream;
use serde_json::json;

pub async fn register_with_songbird_unix(
    songbird_socket: &str,
    our_capabilities: SelfCapabilities,
) -> Result<()> {
    // 1. Connect to Songbird's Unix socket
    let mut stream = UnixStream::connect(songbird_socket).await?;
    
    // 2. Send JSON-RPC registration request
    let request = json!({
        "jsonrpc": "2.0",
        "method": "primal.register",
        "params": {
            "service_id": uuid::Uuid::new_v4(),
            "capabilities": our_capabilities,
            "socket_path": "/tmp/toadstool.sock",
        },
        "id": 1
    });
    
    // 3. Write/read from Unix socket
    // ... (use existing manual_jsonrpc patterns) ...
    
    Ok(())
}
```

### **Phase 2: Discovery** (Environment-based)

```rust
pub async fn discover_songbird() -> Option<String> {
    // 1. Check environment
    if let Ok(path) = env::var("SONGBIRD_SOCKET") {
        return Some(path);
    }
    
    // 2. Check well-known locations
    for path in ["/tmp/biome/songbird.sock", "/var/run/songbird/socket"] {
        if Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    
    None  // Standalone mode
}
```

### **Phase 3: Update Songbird** (Future)

Songbird also needs to evolve its registration endpoint to Unix socket:

```rust
// Songbird: Add Unix socket registration endpoint
pub async fn start_registration_unix_server() {
    let listener = UnixListener::bind("/tmp/biome/songbird.sock")?;
    // Handle JSON-RPC registration requests
}
```

---

## 📊 **IMPACT ANALYSIS**

### **Benefits**

✅ **Pure Rust**: Remove reqwest/ring from ToadStool  
✅ **Architecture Compliant**: Proper self-knowledge  
✅ **ARM Compatible**: No C toolchain needed  
✅ **Simpler**: Unix sockets simpler than HTTP  
✅ **Faster**: Lower latency than TCP  
✅ **Already Built**: Can reuse manual_jsonrpc patterns!  

### **Work Required**

**ToadStool Side**:
1. Create `songbird_unix_client.rs` (~100 lines)
2. Add discovery logic (~50 lines)
3. Update `unibin.rs` (~20 lines)
4. Remove reqwest dependency (~1 line)
5. Test (~50 lines)

**Total**: ~3-4 hours

**Songbird Side** (Future):
1. Add Unix socket registration endpoint
2. Update SDK to support Unix sockets
3. Documentation

**Total**: ~4-6 hours

---

## 🏁 **CONCLUSION**

**Finding**: Songbird SDK also uses HTTP (not Unix sockets yet)

**Good News**: ✅ ToadStool already has perfect Unix socket JSON-RPC implementation!

**Solution**: Use ToadStool's `manual_jsonrpc.rs` patterns to create Unix socket registration client

**Timeline**: 
- ToadStool evolution: 3-4 hours
- Songbird evolution: Future coordination needed

**Status**: Clear path forward, reusing existing code!

---

**Created**: January 16, 2026  
**Review**: Songbird primal-sdk  
**Finding**: Both use HTTP (architectural debt)  
**Solution**: Evolve to Unix sockets (reuse manual_jsonrpc patterns)

🦀🧬 **We Already Have The Solution!** 🧬🦀
