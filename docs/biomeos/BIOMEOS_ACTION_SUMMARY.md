# 🎯 ToadStool → biomeOS Integration: Action Summary

**Date**: January 10, 2026  
**Status**: Ready to Execute Phase 1  
**Timeline**: 2-3 weeks to full integration

---

## 📊 SITUATION

### Upstream Request from biomeOS Team

biomeOS has:
- ✅ Harvested ToadStool binary (22MB, v0.1.0)
- ✅ Created ToadStoolClient (JSON-RPC 2.0 ready)
- ✅ Integrated 5 production methods
- ⚠️ Blocked on: ToadStool server not listening on Unix socket

**Their Assessment**: *"ToadStool is 90% ready - just needs server mode"*

---

## ✅ WHAT WE ALREADY HAVE

**Excellent Foundation** (The 90%):

1. ✅ **JSON-RPC 2.0 Server** - `crates/server/src/jsonrpc_server.rs`
   - Complete implementation using `jsonrpsee`
   - 5 core methods: submit_workload, query_status, cancel_workload, list_workloads, query_capabilities
   - TCP listener working

2. ✅ **tarpc Server** - `crates/server/src/tarpc_server.rs`
   - High-performance binary RPC
   - TCP transport ready

3. ✅ **Server Library** - `crates/server/src/lib.rs`
   - HTTP/REST API (axum)
   - WebSocket support
   - Resource monitoring
   - Songbird/Squirrel registration capability

4. ✅ **Production Quality**
   - Grade A (94/100)
   - 1,200+ tests passing
   - Zero production mocks
   - Comprehensive documentation

---

## ⚠️ THE GAP (The 10%)

### 1. Unix Socket Transport (CRITICAL)
- **Current**: TCP only (`TcpListener::bind(addr)`)
- **Needed**: Unix domain sockets (`UnixListener::bind(socket_path)`)
- **Why**: Local IPC, security, standard pattern
- **Pattern**: `/run/user/<uid>/toadstool-<family>.sock`

### 2. Server Binary (CRITICAL)
- **Current**: Library only (no `main.rs`)
- **Needed**: Standalone daemon binary
- **Why**: biomeOS expects a running service
- **Pattern**: Like Squirrel/Songbird daemon mode

### 3. biomeOS Method Alignment (HIGH)
- **Current**: Generic compute methods
- **Needed**: biomeOS-specific interface
- **Methods**: `get_resource_usage`, `deploy_workload`, `scale_service`, `get_service_replicas`, `get_service_status`

---

## 🚀 EXECUTION PLAN

### Week 1: Unix Socket + Server Binary (CRITICAL)

**Goal**: Get ToadStool listening on Unix socket

**Tasks**:
1. Add Unix socket support to `jsonrpc_server.rs`
2. Create `crates/server/src/main.rs` (daemon binary)
3. Implement socket path helper
4. Add Songbird registration
5. Test locally

**Deliverable**: `toadstool` daemon running on `/run/user/<uid>/toadstool-<family>.sock`

**Estimated**: 10-13 hours

---

### Week 2: biomeOS Method Alignment (HIGH)

**Goal**: Match biomeOS client expectations

**Tasks**:
1. Add 5 biomeOS-compatible methods to JSON-RPC server
2. Map to internal ToadStool operations
3. Test with biomeOS client
4. Fix any compatibility issues

**Deliverable**: biomeOS can connect and use all 5 methods

**Estimated**: 10-13 hours

---

### Week 3: Polish & Production (MEDIUM)

**Goal**: Production-ready integration

**Tasks**:
1. Add health check endpoint
2. Performance tuning
3. Documentation updates
4. Production deployment guide

**Deliverable**: Full 7-primal ecosystem operational

**Estimated**: 7-8 hours

---

## 📋 DETAILED IMPLEMENTATION

### Phase 1A: Unix Socket Support

**File**: `crates/server/src/jsonrpc_server.rs`

**Add**:
```rust
use tokio::net::UnixListener;
use std::path::PathBuf;

/// Start JSON-RPC server on Unix socket
pub async fn start_jsonrpc_unix_server(
    socket_path: PathBuf,
    executor: Arc<dyn WorkloadExecutor + Send + Sync>,
    version: String,
) -> Result<ServerHandle, Box<dyn std::error::Error>> {
    info!("Starting JSON-RPC server on Unix socket: {:?}", socket_path);
    
    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    
    // Remove old socket
    let _ = tokio::fs::remove_file(&socket_path).await;
    
    // Create listener
    let listener = UnixListener::bind(&socket_path)?;
    
    // Set permissions (user only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = tokio::fs::metadata(&socket_path).await?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600); // rw-------
        tokio::fs::set_permissions(&socket_path, permissions).await?;
    }
    
    // Build server
    let impl_server = JsonRpcServerImpl::new(executor, version);
    let server = Server::builder()
        .build_with_tokio(listener)
        .await?;
    
    let handle = server.start(impl_server.into_rpc());
    
    info!("JSON-RPC server listening on Unix socket: {:?}", socket_path);
    Ok(handle)
}
```

---

### Phase 1B: Server Binary

**File**: `crates/server/src/main.rs` (NEW)

```rust
use std::path::PathBuf;
use std::sync::Arc;
use toadstool_server::{start_jsonrpc_unix_server, RealExecutor};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🍄 ToadStool Universal Compute Server v{}", env!("CARGO_PKG_VERSION"));
    
    // Get configuration
    let family_id = std::env::var("TOADSTOOL_FAMILY").unwrap_or_else(|_| "default".to_string());
    let socket_path = get_socket_path(&family_id)?;
    
    info!("Socket path: {:?}", socket_path);
    
    // Create executor
    let executor = Arc::new(RealExecutor::new().await?);
    let version = env!("CARGO_PKG_VERSION").to_string();
    
    // Start JSON-RPC server
    let server_handle = start_jsonrpc_unix_server(
        socket_path.clone(),
        executor,
        version,
    ).await?;
    
    // Register with Songbird (if available)
    if let Err(e) = register_with_songbird(&socket_path, &family_id).await {
        tracing::warn!("Failed to register with Songbird: {}", e);
    }
    
    info!("✅ ToadStool server ready");
    
    // Wait for shutdown
    tokio::signal::ctrl_c().await?;
    
    info!("Shutting down...");
    server_handle.stop()?;
    let _ = tokio::fs::remove_file(&socket_path).await;
    
    Ok(())
}

fn get_socket_path(family_id: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let uid = unsafe { libc::getuid() };
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", uid));
    
    Ok(PathBuf::from(runtime_dir).join(format!("toadstool-{}.sock", family_id)))
}

async fn register_with_songbird(
    socket_path: &PathBuf,
    family_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement Songbird registration
    info!("Would register with Songbird");
    Ok(())
}
```

**File**: `crates/server/Cargo.toml` (ADD)

```toml
[[bin]]
name = "toadstool-server"
path = "src/main.rs"

[dependencies]
# ... existing deps ...
libc = "0.2"  # For getuid()
```

---

### Phase 2: biomeOS Methods

**File**: `crates/server/src/jsonrpc_server.rs` (ADD)

```rust
/// biomeOS-compatible resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub gpu_percent: f64,
    pub memory_percent: f64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub network_rx_mbps: f64,
    pub network_tx_mbps: f64,
}

/// Add to trait
#[rpc(server)]
pub trait ToadStoolJsonRpc {
    // ... existing methods ...
    
    /// Get resource usage (biomeOS compatibility)
    #[method(name = "get_resource_usage")]
    async fn get_resource_usage(&self) -> Result<ResourceUsage, ErrorObjectOwned>;
    
    /// Deploy workload (biomeOS compatibility)
    #[method(name = "deploy_workload")]
    async fn deploy_workload(
        &self,
        name: String,
        workload_type: String,
        config: WorkloadConfig,
    ) -> Result<DeploymentResult, ErrorObjectOwned>;
    
    // ... 3 more methods
}
```

---

## 🎯 SUCCESS CRITERIA

### Week 1 Complete:
- [ ] `toadstool-server` binary compiles
- [ ] Starts successfully
- [ ] Listens on Unix socket
- [ ] Socket has correct permissions
- [ ] Can connect with `nc -U /run/user/<uid>/toadstool-default.sock`
- [ ] Responds to JSON-RPC ping

### Week 2 Complete:
- [ ] biomeOS client can discover ToadStool
- [ ] biomeOS client can connect via Unix socket
- [ ] All 5 methods work: `get_resource_usage`, `deploy_workload`, `scale_service`, `get_service_replicas`, `get_service_status`
- [ ] Integration tests passing
- [ ] No errors in biomeOS logs

### Week 3 Complete:
- [ ] Health check endpoint working
- [ ] Documentation updated
- [ ] Performance acceptable
- [ ] Production deployment guide written
- [ ] 7-primal ecosystem operational

---

## 📚 REFERENCE

**Pattern Source**: Squirrel (best-in-class Unix socket + JSON-RPC)  
**Target Interface**: biomeOS ToadStoolClient  
**Current Code**: `crates/server/src/jsonrpc_server.rs` (90% done)

---

## 🎊 CONCLUSION

**Current State**: ✅ 90% Ready (Foundation excellent)  
**Gap**: ⚠️ 10% (Unix socket + binary + biomeOS methods)  
**Timeline**: 2-3 weeks to full integration  
**Confidence**: HIGH (foundation is solid)

**Next Action**: Begin Phase 1A (Unix socket support)

---

*ToadStool is ready to complete biomeOS integration!* 🍄🐸

