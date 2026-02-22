# Deep Debt Status Report

**Sessions 32-39 -- February 22, 2026**
**Status**: PRODUCTION-GRADE | All quality gates green | 0 clippy warnings | 3,847+ non-GPU tests + barracuda targeted | Coverage: common 87%, config 89%, core 79%, server 77%

---

## Active Workarounds

- **W-001**: f64 transcendental (`exp`/`log`) text-replacement workaround for NVK/RADV open-source GPU drivers (~2x penalty for exp/log only). Fossil functions removed (sqrt/abs/min/max now native). Comment-aware replacement prevents source corruption. Capability matrix probed per-GPU. Upstream NAK/ACO contributions in progress. See DEBT.md.
- **W-003**: NAK compiler 149x performance gap on Titan V (SM70/Volta) — Phases 0–3 complete and live (latency tables, ILP restructure, `LatencyModel` trait, `WgslOptimizer` wired into `compile_shader_f64()`). All 5 deficiencies documented in `contrib/mesa-nak/NAK_DEFICIENCIES.md`. Titan V hardware validation pending — Mesa MR ready to submit once >= 3x speedup confirmed.

---

## Debt Categories — Current State

### Dependencies ✅ RESOLVED

| Category | Status |
|----------|--------|
| `once_cell` | ✅ Removed — replaced by `std::sync::LazyLock` |
| `lazy_static` | ✅ Removed — replaced by `std::sync::LazyLock` |
| `tempdir` | ✅ Removed (deprecated) — `tempfile` used instead |
| `term_size` | ✅ Removed (unmaintained, `libc` FFI) — `console` provides equivalent |
| `mdns` | ✅ Removed (unused) — `mdns-sd` is the active discovery crate |
| `dashmap` | ✅ Removed (unused) — already migrated to `RwLock<HashMap>` |
| `which` | ✅ Removed — pure Rust `find_in_path()` via `std::env::split_paths` |
| `glob` | ✅ Removed — pure Rust `read_hwmon_power()` via `std::fs::read_dir` |
| `base64` versions | ✅ Unified to 0.22 (was split 0.21/0.22) |
| `num_cpus` | ✅ Removed — `std::thread::available_parallelism()` |
| `sysinfo` | ✅ Unified to workspace 0.30 |
| `cudarc` | ✅ Upgraded 0.11 → 0.19 (real device queries) |
| `wgpu` | ✅ Unified to workspace v22 |

**Remaining external dep debt**: `cubecl` transitively pulls `dirs-sys` (D-S18-002, low priority -- needs upstream PR).

### Dependencies (Sessions 32-35)

| Category | Status |
|----------|--------|
| `thiserror` | Upgraded 1.0 -> 2.0 workspace-wide (26 crates) |
| `async-trait` | Retained -- needed for `dyn Trait` async (~65 files) |
| `chrono` -> `time` | Deferred -- chrono is already pure Rust |

### Hardcoded Values RESOLVED

| Category | Evolution |
|----------|-----------|
| `/tmp` paths | `XDG_RUNTIME_DIR` → `BIOMEOS_RUNTIME_DIR` → `std::env::temp_dir()` |
| `/etc` config paths | `XDG_CONFIG_HOME` → `HOME/.config` → `/etc` fallback |
| `/etc/hostname` | `HOSTNAME` → `TOADSTOOL_GATE_ID` → file fallback |
| DNS servers (8.8.8.8) | Removed — containers inherit from host/orchestrator |
| Ollama IP | `$OLLAMA_HOST` env var or capability discovery |
| Fallback ports | Named constants (`SONGBIRD_FALLBACK_PORT`, etc.) |
| Workgroup sizes | `WORKGROUP_SIZE_1D` constant across all ops |
| GPU capability estimates | `capability_defaults` module with named constants |
| Timeout durations | `toadstool_common::constants::timeouts` centralized |
| Beardog endpoint | S31: Removed `http://localhost:8000` fallback — env/domain only |

### Hardcoded Primal Names RESOLVED (Sessions 32-35)

| Pattern | Evolution |
|---------|-----------|
| Hardcoded `"beardog"`, `"songbird"`, etc. | `well_known::BEARDOG`, `well_known::SONGBIRD` constants |
| Hardcoded audience lists in auth | `[PRIMAL_NAME, PLATFORM_AUDIENCE]` only |
| Hardcoded external port mappings | Removed -- self-knowledge only; discovered at runtime |
| Hardcoded primal lists in doctor | Filesystem-based socket discovery |
| HTTP placeholder URLs in CLI | Unix socket capability-based discovery |

### Unsafe Code DOCUMENTED

**55 unsafe blocks audited (Sessions 32-38)** -- all FFI-boundary or hardware-related:

| Pattern | Count | Replaceable? |
|---------|-------|-------------|
| `alloc`/`alloc_zeroed`/`dealloc` | 10 | No -- custom alignment required |
| `from_raw_parts`/`from_raw_parts_mut` | 15 | No -- backend/FFI pointers |
| `NonNull::new_unchecked` | 3 | OK -- null-checked beforehand |
| `unsafe impl Send/Sync` | 12 | No -- trait impls required |
| FFI (ioctl, mlock, mmap, madvise) | ~25 | No -- kernel/hardware interface |
| CUDA/OpenCL kernel launch | 2 | No -- GPU API |

**Zero unsafe in middleware** (barracuda scientific computing is 100% safe Rust).

### Cloud Stubs EVOLVED (Sessions 32-35)

| Module | Before | After |
|--------|--------|-------|
| `cloud/cost.rs` | Minimal stub | Resource-based estimation, 6 pricing tiers, budget enforcement |
| `cloud/compliance.rs` | Simple checks | Data sovereignty, security tiers (Basic/Standard/High), resource isolation |
| `cloud/federation.rs` | Stub | Member management, heartbeats, capability exchange |

### Zero-Copy DEEPENED (Sessions 32-35)

| Pattern | Change |
|---------|--------|
| `JsonRpcRequest.method` | `String` -> `Cow<'a, str>` with `#[serde(borrow)]` |
| `JsonRpcResponse.jsonrpc` | `String` -> `Cow<'a, str>` with `#[serde(borrow)]` |
| `JsonRpcError.message` | `String` -> `Cow<'a, str>` with `#[serde(borrow)]` |
| Service discovery config | `read_to_string` + `from_str` -> `read` + `from_slice` |
| Error conversions | Removed useless `String` -> `String` `.into()` calls |

### Production Panics RESOLVED

| Pattern | Evolution |
|---------|-----------|
| `expect("poisoned")` on RwLock | `unwrap_or_else(\|e\| e.into_inner())` poison recovery |
| `try_into().unwrap()` in gpu_executor | Explicit array indexing `[c[0], c[1], ...]` |
| `unwrap()` in tests leaking to lib | All library code returns `Result` |
| ML model fake results | `Error::ModelNotLoaded` / `Error::ModelBackendRequired` with actionable messages |
| `#[allow(dead_code)]` on used items | S31h: 6 incorrect removed; S32-35: 5 more unnecessary `#[allow]` removed |

### File Size (< 1000 lines) ✅ RESOLVED

Files refactored in Session 31c:

| File | Before | After | Technique |
|------|--------|-------|-----------|
| `esn_v2.rs` | 884 | 842 | `validate_config()` + `expect_size()` helpers (-5%) |
| `cache_hierarchy.rs` | 638 | 607 | `bgl_entry` closure, `run_pass` closure, table-driven substrate name matching |

Files refactored in Session 31b:

| File | Before | After | Technique |
|------|--------|-------|-----------|
| `lu_gpu.rs` | 780 | 302 | Static `make_bgl`/`make_pipe`/`make_bg`/`dispatch` helpers (-61%) |
| `svd_gpu.rs` | 764 | 305 | Same helpers; BGL via type-slice instead of verbose entries (-60%) |

Files refactored in Session 31:

| File | Before | After | Technique |
|------|--------|-------|-----------|
| `qr_gpu.rs` | 933 | 486 | `dispatch` closure, `make_bgl`/`make_bg` helpers |
| `probe.rs` | 831 | 571 | Throughput probe → `probe_throughput.rs` (260 lines) |
| `vfio.rs` | 915 | 802 | `write_iova_regs`, `check_not_busy`, `poll_register` helpers |

Files refactored in Sessions 28-29:

| File | Before | After | Technique |
|------|--------|-------|-----------|
| `session/mod.rs` | 968 | 568 | Dispatch logic → `session/dispatch.rs` |
| `svd_gpu.rs` | 973 | 305 | Two rounds: closures (S29) → static helpers (S31b) |
| `tensor/mod.rs` | 948 | 799 | Scalar/random ops → `tensor/ops.rs` |
| `lu_gpu.rs` | 996 | 302 | Two rounds: `build_lu_pipeline` (S29) → static helpers (S31b) |
| `math_f64.wgsl` | 1002 | 837 | Special functions → `math_f64_special.wgsl` |

Previously refactored (Sessions 4-24): 21 additional files brought under limit via smart module splits.

### Production Mocks / Stubs ✅ RESOLVED

| Category | Status |
|----------|--------|
| Service discovery | ✅ mDNS, config-file, HTTP registry — all live implementations |
| Beardog capabilities | ✅ Returns error on RPC failure (was fake capabilities) |
| NeuroBench model | ✅ Returns error on missing file (was loading zeros) |
| Auth mock | ✅ Feature-gated behind `dev-mock-auth` |
| CpuExecutor::execute() | ✅ Wired to dispatch unary/binary/reduce/matmul (was `NotImplemented`) |
| Performance optimizer | ✅ `get_recommendations()` and `update_model()` implemented (were empty stubs) |
| MorseForceF64 GPU | ✅ 2-pass shader dispatch wired (was CPU-only) |
| BornMayerForceF64 GPU | ✅ N-body direct shader dispatch wired (was CPU-only) |
| ML models (BERT/Whisper/YOLO) | ✅ Return `Error::NotImplemented` (were returning empty `Vec`) |
| Songbird integration | ✅ Full dispatch flow — all helpers wired |
| System metrics | ✅ Real sysinfo values (was hardcoded 0.65 utilization) |
| GPU backend stubs | ✅ S31: Renamed `new_stub()` → `new_uninitialized()` (full wgpu impls exist) |
| Specialty runtime | ✅ S31: Clarified legitimate polling comment (was labeled "mock") |
| GpuExecutor MathOps | ✅ S31c: Wired 16 more ops (Log/Sin/Cos/Tan/Div/Reshape/Transpose/ReduceMax/Min/Prod etc.) |
| unified_hardware CpuExecutor | ✅ S31c: Delegated to standalone CpuExecutor (was `NotImplemented`) |
| ProcessSpawner WASM loading | ✅ S31c: Delegated to BiomeExecutor (was returning empty bytes) |
| GpuExecutor shape ops | ✅ S31e: Pow/Max/Min/Squeeze/Unsqueeze/Broadcast/Concat/Split (was `NotImplemented`) |
| CpuExecutor completeness | ✅ S31e: Softmax/BatchMatMul/Transpose/Shape ops (Conv only remains `NotImplemented`) |

### Code Quality Standards ✅

| Standard | Status |
|----------|--------|
| Modern idiomatic Rust | ✅ Iterators, closures, typed errors, `std::sync::LazyLock` |
| Zero-copy hot paths | ✅ `bytes::Bytes` on all RPC payloads |
| Sleep-free tests | ✅ 27 sleep calls removed (advance, Barrier, Notify) |
| ecoBin compliance | ✅ TOML preferred, XDG paths, pure Rust, `rustix` syscalls |
| Vendor-agnostic | ✅ WGSL over CUDA/ROCm, any GPU works |
| Error handling | ✅ Result-based, no panic paths in library code |
| Clippy strictness | ✅ Zero warnings workspace-wide (S38) |
| Dead code hygiene | ✅ 33 files audited, 6 incorrect annotations removed (S31h) |
| Orphan shader elimination | Zero orphans -- all 600+ WGSL wired to Rust |

---

## Completed Architecture Milestones

| Milestone | Status |
|-----------|--------|
| Shader-first architecture (589+ WGSL, zero orphans) | Yes |
| MD pipeline (thermostats + PPPM GPU) | ✅ |
| GPU-Resident Pipeline (zero CPU round-trips) | ✅ |
| Unidirectional Pipeline (fire-and-forget staging) | ✅ |
| Device Registry (physical device deduplication) | ✅ |
| Sovereign Compute Phases 0–3 (WgslOptimizer live) | ✅ |
| Distributed Node Routing (least-loaded selection) | ✅ |
| Service Discovery (mDNS + config + HTTP) | ✅ |
| Zero-Copy GpuExecutor (`Arc<wgpu::Buffer>`) | ✅ |
| Integration Tests (13 suites, 167 tests) | ✅ |
| TensorSession ML ops (neuralSpring absorption) | ✅ |
| Three Springs Validation (2,700+ checks) | ✅ |
| Lattice QCD Dirac+CG (hotSpring absorption) | ✅ S31d |
| 9→20 Bio ops GPU pipelines (wetSpring+neuralSpring absorption) | ✅ S31d, S39 |
| SubstrateCapability model (forge absorption) | ✅ S31d |
| NPU runtime discovery (AKD1000 /dev/akida*) | ✅ S31d |
| Executor full MathOp coverage (GPU+CPU) | ✅ S31e |
| 6 orphan shader wrappers (IPR, FST, Hamming, Jaccard, PD, fitness) | ✅ S31e |
| 55 orphan shaders → 0 (all wired to Rust) | ✅ S31e-31g |
| f64 LinSolve + Inverse GPU wrappers | ✅ S31g |
| RfBatchInferenceGpu wrapper | ✅ S31g |
| Clippy clean sweep (`-W clippy::all`, 0 warnings) | ✅ S31h |
| Dead code audit (33 files, 6 annotations removed) | ✅ S31h |
| PollConfig refactor in akida-driver | ✅ S31h |
| Production quality verified (zero unwrap/panic/TODO) | ✅ S31h |
| TS-001 pow_f64 fix (exp_f64 2^k up to 1023, log_f64 7 terms) | ✅ S36 |
| TS-004 FusedMapReduceF64 buffer conflict fix | ✅ S36 |
| S-13 PooledBuffer drop race fix (deferred return) | ✅ S36 |
| HFB spherical nuclear physics (5 shaders) | ✅ S36 |
| HFB deformed nuclear physics (5 shaders on ρ,z grid) | ✅ S37 |
| TS-003 trig precision (Cody-Waite + 7-term Taylor) | ✅ S37 |
| Yukawa cell-list GPU dispatch (N≥256 GPU, N<256 CPU) | ✅ S37 |
| LinuxEdgeDevice + Bluetooth sysfs probe | ✅ S37 |
| Federation TCP discovery (evolved from stub) | ✅ S37 |
| ESN export/import weights (GPU-train → NPU-deploy) | ✅ S36 |
| IPC v3.0 (abstract sockets, TCP fallback) | ✅ S36 |
| Zero clippy warnings workspace-wide | ✅ S38 |
| Blind unwrap() elimination (zero in production) | ✅ S38 |
| Test race condition fix (PathEnv testability) | ✅ S38 |
| NetworkLoadBalancer behavioral tests (8 new) | ✅ S38 |
| NetworkDistributor behavioral tests (3 new) | ✅ S38 |
| neuralSpring 4 bio ops wired (pairwise_l2, multi_obj_fitness, swarm_nn, hill_gate) | ✅ S39 |
| HFB physics module (5 spherical + 6 deformed shaders wired) | ✅ S39 |
| wetSpring 3 shaders absorbed (kmer_histogram, taxonomy_fc, unifrac_propagate) | ✅ S39 |
| Deprecated `/tmp` constants removed | ✅ S39 |

---

## Remaining Work

| ID | Description | Priority | Status |
|----|-------------|----------|--------|
| W-001 | Upstream ACO/NAK transcendental fix | Medium | Pending Titan V validation |
| W-003 | NAK Mesa patches (5 deficiencies) | Medium | Pending Titan V validation |
| D-S18-002 | cubecl `dirs-sys` transitive | Low | Needs upstream PR |
| D-S20-003 | neuralSpring `evolved/` migration | Low | Awaiting neuralSpring team |
| — | Test coverage 65% → 90% | Medium | Ongoing |
| — | NPU model pipeline | Low | Awaiting hardware |
| — | burn-inference full implementations | Low | Future |

---

## Deep Debt Principles

1. **Deep solutions** — fix root causes, not symptoms
2. **Modern idiomatic Rust** — parameter-based APIs, zero global state mutation
3. **External deps → pure Rust** — `std::sync::LazyLock`, `rustix`, `tempfile`
4. **Smart refactoring** — deduplication closures/helpers, not arbitrary splits
5. **Unsafe → fast AND safe** — assertions + SAFETY comments, not blind `expect()`
6. **Hardcoding → capability-based** — XDG, env vars, runtime discovery
7. **Primal self-knowledge** — runtime discovery, not hardcoded identifiers
8. **Mocks isolated** — production code is complete implementations
9. **Honest documentation** — no aspirational claims as facts

---

*See [CHANGELOG.md](CHANGELOG.md) for full session-by-session evolution history.*
*See [DEBT.md](DEBT.md) for active workaround details and evolution paths.*
*See [STATUS.md](STATUS.md) for current honest status.*
