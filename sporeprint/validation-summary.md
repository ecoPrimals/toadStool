+++
title = "ToadStool Validation Summary"
description = "Sovereign compute hardware primal — 8,446 lib tests, 131 JSON-RPC methods, 48 crates, VFIO GPU init pipeline, v0.2.0, zero libc, 160 unsafe blocks (all SAFETY-documented, containment crates only), 100% env centralized, security fail-closed, zero dead deps, zero doc/clippy warnings, 16/16 cross-arch, 38/48 WASM, C2 dual-socket, G68 platform containment, wgpu 28 (leads ecosystem), axum excised, fail-safe hardware tests, MSRV 1.92"
date = 2026-08-12

[taxonomies]
primals = ["toadstool"]
springs = ["hotspring", "wetspring", "airspring", "groundspring", "neuralspring", "primalspring"]
+++

## Status

- **Version**: 0.2.0 (Session S380, Aug 12, 2026)
- **S380**: G72 Tier 2 — wgpu 22→28 (MSRV 1.85→1.92), **P1 fix: vulkan-portability→vulkan** (musl depot crash), axum fully excised (BYOB → UDS JSON-RPC), silicon ledger + idle-aware routing (128 JSON-RPC methods), darwin/graftGate cfg fix, mdns feature-gate fix, lock-poisoning alignment, fail-safe wgpu/NPU/socket tests, deep debt audit clean. 8,446 tests, 0 failures.
- **S379**: G72 Dependency Pandemic Tier 1 + last-mile wiring — tokio `["full"]` trimmed, `signal` scoped to CLI+server (workspace 7→6 features), `tokio::fs` fully eliminated (28 files), 7 dead deps removed, 6 deps promoted to workspace (bytemuck aligned 1→1.14), WASM conversion wired, runtime hint inference, ~1,750 LOC excised.
- **S378**: Tokio vestigial segmentation — ~35k LOC feature-gated behind 9 non-default features. Default-build tokio surface 118→65 production files (45% reduction). GPU/WASM `tokio::sync` → `std::sync`. Server background monitors gated. CLI monitoring gated. runtime/edge excluded. `tokio::time` → `std::time` in 10 files.
- **S377**: NUCLEUS manifest convergence — 5→2 `BiomeManifest` structs. All subsystems re-export canonical `toadstool-core` type.
- **S376**: Tokio blast radius reduction — `tokio::fs`→`std::fs` (37 files), `tokio::process`→`std::process` (15 files), RwLock 99→20 files, 7 crates feature-gated (31→38/48 WASM), workspace tokio features 9→6.
- **S375**: WASM push 26→31/48. Canonical `BiomeManifest`. NUCLEUS composition graph.
- **S374**: Tokio deep debt — `runtime` feature gate, needless async removal, std::sync migration, 26/48 WASM. Node Atomic AAR: silicon discovery via coralReef IPC.
- **S373**: Large file decomposition, hardcoding → runtime discovery, zero `missing_docs`.
- **S372**: Vertebrate self-audit (126/126 methods verified), types extraction to `toadstool-core`.
- **S369**: 16/16 native cross-arch targets — first primal fleet-ready.
- **S365**: G68 complete — zero rustix outside hw-safe.
- **Lib tests**: 8,446 (0 failures, unlimited parallelism)
- **JSON-RPC methods**: 131 (direct) + semantic registry aliases
- **Workspace crates**: 48
- **Cross-arch**: 16/16 native targets, 38/48 WASM crates
- **Clippy**: 0 warnings (`-D warnings`)
- **Doc warnings**: 0
- **`cargo deny`**: Clean (aws-lc-sys, ring, openssl, zstd-sys banned; 19+ bans)
- **Unsafe blocks**: 160 (all SAFETY-documented, designated containment crates only: hw-safe, cylinder, nvpmu, display, runtime/gpu, akida-driver; all cylinder `#[allow]` have `reason`)
- **Production unwrap/panic**: 0
- **License**: AGPL-3.0-or-later (SPDX headers on all files)

## Ecosystem Role

ToadStool is the **WHERE** in the Compute Trio:
- **barraCuda** = WHAT (math dispatch)
- **toadStool** = WHERE (hardware discovery, GPU init, substrate routing)
- **coralReef** = HOW (shader compiler / pipeline)

**biomeOS grade**: Node Atomic READY. Wire Standard L3 (partial).

## Key Capabilities

| Domain | Capability |
|--------|-----------|
| GPU Discovery | Multi-adapter sysfs/PCIe, NVIDIA + AMD + Intel via WGPU/Vulkan |
| Sovereign Init | VFIO cold/warm boot pipeline (Kepler → Volta → Ampere+) |
| NPU | Akida neuromorphic via VFIO/kernel/mmap backends |
| IPC | JSON-RPC 2.0 + tarpc over Unix domain sockets |
| Workloads | WASM, container, native, GPU runtimes |
| Discovery | Capability-based (`primal.announce`, `capabilities.list`) |
| Dispatch | `compute.fan_out` — DAG-aware work unit routing |

## Key Binaries

- `toadstool` — UniBin server (JSON-RPC + tarpc, SIGINT/SIGTERM graceful shutdown)
- `toadstool daemon` — CLI daemon mode (workload submission, health, metrics)
- `toadstool run` — Workload execution from TOML specs
- `toadstool discover` — Runtime primal/capability discovery

## Hardware Substrates

"Init Pipeline" describes the implemented path. "Demonstrated" states what has
actually been observed on silicon, which is a different and smaller claim.

**No sovereign VFIO shader dispatch has executed on any NVIDIA GPU.** The
pipeline is wired end to end, but graphics-engine execution is blocked behind
PFIFO runlist configuration and FECS context load. Verified compute on NVIDIA
and AMD today runs through wgpu/Vulkan with a vendor driver present.

| Substrate | Discovery | Init Pipeline | Demonstrated on silicon |
|-----------|-----------|---------------|-------------------------|
| NVIDIA Kepler (K80) | sysfs + PCIe BDF | VFIO sovereign init (cold boot, PMU devinit) | Identity, PMC, PGRAPH ungate. Halts at devinit — Kepler register map incomplete |
| NVIDIA Volta (V100/Titan V) | sysfs + PCIe BDF | VFIO sovereign init (warm/cold, HBM2 training) | **Tier 1 warm infrastructure**, reproducible via warm handoff. FECS dead (`0xBADF5040`) — no dispatch. Cold boot blocked by HBM2 |
| NVIDIA Ampere+ | sysfs + PCIe BDF | VFIO sovereign init (ACR falcon boot) | Not exercised on this fleet |
| AMD Vega 20 | sysfs + PCIe BDF | VFIO metal init pipeline | Init only; compute proven via wgpu, not VFIO |
| CPU (x86, ARM) | /proc/cpuinfo, sysfs | Direct dispatch | Working |
| Akida NPU | VFIO / kernel / mmap | 160-unit neuromorphic driver | Driver + discovery |

## Socket Layout

```
$XDG_RUNTIME_DIR/biomeos/compute.sock       — JSON-RPC primary
$XDG_RUNTIME_DIR/biomeos/compute.tarpc.sock  — tarpc hot-path (C2 naming)
$XDG_RUNTIME_DIR/biomeos/toadstool.sock     — legacy symlink
$XDG_RUNTIME_DIR/ecoPrimals/toadstool/display.sock — display IPC
```

## See Also

- [ToadStool README](https://github.com/ecoPrimals/toadStool)
- [CHANGELOG](https://github.com/ecoPrimals/toadStool/blob/main/CHANGELOG.md)
- [Sovereign Compute Evolution](https://github.com/ecoPrimals/toadStool/blob/main/specs/SOVEREIGN_COMPUTE_EVOLUTION.md)
