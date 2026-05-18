# toadStool → primalSpring: Stadial Gate Response — S263

**Date**: May 17, 2026
**From**: toadStool (compute dispatch primal)
**Session**: S263
**Responding to**: Wave 22 Stadial Gate — All Upstream Primals

---

## Summary

All toadStool-specific items from the primalSpring stadial gate audit are
resolved. Universal standards checklist green on all items. Upstream ask
(`compute.fan_out`) implemented. Version tagged `0.2.0`.

---

## Universal Standards Checklist

| Item | Status | Notes |
|------|--------|-------|
| Health triad | **PASS** | `health.liveness`, `health.readiness`, `health.check` all routed |
| UDS socket | **PASS** | `$XDG_RUNTIME_DIR/biomeos/compute.sock` |
| TCP fallback | **PASS** | `--port` CLI flag, `ports.env` not referenced (TCP is CLI-configured) |
| `server` subcommand | **PASS** | `toadstool server --port <N>` |
| Standalone startup | **PASS** | Defaults to `FAMILY_ID=default`, `NODE_ID=default` |
| `capabilities.list` envelope | **FIXED (S263)** | Added `capabilities` array + `count` field per `CAPABILITY_WIRE_STANDARD.md` |
| `identity.get` | **PASS** | Returns primal, version, domain, capabilities, methods, transport |
| `primal.announce` | **ADDED (S263)** | Self-registration broadcast with capabilities, methods, count, status |
| Method naming | **PASS** | All methods follow `{domain}.{operation}[.{variant}]`; `toadstool.*` legacy retained for backward compat |
| BTSP handshake | **PASS** | ChaCha20-Poly1305 + HKDF with `btsp-v1` |
| FAMILY_ID + INSECURE | **PASS** | Refuses to start |
| `btsp.capabilities` | **ADDED (S263)** | Registered in `capabilities.list` response |
| Zero metadata leakage | **PASS** | Stripped binary, no path/hostname/username |
| UDS-first | **PASS** | TCP off unless explicitly enabled |
| `deny.toml` bans | **FIXED (S263)** | Added `aws-lc-sys` ban; `ring`, `openssl`, `openssl-sys` already banned |
| `edition = "2024"` | **PASS** | Workspace Cargo.toml |
| `notify-plasmidbin.yml` | **PASS** | Present in `.github/workflows/` |
| CHANGELOG / README / CONTEXT | **PASS** | Updated to S263 |
| Stability tiers | **DOCUMENTED** | All methods categorized in `cost_estimates` (Wire L3) |
| Degradation behavior | **DOCUMENTED** | Health triad + `health.drain` for graceful shutdown |
| Downstream pairing | **DOCUMENTED** | wetSpring (264-clone), hotSpring (GPU dispatch), coralReef (shader) |

---

## Upstream Ask: `compute.fan_out`

**Status**: IMPLEMENTED

Wire format matches the proposed contract:

```
→ { "method": "compute.fan_out", "params": {
      "work_units": [{ "unit_id": "clone-001" }, ...],
      "substrate_filter": { "min_cores": 4, "gpu_required": false },
      "dag_session_id": "tenaillon-2016"
    }}
← { "dispatch_id": "<uuid>",
     "dag_session_id": "tenaillon-2016",
     "assigned": [{ "unit_id": "clone-001", "status": "assigned", "substrate": "local_cylinder" }],
     "queued": [],
     "total_units": 1,
     "assigned_count": 1,
     "queued_count": 0,
     "timing": { "dispatch_ms": 0 }}
```

Behavior:
- Units assigned to available substrate (CPU or GPU via `local_cylinder`)
- `gpu_required` filter queues units when no GPU substrate available
- Auto-generates `unit_id` when not provided (format: `{dispatch_id}-{index}`)
- Semantic aliases: `ember.fan_out`, `sovereign.fan_out`
- Wire L3 cost: high energy, GPU-capable

**Degradation**: If `compute.fan_out` is unavailable, sequential clone processing
via `compute.dispatch.submit` produces identical results. Performance enrichment only.

---

## Composition Gaps (3 owned)

| Gap | Status | Resolution |
|-----|--------|------------|
| Sandbox `working_dir` passthrough | **ALREADY RESOLVED** | `ExecutionSpec::Native { working_dir }` + `SandboxSpec { working_directory }` both present |
| Env var expansion in workload TOMLs | **ALREADY RESOLVED** | `load_workload_file()` in CLI expands `${VAR}`, `$VAR`, `$$` escape |
| Data dependency declaration in TOML | **ADDED (S263)** | `DataDependency` struct: `name`, `source`, `blake3`, `required` fields in `WorkloadFile` |

---

## Version

Tagged **v0.2.0** in workspace Cargo.toml (from `0.1.0`).

---

## Metrics

| Metric | Value |
|--------|-------|
| JSON-RPC methods (direct) | 85 |
| Lib tests | 8,945 |
| Clippy warnings | 0 |
| `cargo deny` | Clean |
| Unsafe blocks | 46 (all SAFETY-documented) |
| Production panics | 0 |
| Production TODO/FIXME | 0 |

---

## Additional Fixes (deep debt)

- Fixed `#[expect(non_camel_case_types)]` in `gguf.rs` — added `reason`
- Fixed 4 pre-existing Rust 1.92 clippy issues (`manual_is_multiple_of`, `collapsible_if`, `needless_late_init`, `unnecessary_cast`)
- Added `#[expect(too_many_arguments)]` with reason on `VfioChannel::create_for_profile`

---

## Stadial Pairing Confirmation

| Partner | Pairing | Status |
|---------|---------|--------|
| wetSpring | 264-clone parallelism (Tenaillon 2016) | `compute.fan_out` ready |
| hotSpring | GPU dispatch (sovereign compute pipeline) | VFIO IPC surface live |
| coralReef | Shader compilation dispatch | QMD + metadata aliases wired |

toadStool is **stadial-ready**.
