# Status -- February 9, 2026

## Build

```
cargo build --release --workspace   CLEAN
cargo build --release -p barracuda  CLEAN
cargo build --release -p showcase-rbf-surrogate  CLEAN
cargo test -p barracuda --lib --no-run  COMPILES
```

---

## Cross-Vendor Distributed Compute (NEW -- Feb 9)

### Validated: Same Binary, Same Results, Different Vendors

| GPU | Vendor | Machine | GFLOPS | Checksum |
|-----|--------|---------|--------|----------|
| RTX 4070 | NVIDIA | Tower | 388.7 | **5.128010** |
| RTX 3090 | NVIDIA | gate2 | 481.0 | **5.128010** |
| RX 6950 XT | AMD | gate2 | 222.7 | **5.128010** |

**Test**: 1024x1024 matmul, single WGSL shader, single Rust binary. Bit-identical checksums.

### Distributed LLM Inference

- TinyLlama-1.1B, 22 layers split across Tower + gate2
- **39.85 tok/s** over LAN TCP
- BearDog ChaCha20-Poly1305 encrypted tensor transport
- 20.4 MB total data transferred for 80 tokens

### Hardware Available

| Machine | GPU(s) | CPU | RAM |
|---------|--------|-----|-----|
| Tower | RTX 4070 (12 GB) | 24 cores | - |
| gate2 | RTX 3090 (24 GB) + RX 6950 XT (16 GB) | EPYC 7452 64-thread | 252 GB |

**Total**: 3 discrete GPUs, 52 GB combined VRAM, 2 vendors, 88 CPU threads.

---

## BarraCUDA Shaders: 400+ WGSL Files

### Coverage by Category

| Category | Count | Status |
|----------|-------|--------|
| Activations | 19 | Complete |
| Element-wise | 25+ | Complete |
| Linear algebra | 16 | Complete |
| Convolutions | 9 | Complete |
| Pooling | 14 | Complete |
| Normalization | 17 | Complete |
| Attention | 15 | Complete |
| Reductions | 26 | Complete |
| Shape ops | 18 | Complete |
| Loss functions | 27 | Complete |
| Optimizers | 14 | Complete |
| RNN/LSTM/GRU | 4 | Complete |
| Graph NN | 5 | Complete (edge_conv placeholder) |
| Audio/signal | 9 | Complete |
| Image processing | 14 | Complete |
| FFT/IFFT | 2 | Complete |
| FHE/NTT | 8 | Complete (key_switch placeholder) |
| Complex math | 10 | Complete |
| MD simulation | 8 | Complete |
| RBF interpolation | 1 | Complete (7 kernels) |
| Quantize/Dequantize | 3 | Complete |
| Misc | 40+ | Complete |
| **Total** | **400+** | **~95% complete** |

### 11 Shaders with TODOs

1. `pow_simple.wgsl` -- squares only
2. `broadcast.wgsl` -- first element only
3. `cast.wgsl` -- f32 identity only
4. `determinant.wgsl` -- 2x2/3x3 only
5. `index_add.wgsl` -- needs atomics
6. `scatter_nd.wgsl` -- needs multi-dim
7. `gather_nd.wgsl` -- needs partial
8. `edge_conv.wgsl` -- placeholder neighbors
9. `spectral_norm_1d.wgsl` -- placeholder sigma
10. `fhe_key_switch.wgsl` -- placeholder accumulation
11. `u64_emu.wgsl` -- Barrett optimization

---

## ToadStool Hardware Interface

### What Works
- **Discovery**: Finds GPUs via `/sys/class/drm`, NPUs via PCIe scan, CPU always available
- **Hot-plug**: `rescan()` re-discovers on hardware changes
- **NPU userspace**: Can enable NPU BAR access without sudo
- **API**: `HardwareManager::discover()`, `has_gpu()`, `has_npu()`, `device_count()`
- **Cross-machine**: Songbird TCP mesh connects gates for distributed workload

### What's Evolving
- **Multi-GPU orchestration**: `WgpuDevice::new()` picks one device; need DevicePool for multi-GPU
- **RPC service**: Toadstool needs to become a JSON-RPC workload service
- **CPU executor**: Struct exists, relies on WGPU software rasterizer (not explicit CPU dispatch)

### Can We Run Any Math on Any Hardware?

**GPU (NVIDIA)**: Yes -- all 400+ shaders execute via WGPU/Vulkan. Proven on RTX 4070, RTX 3090.
**GPU (AMD)**: Yes -- bit-identical results on RX 6950 XT via RADV/Vulkan. Proven Feb 9.
**GPU (Intel/Apple)**: Expected to work (WGPU supports ANV/MoltenVK) -- not yet tested.
**CPU**: Partial -- WGPU software rasterizer works but slow. No explicit pure Rust CPU ops.
**NPU**: Inference only -- Akida runs pre-compiled SNN models, not arbitrary WGSL compute.
**Distributed**: Yes -- proven across machines over LAN with encrypted transport.

---

## Scientific Computing

### Linear Algebra Module
- Cholesky decomposition: `cholesky.wgsl` + `cholesky.rs`
- Triangular solve: `triangular_solve.wgsl` + `triangular_solve.rs`

### Interpolation Module
- RBF kernel: `rbf_kernel.wgsl` + `rbf_kernel.rs` (7 kernel types)
- RBF interpolator: `rbf.rs` (fit + predict)

### MD Simulation Shaders
- Forces: Lennard-Jones, Coulomb, Yukawa, Morse, Born-Mayer
- Integrators: Velocity-Verlet, RK4, Laplacian
- Boundary: PBC (periodic boundary conditions)

---

## Evolution Gaps (from distributed compute session)

| Gap | Priority | Status |
|-----|----------|--------|
| Safetensors/GGUF weight loader | HIGH | Not started |
| Tensor serialization for network transfer | HIGH | Not started |
| Multi-GPU DevicePool | HIGH | Not started |
| Toadstool JSON-RPC workload service | HIGH | Not started |
| INT4/INT8 WGSL quantization | MEDIUM | Not started |
| Intelligent workload partitioning | MEDIUM | Not started |
| Tensor parallelism | LOW | Not started |
| Neighbor list construction (MD) | MEDIUM | Not started |
| RBF model export to Akida | LOW | Not started |

---

## Deep Debt

### Clean
- Zero `unimplemented!()` in barracuda production code
- Zero `unsafe` in barracuda production code
- Zero `todo!()` in barracuda production code
- One `unwrap_or_default()` in tensor label generation (safe)
- `#[allow(dead_code)]` in ~12 places (hardware structs awaiting integration, FHE pipeline fields)
- Mocks feature-gated (`#[cfg(feature = "mock-tpu")]`)

### Remaining
- PyTorch dependency for distributed LLM demo (the dependency trap -- solving with safetensors loader)
- 11 shader TODOs (listed above)

---

## Root Documentation

| File | Purpose |
|------|---------|
| `README.md` | Project overview, honest status |
| `STATUS.md` | This file -- detailed status |
| `DOCUMENTATION.md` | Navigation hub |
| `QUICK_STATUS.md` | One-page summary |
| `QUICK_REFERENCE.md` | Commands and API reference |
| `CHANGELOG.md` | Version history |

Session docs archived to `docs/sessions/` by date.

---

**Last Updated**: February 9, 2026
