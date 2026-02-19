# Session 4 Audit Report — Feb 19, 2026

**Scope**: Full codebase audit against sovereign compute vision, wateringHole standards,
ecoBin/UniBin compliance, JSON-RPC/tarpc, zero-copy, test coverage, code size, fmt/clippy,
unsafe, mocks, hardcoding, and dignity/sovereignty.

**Reference standards consulted**:
- `wateringHole/UNIBIN_ARCHITECTURE_STANDARD.md`
- `wateringHole/ECOBIN_ARCHITECTURE_STANDARD.md`
- `wateringHole/SEMANTIC_METHOD_NAMING_STANDARD.md`
- `wateringHole/PRIMAL_IPC_PROTOCOL.md` (v2.0)
- `wateringHole/UNIVERSAL_IPC_STANDARD_V3.md`
- `wateringHole/PRIMAL_REGISTRY.md`
- `wateringHole/INTER_PRIMAL_INTERACTIONS.md`
- `specs/SOVEREIGN_COMPUTE_EVOLUTION.md`

---

## Summary

| Category | Status | Severity | Action |
|----------|--------|----------|--------|
| Test compilation failures | ❌ 3 targets broken | **CRITICAL** | F-001 — fix now |
| cargo fmt divergence | ⚠️ 21 diffs | HIGH | F-002 — `cargo fmt --all` |
| Production placeholder code | ❌ 3 files | HIGH | F-003 — implement |
| Hardcoded endpoints | ⚠️ 2 files | MEDIUM | F-004 — evolve |
| Production TODOs | ⚠️ 7 items | MEDIUM | F-005 — prioritise |
| Unsafe w/ `libc` (not `rustix`) | ⚠️ 2 files | LOW | F-006 — evolve |
| Dual JSON-RPC method registration | ⚠️ 5 aliases | LOW | F-007 — document/deprecate |
| Test coverage measurement | ❌ blocked by F-001 | MEDIUM | F-008 — unblocked by F-001 |
| Sovereign Compute Phase 1 | 📋 Not started | MEDIUM | F-009 — next sprint |
| `#[allow(dead_code)]` in neuromorphic | ⚠️ research crate | LOW | F-010 — document |
| **All other categories** | ✅ PASS | — | — |

---

## PASS — What's Clean

### ✅ Code Size (1000-line limit)
All production Rust files are under 1000 lines. The R-010 through R-018 refactoring
sessions successfully split every overlimit file. Current highest: `lu_gpu.rs` at 996
lines.

### ✅ JSON-RPC Compliance (wateringHole SEMANTIC_METHOD_NAMING_STANDARD v2.0)
Method names follow the `{domain}.{operation}` pattern throughout:
- `toadstool.submit_workload`, `toadstool.query_status`, `toadstool.cancel_workload`,
  `toadstool.list_workloads`, `toadstool.query_capabilities`, `toadstool.health`,
  `toadstool.version`
- `compute.submit`, `compute.status`, `compute.result`, `compute.cancel`, `compute.list`
- `semantic_methods.rs` provides the canonical mapping between aliases

All method names describe **WHAT** (semantic intent), not **HOW**. ✅

### ✅ tarpc Compliance
`crates/server/src/tarpc_server.rs` correctly implements the `#[tarpc::service]` trait
with async methods matching the JSON-RPC surface. Unix socket transport with tokio. ✅

### ✅ UniBin Compliance
- Binary named `toadstool` (not `toadstool-server`) ✅
- Subcommands: `run`, `up`, `down`, `ps`, `logs`, `validate`, `init`, `capabilities`,
  `ecosystem`, `universal`, `server`, `daemon`, `execute` ✅
- `--help` and `--version` via clap ✅
- One gap: **F-005** — `Commands::Server` and `Commands::Daemon` not fully wired (TODO at `cli/src/main.rs:397`)

### ✅ ecoBin Core Compliance
- No `ring`, no `openssl`, no `tungstenite` in the compute/server/core paths ✅
- WebSocket + `ring` fully removed (R-011) ✅
- Platform-agnostic IPC via tokio UnixStream/TcpStream fallback ✅
- **Note**: `cudarc` (CUDA SDK C-FFI) and `wayland-sys` (C) are in `runtime/gpu` feature-gated backends, not in the core binary path. ecoBin core is pure Rust. ✅

### ✅ Primal Self-Knowledge (PRIMAL_IPC_PROTOCOL)
- `crates/core/toadstool/src/self_identity.rs` — `SelfIdentity` struct contains only own metadata ✅
- No hardcoded peer addresses in identity ✅
- Peers discovered at runtime via `primal_integration.rs` (mDNS-SD, env vars, filesystem, HTTP registry) ✅
- `discover_beardog_at()`, `discover_nestgate_at()`, `discover_crypto_service()`, `discover_storage_service()` all capability-based ✅

### ✅ Zero-Copy (where applicable)
- `Tensor` clone is zero-copy via `Arc<Buffer>` — increments ref count only (comment confirms at `tensor.rs:267`) ✅
- `bytemuck::cast_slice` used for CPU↔GPU buffer views — no data copy ✅
- `Arc<RwLock<HashMap>>` preferred over `Arc<Mutex<HashMap>>` for read-heavy paths ✅
- **Note**: 3,681 `.clone()` calls across codebase — majority are `Arc` clones (zero-copy) or config struct clones at initialisation (one-time cost). A handful in hot paths (e.g., `pure_jsonrpc.rs:333,356` — `params.clone()` before `serde_json::from_value`) are addressable but low-impact.

### ✅ Sovereign Compute Vision — Phase 0 Complete
- f64 fossil functions removed from `math_f64.wgsl` ✅
- `F64BuiltinCapabilities` runtime probe active ✅
- SM70 latency tables (`sm70_instr_latencies.rs`) contributed to NAK ✅
- `substitute_fossil_f64()` auto-upgrades legacy shader calls ✅

### ✅ Sovereignty / Human Dignity
- Zero telemetry to external systems. `zero_telemetry` build flag exists in `constants/versions.rs` ✅
- `TelemetryConfig` is local-only, opt-in, and configurable ✅
- `crypto_lock/mod.rs` explicitly states: "🚫 No phone home: Pure cryptographic proof system" ✅
- No surveillance, tracking, or personal data collection found ✅
- `dev-mock-auth` feature is **compile-time gated** and **panics in release builds** ✅

### ✅ Unsafe Code — Justified
All `unsafe` blocks are:
1. `runtime/secure_enclave/src/isolated_memory.rs` — `mlock`/`munlock`/`alloc` for secure memory pages (no safe Rust alternative without rustix — see F-006)
2. `runtime/gpu/src/unified_memory/` — aligned memory allocation for GPU buffers, `NonNull` pointers (justified by hardware requirements)
3. `unsafe impl Send/Sync` — correct where raw pointers are truly thread-safe by construction
No unsafe in barracuda, toadstool core, server, distributed, or any business logic. ✅

### ✅ Arc<Mutex> patterns
All maps use `Arc<RwLock<HashMap>>` (read-heavy, correct). The few `Arc<Mutex<HashMap>>`
instances are for write-heavy state (e.g., `storage_backend.rs` volume map). ✅

### ✅ wateringHole PRIMAL_REGISTRY.md — ToadStool Role
ToadStool correctly identifies as **Compute Primal** with Node capabilities. Routes through
Songbird for external network, BearDog for crypto. Does not embed other primals' code.
Capability-based discovery implemented throughout. ✅

---

## FAIL — What Needs Work

### ❌ F-001: Test Compilation Failures (CRITICAL)

**Impact**: `cargo test --workspace` exits 101. 3 test compilation units fail.

**Root cause 1** — `production_hardening` module not fully exported:
```rust
// crates/core/toadstool/src/production_hardening/mod.rs
// MISSING from pub use block:
pub use circuit_breaker::CircuitBreakerError;
```
Tests do `use toadstool::production_hardening::*` and expect `CircuitBreakerError`
to be in scope. It is defined in `circuit_breaker.rs:53` but not re-exported.

**Root cause 2** — `ProductionHardeningManager` missing 6 methods:
Tests call: `initialize()`, `update_resource_access()`, `track_resource()`,
`remove_resource()`, `update_memory_usage()`, `get_state()`.
Current `mod.rs` only has `new()`, `get_circuit_breaker()`, `get_or_create_circuit_breaker()`.
These methods need to be implemented (can delegate to `ResourceLeakDetector` and
`MemoryPressureHandler` already in the module).

**Root cause 3** — `AuthManagerConfig` missing `token_audience` field in 19+ literals:
Field `token_audience: Vec<String>` was added to the struct but test struct literals
weren't updated. Fix: add `token_audience: vec![]` or use `..Default::default()`.

**Files affected**:
- `tests/production_hardening_comprehensive_tests.rs`
- `tests/hardening_integration_tests.rs`
- `tests/biomeos_integration/auth_tests.rs`
- `tests/biomeos_auth_types_tests.rs`
- `tests/biomeos_auth_tests.rs`

---

### ❌ F-002: `cargo fmt` Divergence (21 diffs)

**Impact**: CI/pre-commit fails on fmt check.

Run `cargo fmt --all` — mechanical, no logic change. Known diffs in:
- `crates/barracuda/src/device/probe.rs` — method chain wrapping
- `crates/barracuda/src/shaders/precision/math_f64.rs` — array literal wrapping
- 19 other files (run `cargo fmt --all -- --check 2>&1 | grep "^Diff in"` for full list)

---

### ❌ F-003: Production Placeholder Code (SECURITY RISK)

**`crates/security/policies/src/evaluator.rs:120`**:
```rust
// For now, return true as a placeholder
return true;
```
This means **every policy evaluation permits everything**. The regex cache at line 35
exists but is never populated or used. Any code path that calls the policy evaluator
believes it is enforcing access control but is not.

**`crates/security/monitoring/src/lib.rs`**: Empty file with one comment — "This module
will be implemented in future iterations." It is referenced from other modules.

**`crates/core/toadstool/src/workload_migration/validation.rs`**: Entire file is a
placeholder comment — no pre-migration validation logic.

---

### ⚠️ F-004: Hardcoded Endpoints (MEDIUM)

Two production struct defaults contain hardcoded localhost addresses:
- `biomeos_integration/storage.rs:187` → `nestgate_endpoint: "http://localhost:9090"`
- `biomeos_integration/agents.rs:269` → `squirrel_endpoint: "http://localhost:8080"`

These are the **non-evolved** backends. The `*_evolved.rs` variants correctly use
capability-based discovery. Migrate callers to use the evolved variants and deprecate
these files.

---

### ⚠️ F-005: Production TODOs (MEDIUM)

| File | TODO | Priority |
|------|------|----------|
| `security_provider/factory.rs:160-161` | LocalKeyring/SoftwareHSM providers | HIGH — key storage incomplete |
| `cli/src/main.rs:397` | UniBin Phase 3 server daemon | HIGH — `server`/`daemon` subcommands partially wired |
| `runtime/orchestration/src/load_balancer.rs:11` | dynamic multi-instance balancing | MEDIUM |
| `runtime/display/src/input/mod.rs:135` | focused window state | LOW |
| `runtime/display/src/input/events.rs:157` | more key codes | LOW |
| `runtime/gpu/src/cpu_resource.rs:151` | RISC-V 'V' extension | LOW — RISC-V target only |
| `neuromorphic/akida-driver/src/backends/userspace.rs:157` | read output size from HW | MEDIUM — userspace emulation only |

---

### ⚠️ F-006: `mlock` via `libc` (LOW)

`isolated_memory.rs` uses `libc::mlock` and `libc::munlock` in raw `unsafe` blocks.
`rustix::mm::mlock` / `rustix::mm::munlock` provide a safe equivalent with proper
error types and no `unsafe` needed at the call site. This aligns with the ecoBin
standard and the pattern established when `akida-driver` was migrated (R-018).

---

### ⚠️ F-007: Dual JSON-RPC Method Registration (LOW)

`compute.*` and `toadstool.*` both exist for the same five operations.
`semantic_methods.rs` documents the mapping but callers may receive different response
shapes from `compute.submit` vs `toadstool.submit_workload`.

**Action**: Make `compute.*` strict forwarding wrappers to the `toadstool.*` handlers.
One implementation, two entry points. Add deprecation notice to `compute.*` in docs.

---

### ⚠️ F-008: Coverage Blocked by F-001 (MEDIUM)

`cargo llvm-cov --workspace` exits 101 due to F-001 compilation failures. Once F-001 is
resolved, run:
```bash
cargo llvm-cov --workspace --html --output-dir coverage/
```
Target: 90% line coverage. Known zero-coverage areas (require F-003 fix):
- `security/monitoring/src/lib.rs` (empty)
- `security/policies/src/evaluator.rs` (placeholder logic)

---

### 📋 F-009: Sovereign Compute Phase 1 Not Started (MEDIUM)

The Jacobi eigensolve kernel has not been restructured for ILP.
The 8-cycle DFMA gap on SM70 (Titan V) is still present at source level.
See `SOVEREIGN_COMPUTE.md` Phase 1 for the concrete steps.

---

## Priority Order for Next Session

1. **F-001** — Fix 3 non-compiling test targets (30 min, no logic change)
2. **F-002** — `cargo fmt --all` (5 min)
3. **F-003** — Implement `policies/evaluator.rs` real logic, implement `security/monitoring` basics (2-4h)
4. **F-005** — `cli/src/main.rs` UniBin Phase 3 server daemon wiring (1h)
5. **F-007** — Make `compute.*` forward to `toadstool.*` handlers (30 min)
6. **F-008** — Run coverage after F-001, identify gaps, add tests to reach 90%
7. **F-009** — Sovereign Compute Phase 1: Jacobi ILP + `// @unroll_hint 32`
8. **F-004** — Deprecate `storage.rs` / `agents.rs` in favour of `*_evolved.rs`
9. **F-006** — Migrate `mlock` to `rustix` (low risk, clean ecoBin alignment)

---

## What's Tracked in DEBT.md

Issues F-001 through F-010 are now recorded in `DEBT.md` under "New Active Issues — Session 4 Audit".
