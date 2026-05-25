# toadStool S275 — Wave 49: Ecosystem Tightening

**Date**: May 25, 2026
**Session**: S275
**From**: toadStool team
**To**: primalSpring (downstream audit)
**Audit**: primalSpring Wave 49 — ecosystem tightening (3 vectors + startup debt)

---

## Summary

All three Wave 49 cleanup vectors resolved. Startup latency pipeline debt
addressed with deferred wgpu GPU enumeration and pre-bound socket optimization.

## Vector A — Stale Deploy Patterns (High — correctness)

| File | Change |
|------|--------|
| `.cargo/config.toml` | `target/release/toadstool` ref → plasmidBin wording |
| `docs/guides/AKIDA_DRIVER_DEPLOYMENT.md` | `target/release/toadstool` Dockerfile → plasmidBin depot |
| `crates/cli/README.md` | `cargo install toadstool-cli` → plasmidBin depot docs |
| `scripts/install-akida-driver.sh` | Prefers plasmidBin depot binary, falls back to local build |

No `which toadstool` patterns found. `notify-plasmidbin.yml` confirmed active.

## Vector B — wateringHole Consolidation (Medium)

36 local handoffs mirrored to central `infra/wateringHole/`:

- **8 active** (S267–S274) → `handoffs/`
- **28 historical** (S243–S266) → `handoffs/archive/`

Local handoffs remain as canonical source.

## Vector C — Showcase Fossilization (Low)

35 files (8 progressive API demos, 2 levels) archived to
`fossilRecord/primals/toadStool/showcase_wave49/`. `showcase/` replaced
with pointer README.

## Pipeline Debt — Startup Latency (>8s → ~3s)

### Root cause

`create_executor()` ran **before** socket bind, with wgpu GPU enumeration
(1–5s Vulkan driver init) and mDNS coordination discovery (3s) on the
critical path.

### Fix: deferred wgpu + pre-bound socket

1. **Deferred wgpu GPU enumeration** — `query_local_capabilities()` now
   returns a fast baseline (cpu, memory, orchestration) immediately and
   spawns wgpu enumeration in a background `tokio::spawn`. Full GPU
   capabilities populate the `OnceLock` asynchronously. Saves 1–5s.

2. **Pre-bound JSON-RPC socket** — `prebind_unix_listener()` binds the
   socket before `create_executor()` runs. `serve_unix_prebound()` accepts
   the pre-bound listener. Health probes can connect during init.

3. Default `LocalDirect` deployment: no orchestrator overhead.

### New public API

- `prebind_unix_listener(path) → UnixListener`
- `serve_unix_prebound(handler, listener)`
- `start_servers_with_fallback(..., jsonrpc_listener: Option<UnixListener>)`

## Additional Cleanup (S275)

| Item | Action |
|------|--------|
| `toadstool.toml` | HTTP-era 364-line template fossilized → `toadstool_wave49.toml`; replaced with 30-line IPC-first config |
| `primal-capabilities.toml` | Added Wave 49 note: legacy primal name sections are reference only |
| `docs/guides/PRIMAL_INTEGRATION_GUIDE.md` | Fossilized (S275) — predates S273 capability migration |
| `docs/reference/PRODUCTION_DEPLOYMENT_GUIDE.md` | Env vars updated to IPC-first (removed Songbird/BearDog/NestGate ports) |
| `docs/reference/TYPES_REFERENCE.md` | Session tag updated to reflect historical status |
| `docs/architecture/DAEMON_MODE_EVOLUTION.md` | Session range updated to S275 |
| `crates/core/cylinder/src/vfio/amd_metal.rs` | `#[allow(dead_code)]` → `#[expect(dead_code, reason)]` |

## Verification Checklist

- [x] No `showcase/` directory (pointer README only)
- [x] Local `wateringHole/` active handoffs mirrored to `infra/wateringHole/handoffs/`
- [x] No `which toadstool` or `target/release/toadstool` in scripts
- [x] `notify-plasmidbin.yml` active in `.github/workflows/`
- [x] Production TODO/FIXME/HACK: 0
- [x] `toadstool.toml` HTTP-era template fossilized

## Metrics

| Metric | Value |
|--------|-------|
| Lib tests | 9,149+ |
| Workspace tests | 23,000+ |
| JSON-RPC methods | 88 |
| Clippy warnings | 0 |
| Showcase files archived | 35 |
| Handoffs mirrored to central | 36 (8 active + 28 archive) |
| Stale deploy refs fixed | 4 |

---

Ready for downstream primalSpring audit.
