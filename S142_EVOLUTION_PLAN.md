# S142+ Evolution Plan — Hardware-First, Spring-Parity, Sovereign Compute

**COMPLETED** — All P0/P1 items from this plan are done. See [SOVEREIGN_COMPUTE_GAPS.md](SOVEREIGN_COMPUTE_GAPS.md) for current status.

**Date**: March 10, 2026 — S142+
**Philosophy**: Real hardware first. Springs earn trust in the primal by reaching
parity. Telemetry is sovereign — it belongs to the system, not surveillance.
Multi-tenant enables both local direct access and cloud rental. Checkpointing
enables cloud-style deployments. Hardware transport gives springs direct access
to heterogeneous compute without CPU roundtrips.

---

## Priorities (Ordered)

### P0: Hardware Test Infrastructure

**Goal**: Close 83%→90% coverage with live hardware on strandgate.

strandgate fleet:
- **NVIDIA Titan V** (SM70, f64 native, NVK Volta quirks)
- **NVIDIA RTX** (Ada Lovelace)
- **AMD RX 6950 XT** (RDNA2, Infinity Cache)
- **2× CPU** (software rasterizer, WGPU CPU backend)
- **Akida AKD1000** (neuromorphic NPU, PCIe)

Current state:
- GPU tests use `TOADSTOOL_GPU_ADAPTER` env var — works
- NPU tests `#[ignore = "requires Akida hardware"]` — 8 tests
- V4L2 tests `#[ignore]` — 2 tests
- CI runs headless `ubuntu-latest` — no GPU, no NPU
- No mock GPU adapter. No Akida simulator.
- Coverage gap: V4L2/display ~3,800 lines, neuromorphic/VFIO ~2,000 lines

Deliverables:
- [x] `scripts/run-hardware-tests.sh` — strandgate-specific test runner (S142)
  - Sets `TOADSTOOL_GPU_ADAPTER` per device
  - Runs `cargo test -- --ignored` for GPU, NPU, V4L2
  - Reports per-device results
- [x] `.github/workflows/hardware.yml` — self-hosted runner job (strandgate) (S142)
  - Runs on push to master after headless CI passes
  - Full `--ignored` test suite with hardware
  - llvm-cov with hardware paths exercised
- [x] Mock parity sim for headless CI (S142)
  - `MockGpuAdapter` — returns synthetic `GpuAdapterInfo` for test assertions
  - `MockNpuBackend` — simulates Akida responses for capability/inference tests
  - Parity: mock produces same API shape as real hardware

### P1: Hardware Transport — PCIe P2P + Streaming

**Goal**: GPU-to-GPU data movement without CPU roundtrip. Springs get faster
multi-device access. PCIe P2P won't match NVLink, but it outperforms CPU.

Current state:
- 3 transports: DRM display (Tx), V4L2 capture (Rx), serial (bidirectional)
- `TransportRouter` with `route_once`/`route_loop` — works
- `TransportMedium::Pcie` and `TransportMedium::NvLink` — spec'd, not implemented
- JSON-RPC `transport.discover/list/route` — implemented
- `transport.open/stream/status` — marked "Future"

Deliverables:
- [x] `PcieTransport` — GPU-to-GPU via PCIe topology (S142)
  - Discover PCIe topology via sysfs (`/sys/bus/pci/devices/`)
  - Bandwidth probing (PCIe gen/width → theoretical max)
  - Falls back to CPU staging when P2P not available
- [x] `transport.stream` JSON-RPC — continuous streaming (S142)
  - Background task with cancel token
  - Throughput metrics reported via `transport.status`
- [x] `transport.open` JSON-RPC — explicit transport registration (S142)
- [x] `transport.status` JSON-RPC — active stream statistics (S142)
- [x] Update `InterconnectTopology` — PCIe switch detection, NUMA awareness (S144: `PcieTopologyGraph`)
- [x] Spring integration: `science.gpu.dispatch` with `target_device` hint (S144: `compile_wgsl` + `target_device`)
  - Springs can say "dispatch to device that has my previous output"
  - Router uses PCIe P2P when source/target share a switch

### P2: Multi-Tenant — Direct + Cloud Rental

**Goal**: hotSpring runs directly on strandgate (single-tenant, full access) OR
as a tenant on shared cloud GPUs. Same `science.gpu.dispatch` API.

Current state:
- Full spec at `specs/MULTITENANT_COMPUTE_ARCHITECTURE.md`
- `JobPriority` (Emergency→Background) — wired throughout
- `PriorityPreemption` in `multi_workload_compositor.rs`
- `TeamResourceQuotas` in BYOB (CPU, memory, GPU count)
- `IsolationLevel` (None→Maximum) for security
- No `ResourceOrchestrator`, no `TenantAllocation`, no fair-share

Deliverables:
- [x] `ResourceOrchestrator` trait + `LocalOrchestrator` impl (S142)
  - Maps `{tenant, priority, resource_request}` → `{device, time_slot}`
  - Single-tenant mode: trivially "give everything"
  - Multi-tenant mode: enforce quotas and priorities
- [ ] `TenantAllocation` tracking — what each tenant is using
- [ ] GPU time-slicing — round-robin when oversubscribed
- [ ] `compute.tenant.register` / `compute.tenant.status` JSON-RPC
- [ ] Integration test: hotSpring as single tenant vs two concurrent tenants

### P3: Sovereign Telemetry

**Goal**: Hardware monitoring for optimization. Belongs to the system, not
surveillance. Opt-in, local-only by default.

Current state:
- `TelemetryConfig` with `metrics_enabled` — opt-in (`TOADSTOOL_TELEMETRY=1`)
- `/metrics` JSON endpoint on server
- Prometheus format in CLI (`format_prometheus()`)
- `toadstool-sysmon`: CPU/memory/disk/load/network via `/proc`
- NPU hwmon: power and temperature via sysfs
- GPU adapter info: `typical_power_watts`, `max_power_watts`
- `tracing` crate throughout — no OpenTelemetry

Deliverables:
- [x] GPU hwmon sysfs reader — temperature, power, clock, utilization (S142: `toadstool-sysmon::gpu`)
  - `/sys/class/drm/card*/device/hwmon/hwmon*/` for AMD/NVIDIA
  - `/sys/class/drm/card*/device/gpu_busy_percent` for AMD
  - `nvidia-smi --query-gpu` fallback for NVIDIA proprietary
- [ ] `hardware.telemetry` JSON-RPC method
  - Returns per-device: temp, power, clock, utilization, memory pressure
  - Springs use this for precision decisions (throttled GPU = suspect results)
- [ ] Evolve `/metrics` to structured format (compatible with Prometheus scrape)
- [ ] Distributed trace context — workload ID propagated through dispatch chain
  - Not OpenTelemetry SDK (too heavy) — lightweight span IDs in JSON-RPC

### P4: Workload Checkpointing

**Goal**: Save enough state that a preempted workload can resume on a different
device. Enables cloud-style deployments with spot instances.

Current state:
- `FaultToleranceConfig.checkpointing_enabled` — config exists (default false)
- NestGate `store_artifact`/`retrieve_artifact` — works
- `StatefulPipeline<S>` in barraCuda — GPU-resident iteration
- No checkpoint API, no GPU state serialization, no NestGate integration

Deliverables:
- [ ] `Checkpointable` trait — barraCuda implements for pipelines
  - `checkpoint(&self) -> bytes::Bytes` — read back GPU buffers
  - `restore(&mut self, data: bytes::Bytes)` — upload to GPU
- [ ] `compute.checkpoint` JSON-RPC — trigger checkpoint of running workload
- [ ] `compute.restore` JSON-RPC — resume from checkpoint
- [ ] NestGate checkpoint storage — key: `checkpoint/{workload_id}/{stage}/{timestamp}`
- [ ] Preemption signal handling — spot instance warnings trigger auto-checkpoint

### Deferred: WASM Runtime

**Status**: Infrastructure without a use case. No spring uses WASM.

The `crates/runtime/wasm/` module exists with wasmi, fuel metering, memory
limits, module caching. WASI instantiation is incomplete. The daemon doesn't
register the WASM engine by default.

When needed: sandboxed plugins, cross-platform edge compute, portable workloads.
Not blocking anything. Will evolve when a consumer materializes.

---

## Memory Management Boundary

| Concern | Owner | Why |
|---------|-------|-----|
| GPU buffer create/destroy | barraCuda/coralReef | They know what data the math needs |
| Memory pressure tracking | toadStool | We own the hardware, we see the pressure |
| Memory pressure response | toadStool | We decide: evict, migrate, or reject |
| Unified memory abstraction | toadStool | We present a uniform view to the ecosystem |
| VRAM quota per tenant | toadStool | Multi-tenant enforcement |
| GPU-resident state | barraCuda | `StatefulPipeline<S>` — math knows its own state |
| Checkpoint serialization | barraCuda (trait) | Math knows which buffers matter |
| Checkpoint storage | toadStool → NestGate | We orchestrate; NestGate stores |
| PCIe P2P / dma-buf | toadStool | We own the transport layer |

toadStool evolves the **presentation and interactions** — telling springs "device
0 has 12GB free, device 1 is at 90% pressure, route to device 0." barraCuda and
coralReef allocate within that guidance.

---

## Spring-Primal Parity

The endgame: springs trust toadStool to handle hardware and optimization, and
focus entirely on science.

| Parity Milestone | Status |
|-----------------|--------|
| Springs submit via `science.gpu.dispatch` | Done |
| Springs get capabilities via `science.gpu.capabilities` | Done |
| Springs get NPU access via `science.npu.dispatch` | Done |
| Springs get precision routing advice | Done |
| Springs get multi-device dispatch with P2P | **P1** |
| Springs get resource allocation in multi-tenant | **P2** |
| Springs get hardware telemetry for decisions | **P3** |
| Springs get checkpoint/resume for cloud | **P4** |
| Springs trust toadStool for all optimization | Endgame |

When direct spring interaction and toadStool evolved interactions reach parity,
springs can drop their own hardware management code and focus entirely on science.

---

## Files Modified / Created

| File | Action | Purpose |
|------|--------|---------|
| `S142_EVOLUTION_PLAN.md` | Created | This document — root tracking |
| `specs/HARDWARE_TRANSPORT_SPEC.md` | Updated | Added PCIe P2P and streaming sections |
| `specs/MULTITENANT_COMPUTE_ARCHITECTURE.md` | Updated | Added cloud rental model |
| `specs/README.md` | Updated | Added S142 evolution context |
