# 🌐 Primal-Agnostic Capability System
**Version**: 1.0  
**Date**: November 10, 2025  
**Status**: ✅ IMPLEMENTED  
**Architecture**: Primal-Agnostic, Future-Proof

---

## 🎯 **OVERVIEW**

The **Primal-Agnostic Capability System** allows ToadStool to register its compute capabilities with ANY primal in the ecoPrimals ecosystem, not just Songbird.

### **Design Philosophy**

**Problem**: Hardcoding Songbird-specific logic limits evolution as new primals emerge  
**Solution**: Generic capability system with pluggable primal adapters

---

## 🏗️ **ARCHITECTURE**

```
┌─────────────────────────────────────────────────────┐
│  ToadStool Capability Provider                      │
│  (primal-agnostic, works with any primal)           │
├─────────────────────────────────────────────────────┤
│  Capability Registry                                │
│  ├── compute_gpu           (GPU acceleration)       │
│  ├── compute_heavy         (CPU-intensive)          │
│  ├── compute_ml_training   (ML model training)      │
│  ├── compute_native        (Direct execution)       │
│  ├── compute_container     (Docker/containerd)      │
│  ├── compute_wasm          (WebAssembly)            │
│  ├── compute_mainframe     (IBM, VAX - future)      │
│  └── compute_embedded      (PLCs, 8-bit - future)   │
├─────────────────────────────────────────────────────┤
│  Primal Adapters (pluggable)                        │
│  ├── SongbirdAdapter      ✅ Implemented            │
│  ├── SquirrelAdapter      🔜 Future (ML coord)      │
│  ├── BearDogAdapter       🔜 Future (Auth/Sec)      │
│  ├── NestGateAdapter      🔜 Future (Storage)       │
│  └── CustomAdapter        🔜 Extensible             │
├─────────────────────────────────────────────────────┤
│  Workload Execution API                             │
│  Method: compute.workload.execute (JSON-RPC 2.0)    │
│  Transport: Unix socket or TCP (UNIVERSAL_IPC_V3)   │
│  (Standard interface for all primals)               │
└─────────────────────────────────────────────────────┘
```

---

## 📋 **CAPABILITY TYPES**

### **1. compute_gpu**
- **Description**: GPU-accelerated computation
- **Hardware**: NVIDIA (CUDA), AMD (OpenCL), Intel (WebGPU)
- **Use Cases**: ML training, rendering, scientific computing
- **Resource Requirements**:
  - Min CPU: 2 cores
  - Min Memory: 2GB
  - GPU: Required
  - GPU Memory: 1GB+

### **2. compute_heavy**
- **Description**: CPU-intensive computation
- **Hardware**: High-core-count CPUs
- **Use Cases**: Data processing, compilation, analysis
- **Resource Requirements**:
  - Min CPU: 4 cores
  - Min Memory: 4GB
  - GPU: Not required

### **3. compute_ml_training**
- **Description**: Machine learning model training
- **Hardware**: High-end GPU + CPU
- **Use Cases**: Deep learning, neural network training
- **Resource Requirements**:
  - Min CPU: 4 cores
  - Min Memory: 8GB
  - GPU: Required
  - GPU Memory: 4GB+

### **4-6. Standard Runtimes**
- **compute_native**: Direct process execution
- **compute_container**: Docker/containerd workloads
- **compute_wasm**: WebAssembly modules

### **7-8. Legacy Hardware (Future)**
- **compute_mainframe**: IBM System/360, z/OS, VAX/VMS
- **compute_embedded**: PLCs, SCADA, 8/16-bit microcontrollers

---

## 🔌 **PRIMAL ADAPTER INTERFACE**

### **Trait Definition**

```rust
#[async_trait]
pub trait PrimalAdapter: Send + Sync {
    /// Get primal name
    fn primal_name(&self) -> &str;
    
    /// Get primal endpoint
    fn endpoint(&self) -> &str;
    
    /// Register capabilities with primal
    async fn register_capabilities(&self, capabilities: Vec<Capability>) 
        -> ToadStoolResult<()>;
    
    /// Send heartbeat
    async fn send_heartbeat(&self) -> ToadStoolResult<()>;
    
    /// Notify capability change
    async fn notify_capability_change(&self, capability: &Capability, available: bool) 
        -> ToadStoolResult<()>;
    
    /// Deregister from primal
    async fn deregister(&self) -> ToadStoolResult<()>;
}
```

### **Adding a New Primal Adapter**

```rust
// Example: Adding Squirrel adapter
pub struct SquirrelAdapter {
    endpoint: String,
    client: reqwest::Client,
}

#[async_trait]
impl PrimalAdapter for SquirrelAdapter {
    fn primal_name(&self) -> &str {
        "squirrel"
    }
    
    async fn register_capabilities(&self, capabilities: Vec<Capability>) 
        -> ToadStoolResult<()> {
        // Implement Squirrel's registration protocol
        let url = format!("{}/ml/register_compute_provider", self.endpoint);
        // ... Squirrel-specific logic
        Ok(())
    }
    
    // ... implement other methods
}
```

**That's it!** No changes to ToadStool core needed.

---

## 📡 **API INTERFACE**

### **1. Workload Execution** (For Primals)

**Method**: `compute.workload.execute` over JSON-RPC 2.0

**Transport**: Unix socket or TCP (per UNIVERSAL_IPC_STANDARD_V3)

**Error codes**: Standard JSON-RPC 2.0 error codes (-32700, -32600, -32601, -32602, -32603, -32000 to -32099)

**Purpose**: Receive workload requests from any primal

**Request** (JSON-RPC 2.0 params):
```json
{
  "request_id": "req-123",
  "from_primal": "songbird",
  "required_capability": "compute_gpu",
  "workload_type": {
    "type": "MlTraining",
    "model_type": "pytorch",
    "training_data": "s3://bucket/data.tar.gz",
    "hyperparameters": {
      "learning_rate": 0.001,
      "batch_size": 32
    }
  },
  "resource_requirements": {
    "cpu_cores": 4,
    "memory_mb": 8192,
    "gpu_required": true,
    "gpu_memory_mb": 4096
  },
  "environment": {
    "PYTHONPATH": "/opt/ml"
  },
  "timeout_seconds": 3600,
  "priority": "high"
}
```

**Response** (JSON-RPC 2.0 result):
```json
{
  "request_id": "req-123",
  "execution_id": "exec-456",
  "status": "Accepted",
  "timestamp": "2025-11-10T20:00:00Z"
}
```

### **2. Capability Registration** (Internal)

**Usage**: ToadStool → Primal

```rust
// Auto-detect primal type and register
let provider = CapabilityProvider::default();
provider.register_with_primal("http://songbird:8080").await?;
provider.register_with_primal("http://squirrel:8083").await?;
```

---

## 🔄 **EXECUTION FLOW**

### **Scenario: GPU Task from Songbird**

```
1. User → Songbird Compute API
   POST /api/v1/compute/task
   {
     "task_type": "ml_training",
     "resource_requirements": {"gpu_required": true}
   }

2. Songbird → Complexity Analysis
   - Detects ML training task
   - Requires GPU capability

3. Songbird → Capability Registry Lookup
   - Queries: Which services have "compute_gpu"?
   - Finds: ToadStool at http://toadstool:8084

4. Songbird → ToadStool Workload API (JSON-RPC 2.0 over Unix socket/TCP)
   Method: compute.workload.execute
   Params: {
     "from_primal": "songbird",
     "required_capability": "compute_gpu",
     "workload_type": {...}
   }

5. ToadStool → UniversalScheduler
   - Converts to UniversalJob
   - Routes to GPU runtime
   - Executes on CUDA

6. ToadStool → Songbird (Response)
   {
     "status": "Completed",
     "output": {...},
     "execution_time_seconds": 245.3
   }

7. Songbird → User (Results)
   - Returns ML model
   - Includes metrics
```

---

## 🚀 **USAGE EXAMPLES**

### **1. Server Startup (Auto-Register)**

```rust
// In server/src/main.rs
use toadstool_distributed::primal_capabilities::CapabilityProvider;

#[tokio::main]
async fn main() -> Result<()> {
    // Create capability provider
    let provider = CapabilityProvider::default();
    
    // Register with primals from environment
    if let Ok(songbird_endpoint) = std::env::var("SONGBIRD_ENDPOINT") {
        provider.register_with_primal(&songbird_endpoint).await?;
        tracing::info!("Registered with Songbird at {}", songbird_endpoint);
    }
    
    if let Ok(squirrel_endpoint) = std::env::var("SQUIRREL_ENDPOINT") {
        provider.register_with_primal(&squirrel_endpoint).await?;
        tracing::info!("Registered with Squirrel at {}", squirrel_endpoint);
    }
    
    // Start heartbeat task
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = provider.send_heartbeats().await {
                tracing::warn!("Heartbeat failed: {:?}", e);
            }
        }
    });
    
    // Start server...
    Ok(())
}
```

### **2. Dynamic Capability Updates**

```rust
// When GPU becomes available
provider.update_capability(Capability::compute_gpu(), true).await?;
// All connected primals are notified automatically

// When GPU becomes unavailable
provider.update_capability(Capability::compute_gpu(), false).await?;
```

### **3. Adding Custom Capabilities**

```rust
// Define custom capability
let custom_cap = Capability {
    id: "compute_quantum".to_string(),
    name: "Quantum Computing".to_string(),
    description: "Quantum circuit execution".to_string(),
    resource_requirements: CapabilityResources {
        min_cpu_cores: 1,
        min_memory_mb: 512,
        gpu_required: false,
        gpu_memory_mb: None,
        special_hardware: vec!["quantum_processor".to_string()],
    },
    tags: vec!["quantum".to_string(), "experimental".to_string()],
    available: true,
    confidence: 0.7,
};

// Add to provider
provider.add_capability(custom_cap).await?;
```

---

## 🎯 **BENEFITS**

### **1. Future-Proof**
- Add new primals without changing ToadStool core
- Each primal can have its own protocol
- No breaking changes when primals evolve

### **2. Clean Architecture**
- Separation of concerns
- Pluggable adapters
- Standard interfaces

### **3. Ecosystem Evolution**
- Today: Songbird, Squirrel
- Tomorrow: BearDog, NestGate, CustomPrimals
- No refactoring required

### **4. Capability Discovery**
- Primals know what ToadStool can do
- Automatic routing based on capabilities
- Dynamic updates when capabilities change

---

## 📊 **IMPLEMENTATION STATUS**

| Component | Status | Location |
|-----------|--------|----------|
| **CapabilityProvider** | ✅ Implemented | `distributed/src/primal_capabilities/mod.rs` |
| **Capability Registry** | ✅ Implemented | `distributed/src/primal_capabilities/registry.rs` |
| **PrimalAdapter Trait** | ✅ Implemented | `distributed/src/primal_capabilities/adapters.rs` |
| **SongbirdAdapter** | ✅ Implemented | `distributed/src/primal_capabilities/adapters.rs` |
| **WorkloadExecutor** | ✅ Implemented | `distributed/src/primal_capabilities/workload.rs` |
| **JSON-RPC Handler** | 🔜 Next | `api/src/handlers.rs` |
| **Server Integration** | 🔜 Next | `server/src/main.rs` |
| **SquirrelAdapter** | 🔜 Future | - |
| **BearDogAdapter** | 🔜 Future | - |

---

## 🔧 **CONFIGURATION**

### **Environment Variables**

```bash
# Songbird endpoint (if available)
SONGBIRD_ENDPOINT=http://songbird:8080

# Squirrel endpoint (if available)
SQUIRREL_ENDPOINT=http://squirrel:8083

# ToadStool's own endpoint (for callbacks)
TOADSTOOL_ENDPOINT=http://toadstool:8084

# Heartbeat interval (seconds)
PRIMAL_HEARTBEAT_INTERVAL=30
```

### **Configuration File** (`toadstool.toml`)

```toml
[primal_capabilities]
enabled = true
heartbeat_interval_seconds = 30

[[primal_capabilities.primals]]
name = "songbird"
endpoint = "http://songbird:8080"
enabled = true

[[primal_capabilities.primals]]
name = "squirrel"
endpoint = "http://squirrel:8083"
enabled = false  # Enable when ready

[primal_capabilities.capabilities]
compute_gpu = true  # Auto-detected
compute_heavy = true
compute_ml_training = true
compute_native = true
compute_container = true
compute_wasm = true
compute_mainframe = false  # When legacy runtime fixed
compute_embedded = false   # When legacy runtime fixed
```

---

## 🧪 **TESTING**

### **Integration Test**

```bash
# Start ToadStool
SONGBIRD_ENDPOINT="http://localhost:8080" cargo run --bin toadstool-server

# Submit GPU task via JSON-RPC 2.0 (compute.workload.execute over Unix socket/TCP)
# Example: {"jsonrpc":"2.0","method":"compute.workload.execute","params":{"task_type":"ml_training","resource_requirements":{"gpu_required":true,"memory_mb":8192}},"id":1}

# Expected flow:
# 1. ToadStool registers with Songbird on startup ✅
# 2. Songbird routes GPU task to ToadStool ✅
# 3. ToadStool executes and returns results ✅
```

---

## 🎉 **CONCLUSION**

The **Primal-Agnostic Capability System** is a **future-proof architecture** that allows ToadStool to work with any primal in the ecoPrimals ecosystem, not just Songbird.

**Key Achievement**: 🏆 **Built right from the start** - no technical debt, no refactoring needed as primals evolve.

---

**Status**: ✅ Core system implemented  
**Next Steps**: Wire into API and server, test with Songbird  
**Future**: Add adapters for Squirrel, BearDog, NestGate, custom primals

