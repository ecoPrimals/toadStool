# 📚 ToadStool Documentation Index

**Last Updated**: January 29, 2026  
**Total Documentation**: 45,000+ lines

---

## 🚀 **START HERE**

### **Essential Reading** (Read in Order)

1. **[START_HERE.md](START_HERE.md)** ⭐ **READ THIS FIRST**
   - 5-minute quick start
   - Build instructions
   - Basic commands
   - Development setup

2. **[README.md](README.md)** - Project overview
   - What is ToadStool
   - Architecture overview
   - Current achievements
   - Use cases

3. **[STATUS.md](STATUS.md)** - Current status
   - Project metrics
   - Test results
   - Recent achievements
   - Roadmap

4. **[This File](ROOT_DOCS_INDEX.md)** - Documentation index
   - Navigate all documentation
   - Find specific topics

---

## 🏗️ **Core Documentation**

### **Platform Overview**

- **[README.md](README.md)** - Project overview & architecture
- **[START_HERE.md](START_HERE.md)** - Quick start guide
- **[STATUS.md](STATUS.md)** - Current status & metrics
- **[TESTING.md](TESTING.md)** - Testing strategy & coverage
- **[PRIMAL_INTEGRATION_GUIDE.md](PRIMAL_INTEGRATION_GUIDE.md)** - Integration with other primals

### **Development**

- **[PEDANTIC_MODE.md](PEDANTIC_MODE.md)** - Code quality standards
- **[DOCUMENTATION.md](DOCUMENTATION.md)** - Documentation overview
- **[CHANGELOG.md](CHANGELOG.md)** - Change history

### **Quick References**

- **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** - Command reference
- **[QUICK_START_ENCRYPTION.md](QUICK_START_ENCRYPTION.md)** - Encryption guide
- **[QUICK_START_GPU.md](QUICK_START_GPU.md)** - GPU compute guide

### **Architecture & Planning**

- **[UNIVERSAL_COMPUTE_ROADMAP.md](UNIVERSAL_COMPUTE_ROADMAP.md)** - Vision & roadmap
- **[TOADSTOOL_PORTABLE_COMPUTE_PLAN.md](TOADSTOOL_PORTABLE_COMPUTE_PLAN.md)** - Portable compute strategy
- **[UNIVERSAL_IPC_IMPLEMENTATION_PLAN_REVISED.md](UNIVERSAL_IPC_IMPLEMENTATION_PLAN_REVISED.md)** - IPC architecture

---

## 🧠 **Neuromorphic Computing** (✅ COMPLETE + VALIDATED - Jan 29, 2026)

### **Strategic Documentation**

- **[showcase/neuromorphic/PURE_RUST_AKIDA_MIGRATION_PLAN.md](showcase/neuromorphic/PURE_RUST_AKIDA_MIGRATION_PLAN.md)** ⭐
  - Complete roadmap (672 lines)
  - 4-phase plan
  - Technical architecture
  - Timeline & milestones

- **[showcase/neuromorphic/GETTING_STARTED_PURE_RUST.md](showcase/neuromorphic/GETTING_STARTED_PURE_RUST.md)** ⭐
  - Week-by-week guide (562 lines)
  - Phase-by-phase implementation
  - Code examples
  - Testing strategies

### **Validation & Analysis** (NEW!)

- **[../../CROSS_SUBSTRATE_VALIDATION_JAN29_2026.md](../../CROSS_SUBSTRATE_VALIDATION_JAN29_2026.md)** ⭐
  - Cross-substrate validation report
  - 7 compute units tested (1 CPU + 4 GPU + 2 NPU)
  - Performance comparison
  - Akida 48-202x faster than CPU!

- **[../../NEUROMORPHIC_VS_GPU_ANALYSIS_JAN29_2026.md](../../NEUROMORPHIC_VS_GPU_ANALYSIS_JAN29_2026.md)** ⭐⭐
  - Comprehensive 20-page analysis
  - GPU vs Neuromorphic comparison
  - Strengths and weaknesses
  - Use case guidance
  - Cost analysis (50-200x power savings!)
  - **Answer**: Complementary, NOT replacement!

- **[../../VALIDATION_SESSION_SUMMARY_JAN29_2026.md](../../VALIDATION_SESSION_SUMMARY_JAN29_2026.md)**
  - Complete session summary
  - All benchmarks and results
  - Deliverables overview

### **Technical Documentation**

- **[crates/neuromorphic/akida-driver/README.md](crates/neuromorphic/akida-driver/README.md)**
  - Pure Rust driver API
  - Hardware discovery (160 NPUs)
  - Capability querying
  - Device I/O operations
  - Model loading (23-26 MB/s)
  - Inference execution (76.3µs latency)
  - Examples (9 total)

- **[crates/neuromorphic/akida-models/README.md](crates/neuromorphic/akida-models/README.md)**
  - Model parser API
  - FlatBuffers parsing (62.68 MB/s)
  - Weight extraction & decoding
  - Quantization decoding (1/2/4/8-bit)
  - Inference API (14,156 inferences/sec)
  - Examples (6 total)

- **[crates/neuromorphic/akida-models/SCHEMA.md](crates/neuromorphic/akida-models/SCHEMA.md)** ⭐
  - Complete file format reference (481 lines)
  - FlatBuffers structure
  - Weight quantization details
  - Parsing strategies
  - Validation methods

- **[crates/neuromorphic/cross-substrate-validation/](crates/neuromorphic/cross-substrate-validation/)** (NEW!)
  - Cross-substrate benchmark framework
  - Basic validation benchmark
  - Comprehensive workload suite (19 workload types)
  - Analysis tools

### **Progress Reports**

- **[showcase/neuromorphic/PURE_RUST_DRIVER_OPERATIONAL_JAN29_2026.md](showcase/neuromorphic/PURE_RUST_DRIVER_OPERATIONAL_JAN29_2026.md)**
  - Phase 1 completion report
  - Driver verification
  - Test results

- **[showcase/neuromorphic/DEEP_DEBT_ELIMINATION_JAN29_2026.md](showcase/neuromorphic/DEEP_DEBT_ELIMINATION_JAN29_2026.md)**
  - Architectural principles
  - Technical debt elimination
  - Quality metrics

- **[../../PHASE4_COMPLETE_ALL_PHASES_DONE_JAN29_2026.md](../../PHASE4_COMPLETE_ALL_PHASES_DONE_JAN29_2026.md)**
  - Phase 4 (Inference) completion
  - All 4 phases complete!
  - Performance metrics
  - Production ready status

---

## 🧪 **Testing**

### **Test Documentation**

- **[TESTING.md](TESTING.md)** - Overall testing strategy
  - Test infrastructure
  - Coverage goals
  - Running tests
  - CI/CD integration

### **Test Suites** (`tests/`)

- **`e2e/`** - End-to-end tests (16 files)
- **`integration/`** - Integration tests (4 files)
- **`chaos/`** - Chaos engineering tests (14 files)
- **`security/`** - Security tests (1 file)
- **`stress/`** - Stress tests (1 file)

### **Test Status**

- ✅ Platform (lib): 1,000+ tests passing
- ✅ Neuromorphic: 23/23 tests passing (100%) ⭐
- ✅ Cross-substrate: 7 compute units validated (NEW!)
- ⏳ Integration: Expanding coverage

---

## 📖 **Detailed Documentation** (`docs/`)

### **By Category**

- **`docs/architecture/`** - System design (35+ files)
- **`docs/runtime/`** - Runtime subsystems
- **`docs/integration/`** - Inter-primal communication
- **`docs/testing/`** - Test strategies
- **`docs/development/`** - Developer guides
- **`docs/specs/`** - Technical specifications (18 files)

### **Session Documentation**

- **`docs/sessions/jan_27_29_2026/`** - Jan 27-29 session docs
  - Build fixes
  - Neuromorphic implementation
  - Test improvements

- **`docs/archive/`** - Historical documentation
  - Previous audit sessions
  - Evolution history
  - Archived reports

---

## 📦 **Module Documentation**

### **Core Modules** (`crates/core/`)

- **`toadstool/`** - Core runtime
  - `src/lib.rs` - Main entry point
  - `src/semantic_methods.rs` - Semantic naming
  
- **`common/`** - Shared utilities
- **`config/`** - Configuration management

### **Runtime Modules** (`crates/runtime/`)

- **`wasm/`** - WASM runtime (wasmi - Pure Rust)
- **`gpu/`** - GPU compute (wgpu - Vulkan/Metal/DX12)
- **`python/`** - Python runtime (PyO3)
- **`container/`** - Container runtime (Docker/Podman)
- **`display/`** - Display backend (DRM/KMS)
- **`native/`** - Native execution
- **`secure_enclave/`** - Secure compute
- **`universal/`** - Universal compute orchestration

### **Neuromorphic Modules** (`crates/neuromorphic/`) ✅ COMPLETE + VALIDATED!

- **`akida-driver/`** - Pure Rust Akida driver
  - Hardware discovery (160 NPUs)
  - Capability querying
  - Device I/O operations
  - Model loading (23-26 MB/s)
  - Inference execution (76.3µs latency)
  - 10/10 tests passing ✅

- **`akida-models/`** - Model parser & inference
  - FlatBuffers parsing (62.68 MB/s)
  - Weight extraction & decoding (1/2/4/8-bit)
  - Quantization decoding
  - Loading integration
  - Inference API (14,156 inferences/sec)
  - 13/13 tests passing ✅

- **`cross-substrate-validation/`** - Validation & benchmarking (NEW!)
  - Cross-substrate comparison framework
  - CPU vs GPU vs Neuromorphic benchmarks
  - 19 workload types tested
  - Performance analysis tools

### **Integration Modules** (`crates/integration/`)

- **`protocols/`** - Inter-primal protocols
- **`beardog/`** - BearDog integration (entropy/security)
- **`songbird/`** - Songbird integration (discovery)
- **`nestgate/`** - Nestgate integration
- **`primals/`** - Primal communication

### **Management Modules** (`crates/management/`)

- **`monitoring/`** - System monitoring
- **`performance/`** - Performance tracking
- **`resources/`** - Resource management

### **Security Modules** (`crates/security/`)

- **`monitoring/`** - Security monitoring
- **`policies/`** - Security policies
- **`sandbox/`** - Sandboxing

### **Other Modules**

- **`cli/`** - Command-line interface
- **`server/`** - Server components
- **`distributed/`** - Distributed compute
- **`testing/`** - Test utilities

---

## 📝 **Examples** (`examples/`)

### **Platform Examples**

- 38 example programs in `examples/`
- Demonstrate core features
- Show best practices
- Include comments & documentation

### **Neuromorphic Examples** ✅ COMPLETE!

**Driver Examples** (`crates/neuromorphic/akida-driver/examples/`):
- `enumerate_devices.rs` - Discover Akida hardware
- `device_info.rs` - Query device capabilities
- `basic_io.rs` - Basic read/write operations

**Model Examples** (`crates/neuromorphic/akida-models/examples/`):
- `parse_fbz.rs` - Parse Akida model files
- `benchmark_parser.rs` - Benchmark parsing performance
- `load_to_device.rs` - Load models to device
- `benchmark_loading.rs` - Benchmark loading performance
- `run_inference.rs` - Run inference on device
- `benchmark_inference.rs` - Benchmark inference performance

### **Showcases** (`showcase/`)

- **`neuromorphic/`** - Neuromorphic demos
  - `01-akida-detection/` - Hardware detection showcase
  - `02-akida-bioinformatics/` - Bioinformatics demo
  - `03-akida-llm-intent/` - LLM intent classification

- **`gpu-universal/`** - GPU compute demos
- **`inter-primal/`** - Inter-primal communication
- **`local-capabilities/`** - Capability discovery
- **`real-world/`** - Production use cases

---

## 🎯 **Specifications** (`specs/`)

18 specification documents:

- `TOADSTOOL_CORE_IMPLEMENTATION_SPEC.md`
- `UNIVERSAL_COMPUTE_PLATFORM.md`
- `UNIVERSAL_COMPUTE_ORCHESTRATOR.md`
- `PRIMAL_CAPABILITY_SYSTEM.md`
- `FRACTAL_COMPOSITION_INFRASTRUCTURE.md`
- `CRYPTO_LOCK_ARCHITECTURE.md`
- `DISPLAY_BACKEND_SPEC.md`
- `BARRACUDA_PURE_RUST_TENSOR_OPS.md`
- `COLLABORATIVE_INTELLIGENCE_RESOURCE_PLANNING.md`
- `PERFORMANCE_OPTIMIZATION_PLAN.md`
- And more...

---

## 🔍 **Finding Documentation**

### **By Topic**

| Topic | Primary Docs |
|-------|-------------|
| **Getting Started** | START_HERE.md, README.md, QUICK_REFERENCE.md |
| **Architecture** | README.md, DOCUMENTATION.md, docs/architecture/ |
| **Testing** | TESTING.md, docs/testing/ |
| **Development** | PEDANTIC_MODE.md, docs/development/ |
| **Neuromorphic** | showcase/neuromorphic/, crates/neuromorphic/ |
| **GPU Compute** | QUICK_START_GPU.md, showcase/gpu-universal/ |
| **Integration** | PRIMAL_INTEGRATION_GUIDE.md, docs/integration/ |
| **Standards** | ../wateringHole/ (ecosystem standards) |

### **By Status**

| Status | Location |
|--------|----------|
| **Current** | Root-level files (*.md) |
| **Active Development** | showcase/, crates/ |
| **Archived** | docs/archive/ |
| **Historical** | CHANGELOG.md, docs/sessions/ |

### **By Phase** (Neuromorphic)

| Phase | Status | Documentation |
|-------|--------|--------------|
| **Phase 1: Foundation** | ✅ 100% | akida-driver/README.md |
| **Phase 2: Model Format** | ✅ 100% | akida-models/README.md, SCHEMA.md |
| **Phase 3: Device Loading** | ✅ 100% | Loading integration docs |
| **Phase 4: Inference** | ✅ 100% | Inference examples, benchmarks |
| **Overall** | ✅ **PRODUCTION READY!** | See completion reports |

---

## 🌍 **Ecosystem Standards** (`../wateringHole/`)

ToadStool implements ecoPrimals wateringHole standards:

- **UNIBIN_ARCHITECTURE_STANDARD.md** - Single binary architecture ✅
- **ECOBIN_ARCHITECTURE_STANDARD.md** - Pure Rust cross-compilation ✅
- **SEMANTIC_METHOD_NAMING_STANDARD.md** - Method naming conventions ✅
- **PRIMAL_IPC_PROTOCOL.md** - Inter-primal communication ✅

---

## 📊 **Documentation Statistics**

| Category | Count | Status |
|----------|-------|--------|
| **Root docs** | 20 files | ✅ Updated Jan 29 |
| **Module docs** | 574 files | ✅ Comprehensive |
| **Specs** | 18 files | ✅ Complete |
| **Examples** | 38+ files | ✅ Working |
| **Neuromorphic docs** | 10+ files | ✅ NEW Jan 29 |
| **Test docs** | 36 test files | ✅ Documented |
| **Total lines** | 36,000+ | ✅ Extensive |

---

## 🎓 **Learning Paths**

### **For New Users**

1. Read [START_HERE.md](START_HERE.md)
2. Try quick start commands
3. Read [README.md](README.md)
4. Explore examples in `examples/`
5. Check [STATUS.md](STATUS.md) for current state

### **For Developers**

1. Read [START_HERE.md](START_HERE.md)
2. Read [PEDANTIC_MODE.md](PEDANTIC_MODE.md)
3. Read [TESTING.md](TESTING.md)
4. Explore codebase in `crates/`
5. Check module READMEs

### **For Neuromorphic Users**

1. Read [START_HERE.md](START_HERE.md) (Neuromorphic section)
2. Read [showcase/neuromorphic/GETTING_STARTED_PURE_RUST.md](showcase/neuromorphic/GETTING_STARTED_PURE_RUST.md)
3. Read [crates/neuromorphic/akida-driver/README.md](crates/neuromorphic/akida-driver/README.md)
4. Try examples in `crates/neuromorphic/*/examples/`
5. Read [crates/neuromorphic/akida-models/SCHEMA.md](crates/neuromorphic/akida-models/SCHEMA.md)

### **For Integrators**

1. Read [PRIMAL_INTEGRATION_GUIDE.md](PRIMAL_INTEGRATION_GUIDE.md)
2. Read wateringHole standards
3. Explore `crates/integration/`
4. Check `showcase/inter-primal/`
5. Review IPC protocols in `docs/integration/`

---

## 📞 **Getting Help**

### **Documentation**
- Start with [START_HERE.md](START_HERE.md)
- Check this index for specific topics
- Search `docs/` for detailed info
- Read module READMEs for code-specific docs

### **Examples**
- Platform: `examples/`
- Neuromorphic: `crates/neuromorphic/*/examples/`
- Showcases: `showcase/`

### **Community**
- File issues in repository
- Check existing documentation first
- See wateringHole for ecosystem standards

---

## ✨ **Recently Updated** (Jan 29, 2026)

### **Root Documentation**
- ✅ README.md - Updated with neuromorphic achievements
- ✅ START_HERE.md - Added neuromorphic quick start
- ✅ STATUS.md - Updated metrics and progress
- ✅ ROOT_DOCS_INDEX.md - This file, reorganized

### **Neuromorphic Documentation** (NEW)
- ✅ PURE_RUST_AKIDA_MIGRATION_PLAN.md - Strategic roadmap
- ✅ GETTING_STARTED_PURE_RUST.md - Implementation guide
- ✅ akida-driver/README.md - Driver documentation
- ✅ akida-models/README.md - Parser documentation
- ✅ akida-models/SCHEMA.md - File format reference
- ✅ Multiple progress reports and status docs

---

## 🎉 **Summary**

**Documentation Status**: Comprehensive & Up-to-Date ✅

- ✅ 45,000+ lines of documentation
- ✅ Updated for Jan 29, 2026
- ✅ **Neuromorphic docs 100% complete (all 4 phases!)** 🎉
- ✅ Clear navigation paths
- ✅ Multiple learning paths
- ✅ Comprehensive examples (47+ files)
- ✅ Production-ready

**Start Point**: [START_HERE.md](START_HERE.md) ⭐

---

**Last Updated**: January 29, 2026  
**Status**: Neuromorphic 100% complete - Production ready!

**Navigation**: Start → README → STATUS → Explore! 🍄🧠✨
