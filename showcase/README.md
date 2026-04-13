# ToadStool Showcase Collection

**Hardware Discovery, Compute Orchestration, and Inter-Primal Compute Patterns**

**Status**: Active | **Updated**: April 13, 2026 -- S203g
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

### ~~Compute Triangle Demo~~ (ARCHIVED S169)

Levels 01 and 02 are archived — the JSON-RPC methods they demonstrated (`shader.compile.*`,
`discovery.*`, `science.*`, `deploy.*`) were removed in S169 (compile → coralReef,
science/deploy → biomeOS). See `00-local-primal/` for current demos.

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

### Level 01: Shader Pipeline — ARCHIVED (S169)

Archived — `shader.compile.*` methods removed from toadStool in S169 (compile is coralReef's
domain). See `01-shader-pipeline/README.md` for details and current alternatives.

### Level 02: Compute Patterns — ARCHIVED (S169)

Archived — `discovery.*`, `science.*`, `deploy.*` methods removed from toadStool in S169
(science/deploy routing is biomeOS's domain). See `02-compute-patterns/README.md` for details.

### Level 03: Ecosystem Integration (toadStool + phase1 primals)

Demonstrates toadStool interacting with the broader ecoPrimals ecosystem.

| Demo | Time | What It Shows |
|------|------|---------------|
| 01-coordination-registration | 60s | Register compute capabilities for cross-tower discovery |
| 02-security-secured-compute | 60s | Signed workload submission via security service tokens |
| 03-storage-artifact-pipeline | 60s | Store/retrieve compute artifacts via storage service |

---

## Archived Hardware Showcases

Pre-progressive hardware showcases archived to `ecoPrimals/infra/wateringHole/fossilRecord/` (S139).
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
