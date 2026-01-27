# 🍄 ToadStool - Universal Compute Runtime

**Version**: 0.1.0  
**Status**: Development (Grade: B 80%)  
**Last Audit**: January 27, 2026

---

## 🚀 **Quick Start**

```bash
# Build
cargo build --release

# Run daemon
./target/release/toadstool daemon

# Run compute workload
./target/release/toadstool run --wasm myworkload.wasm
```

---

## 📊 **Current Status**

**Grade**: **B (80%)** - Good architecture, production blockers identified  
**Production Ready**: ❌ NO (2-3 months needed)  
**Build Status**: ✅ Passing (44s)  
**Test Coverage**: 42.63% (need 75% for production)

### What's Excellent ✅
- Architecture (A+ 95%)
- Code Organization (A+ 98%)
- UniBin/ecoBin Compliance (A 100%)
- Pure Rust (A+ 100%)
- Zero files > 1000 lines

### What Needs Work ⚠️
- Test coverage (42.63% → 75% needed)
- GPU WebGPU backend (Drop causes segfaults)
- Security features incomplete

**Timeline to Production**: 2-3 months

---

## 📚 **Documentation Index**

### Essential Reading
1. **[This File](START_HERE.md)** - You are here
2. **[FINAL_SESSION_SUMMARY_JAN_27_2026.md](FINAL_SESSION_SUMMARY_JAN_27_2026.md)** - Complete audit results
3. **[STATUS_UPDATED_JAN_27_2026.md](STATUS_UPDATED_JAN_27_2026.md)** - Detailed metrics
4. **[NEXT_STEPS_JAN_27_2026.md](NEXT_STEPS_JAN_27_2026.md)** - Roadmap

### Technical Details
- **[GPU_SAFETY_FIX_COMPLETE_JAN_27_2026.md](GPU_SAFETY_FIX_COMPLETE_JAN_27_2026.md)** - GPU safety fixes
- **[TESTING.md](TESTING.md)** - Test infrastructure
- **[PEDANTIC_MODE.md](PEDANTIC_MODE.md)** - Code quality standards

### Archived Documentation
- **[docs/archive/jan_27_2026_audit_session/](docs/archive/jan_27_2026_audit_session/)** - Complete audit session docs (25 files)

---

## 🏗️ **Architecture**

ToadStool is a universal compute runtime that provides:

- **WASM Runtime**: Pure Rust `wasmi` interpreter
- **GPU Compute**: WebGPU, Vulkan, OpenCL support
- **Display**: DRM/KMS direct rendering
- **IPC**: JSON-RPC 2.0 + tarpc over Unix sockets
- **Discovery**: Songbird-based capability discovery
- **Zero C Dependencies**: 100% Pure Rust application code

### UniBin Architecture ✅
- Single binary: `toadstool`
- Multiple modes: `daemon`, `run`, `up`, `down`, etc.
- Backward compatible with legacy names

### ecoBin Compliance ✅
- Cross-compiles to `x86_64-unknown-linux-musl`
- Static linking, no dynamic dependencies
- Universal portability

---

## 🧪 **Testing**

```bash
# Run all tests
cargo test --workspace

# Run with coverage
cargo llvm-cov --workspace --exclude toadstool-runtime-gpu --html

# Run E2E tests
cargo test --test e2e_tests

# Run chaos tests
cargo test --test chaos_engineering_scenarios
```

**Current Coverage**: 42.63% (library code)  
**Target**: 75% minimum for production

---

## 🛠️ **Development**

### Build Requirements
- Rust 1.92.0+
- Linux (for DRM/KMS features)
- Optional: GPU drivers (Vulkan, OpenCL, or WebGPU)

### Code Standards
- **Linting**: `cargo clippy --workspace -- -W clippy::pedantic`
- **Formatting**: `cargo fmt --all`
- **Max file size**: 1000 lines (currently 0 violations)
- **Documentation**: Doc comments on public APIs

### Deep Debt Principles
- ✅ **Fast AND Safe**: No compromises
- ✅ **Real Implementations**: Zero production mocks
- ✅ **Modern Idiomatic Rust**: async/await, tokio
- ✅ **Capability-Based**: Runtime discovery, no hardcoding
- ✅ **Self-Knowledge Only**: Discovers other primals at runtime

---

## 📋 **Current Priorities**

### P0 - Critical (Blocks Production)
1. **Fix WebGPU Backend** (1-2 weeks)
   - Drop implementation causes segfaults
   - CPU backend works perfectly
   - See: GPU_SAFETY_FIX_COMPLETE_JAN_27_2026.md

2. **Expand Test Coverage** (5-8 weeks)
   - Current: 42.63%
   - Target: 75% for production
   - Add E2E workflows, error paths, fault injection

### P1 - High (Should Do Soon)
3. **Complete Pedantic Linting** (2-3 days)
4. **Security Feature Completion** (1-2 weeks)

### P2 - Medium (Can Wait)
5. Zero-copy optimizations
6. Multi-platform validation

---

## 🎯 **Roadmap**

### Month 1: Critical Fixes → B+ (85%)
- Fix WebGPU backend Drop
- Reach 60% test coverage
- Fix remaining pedantic linting

### Month 2: Production Ready → A- (92%)
- Comprehensive E2E tests
- Reach 75% test coverage
- **← PRODUCTION READY**

### Month 3: Polish → A (95%)
- Complete security features
- Reach 80% test coverage
- Multi-platform validation

---

## 🤝 **Contributing**

ToadStool is part of the ecoPrimals ecosystem. See:

- **[ecoPrimals/wateringHole/](../wateringHole/)** - Ecosystem standards
  - UNIBIN_ARCHITECTURE_STANDARD.md
  - ECOBIN_ARCHITECTURE_STANDARD.md
  - SEMANTIC_METHOD_NAMING_STANDARD.md
  - PRIMAL_IPC_PROTOCOL.md

---

## 📞 **Support**

- **Issues**: File in repository
- **Questions**: See documentation in `docs/`
- **Standards**: Check `../wateringHole/` for ecosystem guidelines

---

## 📄 **License**

See LICENSE file for details.

---

**Last Updated**: January 27, 2026  
**Next Audit**: TBD (after Month 2 milestone)

**Truth over celebration. Reality over claims. Production over promises.**

🍄🦀✨
