# 📚 ToadStool Documentation Index

**Last Updated**: January 27, 2026  
**Total Documentation**: 600+ files

---

## 🚀 **START HERE**

1. **[START_HERE.md](START_HERE.md)** ← **READ THIS FIRST**
   - Quick start guide
   - Current status
   - Architecture overview
   - Development guide

2. **[README.md](README.md)** - Project overview with honest metrics

3. **[STATUS.md](STATUS.md)** - Detailed current status (updated Jan 27, 2026)

---

## 📊 **January 27, 2026 Audit Session**

### Essential Documents (At Root)
- **[FINAL_SESSION_SUMMARY_JAN_27_2026.md](FINAL_SESSION_SUMMARY_JAN_27_2026.md)** - Complete 9hr audit results
- **[STATUS_UPDATED_JAN_27_2026.md](STATUS_UPDATED_JAN_27_2026.md)** - Honest metrics (replaces old STATUS.md)
- **[GPU_SAFETY_FIX_COMPLETE_JAN_27_2026.md](GPU_SAFETY_FIX_COMPLETE_JAN_27_2026.md)** - GPU safety Phase 1
- **[GPU_SAFETY_FIX_PLAN_JAN_27_2026.md](GPU_SAFETY_FIX_PLAN_JAN_27_2026.md)** - GPU root cause analysis
- **[NEXT_STEPS_JAN_27_2026.md](NEXT_STEPS_JAN_27_2026.md)** - Roadmap to production

### Archived Audit Docs
- **[docs/archive/jan_27_2026_audit_session/](docs/archive/jan_27_2026_audit_session/)** - 22 detailed audit documents
- **[docs/archive/jan_27_2026_audit_session/INDEX.md](docs/archive/jan_27_2026_audit_session/INDEX.md)** - Archive index

---

## 🏗️ **Architecture & Design**

### Core Documentation
- **[DOCUMENTATION.md](DOCUMENTATION.md)** - Documentation overview
- **[UNIVERSAL_COMPUTE_ROADMAP.md](UNIVERSAL_COMPUTE_ROADMAP.md)** - Vision and roadmap
- **[TOADSTOOL_PORTABLE_COMPUTE_PLAN.md](TOADSTOOL_PORTABLE_COMPUTE_PLAN.md)** - Portable compute strategy
- **[UNIVERSAL_IPC_IMPLEMENTATION_PLAN_REVISED.md](UNIVERSAL_IPC_IMPLEMENTATION_PLAN_REVISED.md)** - IPC architecture

### Integration
- **[PRIMAL_INTEGRATION_GUIDE.md](PRIMAL_INTEGRATION_GUIDE.md)** - How to integrate with other primals
- **[PETALTONGUE_DISPLAY_BACKEND_RESPONSE.md](PETALTONGUE_DISPLAY_BACKEND_RESPONSE.md)** - Display backend details

---

## 🧪 **Testing**

- **[TESTING.md](TESTING.md)** - Test infrastructure overview
- **tests/** - Test suites
  - `e2e/` - End-to-end tests
  - `integration/` - Integration tests
  - `chaos/` - Chaos engineering tests
  - `security/` - Security tests
  - `stress/` - Stress tests

**Current Coverage**: 42.63% (measured)  
**Target**: 75% for production

---

## 🛠️ **Development**

### Code Quality
- **[PEDANTIC_MODE.md](PEDANTIC_MODE.md)** - Linting and code quality standards
- **[.clippy.toml](.clippy.toml)** - Clippy configuration
- **[CHANGELOG.md](CHANGELOG.md)** - Change history

### Quick Starts
- **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** - Command reference
- **[QUICK_START_ENCRYPTION.md](QUICK_START_ENCRYPTION.md)** - Encryption guide
- **[QUICK_START_GPU.md](QUICK_START_GPU.md)** - GPU compute guide

---

## 📦 **Module Documentation**

### Core Modules (`crates/core/`)
- **toadstool** - Core runtime
- **common** - Shared utilities
- **config** - Configuration management

### Runtime Modules (`crates/runtime/`)
- **wasm** - WASM runtime (wasmi)
- **gpu** - GPU compute (WebGPU, Vulkan, OpenCL)
- **display** - Display/rendering (DRM/KMS)

### Integration (`crates/integration/`)
- **protocols** - Inter-primal protocols
- **beardog** - BearDog integration
- **songbird** - Service discovery

### Management (`crates/management/`)
- **orchestration** - Workload orchestration
- **resources** - Resource management
- **lifecycle** - Lifecycle management

### Security (`crates/security/`)
- **monitoring** - Security monitoring
- **access_control** - Access control
- **policy** - Security policies

### CLI (`crates/cli/`)
- **toadstool** - Command-line interface

---

## 📖 **Detailed Documentation** (`docs/`)

### Architecture (`docs/architecture/`)
- 35+ files covering system design

### Runtime (`docs/runtime/`)
- WASM, GPU, Display subsystems

### Integration (`docs/integration/`)
- Inter-primal communication
- Service discovery

### Testing (`docs/testing/`)
- Test strategies
- Coverage reports

### Development (`docs/development/`)
- Developer guides
- Contribution guidelines

### Archive (`docs/archive/`)
- **jan_27_2026_audit_session/** - Complete audit documentation

---

## 🎯 **Specifications** (`specs/`)

18 specification documents covering:
- Core architecture
- Runtime specifications
- Protocol definitions
- Security requirements
- Performance targets

---

## 📝 **Examples** (`examples/`)

38 example programs demonstrating:
- Basic usage
- GPU compute
- Service discovery
- Distributed orchestration
- Universal compute patterns

---

## 🧩 **Ecosystem Standards** (`../wateringHole/`)

ToadStool implements standards from the ecoPrimals wateringHole:
- **UNIBIN_ARCHITECTURE_STANDARD.md** - Single binary architecture
- **ECOBIN_ARCHITECTURE_STANDARD.md** - Pure Rust cross-compilation
- **SEMANTIC_METHOD_NAMING_STANDARD.md** - Method naming conventions
- **PRIMAL_IPC_PROTOCOL.md** - Inter-primal communication

---

## 🔍 **Finding Documentation**

### By Topic
- **Getting Started**: START_HERE.md, README.md, QUICK_REFERENCE.md
- **Architecture**: DOCUMENTATION.md, docs/architecture/
- **Testing**: TESTING.md, docs/testing/
- **Development**: PEDANTIC_MODE.md, docs/development/
- **Integration**: PRIMAL_INTEGRATION_GUIDE.md, docs/integration/
- **Audit Results**: FINAL_SESSION_SUMMARY_JAN_27_2026.md

### By Status
- **Current**: All files at root level
- **Archived**: docs/archive/jan_27_2026_audit_session/
- **Historical**: CHANGELOG.md

---

## 📞 **Support**

- **Issues**: File in repository issue tracker
- **Questions**: Check docs/ or ask in discussions
- **Standards**: See ../wateringHole/ for ecosystem guidelines

---

## 📊 **Documentation Statistics**

- **Total Files**: 600+ documentation files
- **Root Docs**: 18 essential documents
- **Module Docs**: 574 markdown files in docs/
- **Specs**: 18 specification documents
- **Examples**: 38 example programs
- **Audit Session**: 25 files (5 at root, 22 archived)

---

**Navigation Tips**:
1. Start with START_HERE.md
2. Check STATUS.md for current metrics
3. Read FINAL_SESSION_SUMMARY_JAN_27_2026.md for audit results
4. Explore docs/ for detailed technical documentation
5. See archive/ for historical/detailed session documents

**Last Updated**: January 27, 2026  
**Next Update**: After Month 1 milestone (WebGPU fix + 60% coverage)

🍄🦀✨
