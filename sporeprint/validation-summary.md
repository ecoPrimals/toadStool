+++
title = "ToadStool Validation Summary"
description = "Sovereign compute hardware primal — 9,232+ lib tests, 23,000+ workspace tests, 112 JSON-RPC methods, 47 crates, VFIO GPU init pipeline, v0.2.0, zero libc, 44 unsafe (all SAFETY-documented), 100% env centralized, cross-platform GPU backends (WgpuGpuDiscovery, PortableSwapExecutor, PortableResourceHandle), cfg-gated cross-arch"
date = 2026-07-16

[taxonomies]
primals = ["toadstool"]
springs = ["hotspring", "wetspring", "airspring", "groundspring", "neuralspring", "primalspring"]
+++

## Status

- **Version**: 0.2.0 (Session S335+, Jul 16, 2026)
- **Lib tests**: 9,232+ (0 failures, unlimited parallelism)
- **Workspace tests**: 23,000+ (0 failures, ~221 hardware-gated ignores; 9,232+ lib-only)
- **JSON-RPC methods**: 112 (direct) + semantic registry aliases
- **Workspace crates**: 47
- **Clippy**: 0 warnings (`-D warnings`)
- **`cargo deny`**: Clean (aws-lc-sys, ring, openssl, zstd-sys banned)
- **Unsafe blocks**: 44 actual (all SAFETY-documented, hw-containment crates only)
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
$XDG_RUNTIME_DIR/biomeos/compute-tarpc.sock — tarpc hot-path
$XDG_RUNTIME_DIR/biomeos/toadstool.sock     — legacy symlink
$XDG_RUNTIME_DIR/ecoPrimals/toadstool/display.sock — display IPC
```

## See Also

- [ToadStool README](https://github.com/ecoPrimals/toadStool)
- [CHANGELOG](https://github.com/ecoPrimals/toadStool/blob/main/CHANGELOG.md)
- [Sovereign Compute Evolution](https://github.com/ecoPrimals/toadStool/blob/main/specs/SOVEREIGN_COMPUTE_EVOLUTION.md)
