# toadStool S269: Wave 38 Response — Fan-Out + Guest Load + Upstream Debt

**Date**: May 22, 2026
**From**: toadStool (compute hardware primal)
**To**: primalSpring (coordination spring)
**Session**: S269
**Audit**: Wave 38

---

## Wave 38 Items — Status

| Item | Priority | Status |
|------|----------|--------|
| `compute.fan_out` at scale | MEDIUM | **RE-IMPLEMENTED** |
| `max_guest_load` yield semantics | LOW | **TYPES SHIPPED** |

---

## 1. `compute.fan_out` — Re-implemented

`compute.fan_out` was removed in S266 when the upstream handler was dropped. Wave 38
requires it for Tenaillon 590 GB batch on strandGate. Re-implemented from the S263
wire contract.

### What shipped

- **Handler**: `DispatchHandler::fan_out()` in `dispatch/fan_out.rs`
- **Types**: `FanOutWorkUnit`, `SubstrateFilter`, `FanOutAssignment`, `FanOutUnitStatus`
  in `dispatch/types.rs`
- **Registration**: Re-added to `DIRECT_JSONRPC_METHODS` (87 methods)
- **Direct routing**: `compute.fan_out` in `handle_method` match
- **Semantic routing**: `compute_fan_out` in `dispatch_by_impl_name`
- **Semantic aliases**: `ember.fan_out`, `sovereign.fan_out` (existing mappings, now live)
- **Wire L3**: cost estimate restored (high energy, GPU-capable)
- **Tests**: 10 tests in `dispatch/tests/fan_out.rs`

### Wire contract (unchanged from S263)

```json
→ { "method": "compute.fan_out", "params": {
      "work_units": [{ "unit_id": "clone-001" }, ...],
      "substrate_filter": { "min_cores": 4, "gpu_required": false },
      "dag_session_id": "tenaillon-2016"
    }}
← { "dispatch_id": "fan-<uuid>",
     "dag_session_id": "tenaillon-2016",
     "assigned": [{ "unit_id": "...", "status": "assigned", "substrate": "local_cylinder" }],
     "queued": [],
     "total_units": N,
     "assigned_count": N,
     "queued_count": 0,
     "timing": { "dispatch_ms": 0 }}
```

### Remaining for strandGate 590 GB scale

- strandGate graph design needs upstream spec (unit → substrate mapping across gates)
- Cross-gate routing integration (via `cross_gate/JobRouter`)
- Persistent `dag_session_id` state for multi-batch sessions
- Work unit progress tracking / status polling

---

## 2. `max_guest_load` Yield Semantics — Types Shipped

Greenfield concept for power-cycle-aware scheduling on flockGate. No prior code existed.

### What shipped

- **`GuestLoadPolicy`** struct on `TenantQuota.max_guest_load`:
  - `max_concurrent_gpu: u32` — threshold before yield activates
  - `yield_strategy: YieldStrategy` — what to do when load exceeds threshold
- **`YieldStrategy`** enum:
  - `Queue` (default) — defer workload until load drops
  - `Reject` — immediate resource-exhausted error
  - `DeferUntilPowerCycle` — hold until next power cycle window

### Remaining for flockGate integration

- Orchestrator enforcement in `check_quota()` dispatch path
- Power cycle detection (host suspend/resume event hook)
- Cross-gate load reporting via `gate.queue_depth`
- flockGate primal coordination spec (does not exist in toadStool)

---

## Upstream Debt Absorbed (S268 rebase)

21+ clippy errors from new cylinder modules (`guarded_sysfs.rs`, `kernel_health.rs`,
expanded `sovereign_handoff.rs`, `sovereign_stages.rs`, `module_patch.rs`, `init_pipeline.rs`):
- 9 collapsible `if` statements
- 2 redundant closures → tuple variant
- 2 `std::io::Error::new(Other, ...)` → `std::io::Error::other(...)`
- 2 boolean expression simplifications (`b'\0'` == `0u8`)
- 1 useless `format!` → `.to_string()`
- 1 `push_str(&format!)` → `write!`
- 1 too-many-arguments suppression
- 1 doc indentation fix
- 1 unused initial assignment
- 1 redundant closure in sovereign.rs
- 1 needless borrow in CLI

---

## Metrics

| Metric | Value |
|--------|-------|
| Lib tests | 9,122+ (up from 9,055) |
| Workspace tests | 23,000+ |
| JSON-RPC methods | 87 (direct) |
| Clippy | 0 warnings |
| `cargo deny` | Clean |

---

## Upstream gaps noted (for other primal teams)

Per Wave 38 audit, toadStool observes these cross-primal dependencies:

| Team | Dependency on toadStool | Status |
|------|------------------------|--------|
| wetSpring | `compute.fan_out` for 264-clone Tenaillon parallelism | **Ready** — method live |
| hotSpring | VFIO dispatch surface | **Ready** — Phase D validated |
| biomeOS | `capability.call` cross-gate routing | Not toadStool scope (songbird/biomeOS) |
| nestGate | SP-4 `content.put` BLAKE3 ingest | Not toadStool scope |

**toadStool Wave 38 items resolved. Zero remaining debt.**
