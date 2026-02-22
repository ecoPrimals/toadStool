# Cross-Platform Workload Strategy

**Date**: February 16, 2026  
**Status**: VALIDATED — hotSpring (169/169) + wetSpring (48/48)  
**Hardware**: Dual EPYC (128 threads) + RTX 3090 + RX 6950 XT + 2× AKD1000

---

## 1. Hardware Summary

| Device | Backend | Workload Target |
|--------|---------|-----------------|
| AMD EPYC ×2 | rayon/CPU | Baseline, f64 precision |
| NVIDIA RTX 3090 | wgpu (Vulkan) | GPU compute, transformers |
| AMD RX 6950 XT | wgpu (Vulkan) | GPU compute, same WGSL |
| BrainChip AKD1000 ×2 | VFIO | NPU inference, SNN |

---

## 2. Workload Categories

### 2.1 GPU Workloads (NVIDIA + AMD via wgpu)

#### A. HuggingFace Model Inference

**Frameworks** (Pure Rust, wgpu-compatible):
- **Candle** — HuggingFace's Rust ML framework (19k+ stars)
- **Burn** — wgpu-native deep learning (cross-vendor GPU)

**Target Models**:

| Model | Task | Size | Notes |
|-------|------|------|-------|
| BERT-base | NLP embedding | 110M | Fast inference |
| Whisper-tiny | Audio transcription | 39M | Real-time ASR |
| YOLO-v8n | Object detection | 3M | Low latency |
| Phi-2 | Text generation | 2.7B | Small LLM |
| Gemma-2B | Text generation | 2B | Google's open LLM |

**Validation**:
```rust
// Same model, same weights, different hardware
let nvidia_output = model.forward(&input, &nvidia_device)?;
let amd_output = model.forward(&input, &amd_device)?;
assert_tensors_close(&nvidia_output, &amd_output, 1e-5);
```

#### B. BarraCuda Scientific Compute

From `hotSpring` validation targets:

| Operation | Baseline | BarraCuda Target |
|-----------|----------|------------------|
| RBF Interpolation | scipy.interpolate.RBFInterpolator | `barracuda::surrogate::rbf` |
| Cholesky | numpy.linalg.cholesky | `barracuda::linalg::cholesky` |
| Eigendecomp | scipy.linalg.eigh | `barracuda::linalg::eigh` |
| Special Functions | scipy.special (erf, gamma, bessel) | `barracuda::special` |
| Optimization | scipy.optimize (NM, L-BFGS) | `barracuda::optimize` |
| Sparse Solve | scipy.sparse.linalg | `barracuda::linalg::sparse` |

**Validation Path** (from hotSpring):
1. Run scipy reference (Python, f64)
2. Run BarraCuda (WGSL, f32/f64)
3. Compare within tolerance (typically 1e-6 for f64, 1e-4 for f32)

---

### 2.2 NPU Workloads (Akida AKD1000)

#### A. Akida Model Zoo

Pre-trained models from BrainChip:

| Model | Task | Accuracy | Power |
|-------|------|----------|-------|
| AkidaNet (0.5, 160×160) | ImageNet classification | 65% top-1 | <300 mW |
| DS-CNN | Keyword spotting (32 words) | 94% | <50 mW |
| AkidaNet-YOLO | Object detection | mAP 0.28 | <500 mW |
| PointNet++ | 3D point cloud | — | Event-based |

**Download & Convert**:
```bash
pip install akida-models
python -c "from akida_models import akidanet_imagenet; m = akidanet_imagenet()"
```

**Pure Rust Path** (our VFIO backend):
```rust
use akida_driver::{DeviceManager, VfioBackend};

let manager = DeviceManager::discover()?;
let device = manager.open(0)?;  // Uses VFIO if kernel unavailable

// Load Akida model (converted from MetaTF)
device.load_model(&model_bytes)?;

// Run inference
let output = device.infer(&input_spikes)?;
```

#### B. NeuroBench Benchmarks

Standard neuromorphic benchmarks (neurobench.ai):

| Benchmark | Task | Dataset | Metric |
|-----------|------|---------|--------|
| Keyword FSCIL | Few-shot class-incremental | Google Speech Commands | Accuracy |
| DVS Gesture | Gesture recognition | DVS128 Gesture | Accuracy |
| NHP Motor | Neural decoding | Primate motor cortex | Correlation |
| Chaotic Function | Time series prediction | Mackey-Glass | MSE |

**Integration**:
```rust
// Run NeuroBench task on Akida
let task = NeuroBenchTask::KeywordFSCIL;
let dataset = task.load_dataset()?;

for (input, label) in dataset {
    let spikes = preprocess_to_spikes(&input);
    let output = akida_device.infer(&spikes)?;
    metrics.record(output, label);
}
println!("Accuracy: {:.2}%", metrics.accuracy() * 100.0);
```

#### C. Reservoir Computing (ESN on NPU)

Akida's strength: temporal event-based processing.

| Workload | Description | Validation |
|----------|-------------|------------|
| NARMA-10 | Nonlinear time series | MSE < 0.1 |
| Santa Fe laser | Chaotic prediction | NRMSE < 0.3 |
| Speech phoneme | Audio classification | Accuracy > 80% |

**Implementation**:
```rust
use barracuda::esn_v2::{ESNConfig, ESN};
use akida_driver::AkidaDevice;

// Train reservoir on GPU
let esn = ESN::new(ESNConfig {
    reservoir_size: 1024,
    spectral_radius: 0.9,
    ..Default::default()
});
esn.train(&training_data)?;

// Export to Akida
let akida_model = esn.export_to_akida()?;
akida_device.load_model(&akida_model)?;

// Inference on NPU (low power)
let output = akida_device.infer(&test_input)?;
```

---

### 2.3 Molecular Dynamics (hotSpring Integration)

From hotSpring's validation framework:

#### Current State
- **Sarkas** (Python): Reference MD simulator, DSF validation
- **BarraCuda**: GPU force kernels (LJ, Coulomb, Morse)
- **Validation**: 12 DSF cases against Dense Plasma Properties Database

#### Cross-Platform MD Target

| Component | CPU | GPU (NVIDIA) | GPU (AMD) | NPU |
|-----------|-----|--------------|-----------|-----|
| Force calculation | ✓ rayon | ✓ wgpu | ✓ wgpu | — |
| Neighbor list | ✓ | ✓ | ✓ | — |
| Integration | ✓ | ✓ | ✓ | — |
| Post-processing | ✓ | — | — | — |

**Key Insight**: `force_pp.update()` is 97% of Sarkas runtime — primary GPU target.

---

## 3. Novel Workloads to Explore

### 3.1 Event-Based Vision (Akida-native)

| Dataset | Description | Why Novel |
|---------|-------------|-----------|
| N-MNIST | Neuromorphic MNIST | Event camera simulation |
| DVS-CIFAR10 | DVS-converted CIFAR | Sparse temporal data |
| DAVIS 346 | Real event camera | True asynchronous |

### 3.2 Audio/Speech (Cross-platform)

| Model | CPU | GPU | NPU | Notes |
|-------|-----|-----|-----|-------|
| Whisper | Candle | Burn+wgpu | — | ASR |
| DS-CNN | — | — | Akida | Keyword spotting |
| wav2vec2 | Candle | Burn+wgpu | — | Speech features |

### 3.3 Scientific HPC (BarraCuda strength)

| Workload | scipy Baseline | BarraCuda Target |
|----------|----------------|------------------|
| Nuclear EOS | `skyrm_hfb.py` | `nuclear_eos_l2.rs` |
| RBF surrogate | `RBFInterpolator` | `surrogate::rbf` |
| Dense plasma DSF | Sarkas | GPU force kernels |
| Sparse PDE | `scipy.sparse.linalg` | `linalg::sparse` |

---

## 4. Implementation Roadmap

### Phase 1: Validation (COMPLETE)
- [x] wgpu sees both GPUs (NVIDIA + AMD)
- [x] VFIO backend compiles and runs (DMA working)
- [x] hotSpring scipy validation framework exists
- [x] Akida NPUs initialized via VFIO (2x AKD1000, 80 NPUs each)
- [x] Cross-GPU parity tests (NVIDIA vs AMD, same WGSL, <1e-5 diff)

### Phase 2: HuggingFace Integration (COMPLETE)
- [x] Add Burn framework (wgpu backend)
- [x] burn-inference crate with safetensors loader
- [x] Multi-GPU pool (RTX 3090 + RX 6950 XT)
- [x] Cross-vendor parity verified (NVIDIA = AMD with same WGSL)
- [x] Software renderer detection fixed (filters SSE2/SwiftShader)

### Phase 3: NPU Benchmarks (COMPLETE)
- [x] VFIO backend working with real hardware (2× AKD1000)
- [x] Basic inference via VFIO DMA (sub-millisecond latency)
- [x] Convert Akida Model Zoo to VFIO-compatible format
  - ModelZoo manager: `crates/neuromorphic/akida-models/src/zoo.rs`
  - CLI tool: `cargo run -p akida-models --bin model_zoo`
  - Stub model generation for testing
- [x] Implement NeuroBench harness in Rust
  - Full harness: `crates/neuromorphic/neurobench-runner/`
  - CLI runner: `cargo run -p neurobench-runner --bin neurobench`
  - Dataset loading: DVS Gesture, Keyword FSCIL, Chaotic, NHP Motor
  - Metrics: accuracy, latency (p95/p99), power, throughput
- [x] Run DVS Gesture, Keyword FSCIL on AKD1000 (via neurobench CLI)
- [x] Compare power/latency vs GPU inference (NPU: 0.3ms, GPU: 11-90ms)

### Phase 4: End-to-End Pipelines (VALIDATED)
- [x] Cascade: GPU preprocess → NPU inference → CPU postprocess
- [x] Heterogeneous ensemble: All silicon active (2 GPU + 2 NPU + 128 CPU)
- [x] Streaming pipeline: 85 ops/sec continuous throughput
- [x] Multi-GPU: Load balance between NVIDIA and AMD (same WGSL)

---

## 5. Resources

### Akida
- Model Zoo: https://doc.brainchipinc.com/model_zoo_performance.html
- MetaTF: https://pypi.org/project/akida-models/
- Edge Impulse: https://docs.edgeimpulse.com/hardware/boards/brainchip-akd1000

### NeuroBench
- Website: https://neurobench.ai/
- GitHub: https://github.com/NeuroBench/neurobench
- Docs: https://neurobench.readthedocs.io/

### Rust ML Frameworks
- Candle: https://github.com/huggingface/candle
- Burn: https://burn.dev/ (wgpu-native)

### hotSpring
- Location: `ecoPrimals/hotSpring/`
- Validation binaries: `barracuda/src/bin/validate_*.rs`
- Sarkas control: `control/sarkas/simulations/dsf-study/`

---

## 6. Success Metrics

| Metric | Target | **Measured (Feb 13, 2026)** |
|--------|--------|---------------------------|
| GPU parity (NVIDIA vs AMD) | <1e-5 difference | **<1e-5 ✓** |
| NPU inference latency | <1ms per sample | **0.27-0.39ms ✓** |
| NPU power efficiency | <1W for keyword spotting | **1.5W (idle ready)** |
| scipy parity | <1e-6 for f64 operations | *Pending hotSpring* |
| BarraCuda vs CUDA | >90% performance parity | *Pending hotSpring* |

### CUDA Parity Benchmark Results (Feb 13, 2026)

### Three-Way Performance Comparison (16M Elements)

| Backend | Device | vector_add | vector_mul | Bandwidth | Gap |
|---------|--------|------------|------------|-----------|-----|
| **CUDA (native)** | RTX 3090 | 229μs | 230μs | 837 GB/s | Baseline |
| **ROCm (native)** | RX 6950 XT | TBD | TBD | TBD | Pending |
| BarraCuda (wgpu) | RTX 3090 | 3067μs | 2725μs | 70 GB/s | **13x** |
| BarraCuda (wgpu) | RX 6950 XT | 1260μs | 913μs | 180 GB/s | **5x** |

### Key Insight: AMD Wins via Vulkan

The AMD RX 6950 XT with RADV (Mesa) driver performs **2.5x better** than
NVIDIA RTX 3090 via wgpu/Vulkan compute:

- AMD: 180 GB/s bandwidth via Vulkan
- NVIDIA: 70 GB/s bandwidth via Vulkan (but 837 GB/s via CUDA!)

NVIDIA's proprietary driver is optimized for CUDA, not Vulkan compute.
This makes AMD the better platform for vendor-agnostic wgpu workloads.

### Optimization Roadmap

| Phase | Target Gap | Strategy |
|-------|------------|----------|
| Current | 13x (NVIDIA), 5x (AMD) | Baseline |
| Phase 1 | 5x | Pipeline cache + batch submit |
| Phase 2 | 2x | Fused kernels + compute graph |
| Phase 3 | 1.5x | Async queues + memory optimization |

See `BARRACUDA_PARITY_ROADMAP.md` for detailed optimization plan.

## Benchmark Findings (Feb 13, 2026)

| Device | Workload | Latency | Notes |
|--------|----------|---------|-------|
| NVIDIA RTX 3090 | 1024×1024 tensor add | 21.51ms | Vulkan via wgpu |
| AMD RX 6950 XT | 1024×1024 tensor add | ~11ms | RADV NAVI21 |
| Akida AKD1000 #1 | VFIO inference | 0.39ms | 80 NPUs |
| Akida AKD1000 #2 | VFIO inference | 0.27ms | 80 NPUs |
| EPYC ×2 | Sparse CG (80×80) | 0.44ms | 128 threads |

**Note**: Software renderer "RTX 3090/PCIe/SSE2" was incorrectly detected as GPU, now filtered.

**Streaming Pipeline**: 85.4 ops/sec, 5.58ms avg latency

---

## 7. Related Specs

- `CROSS_VENDOR_BENCHMARK_SPEC.md` — Hardware benchmark methodology
- `GENERIC_PRECISION_EVOLUTION.md` — f32/f64 precision strategy
- `NPU_DRIVER_ARCHITECTURE.md` — VFIO backend design
- `specs/BARRACUDA_PHASE5_EVOLUTION_HOTSPRING.md` — Phase 5 status
