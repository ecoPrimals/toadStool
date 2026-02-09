# ToadStool Quick Reference

**February 9, 2026**

---

## Build

```bash
# Full workspace
cargo build --release

# Just barracuda
cargo build --release -p barracuda

# RBF showcase
cargo build --release -p showcase-rbf-surrogate
```

---

## Test

```bash
# BarraCUDA linear algebra
cargo test -p barracuda --lib ops::linalg --release

# BarraCUDA interpolation (RBF)
cargo test -p barracuda --lib ops::interpolation --release

# ToadStool hardware discovery
cargo test -p toadstool-core

# NPU drivers
cargo test -p akida-driver --lib

# Full barracuda test suite
cargo test -p barracuda --lib --release
```

---

## Showcases

```bash
# RBF surrogate learning
cd showcase/rbf-surrogate && ./demo.sh

# NPU detection
cd showcase/neuromorphic/01-akida-detection && ./demo.sh

# NPU bioinformatics
cd showcase/neuromorphic/02-akida-bioinformatics && ./demo-kmer-filtering.sh

# GPU validation
cd showcase/barracuda-validation && cargo test --release
```

---

## Hardware Discovery API

```rust
use toadstool_core::HardwareManager;

let hw = HardwareManager::discover()?;
println!("Devices: {}", hw.device_count());
println!("GPU: {}", hw.has_gpu());
println!("NPU: {}", hw.has_npu());

// Rescan after hardware change
hw.rescan()?;
```

---

## BarraCUDA Device API

```rust
use barracuda::device::WgpuDevice;

// Auto-select best GPU
let device = WgpuDevice::new().await?;

// Explicit GPU
let gpu = WgpuDevice::new_gpu().await?;

// Explicit CPU (software rasterizer)
let cpu = WgpuDevice::new_cpu().await?;

// From ToadStool selection
use barracuda::device::{select_best_device, HardwareWorkload};
let selection = select_best_device(HardwareWorkload::TensorOps)?;
let device = WgpuDevice::from_selection(selection).await?;

// Enumerate all adapters
let adapters = WgpuDevice::enumerate_adapters().await;
```

---

## Key Operations

```rust
use barracuda::tensor::Tensor;

// Create tensors
let a = Tensor::randn(vec![1024, 1024]).await?;
let b = Tensor::randn(vec![1024, 1024]).await?;

// Matrix multiply (WGSL shader, runs on any GPU)
let c = a.matmul(&b).await?;

// Read results back
let data = c.to_vec()?;
```

---

## Cross-Vendor Compute

Same binary runs on NVIDIA, AMD, Intel, Apple:

```
Rust binary (cargo build --release)
  -> BarraCUDA (WGSL shaders)
    -> wgpu (auto-selects backend)
      -> Vulkan (NVIDIA/AMD/Intel)
      -> Metal (Apple)
      -> DX12 (Windows)
      -> CPU (software rasterizer fallback)
```

Proven bit-identical on: RTX 4070, RTX 3090, RX 6950 XT.

---

## Documentation

| File | What |
|------|------|
| [README.md](README.md) | Overview, architecture, status |
| [STATUS.md](STATUS.md) | Detailed technical status |
| [DOCUMENTATION.md](DOCUMENTATION.md) | Navigation hub |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [TESTING.md](TESTING.md) | Test strategy |
| [QUICK_START_GPU.md](QUICK_START_GPU.md) | GPU quick start |
| [QUICK_START_ENCRYPTION.md](QUICK_START_ENCRYPTION.md) | FHE quick start |

---

**Last Updated**: February 9, 2026
