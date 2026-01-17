# Universal Compute Orchestrator - Development Roadmap

**Version**: 1.0.0  
**Status**: IN PROGRESS  
**Started**: January 17, 2026  
**Target Completion**: February 14, 2026 (4 weeks)

---

## 📊 **CURRENT STATUS**

**ToadStool Evolution**: 95% Pure Rust → 100% Pure Rust + Universal Compute

**Completed**:
- ✅ UniBin architecture (100%)
- ✅ HTTP dependencies removed (reqwest, ring, openssl)
- ✅ Compression C deps removed (lz4-sys, zstd-sys from secure_enclave)
- ✅ Architectural inversion designed

**In Progress**:
- ⏳ sys-info → sysinfo migration (Phase 1)

**Remaining**:
- ⏳ wasmi WASM runtime (Phase 1)
- ⏳ C/C++ runtime (Phase 2)
- ⏳ WASM-JIT runtime (Phase 3)
- ⏳ Ecosystem integration (Phase 4)

---

## 🎯 **VISION & GOALS**

### **Mission**

Transform ToadStool into a **Universal Compute Orchestrator** - execute any workload, any runtime, anywhere, safely.

### **Core Goals**

1. **100% Pure Rust Core**: Zero C dependencies in ToadStool binary
2. **TRUE UniBin**: Trivial cross-compilation (`cargo build --target <any>`)
3. **Runtime Flexibility**: Support short + long WASM, C, Python, Native, Container
4. **Security**: All external runtimes execute in sandboxes
5. **Ecosystem Service**: Universal compute platform for all primals

### **Philosophy**

> *"Pragmatic is for lesser projects. ToadStool aims for world-class quality."*

**Architectural Principle**: Inversion of Control
- **Old**: ToadStool depends on C (embedded)
- **New**: C depends on ToadStool (orchestrated)

---

## 📋 **PHASE TRACKING**

### **Phase 1: Pure Rust Core** 🔴 IN PROGRESS

**Objective**: Remove all C dependencies from ToadStool binary

**Timeline**: Week 1 (Jan 17-24, 2026)  
**Progress**: 40% complete

| Task | Effort | Status | Owner | Due Date |
|------|--------|--------|-------|----------|
| sys-info → sysinfo | 1-2 hours | ⏳ NEXT | Core Team | Jan 17 |
| wasmi research | 1 day | ⏳ PENDING | Runtime Team | Jan 18 |
| wasmi implementation | 3-4 days | ⏳ PENDING | Runtime Team | Jan 22 |
| Testing & validation | 1-2 days | ⏳ PENDING | QA Team | Jan 24 |
| Documentation | 1 day | ⏳ PENDING | Docs Team | Jan 24 |

**Blockers**: None

**Success Criteria**:
- [ ] `cargo tree | grep -E "\-sys "` shows only thin wrappers
- [ ] ARM cross-compilation works without C toolchain
- [ ] All existing tests pass with wasmi
- [ ] Performance acceptable for short WASM workloads

**Deliverables**:
- [ ] ToadStool binary: 100% Pure Rust
- [ ] wasmi runtime: Functional
- [ ] Migration guide: sys-info → sysinfo
- [ ] Documentation: wasmi integration

---

### **Phase 2: C/C++ Runtime** 🟡 PLANNED

**Objective**: Enable C/C++ code execution as orchestrated runtime

**Timeline**: Week 2 (Jan 25-31, 2026)  
**Progress**: 0% complete

| Task | Effort | Status | Owner | Due Date |
|------|--------|--------|-------|----------|
| C compiler detection | 1 day | ⏳ PENDING | Runtime Team | Jan 25 |
| Sandbox framework | 2 days | ⏳ PENDING | Security Team | Jan 27 |
| C runtime API | 1 day | ⏳ PENDING | Runtime Team | Jan 28 |
| Security testing | 2 days | ⏳ PENDING | Security Team | Jan 30 |
| Documentation | 1 day | ⏳ PENDING | Docs Team | Jan 31 |

**Dependencies**: Phase 1 complete

**Success Criteria**:
- [ ] Can compile C code with gcc/clang
- [ ] Can execute compiled binaries in sandbox
- [ ] Sandbox prevents escape attempts
- [ ] Resource limits enforced
- [ ] Audit trail complete

**Deliverables**:
- [ ] C Runtime module (`crates/runtime/c/`)
- [ ] Security test suite
- [ ] API documentation
- [ ] Example C workloads

---

### **Phase 3: WASM-JIT Runtime** 🟢 PLANNED

**Objective**: Enable long-running WASM via wasmtime subprocess

**Timeline**: Week 3 (Feb 1-7, 2026)  
**Progress**: 0% complete

| Task | Effort | Status | Owner | Due Date |
|------|--------|--------|-------|----------|
| wasmtime subprocess executor | 1-2 days | ⏳ PENDING | Runtime Team | Feb 2 |
| Runtime selection logic | 1 day | ⏳ PENDING | Runtime Team | Feb 3 |
| Performance benchmarking | 1-2 days | ⏳ PENDING | Perf Team | Feb 5 |
| Auto-selection heuristics | 1 day | ⏳ PENDING | Runtime Team | Feb 6 |
| Documentation | 1 day | ⏳ PENDING | Docs Team | Feb 7 |

**Dependencies**: Phase 1 complete

**Success Criteria**:
- [ ] WASM-JIT runtime functional via subprocess
- [ ] Performance > 80% of native for long workloads
- [ ] Auto-selection chooses optimal runtime
- [ ] Benchmark results documented

**Deliverables**:
- [ ] WASM-JIT runtime module
- [ ] Runtime selection engine
- [ ] Performance benchmarks (wasmi vs wasmtime)
- [ ] Documentation: Short vs Long WASM

---

### **Phase 4: Ecosystem Integration** 🟢 PLANNED

**Objective**: Integrate with other primals and document patterns

**Timeline**: Week 4 (Feb 8-14, 2026)  
**Progress**: 0% complete

| Task | Effort | Status | Owner | Due Date |
|------|--------|--------|-------|----------|
| Update RPC protocol | 1 day | ⏳ PENDING | Protocol Team | Feb 8 |
| Update primal clients | 2 days | ⏳ PENDING | Integration Team | Feb 10 |
| E2E testing with ecosystem | 2 days | ⏳ PENDING | QA Team | Feb 12 |
| Documentation & examples | 2 days | ⏳ PENDING | Docs Team | Feb 14 |

**Dependencies**: Phases 1-3 complete

**Success Criteria**:
- [ ] Squirrel can request C execution
- [ ] Songbird can request WASM-JIT execution
- [ ] BearDog can request any runtime
- [ ] Performance acceptable across ecosystem
- [ ] Complete documentation

**Deliverables**:
- [ ] Updated RPC protocol spec
- [ ] Primal integration examples
- [ ] Ecosystem documentation
- [ ] Performance report

---

## 📊 **METRICS & KPIs**

### **Purity Metrics**

| Metric | Current | Phase 1 Target | Final Target |
|--------|---------|----------------|--------------|
| Pure Rust % | 95% | 100% | 100% |
| C Dependencies | 1 (sys-info) | 0 | 0 |
| Thin Wrappers | 3 acceptable | 3 acceptable | 3 acceptable |
| Cross-Compile | Blocked | Works | Works |

### **Runtime Portfolio**

| Runtime | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|---------|---------|---------|---------|---------|
| Native (Rust) | ✅ Exists | ✅ | ✅ | ✅ |
| WASM (wasmi) | ⏳ Implement | ✅ | ✅ | ✅ |
| Python | ✅ Exists | ✅ | ✅ | ✅ |
| C/C++ | ❌ | ⏳ Implement | ✅ | ✅ |
| WASM-JIT | ❌ | ❌ | ⏳ Implement | ✅ |
| Container | ❌ Future | ❌ | ❌ | ⏳ Planned |

### **Performance Targets**

| Workload Type | Metric | Current | Target |
|---------------|--------|---------|--------|
| Short WASM | Startup | N/A | < 10ms |
| Short WASM | Execution | N/A | 20-50% native |
| Long WASM | Startup | N/A | < 1s |
| Long WASM | Execution | N/A | > 80% native |
| C Execution | Compile | N/A | < 5s |
| C Execution | Execute | N/A | 100% native |

---

## 🚧 **RISKS & MITIGATION**

### **Risk 1: wasmi Performance**

**Risk**: wasmi may be too slow for some workloads  
**Impact**: Medium  
**Probability**: Low  
**Mitigation**:
- Benchmark early (Phase 1)
- Provide WASM-JIT fallback (Phase 3)
- Auto-selection heuristics (Phase 3)

### **Risk 2: C Sandbox Security**

**Risk**: Sandbox escape vulnerabilities  
**Impact**: High  
**Probability**: Low  
**Mitigation**:
- Use proven sandbox tech (seccomp, namespaces)
- Security audit (Phase 2)
- Extensive security testing
- Bug bounty program

### **Risk 3: Ecosystem Adoption**

**Risk**: Other primals don't adopt new runtime model  
**Impact**: Medium  
**Probability**: Low  
**Mitigation**:
- Clear documentation (Phase 4)
- Example integrations (Phase 4)
- Performance benefits demonstration
- Direct support for integration

### **Risk 4: Schedule Slip**

**Risk**: Implementation takes longer than planned  
**Impact**: Low  
**Probability**: Medium  
**Mitigation**:
- Aggressive Phase 1 execution
- Parallel work where possible
- Cut non-critical features if needed
- Phase 4 can slip without blocking 100% Pure Rust

---

## 📅 **MILESTONE SCHEDULE**

### **Week 1: Pure Rust Foundation**

**Milestone**: ToadStool 100% Pure Rust  
**Date**: January 24, 2026

**Deliverables**:
- ✅ sys-info migrated to sysinfo
- ✅ wasmi runtime functional
- ✅ All tests passing
- ✅ ARM cross-compilation works

**Celebration**: 🎉 100% Pure Rust achieved!

---

### **Week 2: C Runtime**

**Milestone**: C code execution capability  
**Date**: January 31, 2026

**Deliverables**:
- ✅ C compiler integration
- ✅ Sandbox execution
- ✅ Security validated
- ✅ Documentation complete

**Celebration**: 🎉 C as runtime not dependency!

---

### **Week 3: WASM Dual Mode**

**Milestone**: Short + Long WASM support  
**Date**: February 7, 2026

**Deliverables**:
- ✅ WASM-JIT runtime via subprocess
- ✅ Auto-selection working
- ✅ Performance benchmarked
- ✅ Documentation complete

**Celebration**: 🎉 Best of both worlds!

---

### **Week 4: Ecosystem Ready**

**Milestone**: Universal Compute Platform  
**Date**: February 14, 2026

**Deliverables**:
- ✅ RPC protocol updated
- ✅ Primal integrations working
- ✅ E2E tests passing
- ✅ Complete documentation

**Celebration**: 🎉 **UNIVERSAL COMPUTE ORCHESTRATOR COMPLETE!**

---

## 📚 **DOCUMENTATION TRACKING**

### **Specifications**

- [x] `specs/UNIVERSAL_COMPUTE_ORCHESTRATOR.md` - Complete spec
- [ ] `specs/RUNTIME_SELECTION_ALGORITHM.md` - Auto-selection logic
- [ ] `specs/SECURITY_MODEL.md` - Sandbox architecture
- [ ] `specs/PERFORMANCE_TARGETS.md` - Benchmarks and SLAs

### **Design Documents**

- [x] `ARCHITECTURAL_INVERSION_C_AS_RUNTIME_JAN_17_2026.md` - Core insight
- [x] `TRUE_100_PERCENT_PURE_RUST_EVOLUTION_PLAN_JAN_17_2026.md` - Pure Rust plan
- [ ] `WASM_DUAL_MODE_ARCHITECTURE.md` - Short vs Long WASM
- [ ] `C_RUNTIME_SECURITY_DESIGN.md` - Sandbox implementation

### **Migration Guides**

- [ ] `MIGRATION_SYS_INFO_TO_SYSINFO.md` - Phase 1
- [ ] `MIGRATION_WASMTIME_TO_WASMI.md` - Phase 1
- [ ] `USING_C_RUNTIME.md` - Phase 2
- [ ] `RUNTIME_SELECTION_GUIDE.md` - Phase 3

### **API Documentation**

- [ ] `api/RUNTIME_MANAGER.md` - Runtime management API
- [ ] `api/WORKLOAD_SUBMISSION.md` - Workload submission
- [ ] `api/RUNTIME_SELECTION.md` - Selection API
- [ ] `api/SECURITY_SANDBOX.md` - Sandbox configuration

---

## 🔗 **RELATED WORK**

### **Previous Milestones**

- ✅ UniBin 100% (Jan 16, 2026) - [UNIBIN_100_COMPLETE_JAN_16_2026.md](UNIBIN_100_COMPLETE_JAN_16_2026.md)
- ✅ WASM Pure Rust (Jan 16, 2026) - [PURE_RUST_WASM_EVOLUTION_JAN_16_2026.md](PURE_RUST_WASM_EVOLUTION_JAN_16_2026.md)
- ✅ 95% Pure Rust (Jan 17, 2026) - [TRUE_UNIBIN_95_PERCENT_COMPLETE_JAN_17_2026.md](TRUE_UNIBIN_95_PERCENT_COMPLETE_JAN_17_2026.md)

### **Dependencies**

- `wasmi` v1.0+ - Pure Rust WASM interpreter
- `sysinfo` v0.37+ - Pure Rust system info
- `notify` v7.0+ - Pure Rust file watching (future)
- `etcetera` v0.8+ - Pure Rust directories (future)

### **Ecosystem Coordination**

- **NestGate**: Already 100% Pure Rust (reference)
- **Squirrel**: Will use C runtime for legacy AI models
- **Songbird**: Will use WASM-JIT for long-running workloads
- **BearDog**: Will use all runtimes as needed

---

## 📞 **COMMUNICATION**

### **Status Updates**

**Frequency**: Daily during Phase 1, Weekly for Phases 2-4  
**Channel**: `#toadstool-dev` on ecosystem chat  
**Format**: Progress, blockers, next steps

### **Stakeholders**

- **Architecture Board**: Major decisions, milestone approvals
- **Security Team**: Sandbox design, security testing
- **Integration Team**: Ecosystem coordination
- **Documentation Team**: All documentation

### **Review Meetings**

- **Phase Reviews**: End of each phase
- **Architecture Review**: Phase 1 complete (Jan 24)
- **Security Review**: Phase 2 complete (Jan 31)
- **Performance Review**: Phase 3 complete (Feb 7)
- **Final Review**: Phase 4 complete (Feb 14)

---

## 🎯 **SUCCESS DEFINITION**

**ToadStool achieves Universal Compute Orchestrator status when**:

1. ✅ **100% Pure Rust**: Zero C dependencies in binary
2. ✅ **TRUE UniBin**: Cross-compiles to any architecture trivially
3. ✅ **Runtime Portfolio**: Native, WASM (short), WASM-JIT (long), Python, C
4. ✅ **Security**: All external runtimes sandboxed
5. ✅ **Performance**: Meets or exceeds targets
6. ✅ **Ecosystem**: Other primals successfully integrated
7. ✅ **Documentation**: Complete and comprehensive

**Final Verification**:
```bash
# Pure Rust verification
cargo tree | grep -E "\-sys " | grep -v "linux-raw-sys"
# Should show NOTHING!

# Cross-compilation verification
cargo build --target aarch64-unknown-linux-gnu  # ✅
cargo build --target riscv64gc-unknown-linux-gnu  # ✅

# Runtime verification
toadstool runtimes list
# Should show: native, wasm, wasm-jit, python, c

# Ecosystem verification
squirrel request-compute --runtime c legacy_model.c  # ✅
songbird request-compute --runtime wasm-jit long_compute.wasm  # ✅
```

**When all pass**: 🎉 **UNIVERSAL COMPUTE ORCHESTRATOR COMPLETE!** 🎉

---

**Created**: January 17, 2026  
**Last Updated**: January 17, 2026  
**Next Update**: Daily during Phase 1  
**Status**: 🔴 IN PROGRESS - Phase 1

🦀🧬✨ **Universal Compute Orchestrator - Building World-Class!** ✨🧬🦀
