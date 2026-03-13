# ToadStool Showcase Collection

**Hardware Discovery, Compute Orchestration, and Inter-Primal Compute Patterns**

**Status**: Active | **Updated**: March 13, 2026 -- S152
**License**: AGPL-3.0-only

---

## Quick Start

### Run the Local Primal Showcase (30 minutes automated tour)

```bash
cd showcase/00-local-primal
./01-hello-compute/demo.sh          # 30s  - Health, version, capabilities
./02-hardware-discovery/demo.sh     # 60s  - CPU, GPU, NPU substrate probing
./03-workload-lifecycle/demo.sh     # 60s  - Submit, status, result, cancel
./04-resource-management/demo.sh    # 30s  - Resource estimation and optimization
./05-gpu-job-queue/demo.sh          # 60s  - GPU dispatch and queue management
```

### Compute Triangle Demo (toadStool + barraCuda + coralReef)

```bash
cd showcase/02-compute-patterns
./04-shader-to-gpu/demo.sh          # The headline demo: compile -> dispatch -> execute
```

---

## Showcase Levels

### Level 00: Local Primal (no external services needed)

Demonstrates toadStool's core capabilities in isolation. Each demo uses library
APIs directly and runs on any machine with Rust installed.

| Demo | Time | What It Shows |
|------|------|---------------|
| 01-hello-compute | 30s | Health check, version, capability enumeration |
| 02-hardware-discovery | 60s | CPU/GPU/NPU substrate probing via /proc + wgpu |
| 03-workload-lifecycle | 60s | Full compute.submit -> status -> result -> cancel |
| 04-resource-management | 30s | Resource estimation, validation, optimization suggestions |
| 05-gpu-job-queue | 60s | GPU job dispatch, queue management, capabilities |

### Level 01: Shader Pipeline (toadStool + coralReef)

Demonstrates shader compilation with naga fallback and coralReef integration.

| Demo | Time | What It Shows |
|------|------|---------------|
| 01-naga-fallback | 30s | WGSL compile via naga (no coralReef needed) |
| 02-coralreef-compile | 60s | WGSL/SPIR-V compile via coralReef socket |
| 03-compile-status | 30s | Async compilation status polling |

### Level 02: Compute Patterns (toadStool + barraCuda + coralReef)

Demonstrates the compute triangle: toadStool decides WHERE, barraCuda decides WHAT,
coralReef compiles HOW.

| Demo | Time | What It Shows |
|------|------|---------------|
| 01-capability-discovery | 30s | Runtime discovery of compute.sock, coralreef.sock |
| 02-science-dispatch | 60s | science.compute.submit + science.gpu.dispatch |
| 03-deploy-graph | 60s | deploy.capability_call routing to barraCuda |
| 04-shader-to-gpu | 120s | Full triangle: compile -> dispatch -> execute |

### Level 03: Ecosystem Integration (toadStool + phase1 primals)

Demonstrates toadStool interacting with the broader ecoPrimals ecosystem.

| Demo | Time | What It Shows |
|------|------|---------------|
| 01-songbird-registration | 60s | Register compute capabilities for cross-tower discovery |
| 02-beardog-secured-compute | 60s | Signed workload submission via beardog tokens |
| 03-nestgate-artifact-storage | 60s | Store/retrieve compute artifacts via nestgate |

---

## Archived Hardware Showcases

Pre-progressive hardware showcases archived to `ecoPrimals/fossil/toadStool/showcase-hardware-S139/` (S139).
These predate the progressive showcase and required specialized hardware: neuromorphic, gpu-universal, homomorphic-computing, akida-characterization, barracuda-validation, whitePaper.

---

## Building

All showcases are excluded from the main workspace build.
Build individually:

```bash
cd showcase/00-local-primal/01-hello-compute
cargo build --release
```

Or run a demo directly:

```bash
./demo.sh
```

---

*See [00_SHOWCASE_INDEX.md](00_SHOWCASE_INDEX.md) for the full learning path.*
*See [QUICK_START.md](QUICK_START.md) for a 5-minute guided tour.*
