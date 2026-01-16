# Primal Knowledge Evolution - Deep Debt Resolution

**Date**: January 16, 2026  
**Discovery**: While reviewing NestGate handoff  
**Issue**: Code still contains hardcoded primal knowledge  
**Solution**: Evolve to pure capability-based infrastructure  

---

## 🔍 DEEP DEBT IDENTIFIED

### **Issue**: Hardcoded Primal Names

**Location**: `crates/core/common/src/primal_sockets.rs`

**Problem**:
```rust
// ❌ DEEP DEBT: Hardcoded primal knowledge
pub fn get_nestgate_socket_path() -> PathBuf { ... }
pub fn get_beardog_socket_path() -> PathBuf { ... }
pub fn get_songbird_socket_path() -> PathBuf { ... }
pub fn get_squirrel_socket_path() -> PathBuf { ... }
```

**Violation**: TRUE PRIMAL principle - each primal should only know itself!

---

### **Issue**: StorageClient Still Uses Hardcoded Path

**Location**: `crates/integration/nestgate/src/client.rs:163`

**Problem**:
```rust
// ❌ DEEP DEBT: Hardcoded "nestgate" knowledge
pub async fn with_config(config: NestGateConfig) -> NestGateResult<Self> {
    let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(
        toadstool_common::primal_sockets::get_nestgate_socket_path()  // ❌ Hardcoded!
    );
    ...
    service_name: "nestgate".to_string(),  // ❌ Hardcoded!
}
```

---

## ✅ WHAT WE HAVE (Already Good!)

### **1. Capability Infrastructure** ✅

```rust
// ✅ GOOD: Capability-based discovery
pub enum Capability {
    Storage(StorageCapability),
    Crypto(CryptoCapability),
    Compute(ComputeCapability),
    // ...
}
```

### **2. Generic Socket Discovery** ✅

```rust
// ✅ GOOD: Vendor-agnostic socket path resolution
pub fn get_socket_path_for_service(service_name: &str) -> PathBuf {
    match service_name.to_lowercase().as_str() {
        "beardog" | "bear-dog" => get_beardog_socket_path(),
        // ... existing mappings
        _ => {
            // Generic pattern for ANY service!
            let runtime_dir = get_runtime_dir();
            let family = get_family_id();
            PathBuf::from(&runtime_dir).join(format!("{}-{}.sock", service_name, family))
        }
    }
}
```

### **3. StorageClient::discover()** ✅

```rust
// ✅ EXCELLENT: Capability-based discovery (already implemented!)
pub async fn discover() -> NestGateResult<Self> {
    Self::discover_with_capability(
        Capability::Storage(StorageCapability::ArtifactStorage)
    ).await
}
```

**This is PERFECT TRUE PRIMAL architecture!**

---

## 🎯 EVOLUTION PLAN

### **Phase 1: Deprecate Hardcoded Functions**

**Action**: Mark primal-specific socket functions as deprecated

**Files**: `crates/core/common/src/primal_sockets.rs`

**Changes**:
```rust
/// Get BearDog unix socket path
///
/// **DEPRECATED**: Use `get_socket_path_for_service("beardog")` instead
/// This function violates TRUE PRIMAL self-knowledge principle.
#[deprecated(
    since = "4.9.0",
    note = "Use capability-based discovery + get_socket_path_for_service() instead"
)]
pub fn get_beardog_socket_path() -> PathBuf { ... }
```

**Rationale**: These functions exist for backward compatibility but should not be used in new code.

---

### **Phase 2: Fix StorageClient::with_config()**

**Action**: Remove hardcoded "nestgate" knowledge

**File**: `crates/integration/nestgate/src/client.rs`

**Current** (Line 163):
```rust
pub async fn with_config(config: NestGateConfig) -> NestGateResult<Self> {
    // ❌ Hardcoded "nestgate" knowledge
    let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(
        toadstool_common::primal_sockets::get_nestgate_socket_path()
    );
    
    let client = Self {
        rpc_client,
        config,
        service_name: "nestgate".to_string(),  // ❌ Hardcoded!
    };
    ...
}
```

**Evolved**:
```rust
pub async fn with_config(config: NestGateConfig, service_name: Option<String>) -> NestGateResult<Self> {
    // ✅ Use provided service name or discover
    let service_name = service_name.unwrap_or_else(|| {
        // Try to discover via capability
        // Fallback to "nestgate" if discovery not available
        "nestgate".to_string()
    });
    
    // ✅ Generic socket path resolution
    let socket_path = toadstool_common::primal_sockets::get_socket_path_for_service(&service_name);
    let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);
    
    let client = Self {
        rpc_client,
        config,
        service_name,  // ✅ Dynamic!
    };
    ...
}
```

---

### **Phase 3: Update Documentation**

**Action**: Clarify preferred patterns

**Files**: 
- `README.md`
- `crates/integration/nestgate/README.md`
- `primal-capabilities.toml`

**Message**:
- ✅ **Preferred**: Use `StorageClient::discover()` for capability-based discovery
- ⚠️ **Acceptable**: Use `StorageClient::connect(endpoint)` with discovered service name
- ❌ **Avoid**: Direct socket path functions (deprecated)

---

## 💡 PHILOSOPHY: TRUE PRIMAL ARCHITECTURE

### **What ToadStool Should Know**

✅ **Self-Knowledge**:
- "I am ToadStool"
- "I provide compute capabilities"
- "I need storage capabilities"
- "My socket is at `get_toadstool_socket_path()`"

✅ **Discovery Knowledge**:
- "I can discover services by capability"
- "I can connect via unix sockets"
- "I can fallback to environment variables"

### **What ToadStool Should NOT Know**

❌ **Primal-Specific Knowledge**:
- "NestGate provides storage" ← Generic: "ANY service with storage:artifact"
- "NestGate socket is at X" ← Dynamic: "Discovered service socket is at X"
- "BearDog provides crypto" ← Generic: "ANY service with crypto:operations"

---

## 🎊 BENEFITS OF EVOLUTION

### **1. Vendor Flexibility**

**Before** (Hardcoded):
```rust
// Can ONLY use NestGate
let client = StorageClient::with_config(config).await?;
// Hardcoded to: /run/user/1000/nestgate-default.sock
```

**After** (Capability-Based):
```rust
// Can use NestGate, MinIO, S3, GCS, or ANY storage!
let client = StorageClient::discover().await?;
// Connects to: ANY service advertising storage:artifact capability
```

---

### **2. Multi-Provider Support**

```rust
// Discover ALL storage providers
let discovery = CapabilityDiscovery::new()?;
let providers = discovery
    .find_by_capability(Capability::Storage(StorageCapability::ArtifactStorage))
    .await?;

// Choose based on criteria (latency, capacity, trust, etc.)
for provider in providers {
    println!("Found: {} (latency: {}ms)", provider.name, provider.latency);
}

// Use best provider
let best = providers.first().unwrap();
let client = StorageClient::connect(&best.name).await?;
```

---

### **3. Federation & Federation**

```rust
// Primary provider
let primary = StorageClient::discover().await?;

// Backup provider (different implementation!)
let backup = StorageClient::discover_with_capability(
    Capability::Storage(StorageCapability::ObjectStorage)
).await?;

// Use primary, fallback to backup
match primary.store_artifact("data.bin", data).await {
    Ok(result) => Ok(result),
    Err(_) => backup.store_artifact("data.bin", data).await,
}
```

---

### **4. Testing & Development**

```rust
// Production: Uses NestGate (via discovery)
let client = StorageClient::discover().await?;

// Development: Uses MinIO (via environment variable)
std::env::set_var("STORAGE_SERVICE", "minio-local");
let client = StorageClient::discover().await?;

// Testing: Uses mock storage (implements same capability!)
let client = StorageClient::discover().await?;
// Connects to test-storage advertising storage:artifact
```

---

## 📊 IMPLEMENTATION STATUS

### **Current State**

| Component | Status | Grade |
|-----------|--------|-------|
| **Capability Infrastructure** | ✅ Complete | A++ |
| **Generic Socket Discovery** | ✅ Complete | A++ |
| **StorageClient::discover()** | ✅ Complete | A++ |
| **StorageClient::with_config()** | ⚠️ Hardcoded | C |
| **Primal Socket Functions** | ⚠️ Not Deprecated | C |
| **Documentation** | ⚠️ Needs Update | B |

**Overall**: B+ (Good foundation, needs evolution)

---

### **After Evolution**

| Component | Status | Grade |
|-----------|--------|-------|
| **Capability Infrastructure** | ✅ Complete | A++ |
| **Generic Socket Discovery** | ✅ Complete | A++ |
| **StorageClient::discover()** | ✅ Complete | A++ |
| **StorageClient::with_config()** | ✅ Dynamic | A++ |
| **Primal Socket Functions** | ✅ Deprecated | A+ |
| **Documentation** | ✅ Complete | A++ |

**Overall**: A++ (TRUE PRIMAL mastery!)

---

## 🚀 EXECUTION STEPS

### **Step 1**: Deprecate Hardcoded Socket Functions

**Files**: `primal_sockets.rs`  
**Time**: 5 minutes  
**Impact**: Documentation only (backward compatible)

---

### **Step 2**: Evolve StorageClient::with_config()

**Files**: `client.rs`  
**Time**: 10 minutes  
**Impact**: More flexible, still backward compatible

---

### **Step 3**: Update Documentation

**Files**: `README.md`, integration docs  
**Time**: 10 minutes  
**Impact**: Clarify best practices

---

### **Step 4**: Test & Validate

**Time**: 5 minutes  
**Validation**:
- ✅ Existing tests pass
- ✅ `discover()` works
- ✅ `with_config()` more flexible
- ✅ Deprecated functions still work (with warnings)

---

## 🎯 SUCCESS CRITERIA

### **Code Quality**

- ✅ Zero hardcoded primal names in production code paths
- ✅ Capability-based discovery as primary pattern
- ✅ Generic socket resolution for discovered services
- ✅ Deprecated functions marked clearly

### **Philosophy Alignment**

- ✅ TRUE PRIMAL: Self-knowledge only
- ✅ Runtime discovery: No hardcoding
- ✅ Vendor-agnostic: Works with ANY provider
- ✅ Capability-based: Match by need, not name

---

## 📚 REFERENCES

### **NestGate Handoff**

From NestGate team handoff document:
> "Capability-Based Discovery: `let storage_primal = discover_capability("block-storage").await?;`"

### **primal-capabilities.toml**

```toml
[primals.nestgate]
capabilities = [
    "storage",
    "distributed-storage",
    "object-storage",
    "file-storage",
    "block-storage",
    # ...
]
```

### **TRUE PRIMAL Philosophy**

> "Each primal knows only itself. Everything else is discovered."

---

**Created**: January 16, 2026  
**Purpose**: Document primal knowledge deep debt and evolution path  
**Status**: Ready for execution  
**Impact**: Complete TRUE PRIMAL philosophy alignment!

🦀 **LET'S EVOLVE TO PURE CAPABILITY-BASED ARCHITECTURE!** 🦀✨
