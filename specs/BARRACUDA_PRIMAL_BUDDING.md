# barraCuda Primal Budding — Architecture Spec

**Date**: March 2–3, 2026 (Session 88–89)
**Status**: Phase 5 complete — zero cross-dependencies, fully untangled
**Classification**: Core architecture evolution
**Handoff**: `ecoPrimals/wateringHole/handoffs/TOADSTOOL_S88_BARRACUDA_PRIMAL_BUDDING_PROPOSAL_MAR02_2026.md`
**Scaffold**: `ecoPrimals/barraCuda/` (created via sourDough)

---

## Naming

**barraCuda** — *BARrier-free Rust Abstracted Cross-platform Unified Dimensional Algebra*

More concept than exact acronym. The barracuda stands still until it strikes —
fast, silent, instant math across any silicon. barraCuda is vendor-agnostic.
It runs on any GPU via WGSL/wgpu. One source, any backend, identical results.

All documentation, specs, and code should use "barraCuda" (camelCase)
consistently.

---

## Context

barraCuda has grown from ToadStool's GPU backend into the ecosystem's universal
math engine. 766 WGSL shaders, 2,866+ tests, 5 Springs consuming it, FHE on GPU,
lattice QCD, spectral analysis, molecular dynamics, hydrology — it is a separate
system wearing a library costume.

The Springs don't need ToadStool's orchestration runtime to multiply matrices.
They need barraCuda. The monolithic workspace couples their compile times,
hides API breakage, and prevents them from evolving other primals alongside
GPU compute.

This spec describes how ToadStool prepares for barraCuda to bud into its own
primal.

---

## Current Coupling Surface

### barracuda → toadstool-core dependency

```toml
# crates/barracuda/Cargo.toml (current)
toadstool-core = { path = "../toadstool-core" }
```

Usage to audit:

| Module | toadstool-core usage | Decoupling path |
|--------|---------------------|-----------------|
| `device/` | Capability reporting | Move to barracuda-internal |
| `npu/` | Driver registration | Feature-gate |
| `session/` | Session lifecycle | Feature-gate or remove |
| `provenance/` | Lineage tags | Standalone trait |

### toadstool runtime → barracuda dependency

The runtime uses barracuda for:
- GPU device pool management
- Compute dispatch for IPC-received workloads
- Tensor serialization for cross-node transfer
- Capability probing (what GPU ops are available)

This direction stays — ToadStool depends on barraCuda, not the reverse.

---

## Phase 0 — Decouple ✅ COMPLETE (Session 89)

### 0.1 Feature-gate toadstool-core ✅

```toml
# barraCuda/crates/barracuda/Cargo.toml
[features]
default = ["gpu"]
gpu = ["dep:wgpu", "dep:bytemuck", "dep:naga"]
toadstool = ["dep:toadstool-core"]   # optional
npu-akida = ["dep:akida-driver"]     # optional
```

Decoupling surface (only 2 files):
- `src/device/toadstool_integration.rs` → `#[cfg(feature = "toadstool")]`
- `src/npu/ml_backend.rs` + `src/npu/ops/` → `#[cfg(feature = "npu-akida")]`
- `DeviceSelection`/`HardwareWorkload` extracted to `device/mod.rs` (always available)
- `npu_bridge.rs` stubbed: `is_npu_available() → false` when `npu-akida` off

### 0.2 Standalone compilation ✅

```bash
cd barraCuda && cargo check -p barracuda          # clean
cd barraCuda && cargo clippy -p barracuda -- -D warnings  # clean
cd barraCuda && cargo test -p barracuda --lib      # 2,832 passed, 0 failed
```

40 tests gated behind `toadstool`/`npu-akida` features (from toadStool's 2,872).

### 0.3 API surface audit

Before 1.0.0, every `pub` item in barracuda must be intentional:

- [ ] `spectral/mod.rs` re-exports complete (anderson_4d, wegner_block_4d)
- [ ] All `#[repr(C)]` structs have constructors (SeasonalGpuParams::new)
- [ ] No private padding fields blocking construction
- [ ] MultiHeadEsn::from_exported_weights() exists
- [ ] tolerances module has all cross-spring constants
- [ ] BREAKING_CHANGES.md tracks API changes per session

---

## Phase 1 — Boundary Hardening

### 1.1 Extract barracuda-types

If shared types exist between toadstool-core and barracuda (capability enums,
error variants, device identifiers), extract them to a thin `barracuda-types`
crate that both can depend on without circular dependency.

### 1.2 IPC contract for barracuda-as-primal

When barraCuda becomes a primal, it needs IPC endpoints:

```
barracuda.device.list          → [{adapter, features, limits}]
barracuda.device.probe         → {f64_support, max_buffers, df64_available}
barracuda.compute.dispatch     → {shader, inputs, params} → {outputs}
barracuda.fhe.ntt              → {poly, modulus, degree} → {transformed}
barracuda.validate.gpu_stack   → {fhe_pass, qcd_pass, df64_pass}
```

These map to existing Rust APIs. The IPC layer is thin.

### 1.3 GPU validation binary

`barracuda validate-gpu` runs the FHE + lattice QCD canary suite:

| Test | Pass criteria |
|------|--------------|
| FHE NTT round-trip (12289, 65537) | Bit-perfect: INTT(NTT(p)) == p |
| FHE polynomial multiplication | Matches symbolic reference |
| SU(3) plaquette (4^4 lattice) | Within statistical error of strong-coupling |
| CG convergence | ≤ reference iteration count ± 2 |
| DF64 unitarity after N updates | ‖U·U† - I‖ < DF64 epsilon |

Any consumer can run this to verify their GPU is trustworthy for scientific
compute before running domain workloads.

---

## Phase 2 — Repo Extraction ✅ COMPLETE (Session 89)

### 2.1 ecoPrimals/barraCuda/ — live on GitHub

```
ecoPrimals/barraCuda/
├── crates/
│   ├── barracuda-core/       (primal lifecycle, device discovery, health)
│   ├── barracuda/            (full compute library: 956 .rs, 767 WGSL, 61 tests)
├── specs/
│   └── BARRACUDA_SPECIFICATION.md
├── Cargo.toml                (workspace, MSRV 1.87)
├── Cargo.lock
├── README.md
└── .gitignore
```

Sub-crate split (barracuda-ops, barracuda-esn, etc.) is future evolution
*after* Springs validate the standalone library.

### 2.2 ToadStool workspace update

```toml
# phase1/toadStool/Cargo.toml (after extraction)
[dependencies]
barracuda = { version = "1.0", features = ["gpu"] }
# or during development:
# barracuda = { path = "../../barracuda" }
```

ToadStool's own crate count drops significantly. Compile time improves because
barracuda is either pre-built or a separate compilation unit.

### 2.3 Spring migration

Springs update from:
```toml
# Current: implicit via toadstool workspace
barracuda = { path = "../../../phase1/toadStool/crates/barracuda" }
```

To:
```toml
# After: direct versioned dependency
barracuda = { version = "1.0", features = ["gpu"] }
# or during development:
# barracuda = { path = "../../barracuda" }
```

---

## Phase 3 — Multi-Primal Springs

With barraCuda decoupled, Springs can evolve other primals:

### What becomes possible

| Spring capability | Requires | Current blocker |
|------------------|----------|----------------|
| GPU physics + encrypted provenance | barracuda + beardog | beardog can't compose with toadstool workspace |
| GPU genomics + data archival | barracuda + nestgate | nestgate excluded by toadstool coupling |
| GPU ML + distributed inference | barracuda + squirrel + songbird | multiple primals can't coexist |
| GPU hydrology + edge deployment | barracuda + airspring-local | lightweight dep chain needed |

### BearDog + barraCuda composition

BearDog provides cryptographic scaffolding (Ed25519, ChaCha20-Poly1305, BLAKE3,
X.509, genetic lineage). barraCuda provides FHE GPU compute (NTT, INTT,
pointwise mod-mul). Together they enable:

1. BearDog generates FHE scheme parameters and keys
2. BearDog encrypts data using FHE scheme
3. barraCuda performs homomorphic computation on GPU
4. BearDog decrypts results

Neither depends on the other at the crate level. They compose at the primal IPC
level or as separate deps in a Spring. This is the sovereign FHE pipeline — no
CUDA, no cloud, cross-vendor.

---

## Phase 4 — Deprecation & Rewire ✅ COMPLETE (Session 89)

### 4.1 Architecture demarcation

`specs/ARCHITECTURE_DEMARCATION.md` codifies the 3-layer ownership:
- **barraCuda** — "WHAT to compute" (math, shaders, wgpu, compute fabric)
- **toadStool** — "WHERE and HOW" (multi-framework routing, orchestration, distribution)
- **songBird** — "the wire" (network, discovery, NAT traversal)

Infrastructure audit confirmed zero functional duplication between barraCuda's
17 GPU modules and toadStool's 4 runtime crates.

### 4.2 hotSpring first-consumer validation

hotSpring rewired with a single-line Cargo.toml path swap. 716/716 tests pass.
No code changes needed. API is identical.

hotSpring also found 37 test failures in barraCuda's full suite (beyond --lib):
- 36 from `sin_f64_safe` using f64 `%` operator (naga 22 rejects). Fixed.
- 1 from tokio test flavor (block_in_place needs multi_thread). Fixed.

### 4.3 toadStool workspace rewired

```toml
# crates/core/toadstool/Cargo.toml
barracuda = { path = "../../../../../barraCuda/crates/barracuda" }

# crates/cli/Cargo.toml
barracuda = { path = "../../../../barraCuda/crates/barracuda", default-features = false, optional = true }

# crates/integration-tests/Cargo.toml
barracuda = { path = "../../../../barraCuda/crates/barracuda" }
```

Embedded `crates/barracuda/` removed from workspace members, `DEPRECATED.md` added.
Full toadStool workspace builds clean against standalone barraCuda.

### 4.4 Domain model feature gates

barraCuda v0.2.1 adds `domain-models` umbrella feature with per-module flags,
enabling Springs to opt out of domain modules they don't need (faster compile).

## Phase 5 — Complete Untangle `COMPLETE`

### 5.1 toadstool-core coupling eliminated

- `device/toadstool_integration.rs` — deleted (entire file)
- `device/wgpu_device/creation.rs` — `from_selection()` method removed
- `device/wgpu_device/mod.rs` — 2 toadstool-gated tests removed
- `device/mod.rs` — module declaration and re-exports removed
- `toadstool` feature removed from Cargo.toml

### 5.2 akida-driver coupling eliminated

- `npu/ml_backend.rs` and `npu/ops/` modules removed
- `ops/npu_bridge.rs` — `with_npu_backend()`, `NPU_BACKEND` static removed; `is_npu_available()` returns false
- `ops/matmul.rs` — NPU routing branch and `matmul_npu()` removed
- `ops/softmax.rs` — NPU routing branch and `softmax_npu()` removed
- `lib.rs` — `NpuMlBackend` re-export removed
- `npu-akida` feature removed from Cargo.toml

### 5.3 Verified zero cross-dependencies

```
barraCuda → toadStool: ZERO (rg scan confirmed)
barraCuda → sourDough: sourdough-core only (via barracuda-core)
barraCuda Cargo.toml: no toadstool-core, no akida-driver
cargo check (3 configs): all pass
cargo clippy: 0 warnings
cargo test --lib: 2,835 pass, 0 fail
```

### 5.4 Showcases rewired

- `showcase/rbf-surrogate/Cargo.toml` → standalone barraCuda path
- `showcase/cross-platform/Cargo.toml` → standalone barraCuda path

---

## What ToadStool Becomes

After budding, ToadStool is:

- **Node atomic**: basic CPU computation, no GPU required
- **Orchestration**: primal lifecycle, IPC, discovery, biomeOS integration
- **Bridge**: connects to barraCuda (and other primals) for specialized compute
- **Lighter**: faster compile, smaller surface area, clearer mission

ToadStool retains its role as the compute orchestration primal in NUCLEUS. It
just no longer carries the full math engine inside its workspace. The mycelial
model from SOVEREIGN_COMPUTE_EVOLUTION.md still holds — ToadStool nodes spread
through computational substrates, discovering barraCuda instances for GPU math.

---

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Coordination overhead (two repos) | Path deps for development, SemVer for releases |
| First version numbering debate | Start at 1.0.0 — 766 shaders, 5 consumers, production-grade |
| Integration test gaps | Bridge crate in toadStool with cross-crate tests |
| Spring migration disruption | Phase over 2-3 sessions; path deps during transition |
| Loss of atomic commits | Rare in practice; most changes are shader-only or runtime-only |

---

## Success Criteria

- [x] `cargo check -p barracuda --features gpu` passes (S89)
- [x] `cargo clippy -p barracuda -- -D warnings` passes (S89)
- [x] `cargo test -p barracuda --lib` passes — 2,832 tests, 0 failures (S89)
- [x] No `toadstool-core` usage without `#[cfg(feature = "toadstool")]` (S89)
- [x] No `akida-driver` usage without `#[cfg(feature = "npu-akida")]` (S89)
- [x] barracuda-core wired to barracuda compute library (S89)
- [x] toadStool unchanged — all original tests still pass (S89)
- [x] hotSpring validates as first consumer — 716/716 tests pass (S89)
- [x] `barracuda validate-gpu` binary exists (S89)
- [x] SemVer CHANGELOG tracks every API change (S89)
- [x] Architecture demarcation spec codified (S89)
- [x] Domain models feature-gated for future Spring absorption (S89)
- [x] toadStool workspace deprecated embedded barracuda, rewired to standalone (S89)
- [x] Full toadStool workspace builds clean against standalone barraCuda (S89)
- [x] **Zero cross-dependencies**: toadstool_integration.rs deleted, akida-driver coupling removed (S89)
- [x] **2,835 lib tests pass** after untangle (S89)
- [x] **Showcases rewired** to standalone barraCuda (S89)
- [x] **Handoff published**: `wateringHole/handoffs/BARRACUDA_S89_UNTANGLE_AND_HANDOFF_MAR03_2026.md` (S89)
- [ ] All 5 Springs can build against barracuda without toadstool workspace
- [ ] `validate-gpu` passes on Intel, AMD, NVIDIA (NVK sin_f64_safe fixed)
- [ ] hotSpring QCD runs with barraCuda math + toadStool hardware dispatch

---

## References

- `SOVEREIGN_COMPUTE_EVOLUTION.md` — barraCuda as "unified math language"
- `PRIMAL_CAPABILITY_SYSTEM.md` — capability-based discovery (implemented)
- `wateringHole/UNIVERSAL_IPC_EVOLUTION_HANDOFF.md` — JSON-RPC 2.0 primal protocol
- `wateringHole/handoffs/TOADSTOOL_S88_BARRACUDA_PRIMAL_BUDDING_PROPOSAL_MAR02_2026.md`
- S87: FHE shader fixes proving GPU validation canary viability
- S86: ComputeDispatch evolution (144 ops migrated)
