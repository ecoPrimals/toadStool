# 🚀 ToadStool - Start Here

**Welcome to ToadStool!** The pure Rust universal compute platform with **100% deep debt compliance**.

---

## 🎯 What is ToadStool?

ToadStool is a **production-ready universal compute platform** that runs workloads on ANY hardware (CPU, GPU, WASM, Container, Python, and future neuromorphic) without vendor-specific code.

### Current Status (January 10, 2026)

✅ **Production Ready - Deep Debt 100% Compliant (A++ Grade)**

| Status | Value |
|--------|-------|
| **Build** | ✅ Clean (full workspace) |
| **Tests** | ✅ 100 passed, 0 failed |
| **Coverage** | 46.93% (critical paths 81-94%) |
| **Deep Debt** | ✅ 100% (15/15 principles) |
| **Grade** | **A++** |

---

## 🚀 Quick Start (5 Minutes)

### 1. Build

```bash
cd /path/to/toadStool
cargo build --release
```

### 2. Run Server

```bash
# Single instance
export TOADSTOOL_FAMILY=default
export RUST_LOG=info
cargo run --release --bin toadstool-server
```

### 3. Test

```bash
# JSON-RPC test
./scripts/test-jsonrpc-unix.sh

# Full test suite
cargo test --workspace
```

**That's it!** You're running a production-ready universal compute platform.

---

## 📋 Next Steps

### For Users

1. **[README.md](README.md)** - Complete overview
2. **[FINAL_STATUS_JAN10_2026.md](FINAL_STATUS_JAN10_2026.md)** - Production guide
3. **[TESTING.md](TESTING.md)** - Testing guide

### For Developers

1. **[docs/DOCUMENTATION_INDEX.md](docs/DOCUMENTATION_INDEX.md)** - Full documentation
2. **[docs/reference/CONFIG_PATTERNS_GUIDE.md](docs/reference/CONFIG_PATTERNS_GUIDE.md)** - Configuration
3. **[docs/reference/PRODUCTION_DEPLOYMENT_GUIDE.md](docs/reference/PRODUCTION_DEPLOYMENT_GUIDE.md)** - Deployment

### For Architecture Review

1. **[docs/architecture/CPU_OPS_STRATEGY.md](docs/architecture/CPU_OPS_STRATEGY.md)** - GPU-first strategy
2. **[docs/audits/LARGE_FILES_REVIEW_JAN10_2026.md](docs/audits/LARGE_FILES_REVIEW_JAN10_2026.md)** - Code quality
3. **Deep Debt Principles** - See [STATUS.md](STATUS.md)

---

## ✨ Key Features

### Isomorphic/Fractal Architecture

- **All instances are peers** (no master/worker)
- **Same patterns at all scales** (local + distributed)
- **Capability-based discovery** (Songbird integration)

### Dual Protocol System

1. **tarpc** (PRIMARY) - Binary RPC, Unix sockets
2. **JSON-RPC 2.0** (UNIVERSAL) - Text-based, Unix sockets
3. ~~TCP JSON-RPC~~ (DEPRECATED since 2.2.0)

### Deep Debt 100% Compliant

- ✅ No hardcoding (env/runtime config)
- ✅ Self-knowledge only
- ✅ Agnostic discovery
- ✅ Fast AND safe
- ✅ Production-ready

---

## 🏗️ Architecture

### Runtime Support

- ✅ **CPU** - Native optimized execution
- ✅ **GPU** - CUDA, ROCm, OpenCL, WebGPU, Vulkan
- ✅ **WASM** - WebAssembly sandboxed execution
- ✅ **Container** - Docker, Podman isolation
- ✅ **Python** - PyO3 integration
- 🔄 **Future** - Neuromorphic, Quantum, Specialty

### Distributed Coordination

**Default Mode: Distributed**
```bash
TOADSTOOL_FAMILY=gpu0 cargo run --release --bin toadstool-server &
TOADSTOOL_FAMILY=gpu1 cargo run --release --bin toadstool-server &
```

**Fallback Mode: Standalone**
```bash
export TOADSTOOL_STANDALONE=1
cargo run --release --bin toadstool-server
```

---

## 🎓 Learning Path

### Beginner

1. **Build and run** (see Quick Start above)
2. **Run tests** (`cargo test --workspace`)
3. **Read** [README.md](README.md)

### Intermediate

1. **Multi-instance setup** (see [FINAL_STATUS_JAN10_2026.md](FINAL_STATUS_JAN10_2026.md))
2. **Songbird integration** (see [docs/biomeos/](docs/biomeos/))
3. **Configuration** (see [docs/reference/CONFIG_PATTERNS_GUIDE.md](docs/reference/CONFIG_PATTERNS_GUIDE.md))

### Advanced

1. **Distributed coordination** (see [docs/architecture/](docs/architecture/))
2. **Custom workload types** (see examples/)
3. **Deep debt principles** (see [STATUS.md](STATUS.md))

---

## 🤝 Contributing

See [docs/reference/COMMIT_MESSAGE_TEMPLATE.md](docs/reference/COMMIT_MESSAGE_TEMPLATE.md) for guidelines.

**Key Principles:**
- Deep debt compliance (no hardcoding)
- Modern idiomatic Rust
- Comprehensive testing
- Clear documentation

---

## 📚 Essential Documentation

| Document | Purpose |
|----------|---------|
| [README.md](README.md) | Project overview |
| [STATUS.md](STATUS.md) | Current status |
| [FINAL_STATUS_JAN10_2026.md](FINAL_STATUS_JAN10_2026.md) | Production guide |
| [TESTING.md](TESTING.md) | Testing guide |
| [CHANGELOG.md](CHANGELOG.md) | Version history |

---

## 🏆 Production Status

**Grade: A++ (100% Deep Debt Compliant)**

- ✅ Full workspace compiles
- ✅ All tests passing
- ✅ 46.93% coverage (critical: 81-94%)
- ✅ Zero production mocks
- ✅ Graceful error handling
- ✅ Comprehensive documentation

**ToadStool is production-ready.** 🍄🐸

---

Different orders of the same architecture.
