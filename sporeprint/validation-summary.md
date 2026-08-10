+++
title = "ToadStool Validation Summary"
description = "Sovereign compute hardware primal — 9,008+ lib tests, 126 JSON-RPC methods, 48 crates, VFIO GPU init pipeline, v0.2.0, zero libc, 138 unsafe blocks (all SAFETY-documented, containment crates only), 100% env centralized, security fail-closed, zero dead deps, zero doc/clippy warnings, 16/16 cross-arch, 38/48 WASM, C2 dual-socket, G68 platform containment, Tokio blast radius reduced, NUCLEUS manifest converged (5→2 structs)"
date = 2026-08-10

[taxonomies]
primals = ["toadstool"]
springs = ["hotspring", "wetspring", "airspring", "groundspring", "neuralspring", "primalspring"]
+++

## Status

- **Version**: 0.2.0 (Session S377, Aug 10, 2026)
- **S377**: NUCLEUS manifest convergence — 5→2 `BiomeManifest` structs. All subsystems re-export canonical `toadstool-core` type.
- **S376**: Tokio blast radius reduction — `tokio::fs`→`std::fs` (37 files), `tokio::process`→`std::process` (15 files), RwLock 99→20 files, 7 crates feature-gated (31→38/48 WASM), workspace tokio features 9→7.
- **S375**: WASM push 26→31/48. Canonical `BiomeManifest`. NUCLEUS composition graph.
- **S374**: Tokio deep debt — `runtime` feature gate, needless async removal, std::sync migration, 26/48 WASM. Node Atomic AAR: silicon discovery via coralReef IPC.
- **S373**: Large file decomposition, hardcoding → runtime discovery, zero `missing_docs`.
- **S372**: Vertebrate self-audit (126/126 methods verified), types extraction to `toadstool-core`.
- **S369**: 16/16 native cross-arch targets — first primal fleet-ready.
- **S365**: G68 complete — zero rustix outside hw-safe.
- **Lib tests**: 9,008+ (0 failures, unlimited parallelism)
- **JSON-RPC methods**: 126 (direct) + semantic registry aliases
- **Workspace crates**: 48
- **Cross-arch**: 16/16 native targets, 38/48 WASM crates
- **Clippy**: 0 warnings (`-D warnings`)
- **Doc warnings**: 0
- **`cargo deny`**: Clean (aws-lc-sys, ring, openssl, zstd-sys banned; 19+ bans)
- **Unsafe blocks**: 138 (all SAFETY-documented, designated containment crates only: hw-safe, cylinder, nvpmu, display, runtime/gpu, ffi_loader; all cylinder `#[allow]` have `reason`)
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

| Substrate | Discovery | Init Pipeline |
|-----------|-----------|---------------|
| NVIDIA Kepler (K80) | sysfs + PCIe BDF | VFIO sovereign init (cold boot, PMU devinit) |
| NVIDIA Volta (V100) | sysfs + PCIe BDF | VFIO sovereign init (warm/cold, HBM2 training) |
| NVIDIA Ampere+ | sysfs + PCIe BDF | VFIO sovereign init (ACR falcon boot) |
| AMD Vega 20 | sysfs + PCIe BDF | VFIO metal init pipeline |
| CPU (x86, ARM) | /proc/cpuinfo, sysfs | Direct dispatch |
| Akida NPU | VFIO / kernel / mmap | 160-unit neuromorphic driver |

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
