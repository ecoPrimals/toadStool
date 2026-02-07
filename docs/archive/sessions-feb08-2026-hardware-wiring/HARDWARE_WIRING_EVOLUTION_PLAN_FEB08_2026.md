# ToadStool Hardware Wiring Evolution Plan
## February 8, 2026 - Deep Debt Elimination

**Priority**: HIGH - Blocks white paper citable claims  
**Principle**: Real hardware execution, zero simulation, complete transparency  
**Status**: Investigation complete, ready for execution

---

## Executive Summary

**Audit Findings**:
- ✅ **Real & Validated**: BarraCUDA core ops (NTT, FFT, Complex, MD forces) execute on actual GPU
- ✅ **Real Hardware Present**: 2x Akida AKD1000 at PCIe a1:00.0, e2:00.0
- ❌ **Fake Execution**: Multiple showcase benchmarks use `sleep()`, `* 3.0` multipliers, hardcoded values
- ⚠️  **Simplified Architectures**: Some ML demos use single-op placeholders instead of full models

**Goal**: Convert every ❌ to ✅ following deep debt principles

---

## Current Status Assessment

### ✅ Real - Production Ready (Keep As-Is)

| Component | Evidence | Status |
|-----------|----------|--------|
| BarraCUDA Core Ops | 250+ ops, 100% WGSL, 40/40 scientific tests | ✅ REAL |
| GPU Detection | wgpu auto-discovery, NVIDIA/AMD validated | ✅ REAL |
| Akida Hardware | 2x AKD1000, `/dev/akida0-1`, driver loaded | ✅ REAL |
| LeNet5 GPU Demo | OpenCL executor, real kernels | ✅ REAL |
| MNIST NPU | `akida_driver::InferenceExecutor::infer()` | ✅ REAL |
| K-mer NPU | Real Akida filter execution | ✅ REAL |
| WGSL Shaders | matmul.wgsl, relu.wgsl, conv2d.wgsl | ✅ REAL |
| FHE Primitives | NTT/INTT/PolyAdd on GPU, 110× speedup | ✅ REAL |

### ❌ Fake - Needs Immediate Fix

| File | Line | Issue | Fix Effort |
|------|------|-------|------------|
| `pipeline_validation_actual_hardware.rs` | 407-411 | NPU = `tokio::time::sleep()` | 1-2 days |
| `pipeline_validation_actual_hardware.rs` | 428 | NPU = `sleep(events * 50)` | 1-2 days |
| `pipeline_validation_actual_hardware.rs` | 465 | NPU = `sleep(events * 50)` | 1-2 days |
| `real_cuda_vs_barracuda.rs` | 281 | GPU = `thread::sleep(100ms)` | DELETE |
| `vendor_agnostic_demo.rs` | 190, 223 | GPU = `forward_cpu()` | DELETE |
| `measurement/power.rs` | 270 | Akida power hardcoded `2.0W` | 1 day |
| `substrates/gpu.rs` | 526 | GPU power hardcoded `150.0W` | 2 hours |

### ⚠️  Simplified - Needs Architecture Completion

| Component | Current | Target | Effort |
|-----------|---------|--------|--------|
| Transformer | Single MatMul per layer | Full multi-head attention | 3-5 days |
| Audio STFT | Tensor ops, no FFT | Use `fft_1d.wgsl` (exists!) | 1-2 days |
| Vision | Simplified Conv2D | Full MobileNet/ResNet | 2-3 days |

---

## Evolution Plan - Phased Approach

### Phase 1: Delete Fakes (Immediate - 30 minutes)

**Principle**: If it's fake, delete it. Don't patch lies.

```bash
# DELETE fake GPU demos
rm showcase/gpu-universal/ml-inference/src/bin/real_cuda_vs_barracuda.rs
rm showcase/gpu-universal/ml-inference/src/bin/vendor_agnostic_demo.rs

# Audit cuda_vs_barracuda_benchmark.rs
# If fake → delete, If real → document, If fixable → fix

# Update shell scripts to point to REAL demos
# - cross_hardware_demo.sh → lenet5_demo (REAL)
# - prove-no-cuda-lockin.sh → comprehensive_benchmark (REAL)
```

**Commit Message**: `chore: Delete fake GPU demos - preserve integrity`

**Deep Debt**: Zero tolerance for simulation claiming hardware execution

---

### Phase 2: Wire Pipeline NPU (High Priority - 1-2 days)

**Target**: `pipeline_validation_actual_hardware.rs` lines 407-411, 428, 465

**Current (FAKE)**:
```rust
// TODO: Wire actual Akida inference
for _ in 0..iterations {
    let events = (iterations as f32 * (1.0 - sparsity)) as u64;
    tokio::time::sleep(tokio::time::Duration::from_micros(events)).await;
}
```

**Target (REAL)** - Pattern from `mnist_npu.rs`:
```rust
// Real Akida inference using akida_driver
use akida_driver::{InferenceConfig, InferenceExecutor};

let config = InferenceConfig::new(
    vec![INPUT_SIZE],  // input shape
    vec![OUTPUT_SIZE], // output shape
    1,                 // batch size
    1                  // device index (0 or 1)
);

let executor = InferenceExecutor::new(config)?;

for _ in 0..iterations {
    // Convert sparse events to Akida-compatible input
    let events_vec = generate_sparse_events(sparsity);
    let result = executor.infer(&events_vec, device_index)?;
    // Process result
}
```

**Implementation Steps**:
1. Import `akida_driver` (already in workspace)
2. Create minimal SNN model for sparse event processing
3. Load `.akd` model file
4. Replace `sleep()` with real `executor.infer()`
5. Measure actual execution time
6. Remove `// TODO: Wire actual Akida inference`

**Testing**:
```bash
cargo test --package homomorphic-computing --example pipeline_validation_actual_hardware
```

**Commit Message**: `feat: Wire pipeline NPU with real Akida execution`

---

### Phase 3: Wire Power Measurement (1 day)

#### 3A: Akida Power (measurement/power.rs:270)

**Current (FAKE)**:
```rust
chip_power.push(("NPU".to_string(), 2.0)); // Akida measured
```

**Target (REAL)**:
```rust
// Query actual Akida power via SDK or sysfs
use akida_driver::DeviceManager;

let manager = DeviceManager::discover()?;
let device = manager.device(device_index)?;

// Option 1: If SDK provides power API
let power_watts = device.query_power()?;

// Option 2: Read from sysfs (if available)
let power_path = format!("/sys/bus/pci/devices/{}/power_state", device.pcie_address());
let power_watts = read_akida_power_sysfs(&power_path)?;

// Option 3: External power meter (most accurate)
// Use USB power meter connected to Akida PCIe slot

chip_power.push(("NPU".to_string(), power_watts));
```

#### 3B: GPU Power (substrates/gpu.rs:526)

**Current (FAKE)**:
```rust
Some(150.0) // Hardcoded
```

**Target (REAL)**:
```bash
# Test nvidia-smi first
nvidia-smi --query-gpu=power.draw --format=csv,noheader,nounits
```

```rust
use std::process::Command;

fn query_nvidia_power() -> Result<f32> {
    let output = Command::new("nvidia-smi")
        .args(&["--query-gpu=power.draw", "--format=csv,noheader,nounits"])
        .output()?;
    
    let power_str = String::from_utf8(output.stdout)?;
    let power_watts = power_str.trim().parse::<f32>()?;
    Ok(power_watts)
}

// Or use NVML bindings for better integration
```

**Testing**:
```bash
# Verify nvidia-smi works
nvidia-smi --query-gpu=power.draw --format=csv,noheader,nounits

# Run benchmark with real power
cargo run --example pipeline_validation_actual_hardware
```

**Commit Message**: `feat: Wire real power measurement for NPU and GPU`

---

### Phase 4: Wire FHE Operation Validation (1 day)

**Target**: `fhe_operation_validation.rs:194`

**Current (FAKE)**:
```rust
// TODO: Replace with actual BarraCUDA FHE operation execution
```

**Target (REAL)**:
```rust
use barracuda::ops::fhe::{FheNtt, FheIntt, FhePolyAdd, FhePolyMul};
use barracuda::tensor::Tensor;
use barracuda::device::WgpuDevice;

// Real BarraCUDA FHE execution
let device = Arc::new(WgpuDevice::new().await?);

// NTT operation
let input = Tensor::from_data(&poly_data, vec![N], device.clone())?;
let ntt_result = FheNtt::new(input, N_POWER)?.execute()?;

// INTT operation
let intt_result = FheIntt::new(ntt_result, N_POWER)?.execute()?;

// Polynomial addition
let poly_a = Tensor::from_data(&a_data, vec![N], device.clone())?;
let poly_b = Tensor::from_data(&b_data, vec![N], device)?;
let add_result = FhePolyAdd::new(poly_a, poly_b)?.execute()?;

// Polynomial multiplication (in NTT domain)
let mul_result = FhePolyMul::new(ntt_a, ntt_b)?.execute()?;
```

**Testing**:
```bash
cargo test --package whitePaper fhe_operation_validation
```

**Commit Message**: `feat: Wire FHE validation with real BarraCUDA ops`

---

### Phase 5: Audit All Showcase Benchmarks (2-3 days)

**Systematic Search**:
```bash
# Find all sleep() calls
rg "tokio::time::sleep|thread::sleep" showcase/ --type rust

# Find all throughput multipliers
rg "throughput \* [0-9]" showcase/ --type rust

# Find all TODOs related to wiring
rg "TODO.*Wire|TODO.*Replace|TODO.*actual" showcase/ --type rust
```

**For Each Finding**:
1. **If fake**: Delete file or replace with real implementation
2. **If TODO**: Wire to real hardware
3. **If hardcoded**: Query from system
4. **If simplified**: Document honestly or complete architecture

**Create Audit Report**: `SHOWCASE_WIRING_AUDIT_FEB08_2026.md`

---

### Phase 6: ML Architecture Completion (Week 3-4)

#### 6A: Audio STFT → Real FFT (1-2 days)

**Current**: Tensor ops without actual FFT  
**Target**: Use `fft_1d.wgsl` (already implemented!)

```rust
use barracuda::ops::fft::Fft1D;

// Real FFT for audio processing
let audio_tensor = Tensor::from_data(&audio_samples, vec![N, 2], device.clone())?;
let fft_result = Fft1D::new(audio_tensor, N_POWER)?.execute()?;

// STFT using sliding window + FFT
for window in audio.windows(WINDOW_SIZE) {
    let window_tensor = Tensor::from_data(window, vec![WINDOW_SIZE, 2], device.clone())?;
    let spectrum = Fft1D::new(window_tensor, WINDOW_POWER)?.execute()?;
    // Process spectrum
}
```

#### 6B: Transformer → Full Multi-Head Attention (3-5 days)

**Current**: Single MatMul per "layer"  
**Target**: Complete attention mechanism

```rust
// Q, K, V projections
let q = MatMul::new(input.clone(), w_q)?.execute()?;
let k = MatMul::new(input.clone(), w_k)?.execute()?;
let v = MatMul::new(input, w_v)?.execute()?;

// Scaled dot-product attention
let scores = MatMul::new(q, k.transpose()?)?.execute()?;
let scores_scaled = Div::new(scores, sqrt_d_k)?.execute()?;
let attn_weights = Softmax::new(scores_scaled, -1)?.execute()?;
let attn_output = MatMul::new(attn_weights, v)?.execute()?;

// Multi-head concatenation + projection
let concat = Concat::new(heads, -1)?.execute()?;
let output = MatMul::new(concat, w_o)?.execute()?;

// Feed-forward network
let ff1 = MatMul::new(output.clone(), w_ff1)?.execute()?;
let ff1_gelu = Gelu::new(ff1)?.execute()?;
let ff2 = MatMul::new(ff1_gelu, w_ff2)?.execute()?;

// Residual + LayerNorm
let residual = Add::new(output, ff2)?.execute()?;
let normalized = LayerNorm::new(residual)?.execute()?;
```

#### 6C: Vision → Full Conv Pipeline (2-3 days)

**Current**: Simplified Conv2D  
**Target**: Complete MobileNet/ResNet

```rust
// MobileNet block
let depthwise = DepthwiseConv2D::new(input, dw_weights, stride, padding)?.execute()?;
let dw_bn = BatchNorm::new(depthwise, bn_params)?.execute()?;
let dw_relu = ReLU::new(dw_bn)?.execute()?;

let pointwise = Conv2D::new(dw_relu, pw_weights, 1, 0)?.execute()?;
let pw_bn = BatchNorm::new(pointwise, bn_params)?.execute()?;
let pw_relu = ReLU::new(pw_bn)?.execute()?;

// Residual connection (if applicable)
if use_residual {
    let output = Add::new(input, pw_relu)?.execute()?;
}
```

---

## Success Criteria

### Must-Have (Week 1-2)
- [ ] Zero `sleep()` calls in any timing benchmark
- [ ] Zero throughput multipliers (`* 3.0`, `* 0.6`)
- [ ] Pipeline NPU uses real `akida_driver::InferenceExecutor`
- [ ] Power from hardware telemetry (nvidia-smi, Akida SDK)
- [ ] Fake GPU demos deleted
- [ ] All TODOs documented or resolved

### Should-Have (Week 3-4)
- [ ] Audio STFT uses real `fft_1d.wgsl`
- [ ] Transformer has full multi-head attention
- [ ] Vision has complete Conv pipeline
- [ ] Comprehensive showcase audit complete

### Nice-to-Have (Week 5+)
- [ ] Reservoir computing NPU wired
- [ ] Dense/sparse NPU wired
- [ ] Hybrid raytracing NPU wired (complex, can defer)

---

## Measurement Philosophy

### Deep Debt Principles

**1. Real Hardware Only**
```rust
// ❌ NEVER DO THIS
tokio::time::sleep(Duration::from_micros(estimated_time));

// ✅ ALWAYS DO THIS
let start = Instant::now();
let result = actual_hardware_function()?;
let measured_time = start.elapsed();
```

**2. Transparent Limitations**
```rust
// ❌ NEVER DO THIS
let throughput = measured_throughput * 3.0; // "Expected production"

// ✅ ALWAYS DO THIS
let throughput = measured_throughput;
// Document: "Simplified model, full pipeline would add X% overhead"
```

**3. Honest Documentation**
```markdown
## What's Real
- ✅ GPU execution via WGSL shaders
- ✅ NPU execution via akida_driver
- ✅ Power from hardware telemetry

## What's Simplified
- ⚠️  Transformer uses single-layer attention (full model would be 12+ layers)
- ⚠️  Vision uses simplified Conv (full ResNet would have 50+ layers)

## What's Not Implemented
- ❌ Distributed multi-NPU (hardware present, software pending)
- ❌ FHE scheme layer (pinned to BearDog/NUCLEUS)
```

---

## Timeline Estimate

| Week | Focus | Deliverable |
|------|-------|-------------|
| 1 | Delete fakes, wire pipeline NPU | Pipeline dispatch ✅ REAL |
| 2 | Wire power measurement, FHE ops | All power ✅ REAL |
| 3 | Audio FFT, showcase audit | Audio + audit complete |
| 4 | Transformer attention, vision pipeline | ML architectures ✅ COMPLETE |
| 5+ | Optional: Reservoir, dense/sparse, raytracing | Full coverage |

**Critical Path**: Weeks 1-2 (unblock white paper claims)  
**Total Effort**: ~6-8 weeks for 100% completion

---

## Risks & Mitigation

### Risk 1: Akida SDK API Unclear

**Mitigation**: Use existing `mnist_npu.rs` and `kmer_npu` patterns. If SDK incomplete, document honestly and query via sysfs/PCIe.

### Risk 2: Performance Regression

**Mitigation**: Real measurements may be slower than estimates. Document actual performance, don't fake to match expectations.

### Risk 3: Complex NPU Models

**Mitigation**: Start with simple SNN for sparse events. Defer complex reservoir/raytracing to Week 5+.

---

## Next Actions

### Immediate (Today)
1. ✅ Complete this evolution plan
2. Delete fake GPU demos
3. Create audit report template

### Tomorrow
4. Wire pipeline NPU (line 407-411)
5. Test with real Akida execution
6. Document results

### This Week
7. Wire power measurement
8. Wire FHE ops validation
9. Complete Phase 1-4

---

**Status**: Evolution plan complete, ready for execution  
**Principle**: Deep debt elimination - real hardware, zero simulation  
**Target**: 100% hardware-validated showcase by Week 4  
**Fossil Record**: All decisions and trade-offs documented

---

*Created: February 8, 2026 5:30 AM*  
*Next Review: After Phase 1 completion*  
*Priority: HIGH - Blocks white paper publication*
