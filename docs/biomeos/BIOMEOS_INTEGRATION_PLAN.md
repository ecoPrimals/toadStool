# 🐸 ToadStool → biomeOS Integration Evolution Plan

**Date**: January 10, 2026  
**Upstream Request**: biomeOS Team  
**Status**: ✅ **90% READY - Final 10% Planned**

---

## 📊 CURRENT STATE ANALYSIS

### What We Already Have ✅

**1. JSON-RPC 2.0 Server** (`crates/server/src/jsonrpc_server.rs`)
- ✅ Complete implementation using `jsonrpsee`
- ✅ 5 methods implemented (submit_workload, query_status, cancel_workload, list_workloads, query_capabilities)
- ✅ Type-safe with serde serialization
- ✅ Standard compliant (JSON-RPC 2.0)

**2. tarpc Server** (`crates/server/src/tarpc_server.rs`)  
- ✅ High-performance binary RPC
- ✅ Complete implementation
- ✅ TCP transport ready

**3. Server Library** (`crates/server/src/lib.rs`)
- ✅ HTTP/REST API (axum-based)
- ✅ WebSocket support
- ✅ Songbird/Squirrel registration capability
- ✅ Resource monitoring
- ✅ Background services

### What's Missing ⚠️ (The 10%)

**1. Unix Socket Transport**
- Current: TCP only (`TcpListener`)
- Needed: Unix domain sockets for local IPC
- Pattern: `/run/user/<uid>/toadstool-<family>.sock`

**2. Server Binary Entry Point**
- Current: Library only (no `src/main.rs`)
- Needed: Standalone daemon binary
- Pattern: Like Squirrel's daemon mode

**3. biomeOS-Specific Methods**
- Current: Generic compute methods
- Needed: biomeOS client interface alignment
- Methods: `get_resource_usage`, `deploy_workload`, `scale_service`, etc.

---

## 🎯 EVOLUTION ROADMAP

### Phase 1: Unix Socket Support (HIGH PRIORITY)

**Goal**: Enable JSON-RPC over Unix sockets

**Implementation**:
```rust
// Add to crates/server/src/jsonrpc_server.rs

use tokio::net::UnixListener;

/// Start JSON-RPC server on Unix socket
pub async fn start_jsonrpc_unix_server(
    socket_path: std::path::PathBuf,
    executor: Arc<dyn WorkloadExecutor + Send + Sync>,
    version: String,
) -> Result<ServerHandle, ServerError> {
    info!("Starting JSON-RPC server on Unix socket: {:?}", socket_path);
    
    // Ensure socket directory exists
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    
    // Remove old socket if exists
    let _ = tokio::fs::remove_file(&socket_path).await;
    
    // Create Unix listener
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
    
    // Create JSON-RPC server
    let impl_server = JsonRpcServerImpl::new(executor, version);
    let server = Server::builder()
        .build_with_tokio(listener)
        .await?;
    
    let handle = server.start(impl_server.into_rpc());
    
    info!("JSON-RPC server listening on Unix socket: {:?}", socket_path);
    Ok(handle)
}
```

**Files to Modify**:
- `crates/server/src/jsonrpc_server.rs` - Add Unix socket support
- `crates/server/Cargo.toml` - Add dependencies if needed

**Estimated Time**: 2-4 hours

---

### Phase 2: Server Binary (HIGH PRIORITY)

**Goal**: Create standalone daemon binary

**Implementation**:
```rust
// Create: crates/server/src/main.rs

use std::path::PathBuf;
use toadstool_server::{JsonRpcConfig, start_jsonrpc_unix_server};
use toadstool_config::ToadStoolConfig;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("🍄 ToadStool Universal Compute Server starting...");
    
    // Load configuration
    let config = ToadStoolConfig::from_env()?;
    let family_id = config.family_id.unwrap_or_else(|| "default".to_string());
    
    // Determine socket path: /run/user/<uid>/toadstool-<family>.sock
    let socket_path = get_socket_path(&family_id)?;
    info!("Socket path: {:?}", socket_path);
    
    // Create executor (workload handler)
    let executor = create_executor(config.clone()).await?;
    let version = env!("CARGO_PKG_VERSION").to_string();
    
    // Start JSON-RPC server on Unix socket
    let server_handle = start_jsonrpc_unix_server(
        socket_path.clone(),
        Arc::new(executor),
        version.clone()
    ).await?;
    
    // Register with Songbird if available
    if let Some(songbird_endpoint) = config.songbird_endpoint {
        register_with_songbird(&songbird_endpoint, &socket_path, &family_id).await?;
    }
    
    info!("✅ ToadStool server ready - listening on {:?}", socket_path);
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    
    info!("Shutting down ToadStool server...");
    server_handle.stop()?;
    
    // Cleanup socket
    let _ = tokio::fs::remove_file(&socket_path).await;
    
    info!("ToadStool server stopped");
    Ok(())
}

fn get_socket_path(family_id: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    use std::env;
    
    // Get UID for /run/user/<uid>/
    let uid = unsafe { libc::getuid() };
    let runtime_dir = env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", uid));
    
    let socket_path = PathBuf::from(runtime_dir)
        .join(format!("toadstool-{}.sock", family_id));
    
    Ok(socket_path)
}

async fn create_executor(
    config: ToadStoolConfig
) -> Result<impl toadstool_server::WorkloadExecutor, Box<dyn std::error::Error>> {
    // Create real executor implementation
    // This connects to ToadStool's runtime engines
    Ok(toadstool_server::RealExecutor::new(config).await?)
}

async fn register_with_songbird(
    songbird_endpoint: &str,
    socket_path: &PathBuf,
    family_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Registering with Songbird at {}", songbird_endpoint);
    
    use toadstool_common::primal_identity::{ServiceInfo, ServiceCapability};
    
    let service_info = ServiceInfo {
        name: "toadstool".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        family: family_id.to_string(),
        socket_path: socket_path.to_string_lossy().to_string(),
        protocol: "json-rpc-2.0".to_string(),
        capabilities: vec![
            ServiceCapability::Compute,
            ServiceCapability::GPU,
            ServiceCapability::Orchestration,
        ],
    };
    
    // Register via Songbird client
    // (Implementation depends on Songbird's client API)
    
    info!("✅ Registered with Songbird");
    Ok(())
}
```

**Files to Create**:
- `crates/server/src/main.rs` - Server binary entry point
- `crates/server/src/executor.rs` - Real executor implementation

**Files to Modify**:
- `crates/server/Cargo.toml` - Add `[[bin]]` section
- `crates/server/src/lib.rs` - Export necessary types

**Estimated Time**: 4-6 hours

---

### Phase 3: biomeOS Method Alignment (MEDIUM PRIORITY)

**Goal**: Align JSON-RPC methods with biomeOS client expectations

**biomeOS Expected Interface** (from their client):
```rust
// From biomeOS: crates/biomeos-core/src/clients/toadstool.rs

1. get_resource_usage() -> ResourceUsage
2. deploy_workload(name, workload_type, config) -> DeploymentResult
3. scale_service(service_id, replicas) -> ScaleResult
4. get_service_replicas(service_id) -> u32
5. get_service_status(service_id) -> ServiceStatus
```

**Current ToadStool Interface**:
```rust
1. submit_workload(submission) -> WorkloadResult
2. query_status(workload_id) -> WorkloadResult
3. cancel_workload(workload_id) -> bool
4. list_workloads() -> Vec<String>
5. query_capabilities() -> ComputeCapabilities
```

**Mapping Strategy**:

**Option A: Add Compatibility Methods** (RECOMMENDED)
```rust
// Add to jsonrpc_server.rs

#[rpc(server, namespace = "toadstool")]
pub trait ToadStoolBiomeOSRpc {
    /// Get current resource usage (biomeOS compatibility)
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
    
    /// Scale service replicas (biomeOS compatibility)
    #[method(name = "scale_service")]
    async fn scale_service(
        &self,
        service_id: String,
        replicas: u32,
    ) -> Result<ScaleResult, ErrorObjectOwned>;
    
    /// Get service replica count (biomeOS compatibility)
    #[method(name = "get_service_replicas")]
    async fn get_service_replicas(
        &self,
        service_id: String,
    ) -> Result<u32, ErrorObjectOwned>;
    
    /// Get service status (biomeOS compatibility)
    #[method(name = "get_service_status")]
    async fn get_service_status(
        &self,
        service_id: String,
    ) -> Result<ServiceStatus, ErrorObjectOwned>;
}
```

**Implementation**:
```rust
impl ToadStoolBiomeOSRpcServer for JsonRpcServerImpl {
    async fn get_resource_usage(&self) -> Result<ResourceUsage, ErrorObjectOwned> {
        // Map from internal resource monitor
        let monitor = &self.resource_monitor;
        let metrics = monitor.get_metrics().await?;
        
        Ok(ResourceUsage {
            cpu_percent: metrics.cpu_usage,
            gpu_percent: metrics.gpu_usage.unwrap_or(0.0),
            memory_percent: metrics.memory_usage,
            memory_used_mb: metrics.memory_used / 1024 / 1024,
            memory_total_mb: metrics.memory_total / 1024 / 1024,
            network_rx_mbps: metrics.network_rx,
            network_tx_mbps: metrics.network_tx,
        })
    }
    
    async fn deploy_workload(
        &self,
        name: String,
        workload_type: String,
        config: WorkloadConfig,
    ) -> Result<DeploymentResult, ErrorObjectOwned> {
        // Map to internal submit_workload
        let submission = JsonWorkloadSubmission {
            workload_id: format!("{}_{}", name, uuid::Uuid::new_v4()),
            workload_type,
            data: config.data,
            metadata: config.metadata,
            priority: config.priority.unwrap_or(WorkloadPriority::Normal),
            requirements: config.requirements.unwrap_or_default(),
        };
        
        let result = self.submit_workload(submission).await?;
        
        Ok(DeploymentResult {
            workload_id: result.workload_id,
            status: "deployed".to_string(),
        })
    }
    
    // ... implement remaining methods
}
```

**Files to Modify**:
- `crates/server/src/jsonrpc_server.rs` - Add biomeOS compatibility methods
- `crates/server/src/types.rs` (new) - biomeOS type definitions

**Estimated Time**: 4-6 hours

---

### Phase 4: Health Check Endpoint (LOW PRIORITY)

**Goal**: Add health check method

**Implementation**:
```rust
#[method(name = "health_check")]
async fn health_check(&self) -> Result<HealthStatus, ErrorObjectOwned> {
    Ok(HealthStatus {
        status: "healthy".to_string(),
        version: self.version.clone(),
        uptime_seconds: self.get_uptime(),
        active_workloads: self.get_active_count().await,
        resource_availability: self.get_resource_availability().await,
    })
}
```

**Estimated Time**: 1-2 hours

---

## 📋 COMPLETE IMPLEMENTATION CHECKLIST

### Phase 1: Unix Socket Support ⚠️
- [ ] Add `UnixListener` support to `jsonrpc_server.rs`
- [ ] Implement `start_jsonrpc_unix_server()` function
- [ ] Add socket path configuration
- [ ] Set proper Unix permissions (0o600)
- [ ] Test Unix socket connection

### Phase 2: Server Binary ⚠️
- [ ] Create `crates/server/src/main.rs`
- [ ] Implement `get_socket_path()` helper
- [ ] Implement executor creation
- [ ] Add Songbird registration
- [ ] Add signal handling (graceful shutdown)
- [ ] Update Cargo.toml with `[[bin]]`
- [ ] Test daemon startup

### Phase 3: biomeOS Method Alignment ⚠️
- [ ] Define biomeOS-compatible types
- [ ] Implement `get_resource_usage()`
- [ ] Implement `deploy_workload()`
- [ ] Implement `scale_service()`
- [ ] Implement `get_service_replicas()`
- [ ] Implement `get_service_status()`
- [ ] Test with biomeOS client

### Phase 4: Health Check ✅ (Can be added later)
- [ ] Implement `health_check()` method
- [ ] Add uptime tracking
- [ ] Add resource availability checks

---

## 🎯 PRIORITY & TIMELINE

### Week 1 (Jan 10-17): Unix Socket + Server Binary
**Priority**: HIGH  
**Blocking**: Yes - Required for any biomeOS integration

**Tasks**:
1. Add Unix socket support (2-4 hours)
2. Create server binary (4-6 hours)
3. Test locally (2 hours)
4. Document usage (1 hour)

**Total**: ~10-13 hours

### Week 2 (Jan 18-24): biomeOS Method Alignment
**Priority**: HIGH  
**Blocking**: Yes - Required for biomeOS client compatibility

**Tasks**:
1. Define biomeOS types (2 hours)
2. Implement 5 compatibility methods (4-6 hours)
3. Integration testing with biomeOS (2-3 hours)
4. Fix any issues (2 hours)

**Total**: ~10-13 hours

### Week 3 (Jan 25-31): Polish & Production
**Priority**: MEDIUM  
**Blocking**: No - Nice to have

**Tasks**:
1. Add health check (1-2 hours)
2. Performance tuning (2 hours)
3. Documentation updates (2 hours)
4. Production deployment (2 hours)

**Total**: ~7-8 hours

---

## 🚀 EXPECTED OUTCOME

### After Week 2: FULL BIOMEOS INTEGRATION ✅

**ToadStool will provide**:
1. ✅ JSON-RPC 2.0 server on Unix socket
2. ✅ Socket path: `/run/user/<uid>/toadstool-<family>.sock`
3. ✅ 5 biomeOS-compatible methods
4. ✅ Songbird registration on startup
5. ✅ Health check endpoint
6. ✅ Graceful shutdown

**biomeOS will be able to**:
1. ✅ Discover ToadStool via Songbird
2. ✅ Connect via Unix socket
3. ✅ Query resource usage
4. ✅ Deploy compute workloads
5. ✅ Scale services
6. ✅ Monitor health

---

## 📚 REFERENCE IMPLEMENTATIONS

### Squirrel (EXCELLENT Pattern)
**Location**: `/home/eastgate/Development/ecoPrimals/phase1/squirrel/`

**What to copy**:
- Dual protocol (tarpc + JSON-RPC)
- Unix socket setup
- Songbird registration
- Daemon mode

### biomeOS ToadStoolClient (Target Interface)
**Location**: `crates/biomeos-core/src/clients/toadstool.rs`

**What to match**:
- Method signatures
- Type definitions
- Error handling
- Discovery pattern

---

## 🎊 CONCLUSION

**Current Status**: ✅ 90% READY

**Gap**: 10% - Unix socket + server binary + biomeOS methods

**Estimated Time**: 2-3 weeks to full integration

**Next Action**: Proceed with Phase 1 implementation

---

*"CPU, GPU, Neuromorphic - Different orders of the same architecture."* ✅

**ToadStool: Ready to integrate with biomeOS ecosystem!** 🐸🍄

