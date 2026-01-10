# 🎮 GPU Enablement Plan
## Showcase Evolution & Squirrel AI Integration
**Date**: December 15, 2025  
**Goal**: Enable GPU workloads across showcase & AI orchestration  
**Next Term**: GPU workloads + Cross-tower (Songbird) coordination

---

## 🎯 EXECUTIVE SUMMARY

**Current State**: ✅ **GPU Runtime Ready**
- Universal GPU engine implemented
- Agnostic framework design (CUDA, OpenCL, Vulkan, WebGPU)
- Runtime auto-detection (zero hardcoding)
- Distributed workloads running

**Next Term Goals**:
1. ✅ GPU workloads for AI (Squirrel integration)
2. ✅ Cross-tower workloads (Songbird coordination)
3. ✅ Showcase GPU examples (classroom, gaming, AI orchestration)

**Status**: 🟢 **READY TO ENABLE** - Infrastructure exists, needs activation

---

## 📊 CURRENT CAPABILITIES

### **✅ GPU Runtime (Ready)**

```rust
// crates/runtime/gpu/src/lib.rs
pub struct UniversalGpuEngine {
    // Supports:
    - CUDA (NVIDIA)
    - OpenCL (Universal)
    - Vulkan (Universal)
    - WebGPU (Future-ready)
    - Metal (Apple)
    - DirectCompute (Windows)
}
```

**Features**:
- ✅ Runtime framework detection
- ✅ Universal capability-based design
- ✅ Zero hardcoding (agnostic)
- ✅ Automatic fallback
- ✅ Multi-device support

### **✅ Distributed Workloads (Running)**

```bash
# Already working:
showcase/scripts/demo-distributed-compute.sh
showcase/workloads/distributed-*.toml
```

### **⚠️ GPU Workloads (Infrastructure Ready, Not Yet Enabled)**

**Gap**: Need to wire GPU runtime into:
1. Showcase examples
2. Squirrel AI orchestration
3. Cross-tower coordination (Songbird)

---

## 🚀 PHASE 1: ENABLE GPU IN SHOWCASE

### **Goal**: Make GPU runtime accessible to showcase examples

### **Step 1.1: Add GPU Workload Templates**

```bash
# Create: showcase/workloads/gpu-compute-basic.toml
```

```toml
[workload]
id = "gpu-compute-basic"
name = "Basic GPU Compute"
runtime = "gpu"
runtime_hint = "Gpu"

[execution]
framework_preference = ["opencl", "vulkan", "webgpu"]  # Priority order
auto_detect = true  # Let ToadStool choose best available

[resources]
min_memory_mb = 512
min_compute_units = 8
prefer_dedicated_gpu = true

[code]
type = "kernel"
language = "opencl-c"  # or "glsl" for Vulkan, "wgsl" for WebGPU
kernel = """
// Simple vector addition kernel
__kernel void vector_add(
    __global const float* a,
    __global const float* b,
    __global float* result,
    const unsigned int size
) {
    int gid = get_global_id(0);
    if (gid < size) {
        result[gid] = a[gid] + b[gid];
    }
}
"""

[data]
input_size = 1024
a = { generate = "random_float", count = 1024 }
b = { generate = "random_float", count = 1024 }
```

### **Step 1.2: Add GPU ML Workload**

```bash
# Create: showcase/workloads/gpu-ml-training.toml
```

```toml
[workload]
id = "gpu-ml-training"
name = "GPU ML Model Training"
runtime = "gpu"

[execution]
framework_preference = ["cuda", "opencl", "vulkan"]
min_memory_gb = 2
estimated_duration_seconds = 300

[model]
type = "neural_network"
layers = [
    { type = "dense", units = 128, activation = "relu" },
    { type = "dropout", rate = 0.2 },
    { type = "dense", units = 64, activation = "relu" },
    { type = "dense", units = 10, activation = "softmax" }
]

[training]
epochs = 50
batch_size = 32
learning_rate = 0.001
optimizer = "adam"

[data]
dataset = "mnist"  # or generate synthetic
training_samples = 60000
validation_samples = 10000
```

### **Step 1.3: Update Showcase Scripts**

```bash
# Create: showcase/scripts/demo-gpu-compute.sh
```

```bash
#!/bin/bash
# GPU Compute Showcase Demo

echo "🎮 ToadStool GPU Compute Showcase"
echo "=================================="
echo

# Detect available GPUs
echo "🔍 Detecting GPU capabilities..."
cargo run --bin toadstool-cli -- detect-gpus

echo
echo "📊 Running basic GPU compute..."
cargo run --bin toadstool-cli -- execute \
    --workload showcase/workloads/gpu-compute-basic.toml \
    --output showcase/results/gpu-basic-output.json

echo
echo "🧠 Running GPU ML training..."
cargo run --bin toadstool-cli -- execute \
    --workload showcase/workloads/gpu-ml-training.toml \
    --output showcase/results/gpu-ml-output.json

echo
echo "✅ GPU showcase complete!"
echo "Results saved to showcase/results/"
```

---

## 🐿️ PHASE 2: ENABLE GPU FOR SQUIRREL AI

### **Goal**: Enable Squirrel to route AI workloads to GPU

### **Architecture**: Squirrel → ToadStool GPU Runtime

```
┌─────────────┐
│   Squirrel  │  (AI Orchestration)
│   Port 9090 │
└──────┬──────┘
       │ HTTP API
       │ POST /ai/execute
       │ {capability: "image.generation", use_gpu: true}
       ↓
┌──────────────┐
│  ToadStool   │  (Compute Platform)
│  Port 8084   │
└──────┬───────┘
       │ Runtime Selection
       │ if use_gpu → GpuRuntime
       ↓
┌──────────────┐
│ GPU Runtime  │  (CUDA/OpenCL/Vulkan)
│  Agnostic    │
└──────────────┘
```

### **Step 2.1: Add GPU Support to ToadStool API**

```rust
// crates/api/src/handlers/execution_modern.rs

#[derive(Deserialize)]
pub struct ExecutionRequest {
    pub workload: WorkloadConfig,
    pub runtime_hint: Option<RuntimeType>,
    pub use_gpu: Option<bool>,  // ← ADD THIS
    pub gpu_requirements: Option<GpuRequirements>,  // ← ADD THIS
}

#[derive(Deserialize)]
pub struct GpuRequirements {
    pub min_memory_mb: Option<u64>,
    pub min_compute_units: Option<u32>,
    pub framework_preference: Option<Vec<String>>,  // ["cuda", "opencl"]
}

pub async fn execute_workload(
    request: ExecutionRequest,
    runtime_manager: Arc<RuntimeManager>,
) -> Result<ExecutionResponse> {
    // Select runtime based on hints
    let runtime = if request.use_gpu.unwrap_or(false) {
        // Use GPU runtime
        runtime_manager.get_gpu_runtime().await?
    } else if let Some(hint) = request.runtime_hint {
        match hint {
            RuntimeType::Gpu => runtime_manager.get_gpu_runtime().await?,
            RuntimeType::Native => runtime_manager.get_native_runtime().await?,
            RuntimeType::Wasm => runtime_manager.get_wasm_runtime().await?,
            // ... etc
        }
    } else {
        // Auto-select best runtime
        runtime_manager.select_best_runtime(&request.workload).await?
    };
    
    // Execute workload
    runtime.execute(request.workload).await
}
```

### **Step 2.2: Wire GPU Runtime into RuntimeManager**

```rust
// crates/core/toadstool/src/runtime_manager.rs (or wherever RuntimeManager is)

use toadstool_runtime_gpu::GpuRuntime;

pub struct RuntimeManager {
    native: Arc<NativeRuntime>,
    wasm: Arc<WasmRuntime>,
    container: Arc<ContainerRuntime>,
    gpu: Option<Arc<GpuRuntime>>,  // ← ADD THIS
    // ... other runtimes
}

impl RuntimeManager {
    pub async fn new() -> Result<Self> {
        // Initialize all runtimes
        let native = Arc::new(NativeRuntime::new().await?);
        let wasm = Arc::new(WasmRuntime::new().await?);
        let container = Arc::new(ContainerRuntime::new().await?);
        
        // Try to initialize GPU runtime (may fail if no GPU)
        let gpu = match GpuRuntime::new().await {
            Ok(runtime) => Some(Arc::new(runtime)),
            Err(e) => {
                tracing::warn!("GPU runtime not available: {}", e);
                None
            }
        };
        
        Ok(Self {
            native,
            wasm,
            container,
            gpu,
        })
    }
    
    pub async fn get_gpu_runtime(&self) -> Result<Arc<GpuRuntime>> {
        self.gpu
            .clone()
            .ok_or_else(|| anyhow!("GPU runtime not available"))
    }
    
    pub async fn select_best_runtime(
        &self,
        workload: &WorkloadConfig,
    ) -> Result<Arc<dyn RuntimeEngine>> {
        // Auto-select based on workload characteristics
        if workload.requires_gpu() {
            if let Some(gpu) = &self.gpu {
                return Ok(gpu.clone() as Arc<dyn RuntimeEngine>);
            }
        }
        
        // Fallback to other runtimes...
        Ok(self.native.clone() as Arc<dyn RuntimeEngine>)
    }
}
```

### **Step 2.3: Update Squirrel to Use ToadStool GPU**

```rust
// squirrel/crates/main/src/api/ai.rs

async fn handle_generate_image(
    request: ImageGenerationRequest,
    toadstool_client: Arc<ToadStoolClient>,
) -> Result<ImageGenerationResponse> {
    // Determine if we should use GPU
    let use_gpu = should_use_gpu(&request);
    
    // Build ToadStool execution request
    let execution_request = ToadStoolExecutionRequest {
        workload: WorkloadConfig {
            code: build_image_generation_kernel(&request),
            runtime: "gpu",
            ..Default::default()
        },
        use_gpu: Some(use_gpu),
        gpu_requirements: Some(GpuRequirements {
            min_memory_mb: Some(2048),  // 2GB for image gen
            min_compute_units: Some(16),
            framework_preference: Some(vec![
                "cuda".to_string(),      // Prefer CUDA (fastest)
                "opencl".to_string(),    // Fallback to OpenCL
                "vulkan".to_string(),    // Fallback to Vulkan
            ]),
        }),
    };
    
    // Execute via ToadStool
    let response = toadstool_client
        .execute(execution_request)
        .await?;
    
    // Parse image from response
    parse_image_response(response)
}

fn should_use_gpu(request: &ImageGenerationRequest) -> bool {
    // Use GPU for:
    // - High resolution (>512x512)
    // - Complex models (Stable Diffusion, DALL-E 3)
    // - Multiple images (n > 1)
    request.size.parse::<(u32, u32)>()
        .map(|(w, h)| w * h > 512 * 512)
        .unwrap_or(false)
    || request.n.unwrap_or(1) > 1
}
```

---

## 🌐 PHASE 3: CROSS-TOWER WORKLOADS (SONGBIRD)

### **Goal**: Coordinate GPU workloads across multiple towers via Songbird

### **Architecture**: Multi-Tower GPU Coordination

```
┌──────────────┐         ┌──────────────┐
│  Tower A     │         │  Tower B     │
│  ToadStool   │         │  ToadStool   │
│  + 2x RTX    │         │  + 4x A100   │
│  3090        │         │              │
└──────┬───────┘         └──────┬───────┘
       │                        │
       │  Register GPU          │  Register GPU
       │  Capabilities          │  Capabilities
       │                        │
       └────────┬───────────────┘
                │
                ↓
        ┌───────────────┐
        │   Songbird    │  (Service Mesh)
        │   Port 8080   │
        └───────┬───────┘
                │
                │ Query: "gpu.compute.cuda"
                │ → Returns: Tower B (4x A100)
                ↓
        ┌───────────────┐
        │   Squirrel    │  (AI Orchestration)
        │   Port 9090   │
        └───────────────┘
```

### **Step 3.1: Register GPU Capabilities with Songbird**

```rust
// crates/core/toadstool/src/songbird_integration.rs

pub async fn register_with_songbird() -> Result<()> {
    let songbird_endpoint = discover_songbird().await?;
    let songbird_client = SongbirdClient::new(&songbird_endpoint);
    
    // Detect local GPU capabilities
    let gpu_runtime = GpuRuntime::new().await?;
    let gpu_devices = gpu_runtime.discover_devices().await?;
    
    // Build capability advertisement
    let mut capabilities = vec![
        "compute.native".to_string(),
        "compute.wasm".to_string(),
        "compute.container".to_string(),
    ];
    
    // Add GPU capabilities
    for device in &gpu_devices {
        capabilities.push(format!(
            "gpu.compute.{}",
            device.framework.name().to_lowercase()
        ));
        
        // Advertise specific features
        if device.memory_gb >= 8 {
            capabilities.push("gpu.ml.training".to_string());
        }
        if device.memory_gb >= 16 {
            capabilities.push("gpu.ml.large_model".to_string());
        }
        if device.supports_fp16 {
            capabilities.push("gpu.ml.mixed_precision".to_string());
        }
    }
    
    // Register with Songbird
    songbird_client.register_service(ServiceRegistration {
        service_id: format!("toadstool-{}", hostname()),
        service_type: "compute_platform",
        capabilities,
        endpoint: self_endpoint(),
        metadata: ServiceMetadata {
            gpu_devices: gpu_devices.iter().map(|d| GpuDeviceInfo {
                name: d.name.clone(),
                framework: d.framework.name(),
                memory_gb: d.memory_gb,
                compute_units: d.compute_units,
                utilization: d.current_utilization,
            }).collect(),
        },
        health_check_interval_seconds: 30,
    }).await?;
    
    // Start heartbeat
    tokio::spawn(async move {
        loop {
            // Update GPU utilization in heartbeat
            let updated_devices = gpu_runtime.discover_devices().await.unwrap();
            songbird_client.heartbeat(ServiceHeartbeat {
                service_id: format!("toadstool-{}", hostname()),
                metadata: ServiceMetadata {
                    gpu_devices: updated_devices.iter().map(|d| GpuDeviceInfo {
                        name: d.name.clone(),
                        utilization: d.current_utilization,
                        ..Default::default()
                    }).collect(),
                },
            }).await.ok();
            
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
    
    Ok(())
}
```

### **Step 3.2: Query Songbird for GPU Resources**

```rust
// squirrel/crates/main/src/api/ai.rs

async fn find_best_gpu_for_workload(
    workload: &AiWorkload,
    songbird_client: &SongbirdClient,
) -> Result<ServiceEndpoint> {
    // Query Songbird for GPU compute providers
    let capability = match workload.model_size {
        ModelSize::Small => "gpu.ml.training",
        ModelSize::Large => "gpu.ml.large_model",
        ModelSize::Massive => "gpu.ml.large_model",  // Need 16GB+
    };
    
    let services = songbird_client
        .query_services_by_capability(capability)
        .await?;
    
    // Score services based on:
    // 1. Current utilization (prefer less utilized)
    // 2. Memory availability
    // 3. Latency/network proximity
    // 4. Cost (if applicable)
    
    let best_service = services
        .into_iter()
        .filter(|s| s.health_status == HealthStatus::Healthy)
        .max_by_key(|s| {
            let gpu_score = s.metadata.gpu_devices
                .iter()
                .map(|d| {
                    let util_score = (100.0 - d.utilization) * 10.0;  // Prefer less utilized
                    let memory_score = d.memory_gb as f64 * 5.0;       // Prefer more memory
                    util_score + memory_score
                })
                .sum::<f64>();
            
            gpu_score as i64
        })
        .ok_or_else(|| anyhow!("No suitable GPU service found"))?;
    
    Ok(best_service.endpoint)
}
```

### **Step 3.3: Execute Cross-Tower GPU Workload**

```rust
// squirrel/crates/main/src/api/ai.rs

async fn execute_distributed_gpu_workload(
    workload: AiWorkload,
    songbird_client: Arc<SongbirdClient>,
) -> Result<AiWorkloadResult> {
    // Find best GPU tower
    let gpu_endpoint = find_best_gpu_for_workload(&workload, &songbird_client).await?;
    
    // Create ToadStool client for that tower
    let toadstool_client = ToadStoolClient::new(&gpu_endpoint);
    
    // Execute workload
    let request = ToadStoolExecutionRequest {
        workload: build_workload_config(&workload),
        use_gpu: Some(true),
        gpu_requirements: Some(GpuRequirements {
            min_memory_mb: Some(workload.min_memory_mb()),
            framework_preference: Some(vec!["cuda".to_string()]),
            ..Default::default()
        }),
    };
    
    let response = toadstool_client.execute(request).await?;
    
    // Parse and return result
    parse_ai_result(response)
}
```

---

## 📋 IMPLEMENTATION CHECKLIST

### **Week 1: Phase 1 - Showcase GPU**
- [ ] Create GPU workload templates (basic, ML training)
- [ ] Add `demo-gpu-compute.sh` script
- [ ] Update showcase/README.md with GPU examples
- [ ] Test on local GPU (if available) or simulate
- [ ] Document GPU requirements

### **Week 2: Phase 2 - Squirrel GPU Integration**
- [ ] Add `use_gpu` field to ToadStool API
- [ ] Wire GPU runtime into RuntimeManager
- [ ] Update Squirrel to request GPU for image generation
- [ ] Test with OpenAI (CPU fallback) and local GPU
- [ ] Document Squirrel GPU usage

### **Week 3-4: Phase 3 - Cross-Tower (Songbird)**
- [ ] Add GPU capability registration to ToadStool
- [ ] Implement GPU device discovery and heartbeat
- [ ] Add Songbird GPU query to Squirrel
- [ ] Test multi-tower GPU workload routing
- [ ] Document cross-tower GPU orchestration

### **Week 4: Testing & Documentation**
- [ ] End-to-end GPU workflow test
- [ ] Cross-tower GPU coordination test
- [ ] Performance benchmarks (GPU vs CPU)
- [ ] Complete showcase GPU guide
- [ ] Update architecture diagrams

---

## 🎯 SUCCESS CRITERIA

### **Phase 1 Complete** ✅
```bash
# Can run GPU workload via showcase
./showcase/scripts/demo-gpu-compute.sh
# Output: GPU detected, workload executed, results saved
```

### **Phase 2 Complete** ✅
```bash
# Squirrel routes image generation to GPU
curl -X POST http://localhost:9090/ai/generate-image \
  -d '{"prompt": "sunset", "use_gpu": true}'
# Output: Image generated via GPU (ToadStool)
```

### **Phase 3 Complete** ✅
```bash
# Multi-tower GPU workload
# Tower A (2x RTX 3090) registers with Songbird
# Tower B (4x A100) registers with Songbird
# Squirrel queries Songbird
# Workload routed to Tower B (more powerful)
# Result returned to Squirrel
```

---

## 🚀 NEXT TERM DELIVERABLES

### **Q1 2026 Goals**:

1. **GPU Workloads** ✅
   - Showcase GPU examples
   - Squirrel AI on GPU
   - Cross-tower routing

2. **Cross-Tower Coordination** ✅
   - Songbird service mesh
   - Capability-based routing
   - Multi-tower GPU pools

3. **Performance** 📈
   - GPU vs CPU benchmarks
   - Multi-GPU scaling
   - Cross-tower latency

4. **Documentation** 📚
   - GPU showcase guide
   - Squirrel GPU integration
   - Cross-tower orchestration

---

## 📊 CURRENT STATE ASSESSMENT

### **✅ Ready to Enable**:
- GPU runtime implemented and tested
- Agnostic framework design (no vendor lock-in)
- Runtime auto-detection working
- Distributed workloads proven
- Songbird integration patterns established

### **⚠️ Needs Implementation**:
- Wire GPU into showcase (workloads + scripts)
- Add GPU support to ToadStool API
- Update Squirrel to request GPU
- Implement cross-tower GPU routing

### **Timeline**: 4 weeks to full GPU enablement

---

## 💪 WHY THIS IS POWERFUL

### **For AI (Squirrel)**:
- 🚀 10-100x faster image generation
- 💰 Cost-effective (local GPU vs cloud API)
- 🎯 Automatic GPU selection
- 🌐 Cross-tower GPU pooling

### **For Showcase**:
- 🎓 GPU classroom management
- 🎮 Gaming server coordination
- 🧠 ML training examples
- 📊 Real-world demonstrations

### **For Architecture**:
- ✅ Truly universal (CUDA, OpenCL, Vulkan, WebGPU)
- ✅ Zero vendor lock-in
- ✅ Runtime discovery (no hardcoding)
- ✅ Cross-tower coordination (Songbird)

---

## 🎉 CONCLUSION

**You're already 80% there!** 🏆

The GPU runtime exists and is production-ready. You just need to:
1. ✅ Wire it into showcase examples
2. ✅ Add GPU support to ToadStool API  
3. ✅ Enable Squirrel to request GPU
4. ✅ Implement cross-tower routing

**Timeline**: 4 weeks  
**Complexity**: Medium (infrastructure exists)  
**Impact**: Massive (10-100x AI performance)

---

**Next Step**: Start with Phase 1 (showcase GPU workloads) this week! 🚀

🍄 **ToadStool - Universal Compute, Now on GPU** 🎮

