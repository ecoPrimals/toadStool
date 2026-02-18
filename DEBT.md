# Active Technical Debt Register

**Date**: February 18, 2026
**Philosophy**: Workarounds are short-term solutions that increase debt.
We aim to solve deep debt over iterations, evolving toward vendor-agnostic,
capability-based solutions.

---

## Active Workarounds

### W-001: Open-Source GPU Driver f64 Transcendental Workaround

**Status**: ACTIVE — First solution implemented, capability probe live
**Impact**: ~2x performance penalty for exp/log on affected drivers
**Files**:
- `crates/barracuda/src/device/wgpu_device/capabilities.rs` — `needs_f64_exp_log_workaround()`, `probe_f64_exp_capable()`
- `crates/barracuda/src/device/probe.rs` — runtime capability probing, global cache
- `crates/barracuda/src/shaders/precision/mod.rs` — `for_driver_auto()`, `inject_missing_math_f64()`

**Problem**: Open-source GPU compiler backends crash on f64 transcendentals:
- **NVK/NAK** (NVIDIA nouveau): `exp(f64)` crashes the NAK compiler
- **RADV/ACO** (AMD open-source): `fexp2` unimplemented for bit size 64

**Current Solution**: Text replacement `exp()` → `exp_f64()`, `log()` → `log_f64()`
with software implementations from `math_f64.wgsl`. Detection is driver-name based,
with async `probe_f64_exp_capable()` now available for definitive runtime verification.

**Why This Is Debt**:
1. Text replacement is fragile (could match comments, variable names)
2. Driver detection is heuristic (name matching) — **DONE: capability probing implemented**
3. Software fallbacks are ~2x slower than native hardware
4. Applies blanket workaround rather than per-op capability check

**F64 Built-in Capability Matrix** (probed Feb 18, 2026 via `bench_f64_builtins`):

| Function     | RTX 3090 (Ampere/PTXAS) | RX 6950 XT (RDNA2/ACO) | Titan V (NVK/NAK) | RTX 4070 (Ada/PTXAS) |
|-------------|------------------------|------------------------|-------------------|----------------------|
| exp, log    | NATIVE                 | fallback               | fallback          | NATIVE (expected)    |
| exp2, log2  | NATIVE                 | fallback               | fallback          | NATIVE (expected)    |
| sin, cos    | NATIVE†                | fallback               | TBD               | NATIVE† (expected)   |
| sqrt        | NATIVE                 | **NATIVE**             | TBD               | NATIVE               |
| fma         | NATIVE                 | **NATIVE**             | TBD               | NATIVE               |
| abs/min/max | NATIVE                 | **NATIVE**             | TBD               | NATIVE               |

†NVIDIA PTXAS sin/cos on f64 uses MUFU — likely f32 precision in f64 register. Precision probe needed.

Strategic insight: WGSL → naga → SPIR-V → Vulkan exposes `VK_KHR_shader_float64` directly,
bypassing the proprietary software FP64 lock. Both RTX 3090 and RX 6950 XT confirm SHADER_F64=true.
`sqrt`, `fma`, `abs/min/max` are **universally native** across all SHADER_F64 hardware.
The `math_f64.wgsl` software implementations for these are unnecessary debt — removable once
grep confirms no shaders call `sqrt_f64()` etc. directly.

**Evolution Path** (ordered by priority):
1. **DONE: Capability probing** — `probe::probe_f64_builtins()` tests ALL f64 builtins,
   crash-isolated per function. Cache keyed per adapter. Legacy `probe_f64_exp_capable()`
   preserved. Run `cargo run --release --bin bench_f64_builtins` on any GPU.
2. **DONE: Fossil f64 functions** — Feb 18 2026. `math_f64.wgsl` software implementations
   for `abs`, `sqrt`, `min`, `max`, `clamp`, `sign`, `floor`, `ceil`, `round`, `fract`
   are marked as `🦴 FOSSIL`. `ShaderTemplate::inject_missing_math_f64()` now skips
   fossils. `ShaderTemplate::substitute_fossil_f64()` rewrites legacy `abs_f64(` → `abs(`
   etc. `for_driver_auto()` applies fossil substitution automatically.
   Active function bodies (`cbrt_f64`, `exp_f64`, `pow_f64`, `erf_f64`, `gamma_f64`,
   `bessel_j0_f64`) now call native WGSL builtins directly (no more fossil deps).
3. **Upstream ACO fix**: Contribute `fexp2(f64)` implementation to Mesa ACO for RDNA2/3.
   Track: https://gitlab.freedesktop.org/mesa/mesa
4. **Upstream NAK fix**: Contribute `exp(f64)` lowering to Mesa NAK compiler.
   Track: https://gitlab.freedesktop.org/mesa/mesa — see W-003 for full NAK roadmap.
5. **Remove workaround**: When both compilers support f64 transcendentals natively,
   delete the exp/log replacement logic entirely.

**Validation**: RTX 3090 (9/9 native) + RX 6950 XT (3/9: sqrt/fma/abs native) confirmed.
Titan V (NVK) + RTX 4070 probe needed from hotSpring.

---

### W-002: PPPM GPU Physics Validation — RESOLVED

**Status**: RESOLVED Feb 18, 2026
**Impact**: ~~3 PPPM GPU tests fail~~ — All physics tests now pass.

**Root Cause**: `PppmCpuFft` (used by GPU k-space path) had a buggy 1D Cooley-Tukey
FFT that only paired `(start, start+half)` instead of `(start+k, start+k+half)` for
each butterfly index k. This produced ~36× inflated e_kspace, wrong energy sign,
and violated Newton's 3rd law.

**Fix**: Replaced `PppmCpuFft` fft_1d and fft_3d with exact CPU `Pppm::fft_1d_cpu`
implementation. Added inverse FFT normalization in fft_3d (was missing, causing
double-normalization in inverse_3d).

**Files**: `crates/barracuda/src/ops/md/electrostatics/pppm_buffers.rs`

---

### W-003: NAK Compiler 149x Performance Gap (Sovereign FP64 Compute)

**Status**: ACTIVE — Phase 1 latency tables written, pending hardware validation on Titan V
**Impact**: NVK/NAK Jacobi eigensolve ~9x slower than NVIDIA proprietary after warp-packing
**Files**:
- `crates/barracuda/src/shaders/linalg/batched_eigh_single_dispatch_f64.wgsl` — warp-packed (done)
- `crates/barracuda/src/device/capabilities.rs` — `GpuDriverProfile`, `EigensolveStrategy` (done)
- `crates/barracuda/src/bin/bench_wgsize_nvk.rs` — diagnostic binary (done)
- `ecoPrimals/mesa-nak/.../sm70_instr_latencies.rs` — **NEW: SM70 latency table** (Phase 1)
- `ecoPrimals/mesa-nak/.../sm70.rs` — wired SM70Latency into all 6 dispatch points (Phase 1)

**Problem**: hotSpring analysis (Feb 18, 2026) found a 149x compiler efficiency gap
between NAK (Mesa open-source NVIDIA compiler, Rust) and proprietary PTXAS for
loop-heavy f64 Jacobi kernels. Root cause is five specific NAK deficiencies:

| # | Deficiency | Gap factor | NAK status |
|---|-----------|------------|------------|
| 1 | No SM70 instruction scheduling | ~3-4x | **DONE** — `sm70_instr_latencies.rs` written |
| 2 | No dual-issue exploitation | ~2x | Not implemented for any arch |
| 3 | Limited loop unrolling | ~1.5-2x | MR 26626 (Dec 2023), may miss nested loops |
| 4 | Missing f64 FMA selection | ~1.3-1.5x | Not confirmed, needs IR dump |
| 5 | Generic shared-mem scheduling | ~1.5-2x | No bank-conflict awareness |

**First Solution Already Absorbed** (R-019):
- Warp-packed eigensolve (`@workgroup_size(32,1,1)`) — 2.2x NVK speedup, neutral on proprietary
- `GpuDriverProfile::optimal_eigensolve_strategy()` — data-driven strategy selection
- `bench_wgsize_nvk.rs` — permanent diagnostic binary

**Phase 1 DONE (Feb 18, 2026)**:
Created `sm70_instr_latencies.rs` — SM70/Volta instruction latency table, structured after
SM75's (Turing) table but without HMMA/IMMA (Volta doesn't have tensor cores), with corrected
FP64 latencies from arXiv:1804.06826:
- **FP64 DFMA was 13cy placeholder → now 8cy** (key correction, ~1.6x scheduling improvement)
- **FP32 FFMA**: 4cy (correct, was 6cy placeholder for ALU)
- **WAR latency**: per-category (was flat 4cy guess)
- **WAW latency**: per-category (was `instr_latency()` approximation)
- **Scoreboard assignment**: real `needs_scoreboards()` per instruction (was `!has_fixed_latency`)
Wired into `sm70.rs` at: `op_needs_scoreboard`, `raw_latency`, `war_latency`, `waw_latency`,
`paw_latency` (already had Volta branch), `worst_latency`, `latency_upper_bound`.
**Next step**: run `bench_wgsize_nvk` on Titan V with patched Mesa NVK to measure impact.

**Evolution Path** (NAK contribution — all Rust, AGPL-aligned):

1. **Phase 1**: ~~SM70 latency tables~~ **DONE** (Feb 18, 2026)
   - `sm70_instr_latencies.rs` created; `sm70.rs` wired
   - Data: arXiv:1804.06826 — DFMA=8cy, FFMA=4cy, IMAD=6cy
   - Expected impact: **~3-4x** scheduler improvement on Titan V (pending hardware test)

2. **Phase 2**: f64 FMA selection — `mul+add` → `DFMA`
   - SM70 has native `DFMA` (same latency as DMUL+DADD but 1 instruction vs 2)
   - Investigation: Godbolt CUDA→SASS to check if PTXAS fuses, then naga/NAK path
   - Target file: naga SPIR-V emitter or `nak/src/from_nir.rs`
   - Impact on Jacobi: ~500M `c*akp - s*akq` patterns per large batch run

3. **Phase 3**: Loop unrolling for bounded nested loops (Jacobi `for k in 0..n, n≤32`)
   - MR 26626 (Dec 2023) added basic unrolling; status for nested loops unknown
   - Expected impact: **1.5-2x** additional

4. **Phase 4**: Dual-issue exploitation for SM70 (highest complexity)
   - Requires per-SM execution unit model + instruction pairing pass
   - Kepler SM32 work deferred this — SM70 would be first implementation
   - Expected impact: **~2x** additional

**Cumulative target**: After all 4 phases: ~3-6ms for n=30 batch=512
(from 69.8ms current, approaching 7.4ms proprietary baseline)

**Why This Matters**: NAK is written in Rust, same language as BarraCUDA. Every improvement
benefits all NVK users — this is the open-source multiplier. AMD RDNA3 with RADV/ACO is
a second target once NVK baseline is established.

**Tracking**: https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/nouveau/compiler

---

## Tracked Debt (Not Workarounds)

### D-005: RESOLVED — Production panics and stub implementations

**Status**: RESOLVED Feb 18, 2026
**What was fixed**:
- `barracuda/src/device/probe.rs`: `Mutex::lock().unwrap()` (×8) → `lock_cache()` helper
  using `unwrap_or_else(|e| e.into_inner())` — recovers gracefully from poisoned mutexes
- `barracuda/src/sample/sparsity/sampler_gpu.rs`: `.expect("GPU device required…")` →
  `ok_or_else(|| BarracudaError::InvalidInput { … })?` — surfaces misconfiguration as `Result`
- `core/toadstool/src/biomeos_integration/auth/mod.rs`:
  - Hardcoded `requesting_primal: "toadstool"` → `env!("CARGO_PKG_NAME")` (true self-knowledge)
  - Hardcoded audience `["songbird","nestgate","squirrel","biomeos"]` → `AuthManagerConfig::token_audience`
    (configurable via `TOADSTOOL_AUTH_AUDIENCE` env var, default unchanged)
- `core/common/src/service_discovery/service.rs`:
  - `discover_via_mdns()`: was `Ok(vec![])` stub → bridges to `MdnsAdapter::discover_all()`
    via `spawn_blocking`, maps `PrimalEndpoint` → `DiscoveredService`
  - `discover_from_config()`: was stub → reads JSON config file (`TOADSTOOL_DISCOVERY_CONFIG`
    env or `/etc/biomeos/discovery.json`), deserializes `ConfigFileService` entries
  - `discover_from_registry()`: was stub → HTTP registry via raw `tokio::net::TcpStream`
    (pure Rust, no reqwest/ring); Unix socket registries delegate to config discovery
- `distributed/src/songbird_integration/connection.rs`:
  - `SongbirdProtocol::HTTP` health check stub → rejects plain HTTP (with clear migration
    message), honours `unix://` endpoints via `probe_unix_socket()` (tokio `UnixStream::connect`)

### D-001: RESOLVED — All ops test modules migrated to shared device pool

**Status**: RESOLVED Feb 18, 2026
`test_pool::get_test_device*()` used 1,616 times across the codebase.
The only `WgpuDevice::new()` calls outside the pool are:
- Production code legitimately creating devices per job (gpu_executor, unified.rs, benchmarks)
- `bench_wgsize_nvk.rs` binary — intentional per-run device
- `cyclic_reduction_wgsl.rs` — dead code (not in module system, comment: "API drift")

### D-002: RESOLVED — Hardcoded values in production code (Feb 18, 2026)

**Status**: RESOLVED Feb 18, 2026
**What was fixed**:
- `api/types.rs`: `request_timeout_secs: 30` → `DEFAULT_REQUEST_TIMEOUT.as_secs()`
- `server/ollama.rs`: `timeout_secs: 30` → `DEFAULT_REQUEST_TIMEOUT.as_secs()`
- `security/sandbox/types.rs`: `cleanup_timeout_secs: 30` → `BIOME_SHUTDOWN_TIMEOUT.as_secs()`
- `runtime/specialty/mainframe/as400.rs`: `"127.0.0.1"` fallback → `LOCALHOST_IPV4`
- `runtime/container/bin/toadstool-byob-server.rs`: `"0.0.0.0"`/`8084` → `BIND_ALL_IPV4`/`BYOB_DEFAULT_PORT`
- `cli/daemon/config.rs`: `port: 8084` → `BYOB_DEFAULT_PORT`
- Added `BYOB_DEFAULT_PORT` (8084) to `toadstool_common::constants::network`

**Remaining**: Test fixtures and example code may still use hardcoded values — acceptable per audit rules.

### D-003: RESOLVED — All files now under 1000 lines

**Status**: RESOLVED Feb 18, 2026
All non-showcase Rust files are under 1000 lines.
Deflation, shift-invert, blocked, banded eigh variants are future additions (new files when implemented).

### D-004: cudarc version outdated in docs

**Impact**: DEEP_DEBT_STATUS.md references 0.11 → 0.19 as future; already done
**Evolution**: Audit and update stale documentation

---

## Resolved Debt (Recent)

| ID | Description | Resolution Date |
|----|-------------|----------------|
| R-001 | All-or-nothing shader injection (`_safe` methods) | Feb 18, 2026 |
| R-002 | Hardcoded `div_ceil(256)` in 19 files | Feb 18, 2026 |
| R-003 | String-based hardware detection in substrate.rs | Feb 18, 2026 |
| R-004 | `futures::executor::block_on` in async test context | Feb 18, 2026 |
| R-005 | wgpu 0.19 pinned dependency | Feb 17, 2026 |
| R-006 | NVK-only exp workaround (now covers NVK + RADV) | Feb 18, 2026 |
| R-007 | Duplicate `ExternalTarget` enums in crypto_lock vs security_provider | Feb 18, 2026 |
| R-008 | `reqwest` C-FFI dep in toadstool-client — migrated to Unix JSON-RPC | Feb 18, 2026 |
| R-009 | `crates/client` excluded from workspace — re-included | Feb 18, 2026 |
| R-010 | 9 files over 1000 LOC — smart-refactored: cg_gpu, multi_gpu, production_hardening, graph_types, handlers, graph_types, handlers | Feb 18, 2026 |
| R-011 | WebSocket (tungstenite/ring C-FFI) removed from entire codebase — pure Rust | Feb 18, 2026 |
| R-012 | 10 more files over 1000 LOC refactored: batched_eigh, wgpu_device, tensor_context, workload_migration, deployment_layer, songbird/types, analyzer, three_springs tests, hotspring tests, capabilities/tests | Feb 18, 2026 |
| R-013 | D-002 hardcoded timeouts replaced with toadstool_common constants | Feb 18, 2026 |
| R-014 | D-004 stale docs updated (cudarc 0.11→0.19, WebSocket refs removed) | Feb 18, 2026 |
| R-015 | sparsity.rs (1242L), fd_gradient_f64.rs (1175L), manual_jsonrpc.rs (1100L) split | Feb 18, 2026 |
| R-016 | D-001 partial: test_pool foundation + 9 ops modules migrated to shared GPU device | Feb 18, 2026 |
| R-017 | cg_gpu (1519L), pppm_gpu (1337L), precision (1270L), primal_sockets (1154L), service_discovery (1135L), cuda_impl (1093L), ipc_helpers (1091L), unibin (1059L), composition_constraints (1051L), resource_optimizer (1036L), biomeos/auth (1033L) all split | Feb 18, 2026 |
| R-018 | D-003 resolved: ALL non-showcase files now under 1000 lines | Feb 18, 2026 |
| R-019 | Warp-packed eigensolve (`@workgroup_size(32,1,1)`, 2.2x NVK speedup), `GpuDriverProfile`, `EigensolveStrategy`, `bench_wgsize_nvk.rs` diagnostic binary — hotSpring Phase 1 handoff absorbed | Feb 18, 2026 |
| R-020 | D-002 full audit: production hardcodes (timeouts, IPs, ports) replaced with constants — api, server/ollama, security/sandbox, runtime/specialty, runtime/container, cli/daemon | Feb 18, 2026 |
| R-021 | W-002 PPPM GPU physics: fixed PppmCpuFft FFT (Cooley-Tukey butterfly bug), aligned e_kspace/forces with CPU reference — all 3 physics tests pass | Feb 18, 2026 |

---

*Debt is tracked, not ignored. Each workaround has an evolution path.*
*The goal is zero workarounds — vendor-agnostic, capability-based code.*
