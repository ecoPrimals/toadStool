# 🔍 TOADSTOOL IPC DEEP DEBT AUDIT

**Date**: January 31, 2026  
**Auditor**: AI Agent (biomeOS guidance)  
**Target**: `crates/runtime/display/src/ipc/`  
**Reference**: songbird v3.33.0 Isomorphic IPC

═══════════════════════════════════════════════════════════════

## 📋 CURRENT STATE ANALYSIS

### **What Exists** ✅

**Location**: `crates/runtime/display/src/ipc/`
- `server.rs` (276 lines) - JSON-RPC server over Unix sockets
- `client.rs` (205 lines) - JSON-RPC client
- `mod.rs` (79 lines) - Public API
- `types.rs` - JSON-RPC protocol types

**Good Qualities**:
- ✅ Pure Rust (tokio async)
- ✅ Zero unsafe code
- ✅ JSON-RPC 2.0 protocol
- ✅ Clean async/await
- ✅ Capability-based socket discovery (XDG_RUNTIME_DIR)
- ✅ Complete implementation (not a mock!)

═══════════════════════════════════════════════════════════════

## ❌ DEEP DEBT ISSUES IDENTIFIED

### **Issue #1: HARDCODED Unix Socket Only** 🔴

**Severity**: CRITICAL  
**Location**: `server.rs:72`, `client.rs:42`

**Problem**:
```rust
// server.rs:72 - HARDCODED Unix socket!
let listener = UnixListener::bind(&path)
    .map_err(|e| DisplayError::IpcError(format!("Failed to bind socket: {}", e)))?;

// client.rs:42 - HARDCODED Unix socket!
let stream = UnixStream::connect(&path)
    .await
    .map_err(|e| DisplayError::IpcError(format!("Connection failed: {}", e)))?;
```

**Why It's Debt**:
- ❌ **Platform assumption**: Assumes Unix sockets available
- ❌ **No fallback**: Dies on Android/SELinux
- ❌ **Not isomorphic**: Requires platform-specific builds
- ❌ **User configuration**: Requires Android users to configure TCP manually

**Correct Pattern** (from songbird):
```rust
// Try Unix socket first
match self.try_unix_server().await {
    Ok(()) => Ok(()),
    // DETECT platform constraint
    Err(e) if self.is_platform_constraint(&e) => {
        // ADAPT automatically
        self.start_tcp_fallback().await
    }
    Err(e) => Err(e)  // Real error
}
```

---

### **Issue #2: No Platform Constraint Detection** 🔴

**Severity**: HIGH  
**Location**: `server.rs` (missing)

**Problem**: No `is_platform_constraint()` function!

**What's Missing**:
```rust
// MISSING: Platform constraint detection
fn is_platform_constraint(&self, error: &anyhow::Error) -> bool {
    if let Some(io_err) = error.downcast_ref::<std::io::Error>() {
        match io_err.kind() {
            ErrorKind::PermissionDenied => self.is_selinux_enforcing(),
            ErrorKind::Unsupported => true,
            _ => false
        }
    } else {
        false
    }
}

fn is_selinux_enforcing(&self) -> bool {
    std::fs::read_to_string("/sys/fs/selinux/enforce")
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .map(|v| v == 1)
        .unwrap_or(false)
}
```

**Impact**: Can't distinguish between:
- Platform constraint (should fallback) → "Permission denied on Android"
- Real error (should fail) → "Permission denied (wrong user)"

---

### **Issue #3: No TCP Fallback Server** 🔴

**Severity**: CRITICAL  
**Location**: `server.rs` (missing)

**Problem**: No automatic TCP fallback!

**What's Missing** (~100 lines):
```rust
async fn start_tcp_fallback(self: Arc<Self>) -> Result<()> {
    info!("🌐 Starting TCP IPC fallback (isomorphic mode)");
    
    // Bind to localhost only (security!)
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;
    
    info!("✅ TCP IPC listening on {}", local_addr);
    
    // Write discovery file for clients
    self.write_tcp_discovery_file(&local_addr)?;
    
    // Accept loop (same as Unix)
    loop {
        let (stream, _) = listener.accept().await?;
        let handler = self.clone();
        tokio::spawn(async move {
            handler.handle_tcp_connection(stream).await
        });
    }
}
```

**Impact**: No Android support without manual configuration!

---

### **Issue #4: No Discovery File System** 🔴

**Severity**: HIGH  
**Location**: `server.rs` (missing)

**Problem**: Clients can't discover TCP endpoint!

**What's Missing**:
```rust
fn write_tcp_discovery_file(&self, addr: &SocketAddr) -> Result<()> {
    // XDG-compliant discovery file
    let discovery_dirs = [
        env::var("XDG_RUNTIME_DIR").ok(),
        env::var("HOME").map(|h| format!("{}/.local/share", h)),
        Some("/tmp".to_string()),
    ];
    
    for dir in discovery_dirs.iter().filter_map(|d| d.as_ref()) {
        let discovery_file = format!("{}/toadstool-ipc-port", dir);
        
        if let Ok(mut f) = File::create(&discovery_file) {
            writeln!(f, "tcp:{}", addr)?;  // Format: tcp:127.0.0.1:PORT
            info!("📁 TCP discovery file: {}", discovery_file);
            break;
        }
    }
    
    Ok(())
}
```

**Impact**: Clients hardcode paths, no auto-discovery!

---

### **Issue #5: No Polymorphic Client Discovery** 🔴

**Severity**: HIGH  
**Location**: `client.rs` (missing)

**Problem**: Client only supports Unix sockets!

**What's Missing**:
```rust
// MISSING: Endpoint enum
pub enum IpcEndpoint {
    UnixSocket(PathBuf),
    TcpLocal(SocketAddr),
}

// MISSING: Discovery function
pub fn discover_ipc_endpoint() -> Result<IpcEndpoint> {
    // Try Unix socket first
    let socket_paths = get_socket_paths();
    for path in socket_paths {
        if path.exists() {
            return Ok(IpcEndpoint::UnixSocket(path));
        }
    }
    
    // Try TCP discovery file
    if let Ok(endpoint) = discover_tcp_endpoint() {
        return Ok(endpoint);
    }
    
    Err(anyhow::anyhow!("Could not discover IPC endpoint"))
}

// MISSING: Polymorphic streams
trait AsyncStream: AsyncRead + AsyncWrite + Send + Unpin {}
impl AsyncStream for UnixStream {}
impl AsyncStream for TcpStream {}
```

**Impact**: Clients can't adapt to TCP fallback!

---

### **Issue #6: Hardcoded Capabilities** 🟡

**Severity**: MEDIUM  
**Location**: `server.rs:253-263`

**Problem**:
```rust
Ok(serde_json::json!({
    "primal_id": "toadstool-primary",  // ❌ HARDCODED
    "socket_path": "/run/user/1000/toadstool/display.sock",  // ❌ HARDCODED
    "max_windows": 16,  // ❌ HARDCODED
    ...
}))
```

**Should Be**:
```rust
Ok(serde_json::json!({
    "primal_id": self.primal_id,  // ✅ Instance variable
    "socket_path": self.socket_path.display().to_string(),  // ✅ Actual path
    "max_windows": self.manager.max_windows(),  // ✅ Runtime query
    ...
}))
```

---

### **Issue #7: No Try→Detect→Adapt Pattern** 🔴

**Severity**: CRITICAL  
**Location**: `server.rs:86-108` (`serve()` method)

**Current Code**:
```rust
pub async fn serve(self) -> Result<()> {
    let listener = self.listener.ok_or_else(|| {
        DisplayError::IpcError("Server not bound. Call bind() first.".to_string())
    })?;
    
    // ❌ Direct serve - no fallback logic!
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => { /* ... */ }
            Err(e) => { /* ... */ }
        }
    }
}
```

**Should Be** (songbird pattern):
```rust
pub async fn start(self: Arc<Self>) -> Result<()> {
    info!("🔌 Starting IPC server (isomorphic mode)...");
    info!("   Trying Unix socket IPC (optimal)...");
    
    // TRY Unix socket first
    match self.try_unix_server().await {
        Ok(()) => Ok(()),
        
        // DETECT platform constraints
        Err(e) if self.is_platform_constraint(&e) => {
            warn!("⚠️  Unix sockets unavailable: {}", e);
            warn!("   Falling back to TCP IPC...");
            
            // ADAPT to TCP
            self.start_tcp_fallback().await
        }
        
        // Real error
        Err(e) => Err(e)
    }
}
```

═══════════════════════════════════════════════════════════════

## 📊 DEBT SUMMARY

### **Critical Issues** (Must Fix)
1. ❌ **Hardcoded Unix socket only** (no TCP fallback)
2. ❌ **No platform constraint detection**
3. ❌ **No TCP fallback server**
4. ❌ **No Try→Detect→Adapt pattern**

### **High Priority Issues**
5. ❌ **No discovery file system**
6. ❌ **No polymorphic client discovery**

### **Medium Priority Issues**
7. ❌ **Hardcoded capabilities**

### **Deep Debt Violations**

| Principle | Current | Target | Status |
|-----------|---------|--------|--------|
| **Platform-Agnostic** | Unix only | Unix + TCP | ❌ FAIL |
| **Runtime Discovery** | Partial (XDG) | Full (Unix/TCP) | 🟡 PARTIAL |
| **Zero Configuration** | Requires env | Zero config | ❌ FAIL |
| **Automatic Adaptation** | None | Try→Detect→Adapt | ❌ FAIL |
| **Pure Rust** | ✅ Yes | ✅ Yes | ✅ PASS |
| **Zero Unsafe** | ✅ Yes | ✅ Yes | ✅ PASS |
| **Modern Async** | ✅ Yes | ✅ Yes | ✅ PASS |

**Current Grade**: **C** (67/100)  
**Target Grade**: **A++** (205/100) like songbird

═══════════════════════════════════════════════════════════════

## 🎯 EVOLUTION PLAN

### **Phase 1: Server-Side Fallback** (4-6 hours)

**Goal**: Add automatic TCP fallback to display server

**Files to Modify**:
1. `crates/runtime/display/src/ipc/server.rs` (~200 lines added)
   - Add `is_platform_constraint()`
   - Add `is_selinux_enforcing()`
   - Add `try_unix_server()`
   - Add `start_tcp_fallback()`
   - Add `handle_tcp_connection()`
   - Add `write_tcp_discovery_file()`
   - Evolve `serve()` → `start()` with Try→Detect→Adapt

2. `crates/runtime/display/Cargo.toml`
   - Add `tokio = { features = ["net"] }` (for TcpListener)

**Expected Result**:
```log
[INFO] Starting IPC server (isomorphic mode)...
[INFO]    Trying Unix socket IPC (optimal)...
[WARN] ⚠️  Unix sockets unavailable: Permission denied
[WARN]    Detected platform constraint, adapting...
[INFO] 🌐 Starting TCP IPC fallback (isomorphic mode)
[INFO] ✅ TCP IPC listening on 127.0.0.1:45123
```

### **Phase 2: Client-Side Discovery** (2-3 hours)

**Goal**: Clients discover Unix OR TCP endpoints

**Files to Modify**:
1. `crates/runtime/display/src/ipc/client.rs` (~150 lines added)
   - Add `IpcEndpoint` enum
   - Add `discover_ipc_endpoint()`
   - Add `discover_tcp_endpoint()`
   - Add `AsyncStream` trait
   - Evolve `connect()` to use polymorphic discovery

2. `crates/runtime/display/src/ipc/types.rs` (new file)
   - Move common types to module

**Expected Result**:
```rust
// Automatic discovery!
let client = DisplayClient::discover().await?;  // Finds Unix OR TCP!
```

### **Phase 3: Integration & Testing** (1-2 hours)

**Goal**: Validate end-to-end isomorphic operation

**Tests to Add**:
1. `test_unix_socket_server()` - Linux
2. `test_tcp_fallback()` - Android simulation
3. `test_client_discovery()` - Auto-discovery
4. `test_platform_constraint_detection()` - SELinux check

**Validation**:
- Build for x86_64 and aarch64
- Test on Linux (Unix sockets)
- Test on Android (TCP fallback)
- Capture logs proving automatic adaptation

═══════════════════════════════════════════════════════════════

## 📚 REFERENCE IMPLEMENTATION

**Study These Files** from songbird v3.33.0:

1. **Server Fallback**:
   - `crates/songbird-orchestrator/src/ipc/pure_rust_server/server.rs`
   - Lines 250-446: Complete Try→Detect→Adapt pattern

2. **Client Discovery**:
   - `crates/songbird-http-client/src/crypto/socket_discovery.rs`
   - `discover_ipc_endpoint()`: Auto-discover Unix OR TCP

3. **Connection Handling**:
   - `crates/songbird-http-client/src/beardog_client/core.rs`
   - `IpcEndpoint` enum: Polymorphic endpoint type

4. **Polymorphic Streams**:
   - `crates/songbird-http-client/src/beardog_client/rpc.rs`
   - `AsyncStream` trait: Polymorphic streams

**Key Insight**: songbird's pattern is DIRECTLY APPLICABLE to toadstool's display IPC!

═══════════════════════════════════════════════════════════════

## ✅ SUCCESS CRITERIA

Evolution is complete when:

1. ✅ **Builds** for x86_64 and aarch64
2. ✅ **Works on Linux** (uses Unix sockets)
3. ✅ **Works on Android** (automatically falls back to TCP)
4. ✅ **Zero configuration** (no env vars or flags)
5. ✅ **Logs show adaptation** ("⚠️ Unix sockets unavailable, using TCP fallback")
6. ✅ **Discovery works** (clients find Unix OR TCP endpoints)
7. ✅ **Communication works** (display operations via JSON-RPC)
8. ✅ **Deep Debt maintained** (Pure Rust, zero unsafe, runtime discovery)

**Expected Timeline**: 6-8 hours (adapt existing IPC)

═══════════════════════════════════════════════════════════════

## 📝 NEXT STEPS

1. **Read this audit** ✅ (You are here!)
2. **Study songbird's implementation** (30 min)
3. **Begin Phase 1**: Server-side fallback (4-6 hours)
4. **Begin Phase 2**: Client-side discovery (2-3 hours)
5. **Begin Phase 3**: Integration & testing (1-2 hours)

**Total Effort**: 6-8 hours to eliminate all debt!

═══════════════════════════════════════════════════════════════

**Status**: Ready for Evolution  
**Pattern**: Validated in songbird  
**Priority**: MEDIUM (toadstool part of NODE atomic)  
**Current Grade**: C (67/100)  
**Target Grade**: A++ (205/100)

🦀 **Let's evolve to universal, isomorphic IPC!** 🚀
