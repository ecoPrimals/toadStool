# 🚀 START HERE - ToadStool Quick Start

**Welcome!** This is your 5-minute guide to getting started with ToadStool.

**Version**: 3.0.0  
**Updated**: January 14, 2026  
**Status**: ✅ **Production Ready - Grade A (93/100)** 🏆

---

## ⚡ Quick Start (3 Commands)

```bash
# 1. Setup environment
source .envrc

# 2. Build
cargo build --workspace --release

# 3. Test
cargo test --workspace
```

**That's it!** You're ready to use ToadStool.

---

## 🦈 Try GPU Operations (barraCUDA)

```bash
# Navigate to GPU showcase
cd showcase/gpu-universal/ml-inference

# Run ReLU activation demo
cargo run --release --example relu_demo

# Run matrix multiplication
cargo run --release --example matmul_demo

# Try training with Adam optimizer
cargo run --release --example adam_optimizer
```

**🎉 All 21 GPU operations ready to use!**

---

## 🍄 Run ToadStool Server

```bash
# Start server with auto-discovery
cargo run --package toadstool-server --release

# Or specify port and discovery method
cargo run --package toadstool-server --release -- \
  --port 8080 \
  --discovery-method auto
```

Server will:
- ✅ Auto-discover local capabilities (CPU, GPU, memory, network)
- ✅ Start JSON-RPC server (auto-selects available port)
- ✅ Enable workload submission via RPC
- ✅ Monitor system resources in real-time

---

## 📚 What's Next?

### Learn the Basics (30 minutes)

| Time | Document | What You'll Learn |
|------|----------|-------------------|
| **5 min** | [README.md](README.md) | Project overview, architecture, features |
| **5 min** | [STATUS.md](STATUS.md) | Current status, metrics, achievements |
| **10 min** | [TESTING.md](TESTING.md) | Testing guide, coverage reports |
| **10 min** | [DOCUMENTATION.md](DOCUMENTATION.md) | Complete documentation index |

### Try Specialized Features

| Feature | Guide | Time |
|---------|-------|------|
| **GPU Compute** | [QUICK_START_GPU.md](QUICK_START_GPU.md) | 15 min |
| **Encryption** | [QUICK_START_ENCRYPTION.md](QUICK_START_ENCRYPTION.md) | 10 min |
| **Integration** | [PRIMAL_INTEGRATION_GUIDE.md](PRIMAL_INTEGRATION_GUIDE.md) | 20 min |
| **Code Quality** | [PEDANTIC_MODE.md](PEDANTIC_MODE.md) | 10 min |

### Explore Examples

```bash
# Universal runtime examples
cd examples
cargo run --release --example universal_cpu_fallback
cargo run --release --example universal_gpu_preferred

# Distributed coordination
cargo run --release --example distributed_execution

# Container orchestration
cargo run --release --example container_runtime
```

---

## 🎯 Key Features

### Production-Ready Architecture

✅ **Modular Design** - 12-file GPU framework, clean separation  
✅ **Deep Debt Compliant** - 96% (runtime discovery, no hardcoding)  
✅ **Vendor-Agnostic** - Works with NVIDIA, AMD, Intel, Apple GPUs  
✅ **Cross-Platform** - Linux, macOS, Windows  
✅ **100% File Size Compliance** - All files < 1000 lines  
✅ **Comprehensive Testing** - 75.55% coverage, 100 tests passing

### barraCUDA GPU Framework

✅ **21 Operations** - Complete ML/AI operation set  
✅ **Pure Rust** - No C/C++ dependencies  
✅ **Helper Utilities** - 70% boilerplate reduction  
✅ **Tested** - Unit tests for all operations  
✅ **Documented** - Clear examples and guides

### Distributed Execution

✅ **Peer-to-Peer** - No central coordinator required  
✅ **Auto-Discovery** - Finds services via environment, mDNS, K8s  
✅ **Workload Migration** - Move work between nodes  
✅ **Health Monitoring** - Real-time resource tracking  
✅ **Graceful Degradation** - Works optimally with available resources

---

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────┐
│  Your Application                       │
│  (Submit workloads via RPC)             │
└─────────────────────────────────────────┘
           ↓
┌─────────────────────────────────────────┐
│  ToadStool Server                       │
│  - JSON-RPC / tarpc / WebSocket         │
│  - Workload orchestration               │
│  - Resource discovery                   │
└─────────────────────────────────────────┘
           ↓
┌─────────────────────────────────────────┐
│  Runtime Layer (Auto-Selected)          │
│  - GPU (barraCUDA) → NVIDIA/AMD/Intel  │
│  - CPU → Native execution               │
│  - Container → Docker/K8s               │
│  - Universal → Adaptive                 │
└─────────────────────────────────────────┘
           ↓
┌─────────────────────────────────────────┐
│  Hardware Substrate                     │
│  (Discovered at runtime)                │
└─────────────────────────────────────────┘
```

**Key Principle**: Everything discovered at runtime, nothing hardcoded.

---

## 💡 Common Use Cases

### Machine Learning

```rust
use toadstool_runtime_gpu::operations::*;

// Matrix multiplication on GPU
let result = matmul(&weights, &input, &executor).await?;

// Apply activation function
let activated = relu(&result, &executor).await?;

// Normalize
let normalized = layer_norm(&activated, eps, &executor).await?;
```

### Distributed Computing

```rust
use toadstool_client::Client;

let client = Client::connect("http://toadstool-node:8080").await?;

let result = client.submit_workload(WorkloadSubmission {
    runtime: RuntimeType::Gpu,
    resources: ResourceRequirements {
        gpu_memory_mb: 2048,
        cpu_cores: 4,
        ..Default::default()
    },
    payload: your_workload_data,
}).await?;
```

### Container Orchestration

```bash
# Docker
docker run -d \
  --name toadstool \
  -p 8080:8080 \
  --gpus all \
  toadstool:latest

# Kubernetes
kubectl apply -f k8s/toadstool-deployment.yaml
kubectl scale deployment toadstool --replicas=10
```

---

## 🎓 Deep Debt Principles

ToadStool follows **8 Deep Debt Principles**:

1. **No Hardcoding** - All config discovered at runtime
2. **Self-Knowledge** - Each component knows only itself
3. **Runtime Discovery** - Finds other services/resources dynamically
4. **Vendor-Agnostic** - Works with any GPU/cloud/container
5. **Cross-Platform** - Linux, macOS, Windows
6. **Graceful Degradation** - Works optimally with available resources
7. **Pure Rust** - Memory-safe, maintainable
8. **Well-Tested** - Comprehensive test coverage

**Example**: GPU detection discovers ANY GPU vendor at runtime:

```rust
// No hardcoded vendor assumptions!
let gpus = discover_gpus().await?; // Finds NVIDIA, AMD, Intel, Apple
for gpu in gpus {
    println!("Found: {} {} ({})", gpu.vendor, gpu.name, gpu.backend);
}
```

**See [PRIMAL_INTEGRATION_GUIDE.md](PRIMAL_INTEGRATION_GUIDE.md) for complete integration patterns.**

---

## 📊 Current Status

**Grade**: **A (93/100)** 🎉

| Metric | Value | Status |
|--------|-------|--------|
| **File Size Compliance** | 100% | ✅ All < 1000 lines |
| **Build Success** | 100% | ✅ All packages compile |
| **Format Clean** | 100% | ✅ cargo fmt perfect |
| **Deep Debt** | 96% | ✅ Excellent |
| **Test Coverage** | 75.55% | ✅ Good |
| **Tests Passing** | 100 | ✅ Zero failures |

**Path to A+ (96/100)**: Just 3 more points (2-3 weeks)

---

## 🆘 Need Help?

### Documentation

- **[README.md](README.md)** - Complete project overview
- **[STATUS.md](STATUS.md)** - Detailed status and metrics
- **[TESTING.md](TESTING.md)** - Testing guide and coverage
- **[DOCUMENTATION.md](DOCUMENTATION.md)** - Complete index ⭐
- **[docs/architecture/](docs/architecture/)** - Architecture guides
- **[specs/](specs/)** - Technical specifications

### Examples

```bash
# List all examples
ls examples/*.rs

# GPU operations
ls showcase/gpu-universal/ml-inference/examples/*.rs

# Run any example
cargo run --release --example <name>
```

### Common Issues

**Q**: Build fails with "wgpu not found"  
**A**: Enable GPU features: `cargo build --features gpu`

**Q**: Server starts but no GPU detected  
**A**: Check GPU drivers installed. Falls back to CPU gracefully.

**Q**: Tests timeout  
**A**: Some GPU tests need hardware. Skip with: `cargo test --lib`

---

## 🚀 Quick Reference

### Build Commands

```bash
# Full build
cargo build --workspace --release

# Specific crate
cargo build --package toadstool-server --release

# With GPU support
cargo build --features gpu --release

# Check without building
cargo check --workspace
```

### Test Commands

```bash
# All tests
cargo test --workspace

# Unit tests only (fast)
cargo test --workspace --lib

# Specific test
cargo test test_gpu_detection

# With output
cargo test -- --nocapture
```

### Run Commands

```bash
# Server
cargo run --package toadstool-server --release

# CLI
cargo run --package toadstool-cli --release -- --help

# Example
cargo run --release --example universal_cpu_fallback
```

---

## 🎯 Your Learning Path

### Beginner (1 hour)

1. ✅ **Quick Start** - This document (5 min)
2. ✅ **Build & Test** - Verify everything works (10 min)
3. ✅ **Try GPU Demos** - Run barraCUDA examples (15 min)
4. ✅ **Read README** - Understand the project (15 min)
5. ✅ **Check STATUS** - See current metrics (10 min)
6. ✅ **Browse Examples** - Explore code samples (15 min)

### Intermediate (3 hours)

7. 🎯 **Architecture** - Read docs/architecture/ (30 min)
8. 🎯 **Integration** - PRIMAL_INTEGRATION_GUIDE.md (30 min)
9. 🎯 **Testing** - TESTING.md (20 min)
10. 🎯 **GPU Deep Dive** - QUICK_START_GPU.md (30 min)
11. 🎯 **Code Quality** - PEDANTIC_MODE.md (15 min)
12. 🎯 **Run Server** - Deploy your first node (45 min)

### Advanced (Full Day)

13. 🎯 **Specs** - Read specs/ directory (2 hours)
14. 🎯 **Codebase** - Explore crates/ (2 hours)
15. 🎯 **Testing** - Write tests (2 hours)
16. 🎯 **Integration** - Integrate with primals (2 hours)

---

**Welcome to ToadStool!** 🍄

**Grade**: **A (93/100)** 🏆  
**Status**: **Production Ready** ✅  
**Built with ❤️ in Pure Rust** 🦀

---

*"Different orders of the same architecture - composed at runtime, not compile time."*

**Next**: Read [README.md](README.md) or explore [DOCUMENTATION.md](DOCUMENTATION.md)
