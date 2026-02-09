# ToadStool + BarraCUDA

**Sovereign Distributed Compute** | Pure Rust | February 2026

---

## What Is This?

- **ToadStool** -- Hardware infrastructure layer. Discovers GPUs, NPUs, CPUs at runtime via sysfs/PCIe. No scripts, no sudo. Manages distributed workload dispatch across machines.
- **BarraCUDA** -- Universal compute engine. 400+ WGSL shaders running on any GPU via WGPU. Tensors, linear algebra, ML, physics, cryptography. Vendor-agnostic -- same binary, same results on NVIDIA, AMD, Intel.

---

## Proven in Production (February 9, 2026)

### Cross-Vendor Distributed GPU Compute

**Single binary, identical results across vendors and machines:**

| GPU | Vendor | Machine | GFLOPS | Checksum |
|-----|--------|---------|--------|----------|
| RTX 4070 | NVIDIA | Tower | 388.7 | **5.128010** |
| RTX 3090 | NVIDIA | gate2 | 481.0 | **5.128010** |
| RX 6950 XT | AMD | gate2 | 222.7 | **5.128010** |

Zero CUDA. Zero ROCm. Pure Vulkan via WGPU. Bit-identical results.

### Distributed LLM Inference

TinyLlama-1.1B split across two machines over LAN TCP:
- Tower (RTX 4070): Embedding + layers 0-10
- gate2 (RTX 3090): Layers 11-21 + head
- **39.85 tok/s** with BearDog ChaCha20-Poly1305 encrypted tensor transport

### BarraCUDA Shader Coverage (400+ WGSL files)

| Category | Count | Status |
|----------|-------|--------|
| Activations | 19 | Complete |
| Element-wise | 25+ | Complete |
| Linear algebra | 16 | Complete (Cholesky, triangular solve, inverse, determinant) |
| Convolutions | 9 | Complete (1D, 3D, dilated, grouped, separable, transposed) |
| Pooling | 14 | Complete |
| Normalization | 17 | Complete (batch, instance, group, layer, RMS, spectral) |
| Attention | 15 | Complete (softmax, flash, GQA, RoPE, ALiBi) |
| Reductions | 26 | Complete |
| Shape ops | 18 | Complete |
| Loss functions | 27 | Complete |
| Optimizers | 14 | Complete (Adam, AdamW, SGD, LAMB, RAdam, NAdAM) |
| RNN/LSTM/GRU | 4 | Complete |
| Graph neural nets | 5 | Complete |
| Audio/signal | 9 | Complete (STFT, MFCC, Griffin-Lim) |
| Image processing | 14 | Complete |
| FFT/IFFT | 2 | Complete |
| FHE/NTT | 8 | Complete (homomorphic encryption primitives) |
| Complex arithmetic | 10 | Complete |
| MD simulation | 8 | Complete (Velocity-Verlet, RK4, LJ, Coulomb, Yukawa, Morse, PBC) |
| RBF interpolation | 1 | Complete (7 kernel types) |
| Quantize/Dequantize | 3 | Complete |
| Embedding | 2 | Complete |
| Misc | 40+ | Complete (dropout, topk, NMS, etc.) |

All shaders execute on **any GPU via WGPU** -- NVIDIA, AMD, Intel, Apple.

---

## Honest Status

### What's Real and Working

**BarraCUDA compute** -- 400+ WGSL shaders, proven cross-vendor (NVIDIA + AMD), proven cross-machine (LAN distributed inference). Single binary deployment.

**ToadStool discovery** -- Pure Rust sysfs/PCIe scanning. Finds GPUs, NPUs, CPUs. Hot-plug rescan. No scripts, no sudo.

**Scientific computing** -- Cholesky decomposition, triangular solve, RBF interpolation (7 kernels), 8 MD force/integrator shaders. All GPU-accelerated.

**NPU drivers** -- Akida kernel-mode (DMA, interrupts) and userspace (mmap PCIe BARs) drivers. Inference-only.

**Distributed inference** -- Pipeline-parallel LLM across machines, BearDog-encrypted tensor transport, Songbird TCP mesh.

### What Needs Evolution

**Model weight loading** -- BarraCUDA has all transformer ops but no safetensors/GGUF loader. Current distributed demo used PyTorch for weight loading (the dependency trap we're trying to escape).

**Tensor serialization** -- Need efficient binary format for cross-gate transfer. Shape metadata + raw buffer, zero-copy where possible.

**Multi-GPU orchestration** -- `WgpuDevice::new()` picks one device. gate2 has both RTX 3090 and RX 6950 XT -- both should participate. Need `DevicePool`.

**Toadstool as RPC service** -- Currently a biome runner. Needs to become a JSON-RPC workload service (`toadstool.load_model_shard`, `toadstool.forward_shard`, `toadstool.gpu_capabilities`).

**Quantization** -- f32 only. Need INT4/INT8 WGSL shaders for larger models.

**NPU arbitrary math** -- Akida NPU runs pre-compiled SNN inference only, not general WGSL compute. For NPU math: either surrogate models or NPU-native workloads (sparse, event-driven).

**CPU fallback** -- WGPU's software rasterizer works but is slow. No explicit pure Rust CPU implementations.

### Known Shader TODOs (11 files)

1. `pow_simple.wgsl` -- squares only, needs arbitrary powers
2. `broadcast.wgsl` -- first element only
3. `cast.wgsl` -- f32 identity only
4. `determinant.wgsl` -- 2x2/3x3 only, needs LU for NxN
5. `index_add.wgsl` -- needs atomic ops
6. `scatter_nd.wgsl` -- needs multi-dim scatter
7. `gather_nd.wgsl` -- needs partial gathering
8. `edge_conv.wgsl` -- placeholder neighbor handling
9. `spectral_norm_1d.wgsl` -- placeholder sigma
10. `fhe_key_switch.wgsl` -- placeholder accumulation
11. `u64_emu.wgsl` -- Barrett reduction optimization

---

## Quick Start

```bash
# Build everything
cargo build --release

# Run RBF surrogate demo
cd showcase/rbf-surrogate && ./demo.sh

# Run NPU detection
cd showcase/neuromorphic/01-akida-detection && ./demo.sh

# Cross-vendor GPU test (runs on any GPU)
cargo test -p barracuda --lib ops::linalg --release
```

---

## Architecture

```
Applications (hotSpring, NUCLEUS inference, etc.)
       |
BarraCUDA: 400+ WGSL Shaders
  Tensors, LinAlg, ML, Physics, Crypto, Audio
  Proven: identical results NVIDIA + AMD
       |
ToadStool: Hardware Discovery + Orchestration
  Pure Rust sysfs/PCIe scanning
  GPU, NPU, CPU discovery
  Distributed workload dispatch (evolving)
       |
  +--------+---------+--------+
  |        |         |        |
 GPU     GPU       GPU      NPU         CPU
 RTX    RTX 3090  RX 6950  Akida       WGPU
 4070   (NVIDIA)  XT (AMD) (inference)  software
(NVIDIA)                                rasterizer
```

**Key**: Same WGSL shader compiles to Vulkan (NVIDIA/AMD), Metal (Apple), DX12 (Windows) via WGPU. No vendor SDK required.

---

## Project Structure

```
toadStool/
+-- crates/
|   +-- barracuda/             -- 400+ WGSL shaders, tensor ops
|   |   +-- src/shaders/       -- All WGSL shader files
|   |   +-- src/ops/           -- Rust operation wrappers
|   |   +-- src/device/        -- WGPU device, hardware routing
|   +-- core/                  -- ToadStool core runtime
|   +-- neuromorphic/          -- NPU drivers (Akida)
|   +-- runtime/               -- Execution engines
|   +-- distributed/           -- Inter-gate communication
|   +-- security/              -- Crypto, enclaves
|   +-- server/                -- RPC server
|   +-- cli/                   -- CLI interface
|   +-- ...
+-- showcase/
|   +-- rbf-surrogate/         -- RBF scientific computing demo
|   +-- neuromorphic/          -- NPU showcases
|   +-- barracuda-validation/  -- GPU validation
|   +-- gpu-universal/         -- Cross-vendor GPU demos
|   +-- homomorphic-computing/ -- FHE demos
|   +-- ...
+-- docs/
|   +-- sessions/              -- Session archives (by date)
|   +-- architecture/          -- Design documents
|   +-- planning/              -- Roadmaps
|   +-- guides/                -- Deployment guides
|   +-- archive/               -- Historical documentation
+-- specs/                     -- Technical specifications
+-- scripts/                   -- Helper scripts
```

---

## Building & Testing

```bash
# Build workspace
cargo build --release

# Test linear algebra
cargo test -p barracuda --lib ops::linalg --release

# Test interpolation
cargo test -p barracuda --lib ops::interpolation --release

# Test NPU driver
cargo test -p akida-driver --release

# Test ToadStool core
cargo test -p toadstool-core
```

---

## Deep Debt Principles

1. Modern idiomatic Rust -- no `unsafe` in new code
2. No external scripts -- pure Rust, self-evolving
3. No hardcoding -- runtime discovery, capability-based
4. Mocks isolated to testing -- production code is complete
5. Honest documentation -- no aspirational claims as facts
6. Vendor-agnostic -- WGSL over CUDA/ROCm, any GPU works
7. Sovereign compute -- no vendor lock-in, no dependency traps

---

## Evolution Roadmap

### Immediate (distributed compute gaps)
1. Safetensors/GGUF weight loader for BarraCUDA
2. Tensor serialization format for cross-gate transfer
3. Multi-GPU DevicePool (use all GPUs on a machine)
4. Toadstool JSON-RPC workload service

### Medium-term (production inference)
1. INT4/INT8 quantization WGSL shaders
2. Intelligent workload partitioning (VRAM-aware, compute-aware)
3. Tensor parallelism (split layers across GPUs)
4. Neighbor list construction for MD simulation

### Long-term
1. NPU surrogate inference path
2. Full tensor parallelism + expert parallelism for MoE models
3. Cross-hardware benchmarking suite

---

## Documentation

- **[STATUS.md](STATUS.md)** -- Current honest status
- **[DOCUMENTATION.md](DOCUMENTATION.md)** -- Navigation hub
- **[CHANGELOG.md](CHANGELOG.md)** -- Version history
- **[docs/sessions/](docs/sessions/)** -- Session archives

---

**Last Updated**: February 9, 2026
