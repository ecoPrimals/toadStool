# BarraCUDA Primal Budding — Architecture Spec

**Date**: March 2, 2026 (Session 88)
**Status**: RFC — pending Spring feedback
**Classification**: Core architecture evolution
**Handoff**: `ecoPrimals/wateringHole/handoffs/TOADSTOOL_S88_BARRACUDA_PRIMAL_BUDDING_PROPOSAL_MAR02_2026.md`

---

## Context

BarraCUDA has grown from ToadStool's GPU backend into the ecosystem's universal
math engine. 766 WGSL shaders, 2,866+ tests, 5 Springs consuming it, FHE on GPU,
lattice QCD, spectral analysis, molecular dynamics, hydrology — it is a separate
system wearing a library costume.

The Springs don't need ToadStool's orchestration runtime to multiply matrices.
They need BarraCUDA. The monolithic workspace couples their compile times,
hides API breakage, and prevents them from evolving other primals alongside
GPU compute.

This spec describes how ToadStool prepares for BarraCUDA to bud into its own
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

This direction stays — ToadStool depends on BarraCUDA, not the reverse.

---

## Phase 0 — Decouple (This Session Onward)

### 0.1 Feature-gate toadstool-core in barracuda

```toml
# Target: crates/barracuda/Cargo.toml
[features]
default = ["gpu"]
gpu = ["wgpu", "bytemuck", "naga"]
toadstool = ["toadstool-core"]  # NEW: optional integration
```

All code that touches `toadstool-core` moves behind `#[cfg(feature = "toadstool")]`.
BarraCUDA compiles and passes all tests without this feature.

### 0.2 Standalone compilation gate

Add to CI:

```bash
cargo check -p barracuda --no-default-features --features gpu
cargo test -p barracuda --no-default-features --features gpu
```

This proves barracuda has no hard dependency on toadstool-core.

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

When BarraCUDA becomes a primal, it needs IPC endpoints:

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

## Phase 2 — Repo Extraction

### 2.1 Create ecoPrimals/barracuda/

```
ecoPrimals/barracuda/
├── crates/
│   ├── barracuda-core/       (tensor, device, shader compilation)
│   ├── barracuda-ops/        (all ops, shaders, FHE, linalg)
│   └── barracuda-esn/        (ESN, nautilus, reservoir computing)
├── specs/                    (moved from toadStool/specs/BARRACUDA_*)
├── tests/                    (integration, chaos, fault, cross-vendor)
├── CHANGELOG.md
├── BREAKING_CHANGES.md
├── Cargo.toml
└── README.md
```

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

With BarraCUDA decoupled, Springs can evolve other primals:

### What becomes possible

| Spring capability | Requires | Current blocker |
|------------------|----------|----------------|
| GPU physics + encrypted provenance | barracuda + beardog | beardog can't compose with toadstool workspace |
| GPU genomics + data archival | barracuda + nestgate | nestgate excluded by toadstool coupling |
| GPU ML + distributed inference | barracuda + squirrel + songbird | multiple primals can't coexist |
| GPU hydrology + edge deployment | barracuda + airspring-local | lightweight dep chain needed |

### BearDog + BarraCUDA composition

BearDog provides cryptographic scaffolding (Ed25519, ChaCha20-Poly1305, BLAKE3,
X.509, genetic lineage). BarraCUDA provides FHE GPU compute (NTT, INTT,
pointwise mod-mul). Together they enable:

1. BearDog generates FHE scheme parameters and keys
2. BearDog encrypts data using FHE scheme
3. BarraCUDA performs homomorphic computation on GPU
4. BearDog decrypts results

Neither depends on the other at the crate level. They compose at the primal IPC
level or as separate deps in a Spring. This is the sovereign FHE pipeline — no
CUDA, no cloud, cross-vendor.

---

## What ToadStool Becomes

After budding, ToadStool is:

- **Node atomic**: basic CPU computation, no GPU required
- **Orchestration**: primal lifecycle, IPC, discovery, biomeOS integration
- **Bridge**: connects to BarraCUDA (and other primals) for specialized compute
- **Lighter**: faster compile, smaller surface area, clearer mission

ToadStool retains its role as the compute orchestration primal in NUCLEUS. It
just no longer carries the full math engine inside its workspace. The mycelial
model from SOVEREIGN_COMPUTE_EVOLUTION.md still holds — ToadStool nodes spread
through computational substrates, discovering BarraCUDA instances for GPU math.

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

- [ ] `cargo check -p barracuda --no-default-features --features gpu` passes
- [ ] `cargo test -p barracuda --no-default-features --features gpu` passes (2,866+ tests)
- [ ] No `toadstool-core` usage without `#[cfg(feature = "toadstool")]`
- [ ] All 5 Springs can build against barracuda without toadstool workspace
- [ ] `barracuda validate-gpu` binary exists and passes on Intel, AMD, NVIDIA
- [ ] SemVer CHANGELOG tracks every API change
- [ ] At least one Spring evolves a second primal (e.g., BearDog) in same session

---

## References

- `SOVEREIGN_COMPUTE_EVOLUTION.md` — BarraCUDA as "unified math language"
- `PRIMAL_CAPABILITY_SYSTEM.md` — capability-based discovery (implemented)
- `wateringHole/UNIVERSAL_IPC_EVOLUTION_HANDOFF.md` — JSON-RPC 2.0 primal protocol
- `wateringHole/handoffs/TOADSTOOL_S88_BARRACUDA_PRIMAL_BUDDING_PROPOSAL_MAR02_2026.md`
- S87: FHE shader fixes proving GPU validation canary viability
- S86: ComputeDispatch evolution (144 ops migrated)
