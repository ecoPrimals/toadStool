# Unidirectional Pipeline Evolution

**Created**: February 17, 2026  
**Status**: Design Phase  
**Goal**: Eliminate GPU round-trip latency through data flow discipline

---

## Quick Links

| Document | Purpose |
|----------|---------|
| [GPU-Direct Display](docs/planning/GPU_DIRECT_DISPLAY_EXPLORATION_FEB17_2026.md) | Gaming engine patterns, storage textures |
| [Hardware Routing Layer](docs/planning/HARDWARE_ROUTING_LAYER_FEB17_2026.md) | ToadStool as physical interconnect manager |
| [Unidirectional Pipeline](docs/planning/UNIDIRECTIONAL_COMPUTE_PIPELINE_FEB17_2026.md) | Factory model, HDMI as data channel |
| [Software Simulation](docs/planning/SOFTWARE_UNIDIRECTIONAL_SIMULATION_FEB17_2026.md) | Validate without hardware (90/10 split) |

---

## The Problem

hotSpring L2 mega-batch: GPU eigensolve was **70× slower** than pure CPU.

```
Traditional:
  CPU → GPU (upload)  → wait
  GPU → CPU (result)  → wait
  CPU → GPU (next)    → wait
  
  Round-trip latency dominates, not compute.
```

---

## The Solution

**Unidirectional data flow** — separate input and output paths:

```
┌─────────────────────────────────────────────────────────────┐
│  INPUT PATH (90% bandwidth)     OUTPUT PATH (10% bandwidth) │
│  ─────────────────────────      ────────────────────────── │
│  CPU → GPU (continuous)         GPU → CPU (batched, async)  │
│  Fire and forget                Never blocks input          │
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 0: Design ✅ COMPLETE

- [x] GPU-Direct Display exploration
- [x] Hardware Routing Layer concept
- [x] Unidirectional Pipeline architecture
- [x] Software Simulation design

### Phase 1: Ring Buffer Staging ✅ COMPLETE

- [x] `GpuRingBuffer` struct (input/output variants)
- [x] Write head / read head management
- [x] Atomic SPSC for thread safety
- [x] Power-of-two capacity for efficient modulo
- [x] Stats tracking

**Files created:**
- `crates/barracuda/src/staging/ring_buffer.rs`
- `crates/barracuda/src/staging/mod.rs`

### Phase 2: Unidirectional Pipeline ✅ COMPLETE

- [x] `UnidirectionalConfig` struct
- [x] `UnidirectionalPipeline` struct
- [x] `submit()` / `try_submit()` — fire and forget
- [x] `mark_completed()` — work tracking
- [x] `stats()` — metrics
- [x] In-flight work queue
- [x] Bandwidth throttling integration

**Files created:**
- `crates/barracuda/src/staging/unidirectional.rs`

### Phase 3: Bandwidth Throttling ✅ COMPLETE

- [x] `BandwidthThrottler` struct
- [x] Rate limiting for simulation
- [x] Configurable input/output split via `UnidirectionalConfig`
- [x] Strict mode for unidirectional enforcement

### Phase 4: Benchmark

- [ ] Traditional pattern benchmark
- [ ] Unidirectional pattern benchmark
- [ ] Side-by-side comparison
- [ ] Document results

**Files to create:**
- `crates/barracuda/benches/unidirectional_benchmark.rs`

### Phase 5: Hardware (Future)

- [ ] HDMI output encoding
- [ ] Capture card integration (Magewell)
- [ ] GPUDirect support
- [ ] Cross-machine pipeline

---

## Key Metrics

| Metric | Traditional | Unidirectional Target |
|--------|-------------|----------------------|
| Round-trips per work unit | 2 | 0 |
| Input bandwidth utilization | ~50% | ~90% |
| Output bandwidth utilization | ~50% | ~10% |
| Latency (small workloads) | High | Low |

---

## Validation Criteria

```rust
let stats = pipeline.stats();
let validation = stats.validate_unidirectional(&config);

// Success if:
// - Input/output ratio matches config (within 5%)
// - No synchronous readbacks occurred (strict mode)
// - Throughput improved vs traditional baseline
```

---

## Architecture Notes

### Separation of Concerns

```
Songbird   → WHERE (which machine, which endpoint)
ToadStool  → HOW (which physical pipe: PCIe, HDMI, NVLink)
BarraCUDA  → WHAT (the math: shaders, algorithms)
```

### Why HDMI Matters

| Path | Bandwidth | Character |
|------|-----------|-----------|
| PCIe x16 | 64 GB/s | Bidirectional, bursty |
| HDMI 2.1 | 6 GB/s | One-way, continuous streaming |
| DP 2.1 | 9.6 GB/s | One-way, continuous streaming |

For **completed results** (not raw data), 10 GB/s is plenty:
- Eigenvalues only: 12.5M sets/sec
- MD positions: 41K frames/sec (10K particles)

### The Factory Model

```
Traditional: Conversation
  "Here's data" → "Here's result" → "More data" → ...

Unidirectional: Factory
  Raw materials in → Assembly line → Finished products out
  (never stops the line to ship each product)
```

---

## Status Updates

### Feb 17, 2026 (Evening)

- **Phase 1 COMPLETE**: `GpuRingBuffer` implemented with SPSC atomics
- **Phase 2 COMPLETE**: `UnidirectionalPipeline` with fire-and-forget API
- **Phase 3 COMPLETE**: `BandwidthThrottler` integrated
- Deep Debt: Fixed production mocks in beardog/neurobench
- Deep Debt: Identified 100+ hardcoded timeouts for future evolution

### Feb 17, 2026 (Morning)

- Created 4 design documents
- Established architecture and terminology
- Defined implementation phases

---

*Tracking document for unidirectional pipeline evolution*
