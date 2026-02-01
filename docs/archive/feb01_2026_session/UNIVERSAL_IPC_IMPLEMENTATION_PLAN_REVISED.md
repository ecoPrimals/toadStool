# 🌍 Universal IPC Implementation Plan (REVISED)

**Date**: January 19, 2026 (REVISED after architecture review)  
**Status**: READY TO EXECUTE (Service-Based Approach)  
**Priority**: High (Foundation for True Universality)  
**Revision**: Architecture review identified cross-embedding issue - switching to service-based approach

---

## ⚠️ **CRITICAL REVISION**

**Original Plan**: Library-based (`use songbird_universal_ipc`)  
**Issue Found**: Cross-embedding violates primal autonomy  
**New Plan**: Service-based (Songbird as discovery service)  

**See**: `SONGBIRD_IPC_ARCHITECTURE_REVIEW_JAN_19_2026.md` for detailed analysis

---

## 🎯 EXECUTIVE SUMMARY

**Current State**: We already have 90% of the foundation!
- ✅ Pure Rust Unix socket JSON-RPC (`toadstool-common`)
- ✅ Capability-based discovery (BearDog pattern)
- ✅ Environment-based configuration (no hardcoding!)
- ✅ `tokio::net::UnixStream` (works on Unix AND Windows!)

**What We Need**: Songbird as IPC discovery service (NOT library!)

**Estimated Effort**: 8-10 hours total (REDUCED from 15-20!)
- Songbird JSON-RPC service: 4-5 hours
- wateringHole protocol spec: 2-3 hours  
- Per-primal implementation: 2-3 hours each (simple!)

---

## 📊 THE PATTERN: Services, Not Libraries

### **❌ WRONG: Library-Based (Cross-Embedding)**

```rust
// In Squirrel:
use songbird_universal_ipc::ipc;  // ❌ Embeds Songbird code!

let stream = ipc::connect("/primal/beardog").await?;
```

**Issues**:
- ❌ Squirrel depends on Songbird code
- ❌ Version coupling
- ❌ Not autonomous organisms
- ❌ Violates Deep Debt principles

---

### **✅ CORRECT: Service-Based (Autonomy Maintained)**

```rust
// In Squirrel (NO Songbird import!):
use tokio::net::UnixStream;  // Standard tokio API

// 1. Discover BearDog via Songbird service
let songbird = UnixStream::connect("/primal/songbird").await?;
let request = json!({
    "jsonrpc": "2.0",
    "method": "ipc.resolve",
    "params": { "primal": "beardog" },
    "id": 1
});
write_json_rpc(&songbird, &request).await?;

// 2. Get endpoint
let response = read_json_rpc(&songbird).await?;
let endpoint = response.result.endpoint;  // "/primal/beardog"

// 3. Connect directly
let beardog = UnixStream::connect(&endpoint).await?;
```

**Benefits**:
- ✅ Zero code embedding
- ✅ Each primal is autonomous
- ✅ Standard protocol
- ✅ Deep Debt compliant

---

## 🏗️ IMPLEMENTATION PHASES (REVISED)

### **Phase 1: Songbird Internal IPC** (2-3 hours)

**Goal**: Refactor existing Songbird IPC for internal use only

#### **Step 1.1: Rename & Internalize** (30 min)

```bash
# In Songbird repository:
mv crates/universal-ipc crates/ipc-internal
```

```toml
# crates/ipc-internal/Cargo.toml
[package]
name = "songbird-ipc-internal"  # NOT public!
version = "0.1.0"
publish = false  # ⚠️ IMPORTANT: Never publish!

[lib]
# Internal use only - NOT exported from main songbird crate
```

#### **Step 1.2: Keep Excellent Implementation** (Already done!)

Songbird's existing implementation is **excellent**:
- ✅ Platform abstraction trait
- ✅ Unix socket implementation
- ✅ Service registry
- ✅ Capability-based discovery

**Just keep it internal!**

---

### **Phase 2: Songbird JSON-RPC Service** (4-5 hours)

**Goal**: Expose IPC functionality as JSON-RPC service

#### **Step 2.1: Define Service Methods** (1 hour)

```rust
// crates/songbird-server/src/ipc_service.rs

pub struct IpcService {
    registry: Arc<RwLock<ServiceRegistry>>,
    platform_ipc: Box<dyn PlatformIPC>,
}

impl IpcService {
    /// Register a primal service
    /// Method: "ipc.register"
    pub async fn register(
        &self,
        primal_name: String,
        capabilities: Vec<String>,
        endpoint: Option<String>,
    ) -> Result<RegisterResponse> {
        let endpoint = endpoint.unwrap_or_else(|| {
            format!("/primal/{}", primal_name)
        });
        
        self.registry.write().await.insert(
            primal_name.clone(),
            ServiceInfo {
                endpoint,
                capabilities,
                last_seen: Instant::now(),
            },
        );
        
        Ok(RegisterResponse { endpoint })
    }
    
    /// Resolve a primal's endpoint
    /// Method: "ipc.resolve"
    pub async fn resolve(
        &self,
        primal_name: String,
    ) -> Result<ResolveResponse> {
        let registry = self.registry.read().await;
        let service = registry.get(&primal_name)
            .ok_or_else(|| Error::NotFound)?;
        
        Ok(ResolveResponse {
            endpoint: service.endpoint.clone(),
            capabilities: service.capabilities.clone(),
        })
    }
    
    /// List all registered services
    /// Method: "ipc.list"
    pub async fn list(&self) -> Result<Vec<ServiceInfo>> {
        Ok(self.registry.read().await.values().cloned().collect())
    }
    
    /// Find services by capability
    /// Method: "ipc.capabilities"
    pub async fn find_by_capability(
        &self,
        capability: String,
    ) -> Result<Vec<ServiceInfo>> {
        Ok(self.registry.read().await
            .values()
            .filter(|s| s.capabilities.contains(&capability))
            .cloned()
            .collect())
    }
}
```

#### **Step 2.2: Integrate with JSON-RPC Server** (2-3 hours)

```rust
// crates/songbird-server/src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    let ipc_service = Arc::new(IpcService::new());
    
    let rpc_server = ManualJsonRpcServer::new("/primal/songbird");
    
    // Register methods
    rpc_server.register_method("ipc.register", {
        let service = Arc::clone(&ipc_service);
        move |params| {
            let service = Arc::clone(&service);
            async move {
                service.register(
                    params.primal_name,
                    params.capabilities,
                    params.endpoint,
                ).await
            }
        }
    });
    
    rpc_server.register_method("ipc.resolve", {
        let service = Arc::clone(&ipc_service);
        move |params| {
            let service = Arc::clone(&service);
            async move {
                service.resolve(params.primal_name).await
            }
        }
    });
    
    // ... register other methods ...
    
    rpc_server.serve().await?;
    Ok(())
}
```

#### **Step 2.3: Add Tests** (1 hour)

```rust
#[tokio::test]
async fn test_register_and_resolve() {
    let service = IpcService::new();
    
    // Register BearDog
    let response = service.register(
        "beardog".to_string(),
        vec!["crypto".to_string()],
        None,
    ).await.unwrap();
    
    assert_eq!(response.endpoint, "/primal/beardog");
    
    // Resolve BearDog
    let resolved = service.resolve("beardog".to_string()).await.unwrap();
    assert_eq!(resolved.endpoint, "/primal/beardog");
    assert_eq!(resolved.capabilities, vec!["crypto"]);
}
```

---

### **Phase 3: wateringHole Protocol Spec** (2-3 hours)

**Goal**: Document standard IPC protocol for all primals

#### **Step 3.1: Create Protocol Spec** (2 hours)

```markdown
# wateringHole/PRIMAL_IPC_PROTOCOL.md

# Primal IPC Protocol v1.0

## 1. Transport Layer

**Standard**: `tokio::net::UnixStream`

- Unix/Linux: Unix domain sockets
- Windows: Named pipes (via tokio's UnixStream API)
- macOS/iOS: Unix domain sockets

**Benefits**: Zero platform-specific code!

## 2. Path Conventions

**Standard Namespace**: `/primal/<name>`

Examples:
- Songbird: `/primal/songbird`
- BearDog: `/primal/beardog`
- Squirrel: `/primal/squirrel`

**Environment Override**: `<PRIMAL>_SOCKET`

Examples:
- `SONGBIRD_SOCKET=/custom/path.sock`
- `BEARDOG_SOCKET=/tmp/beardog.sock`

## 3. Message Format

**Standard**: JSON-RPC 2.0

```json
{
  "jsonrpc": "2.0",
  "method": "method.name",
  "params": { ... },
  "id": 1
}
```

## 4. Songbird Discovery Service

### Method: `ipc.register`

Register a primal service.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "ipc.register",
  "params": {
    "primal_name": "beardog",
    "capabilities": ["crypto", "encryption"],
    "endpoint": "/primal/beardog"
  },
  "id": 1
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "endpoint": "/primal/beardog",
    "registered_at": "2026-01-19T12:00:00Z"
  },
  "id": 1
}
```

### Method: `ipc.resolve`

Resolve a primal's endpoint.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "ipc.resolve",
  "params": {
    "primal_name": "beardog"
  },
  "id": 1
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "endpoint": "/primal/beardog",
    "capabilities": ["crypto", "encryption"],
    "available": true
  },
  "id": 1
}
```

### Method: `ipc.list`

List all registered services.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "ipc.list",
  "params": {},
  "id": 1
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "services": [
      {
        "primal_name": "beardog",
        "endpoint": "/primal/beardog",
        "capabilities": ["crypto"],
        "last_seen": "2026-01-19T12:00:00Z"
      },
      ...
    ]
  },
  "id": 1
}
```

### Method: `ipc.capabilities`

Find services by capability.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "ipc.capabilities",
  "params": {
    "capability": "crypto"
  },
  "id": 1
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "services": [
      {
        "primal_name": "beardog",
        "endpoint": "/primal/beardog",
        "capabilities": ["crypto", "encryption"]
      }
    ]
  },
  "id": 1
}
```

## 5. Reference Implementation

See `examples/primal_ipc_reference.rs` for complete working example.
```

#### **Step 3.2: Create Reference Implementation** (1 hour)

```rust
// wateringHole/examples/primal_ipc_reference.rs

use tokio::net::UnixStream;
use serde_json::{json, Value};

/// Register with Songbird discovery service
pub async fn register_primal(
    primal_name: &str,
    capabilities: Vec<String>,
) -> Result<String> {
    // Connect to Songbird
    let mut stream = UnixStream::connect("/primal/songbird").await?;
    
    // Send registration request
    let request = json!({
        "jsonrpc": "2.0",
        "method": "ipc.register",
        "params": {
            "primal_name": primal_name,
            "capabilities": capabilities,
        },
        "id": 1
    });
    
    write_json_rpc(&mut stream, &request).await?;
    let response: Value = read_json_rpc(&mut stream).await?;
    
    Ok(response["result"]["endpoint"].as_str().unwrap().to_string())
}

/// Resolve a primal's endpoint
pub async fn resolve_primal(primal_name: &str) -> Result<String> {
    let mut stream = UnixStream::connect("/primal/songbird").await?;
    
    let request = json!({
        "jsonrpc": "2.0",
        "method": "ipc.resolve",
        "params": { "primal_name": primal_name },
        "id": 1
    });
    
    write_json_rpc(&mut stream, &request).await?;
    let response: Value = read_json_rpc(&mut stream).await?;
    
    Ok(response["result"]["endpoint"].as_str().unwrap().to_string())
}

/// Find primals by capability
pub async fn find_by_capability(capability: &str) -> Result<Vec<ServiceInfo>> {
    let mut stream = UnixStream::connect("/primal/songbird").await?;
    
    let request = json!({
        "jsonrpc": "2.0",
        "method": "ipc.capabilities",
        "params": { "capability": capability },
        "id": 1
    });
    
    write_json_rpc(&mut stream, &request).await?;
    let response: Value = read_json_rpc(&mut stream).await?;
    
    serde_json::from_value(response["result"]["services"].clone())
}
```

---

### **Phase 4: Per-Primal Implementation** (2-3 hours each)

**Goal**: Each primal implements standard protocol

#### **Step 4.1: ToadStool Implementation** (2 hours)

```rust
// crates/core/toadstool/src/ipc_helpers.rs

use tokio::net::UnixStream;
use serde_json::json;

/// Register ToadStool with Songbird
pub async fn register_with_songbird() -> Result<()> {
    let mut songbird = UnixStream::connect("/primal/songbird").await?;
    
    let request = json!({
        "jsonrpc": "2.0",
        "method": "ipc.register",
        "params": {
            "primal_name": "toadstool",
            "capabilities": ["compute", "gpu", "wasm"],
        },
        "id": 1
    });
    
    write_json_rpc(&mut songbird, &request).await?;
    let _response = read_json_rpc(&mut songbird).await?;
    
    info!("✅ Registered with Songbird discovery service");
    Ok(())
}

/// Resolve another primal's endpoint
pub async fn resolve_primal(name: &str) -> Result<String> {
    let mut songbird = UnixStream::connect("/primal/songbird").await?;
    
    let request = json!({
        "jsonrpc": "2.0",
        "method": "ipc.resolve",
        "params": { "primal_name": name },
        "id": 1
    });
    
    write_json_rpc(&mut songbird, &request).await?;
    let response = read_json_rpc(&mut songbird).await?;
    
    Ok(response["result"]["endpoint"].as_str().unwrap().to_string())
}

/// Connect to another primal
pub async fn connect_to_primal(name: &str) -> Result<UnixStream> {
    let endpoint = resolve_primal(name).await?;
    let stream = UnixStream::connect(&endpoint).await?;
    Ok(stream)
}
```

#### **Step 4.2: Update Server to Register** (30 min)

```rust
// crates/server/src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().init();
    
    // Register with Songbird (if available)
    if let Err(e) = ipc_helpers::register_with_songbird().await {
        warn!("Could not register with Songbird: {}", e);
        warn!("Operating in standalone mode");
    }
    
    // Start server
    toadstool_server::run_server_main().await
}
```

#### **Step 4.3: Use in Integrations** (30 min)

```rust
// crates/integration/beardog/src/discovery.rs

pub async fn discover_entropy() -> Result<EntropyClient> {
    // Try to resolve via Songbird first
    match ipc_helpers::resolve_primal("beardog").await {
        Ok(endpoint) => {
            let client = UnixJsonRpcClient::new(endpoint);
            Ok(EntropyClient { client, available: true })
        }
        Err(_) => {
            // Fallback to direct connection
            let socket_path = primal_sockets::get_beardog_socket_path();
            let client = UnixJsonRpcClient::new(socket_path);
            Ok(EntropyClient { client, available: false })
        }
    }
}
```

---

## 📊 COMPARISON: Old vs New

### **Old Plan (Library-Based)**

```rust
// ❌ Cross-embedding issue
use songbird_universal_ipc::ipc;
let stream = ipc::connect("/primal/beardog").await?;
```

**Effort**: 15-20 hours  
**Issues**: Cross-embedding, tight coupling, version lock

---

### **New Plan (Service-Based)**

```rust
// ✅ Autonomous organisms
use tokio::net::UnixStream;

let endpoint = resolve_primal("beardog").await?;
let stream = UnixStream::connect(&endpoint).await?;
```

**Effort**: 8-10 hours (REDUCED!)  
**Benefits**: Autonomy, loose coupling, standard protocol

---

## 🎯 BENEFITS OF SERVICE-BASED APPROACH

### **For Primals**:
1. ✅ **Autonomy**: Each primal owns its code
2. ✅ **Simplicity**: ~50 lines of helper code
3. ✅ **Flexibility**: Can evolve independently
4. ✅ **Standard API**: `tokio::net::UnixStream` (familiar!)

### **For Songbird**:
1. ✅ **Keep excellent work**: Internal implementation preserved
2. ✅ **Service provision**: Songbird provides discovery, not code
3. ✅ **Clear role**: Registry and broker, not library

### **For Ecosystem**:
1. ✅ **TRUE PRIMAL pattern**: Services, not libraries
2. ✅ **Deep Debt compliance**: Self-knowledge, capability-based
3. ✅ **Scalability**: Each primal can evolve
4. ✅ **Standard protocol**: wateringHole specification

---

## 🚀 EXECUTION PLAN

### **Immediate** (This Week):
1. ✅ Document architecture review (DONE!)
2. 🔴 Create wateringHole protocol spec (2-3 hours)
3. 🔴 Share with Songbird team (coordination)

### **Short-Term** (Next 1-2 Weeks):
4. Songbird: Refactor to service-based (4-5 hours)
5. ToadStool: Implement standard protocol (2 hours)
6. BearDog: Implement standard protocol (2 hours)

### **Medium-Term** (Next Month):
7. Roll out to all primals (2-3 hours each)
8. E2E testing across ecosystem
9. Documentation and examples

---

## 📋 NEXT STEPS FOR TOADSTOOL

### **1. Document & Share** (DONE! ✅)
- ✅ Create architecture review
- ✅ Update implementation plan
- 🔴 Share with Songbird team

### **2. Implement Pattern** (2 hours)
- Create `ipc_helpers.rs`
- Implement register/resolve functions
- Update server to register on startup

### **3. Update Integrations** (1 hour)
- Update BearDog discovery
- Update any other primal connections
- Add fallback for standalone mode

---

## 🎊 SUMMARY

**Old Approach**: Library-based (cross-embedding issue)  
**New Approach**: Service-based (primal autonomy)  
**Effort**: REDUCED from 15-20h to 8-10h  
**Benefits**: TRUE PRIMAL pattern, Deep Debt compliant  
**Status**: Ready to execute!

---

**Document**: UNIVERSAL_IPC_IMPLEMENTATION_PLAN_REVISED.md  
**Date**: January 19, 2026  
**Status**: ✅ **READY TO EXECUTE (Service-Based)**  
**See Also**: `SONGBIRD_IPC_ARCHITECTURE_REVIEW_JAN_19_2026.md`

🌍🦀✨ **Services, not libraries - True primal autonomy!** ✨🦀🌍
