# ToadStool Scope — What We Do (and Don't Do)

**Date**: February 6, 2026  
**Purpose**: Crystal-clear definition of ToadStool's responsibilities within the ecoPrimals ecosystem  
**Principle**: **Each primal only knows itself**

---

## 🎯 ToadStool's Mission

**ToadStool is a LOCAL hardware orchestration primal that executes BarraCUDA math operations on THIS machine's devices (CPU/GPU/NPU).**

---

## ✅ IN SCOPE: What ToadStool Does

### **1. LOCAL Device Management**
- ✅ Discover hardware on THIS machine (CPU, GPU, NPU)
- ✅ Query device capabilities (compute units, memory, features)
- ✅ Select optimal LOCAL device for workload

**Example**:
```rust
// ToadStool discovers LOCAL hardware
let devices = toadstool.discover_local_devices().await?;
// devices = [CPU: 16 cores, GPU: RTX 3090, NPU: Akida]
```

---

### **2. BarraCUDA Execution**
- ✅ Execute BarraCUDA math operations
- ✅ Route operations to appropriate LOCAL device
- ✅ Manage GPU/NPU resources

**Example**:
```rust
// ToadStool executes BarraCUDA operation
let result = toadstool.execute_barracuda_op(matmul_operation).await?;
// Internally: Routes to GPU if available, else CPU
```

---

### **3. Capability Registration**
- ✅ Advertise capabilities to Songbird (for discovery)
- ✅ Respond to capability queries
- ✅ Update capabilities when hardware changes

**Example**:
```rust
// ToadStool registers with Songbird (at startup)
toadstool.register_capabilities(songbird_endpoint).await?;
// Other primals can now discover this ToadStool via Songbird
```

---

### **4. Graphics API Integration**
- ✅ Integrate Vulkan/WebGPU for rendering
- ✅ Present results from BarraCUDA operations
- ✅ Leverage existing graphics standards (don't reinvent)

**Example**:
```rust
// ToadStool uses Vulkan for presentation
let raytraced = barracuda.raytrace(&scene).await?;
toadstool.vulkan().present(&raytraced)?;
```

---

### **5. Local Workload Routing**
- ✅ Route operations to CPU vs GPU vs NPU (on THIS machine)
- ✅ Balance workloads across local devices
- ✅ Optimize for local hardware capabilities

**Example**:
```rust
// ToadStool routes based on operation characteristics
toadstool.execute(|compute| {
    compute.bvh_construction(...);  // → NPU (sparse, hierarchical)
    compute.matmul(...);            // → GPU (dense, parallel)
    compute.complex_logic(...);     // → CPU (branching)
}).await?;
```

---

## ❌ OUT OF SCOPE: What ToadStool Does NOT Do

### **1. Inter-Primal Coordination** → **BiomeOS** (phase2)
- ❌ ToadStool does NOT coordinate with other towers
- ❌ ToadStool does NOT route workloads to other primals
- ❌ ToadStool does NOT execute graphs across multiple primals

**Why**: Each primal only knows itself. BiomeOS handles inter-primal orchestration.

**Example** (WRONG for ToadStool):
```rust
// ❌ ToadStool does NOT do this:
toadstool.connect_to_another_tower("tower2.example.com").await?;  // WRONG!
```

**Example** (CORRECT - BiomeOS does this):
```rust
// ✅ BiomeOS orchestrates across towers:
biomeos.execute_graph(|graph| {
    graph.add_node("toadstool1", workload1);  // Tower 1
    graph.add_node("toadstool2", workload2);  // Tower 2
}).await?;
```

---

### **2. Primal Discovery** → **Songbird** (phase1/songBird/)
- ❌ ToadStool does NOT discover other primals
- ❌ ToadStool does NOT manage primal registry
- ❌ ToadStool does NOT broker IPC connections

**Why**: Songbird is the IPC broker and discovery service.

**Example** (WRONG for ToadStool):
```rust
// ❌ ToadStool does NOT do this:
let primals = toadstool.discover_all_primals().await?;  // WRONG!
```

**Example** (CORRECT - Songbird does this):
```rust
// ✅ Songbird handles discovery:
let primals = songbird.discover_by_capability("compute").await?;
```

---

### **3. Network Communication** → **Songbird** (phase1/songBird/)
- ❌ ToadStool does NOT manage network sockets
- ❌ ToadStool does NOT handle inter-tower communication
- ❌ ToadStool does NOT manage network latency

**Why**: Songbird handles IPC and network coordination.

**Example** (WRONG for ToadStool):
```rust
// ❌ ToadStool does NOT do this:
toadstool.send_to_remote_tower(data, "tower2:5000").await?;  // WRONG!
```

**Example** (CORRECT - Songbird does this):
```rust
// ✅ Songbird handles network IPC:
songbird.connect("toadstool2").await?.send(data).await?;
```

---

### **4. Graph Execution (Distributed)** → **BiomeOS** (phase2)
- ❌ ToadStool does NOT execute DAGs across primals
- ❌ ToadStool does NOT manage dependencies across towers
- ❌ ToadStool does NOT orchestrate complex workflows

**Why**: BiomeOS handles graph-based deployment and inter-primal workflows.

**Example** (WRONG for ToadStool):
```rust
// ❌ ToadStool does NOT do this:
toadstool.execute_distributed_graph(dag).await?;  // WRONG!
```

**Example** (CORRECT - BiomeOS does this):
```rust
// ✅ BiomeOS executes graphs:
biomeos.execute_graph(neural_api_graph).await?;
```

---

### **5. Capability Translations** → **BiomeOS** (phase2)
- ❌ ToadStool does NOT translate between primal APIs
- ❌ ToadStool does NOT map capabilities across primals
- ❌ ToadStool does NOT handle primal-to-primal semantics

**Why**: BiomeOS handles capability translations and semantic mappings.

**Example** (WRONG for ToadStool):
```rust
// ❌ ToadStool does NOT do this:
toadstool.translate_capability_to_beardog_api(crypto_op).await?;  // WRONG!
```

**Example** (CORRECT - BiomeOS does this):
```rust
// ✅ BiomeOS translates:
biomeos.neural_api().crypto_encrypt(data).await?;
// BiomeOS translates to: beardog.crypto.aes_encrypt(...)
```

---

## 🤝 How ToadStool Interacts with Ecosystem

### **With Songbird** (IPC Broker)
- ✅ Register capabilities on startup
- ✅ Respond to discovery queries
- ✅ Receive workload requests via JSON-RPC 2.0

```rust
// ToadStool → Songbird (registration)
songbird_adapter.register_capabilities(toadstool_capabilities).await?;

// Other Primal → Songbird → ToadStool (discovery & connection)
// 1. Other primal asks Songbird: "Where is compute?"
// 2. Songbird responds: "ToadStool at /run/user/1000/toadstool.sock"
// 3. Other primal connects to ToadStool directly
```

---

### **With BiomeOS** (Orchestrator)
- ✅ Receive workload execution requests
- ✅ Execute BarraCUDA operations locally
- ✅ Return results to BiomeOS

```rust
// BiomeOS → ToadStool (via JSON-RPC)
// BiomeOS sends: { "method": "execute_workload", "params": {...} }
// ToadStool executes locally, returns result
```

---

### **With BarraCUDA** (Math Library)
- ✅ Direct integration (BarraCUDA is a library, not a primal)
- ✅ Execute math operations
- ✅ Route to appropriate device

```rust
// ToadStool uses BarraCUDA directly (library, not primal)
let tensor = barracuda::Tensor::zeros(vec![1024, 1024]).await?;
let result = tensor.matmul(&other).await?;
```

---

### **With WateringHole** (Standards)
- ✅ Implement Universal IPC Standard v3 (independently)
- ✅ Follow Primal Deployment Standard (socket paths)
- ✅ Reference shared knowledge (no code embedding)

```rust
// ToadStool implements wateringHole standards (own code)
// - JSON-RPC 2.0 over Unix sockets
// - 5-tier socket path resolution
// - NO shared crate, implements standard independently
```

---

## 📊 Responsibility Matrix

| Capability | BarraCUDA | ToadStool | Songbird | BiomeOS | WateringHole |
|------------|-----------|-----------|----------|---------|--------------|
| **Math Operations** | ✅ Owns | ✅ Executes | ❌ | ❌ | ❌ |
| **Local Device Discovery** | ❌ | ✅ Owns | ❌ | ❌ | ❌ |
| **Local Workload Routing** | ❌ | ✅ Owns | ❌ | ❌ | ❌ |
| **Primal Discovery** | ❌ | ❌ | ✅ Owns | ❌ | ❌ |
| **Inter-Primal IPC** | ❌ | ❌ | ✅ Owns | ❌ | ❌ |
| **Cross-Tower Orchestration** | ❌ | ❌ | ❌ | ✅ Owns | ❌ |
| **Graph Execution (Distributed)** | ❌ | ❌ | ❌ | ✅ Owns | ❌ |
| **Capability Translations** | ❌ | ❌ | ❌ | ✅ Owns | ❌ |
| **Standards & Protocols** | ❌ | ❌ | ❌ | ❌ | ✅ Owns |
| **Graphics API (Vulkan)** | ❌ | ✅ Uses | ❌ | ❌ | ✅ Standard |

---

## 🧭 Decision Guide: Does This Belong in ToadStool?

### **Ask These Questions**:

1. **Is it about LOCAL hardware?**
   - ✅ YES → ToadStool
   - ❌ NO → Other primal

2. **Is it about math/compute operations?**
   - ✅ YES → BarraCUDA (library, ToadStool executes)
   - ❌ NO → Continue

3. **Does it involve multiple primals/towers?**
   - ✅ YES → BiomeOS (orchestration)
   - ❌ NO → Continue

4. **Is it about inter-primal communication?**
   - ✅ YES → Songbird (IPC broker)
   - ❌ NO → Continue

5. **Is it a protocol/standard?**
   - ✅ YES → WateringHole (standards hub)
   - ❌ NO → Continue

6. **Is it about presenting/rendering results?**
   - ✅ YES → ToadStool (Vulkan/WebGPU integration)
   - ❌ NO → Re-evaluate

---

## 💡 Examples: What Goes Where?

### **Raytracing on CPU/NPU/GPU**

**Question**: Where does this belong?

**Analysis**:
- **Math**: BarraCUDA (BVH construction, ray-triangle intersection)
- **Local routing**: ToadStool (BVH→NPU, Rays→GPU, Shading→CPU)
- **Rendering**: ToadStool (Vulkan presentation)

**Answer**: ✅ ToadStool + BarraCUDA (local execution)

---

### **Distributed ML Training Across 3 Towers**

**Question**: Where does this belong?

**Analysis**:
- **Math**: BarraCUDA (forward/backward pass)
- **Local execution**: ToadStool (executes on each tower)
- **Coordination**: BiomeOS (splits workload, aggregates gradients)
- **Communication**: Songbird (network IPC)

**Answer**: ✅ BiomeOS orchestrates, ToadStool executes locally, Songbird connects

---

### **Discovering Available Compute Primals**

**Question**: Where does this belong?

**Analysis**:
- **Discovery**: Not local hardware, but other primals
- **IPC**: Requires primal registry

**Answer**: ✅ Songbird (discovery service)

---

### **Video Decode + ML Inference + Encode**

**Question**: Where does this belong?

**Analysis**:
- **Decode math**: BarraCUDA (DCT, YUV→RGB)
- **Inference math**: BarraCUDA (matmul, convolutions)
- **Encode math**: BarraCUDA (RGB→YUV, DCT, quantization)
- **Execution**: ToadStool (routes to GPU/NPU)
- **If distributed**: BiomeOS (orchestrates pipeline across towers)

**Answer**: ✅ BarraCUDA math, ToadStool local execution, BiomeOS if distributed

---

## 📋 Summary

### **ToadStool IS**:
- ✅ LOCAL hardware orchestration primal
- ✅ BarraCUDA execution engine
- ✅ Device capability provider
- ✅ Graphics API integrator (Vulkan/WebGPU)

### **ToadStool IS NOT**:
- ❌ Inter-primal coordinator (BiomeOS)
- ❌ IPC broker (Songbird)
- ❌ Network manager (Songbird)
- ❌ Graph executor (BiomeOS)
- ❌ Standards authority (WateringHole)

### **Key Principle**:
> **"ToadStool only knows itself. Discovers others at runtime. Executes BarraCUDA math on LOCAL hardware only."**

---

**This clarity keeps us focused and prevents scope creep!** 🎯
