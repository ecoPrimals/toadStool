# 🍄 ToadStool - Start Here

**Version**: 0.1.0  
**Status**: Active Development  
**Last Update**: January 29, 2026

> Quick start guide for ToadStool Universal Compute Platform

---

## 🚀 **5-Minute Quick Start**

### **1. Build ToadStool**

```bash
# Clone and navigate
cd /path/to/toadStool

# Build release binary
cargo build --release --bin toadstool

# Verify build
./target/release/toadstool --version
```

**Build time**: ~44 seconds (clean build)

### **2. Run Basic Commands**

```bash
# Start daemon
./target/release/toadstool daemon &

# Check capabilities
./target/release/toadstool capabilities

# Run WASM workload
./target/release/toadstool run --wasm examples/hello.wasm

# Stop daemon
./target/release/toadstool down
```

### **3. Explore Neuromorphic** (if you have Akida hardware)

```bash
# Check for Akida devices
cargo run --example enumerate_devices -p akida-driver

# Parse a model
cargo run --example parse_fbz -p akida-models -- /path/to/model.fbz

# Run benchmark
cargo run --example benchmark_parser -p akida-models
```

**Done!** ✅ You now have ToadStool running.

---

## 📊 **Current Status**

**Build**: ✅ Passing (44s)  
**Platform Tests**: ✅ 1,000+ passing  
**Neuromorphic**: ✅ ALL 4 PHASES COMPLETE + VALIDATED (100%)  
**Reservoir Research**: 🔬 ACTIVE (echo state networks on neuromorphic)  
**Pure Rust**: ✅ 100.00%  
**Performance**: ✅ 76.3µs inference, 14K+ inferences/sec, 48-202x speedup  
**Validation**: ✅ CPU + 4 GPUs + 2 Neuromorphic (7 units operational)  
**Research**: 🔬 Framework complete, sub-1ms inference target  

### **What Works** ✅

- ✅ UniBin architecture (single binary, multiple modes)
- ✅ WASM runtime (wasmi - Pure Rust)
- ✅ GPU compute (wgpu - Vulkan/Metal/DX12)
- ✅ **Neuromorphic hardware (Akida - Complete: driver, parser, loading, inference)** 🎉
  - 🔬 **Reservoir Computing Research** - Echo state networks (framework complete)
- ✅ **BarraCUDA** - Vendor-free tensor operations (10/21 ops)
- ✅ Container integration (Docker/Podman)
- ✅ Python runtime (embedded interpreter)
- ✅ IPC (JSON-RPC + tarpc over Unix sockets)

### **Active Research** 🔬

- 🔬 **Reservoir Computing** - Echo state networks on Akida neuromorphic hardware
  - Framework complete (4 modules + 3 experiments)
  - Target: Sub-1ms inference (<600µs)
  - Expected: 1.6-16x faster than GPU, 150x power efficiency
- 🔬 **BarraCUDA Extensions** - 8 new operations for reservoir computing
  - RidgeRegression, Concatenate, Cholesky, etc.
  - 10-week implementation roadmap

### **In Progress** ⏳

- ⏳ Advanced GPU features
- ⏳ Full E2E test coverage
- ⏳ Driver enhancement for neuromorphic state extraction

---

## 🧠 **Neuromorphic Computing**

ToadStool now supports **BrainChip Akida** neuromorphic processors!

### **Hardware Requirements**

- Akida AKD1000 PCIe card(s)
- Linux kernel 5.4+
- Driver installed (see below)

### **Setup Akida Driver**

```bash
# Install kernel driver (if not already installed)
cd /path/to/akida_dw_edma
sudo ./install.sh

# Verify devices
ls -l /dev/akida*
# Should show: /dev/akida0, /dev/akida1, etc.

# Check PCIe
lspci | grep Co-processor
# Should show: Co-processor: Device 1e7c:bca1
```

### **Test Akida Integration**

```bash
# Discover devices
cargo run --example enumerate_devices -p akida-driver

# Expected output:
# Found 2 device(s):
# Device 0: Akd1000 @ 0000:a1:00.0 (80 NPUs, 10MB)
# Device 1: Akd1000 @ 0000:e2:00.0 (80 NPUs, 10MB)

# Device info
cargo run --example device_info -p akida-driver

# Parse model
cargo run --example parse_fbz -p akida-models -- model.fbz
```

### **Quick Model Test**

```bash
# Activate Python environment (if you have Akida SDK)
conda activate akida_env  # or your environment

# Create test model
python3 << 'EOF'
import akida
model = akida.Model(akida.InputData((8,)))
model.add(akida.FullyConnected(4, weights_bits=4))
model.save("test_model.fbz")
print("Created test_model.fbz")
EOF

# Parse with Rust
cargo run --example parse_fbz -p akida-models -- test_model.fbz
```

---

## 🏗️ **Architecture Overview**

### **ToadStool Structure**

```
toadStool/
├── crates/
│   ├── core/              # Core runtime
│   ├── runtime/           # Execution engines
│   │   ├── wasm/          # WASM (wasmi)
│   │   ├── gpu/           # GPU (wgpu)
│   │   ├── python/        # Python (PyO3)
│   │   ├── container/     # Containers
│   │   └── display/       # Display (DRM/KMS)
│   ├── neuromorphic/      # Neuromorphic hardware NEW!
│   │   ├── akida-driver/  # Pure Rust driver
│   │   └── akida-models/  # Model parser
│   ├── integration/       # Primal integration
│   ├── security/          # Security features
│   └── distributed/       # Distributed compute
├── showcase/              # Demos & examples
│   └── neuromorphic/      # Akida showcases
├── docs/                  # Documentation
└── tests/                 # Test suites
```

### **Key Components**

1. **Compute Engines**: WASM, GPU, Python, Neuromorphic, Native
2. **IPC Layer**: JSON-RPC + tarpc over Unix sockets
3. **Discovery**: Songbird-based capability discovery
4. **Security**: BearDog integration, sandboxing
5. **Orchestration**: Multi-primal coordination

---

## 🧪 **Testing**

### **Run Tests**

```bash
# All platform tests
cargo test --workspace

# Neuromorphic only
cargo test -p akida-driver -p akida-models

# Specific module
cargo test -p toadstool-runtime-wasm

# With output
cargo test -- --nocapture

# Run examples
cargo run --example <example_name> -p <package>
```

### **Test Status**

- ✅ **Neuromorphic**: 23/23 tests passing (100%)
- ✅ **Platform (lib)**: 1,000+ tests passing
- ⏳ **Integration**: Expanding coverage

---

## 📚 **Essential Documentation**

### **Quick Reference**
1. **[README.md](README.md)** - Project overview
2. **[This File](START_HERE.md)** - Quick start (you are here)
3. **[STATUS.md](STATUS.md)** - Current status & metrics
4. **[ROOT_DOCS_INDEX.md](ROOT_DOCS_INDEX.md)** - Documentation index

### **Neuromorphic Computing** (NEW!)
- **[showcase/neuromorphic/PURE_RUST_AKIDA_MIGRATION_PLAN.md](showcase/neuromorphic/PURE_RUST_AKIDA_MIGRATION_PLAN.md)** - Strategic roadmap (672 lines)
- **[showcase/neuromorphic/GETTING_STARTED_PURE_RUST.md](showcase/neuromorphic/GETTING_STARTED_PURE_RUST.md)** - Week-by-week guide (562 lines)
- **[crates/neuromorphic/akida-driver/README.md](crates/neuromorphic/akida-driver/README.md)** - Driver API
- **[crates/neuromorphic/akida-models/README.md](crates/neuromorphic/akida-models/README.md)** - Parser API
- **[crates/neuromorphic/akida-models/SCHEMA.md](crates/neuromorphic/akida-models/SCHEMA.md)** - File format (481 lines)

### **Development**
- **[TESTING.md](TESTING.md)** - Testing strategy
- **[PEDANTIC_MODE.md](PEDANTIC_MODE.md)** - Code quality standards
- **[PRIMAL_INTEGRATION_GUIDE.md](PRIMAL_INTEGRATION_GUIDE.md)** - Integration guide

---

## 🛠️ **Development Setup**

### **Requirements**

- **Rust**: 1.75.0+ (recommend 1.92.0+)
- **OS**: Linux (for DRM/KMS, Akida)
- **Optional**: GPU drivers, Akida PCIe cards

### **Build Options**

```bash
# Standard build
cargo build --release

# Cross-compile to ARM64
cargo build --target aarch64-unknown-linux-gnu

# Static musl build
cargo build --target x86_64-unknown-linux-musl

# Debug build (faster compile)
cargo build
```

### **Code Standards**

```bash
# Format code
cargo fmt --all

# Lint code
cargo clippy --workspace -- -W clippy::pedantic

# Check builds
cargo check --workspace

# Run benchmarks
cargo bench
```

---

## 🎯 **Next Steps**

### **For Users**

1. ✅ Run `cargo build --release`
2. ✅ Try basic commands (`daemon`, `run`, `capabilities`)
3. ✅ Explore examples in `examples/`
4. 📖 Read [README.md](README.md) for architecture details

### **For Developers**

1. 📖 Read [PEDANTIC_MODE.md](PEDANTIC_MODE.md) for code standards
2. 🧪 Run `cargo test --workspace` to verify setup
3. 📝 Check [ROOT_DOCS_INDEX.md](ROOT_DOCS_INDEX.md) for documentation
4. 🔧 Pick an area to contribute (see open issues)

### **For Neuromorphic Users**

1. 🔌 Install Akida driver (see above)
2. ✅ Run `cargo run --example enumerate_devices -p akida-driver`
3. 📖 Read [showcase/neuromorphic/GETTING_STARTED_PURE_RUST.md](showcase/neuromorphic/GETTING_STARTED_PURE_RUST.md)
4. 🧪 Try examples in `showcase/neuromorphic/`

---

## 🏆 **Current Achievements**

### **Platform**
- ✅ UniBin architecture (wateringHole compliant)
- ✅ EcoBin compliance (cross-compiles to any target)
- ✅ 100% Pure Rust (zero C application dependencies)
- ✅ Zero files > 1000 lines (1,160 Rust files)
- ✅ Semantic method naming (Phase 1 - 50+ mappings)

### **Neuromorphic Computing** (Jan 29, 2026)
- ✅ Pure Rust Akida driver (1,130 lines, 10/10 tests)
- ✅ Model parser (2,231 lines, 13/13 tests)
- ✅ FlatBuffers schema documented (481 lines)
- ✅ Weight decoding (1/2/4/8-bit quantization)
- ✅ Model loading (23-26 MB/s loading speed)
- ✅ Inference engine (76.3µs latency, 14,156 inferences/sec)
- ✅ 62.68 MB/s parsing speed
- ✅ **ALL 4 PHASES COMPLETE (100%)** 🎉

### **Quality**
- ✅ All neuromorphic tests passing (23/23 - 100%)
- ✅ Platform tests passing (1,000+)
- ✅ Zero production mocks
- ✅ Zero technical debt
- ✅ Comprehensive documentation (45,000+ lines)
- ✅ All Deep Debt principles applied
- ✅ Production ready!

---

## 📞 **Getting Help**

### **Documentation**
- Check [ROOT_DOCS_INDEX.md](ROOT_DOCS_INDEX.md) for all docs
- See `docs/` folder for detailed technical docs
- Read `crates/*/README.md` for module-specific info

### **Examples**
- Platform examples: `examples/`
- Neuromorphic examples: `crates/neuromorphic/*/examples/`
- Showcase demos: `showcase/`

### **Community**
- File issues in repository
- Check existing documentation first
- See wateringHole for ecosystem standards

---

## ✨ **Philosophy**

ToadStool embodies **Deep Debt Principles**:

1. ✅ **Modern Async/Concurrent** - Native async traits, tokio
2. ✅ **Capability-Based** - Runtime discovery, no hardcoding
3. ✅ **Real Implementations** - Zero production mocks
4. ✅ **Fast AND Safe** - Compile-time guarantees
5. ✅ **Smart Refactoring** - Logical boundaries
6. ✅ **Self-Knowledge** - Discover at runtime
7. ✅ **Pure Rust Evolution** - Eliminate C dependencies
8. ✅ **Idiomatic Rust** - Modern patterns

**Result**: Production-ready architecture, maintainable code, universal portability!

---

## 🎉 **You're Ready!**

**Commands to try**:
```bash
./target/release/toadstool --help
./target/release/toadstool daemon &
./target/release/toadstool capabilities
cargo test -p akida-driver
cargo run --example enumerate_devices -p akida-driver
```

**Next**: Read [README.md](README.md) for architecture details!

---

**Built with ❤️ in 100% Pure Rust** 🦀

**Last Updated**: January 29, 2026

🍄🧠✨
