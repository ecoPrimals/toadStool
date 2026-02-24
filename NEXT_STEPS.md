# ToadStool/BarraCuda -- Next Steps

**Updated**: February 23, 2026 -- Sessions 32-50
**Status**: Production-grade | Shader-first architecture | 645+ WGSL f64 shaders | 4,009+ core tests | Zero CPU-only math | Coverage: 84.33% across 5 core crates (config 89%, server 86%, common 84%, toadstool 83%, distributed 82%)

---

## Active Work

### W-004: NAK Mesa Patches (5 Deficiencies → Mesa MRs)

**hotSpring analysis (Feb 19 2026)** identified and documented the 5 NAK compiler
deficiencies responsible for the 149× NVK gap.  WGSL workarounds are live in
`batched_eigh_nak_optimized_f64.wgsl` (2–4× on NVK already).  Mesa patches are the
next step for universal sovereign GPU compute.

| Priority | Deficiency | Expected Gain | Mesa Location |
|----------|-----------|---------------|--------------|
| 1 | Loop unrolling | ~4× | `nak/opt_instr.rs` / `lower_vec.rs` |
| 2 | Register allocation | ~2× | `nak/ra.rs` |
| 3 | Instruction scheduling | ~1.5× | `nak/sched.rs` |
| 4 | FMA fusion | ~1.3× | `nak/lower_fma.rs` |
| 5 | Branch predicates | ~1.1× | `nak/opt_pred.rs` |

See `contrib/mesa-nak/NAK_DEFICIENCIES.md` for full decomposition, patch locations,
and validation strategy.

**Fix #1 alone closes ~4× of the 9× recoverable gap** — every consumer GPU running
NVK becomes a sovereign compute node without proprietary drivers.

---

### W-005: GPU-Resident VACF ✅ RESOLVED (S46)

GPU-resident VACF completed: `vacf_batch_f64.wgsl` + `GpuVelocityRing` + `VacfBatchGpu`.
Production MD is now 100% unidirectional.

---

### W-003: NAK Compiler — Titan V Hardware Validation

**Phases 0–3 COMPLETE**: source-level ILP reordering, loop unrolling, `LatencyModel`, Apple GPU arch, AND **optimizer wired into `compile_shader_f64()`** (Session 18). The Jacobi eigensolve now pre-schedules automatically on every compile.

**Pending**: Run `bench_wgsize_nvk` on Titan V to **measure** the actual speedup from the ILP pre-scheduling and confirm ≥ 3× before submitting the Mesa MR.

| Step | Action | Expected |
|------|--------|----------|
| 1 | Clone Mesa, apply `contrib/mesa-nak/sm70_instr_latencies.rs` patch | — |
| 2 | Build NVK (`-D gallium-drivers=nouveau -D build-tests=true`) | — |
| 3 | Run `bench_wgsize_nvk --n 30 --batch 512 --sweeps 200` on Titan V | ~17–23ms (from 69.8ms) |
| 4 | Compare `@ilp_region` before/after on same hardware | ≥ 3× speedup expected |
| 5 | Submit Mesa MR referencing `sm70_instr_latencies.rs` + benchmark results | — |

**Expected impact**: 3–4× improvement from Phase 1 ILP; additional 1.3–1.5× from Phase 2
FMA selection (NAK Deficiency 4 investigation required first).

---

### W-001: Upstream ACO/NAK Transcendental Fix

**Status**: Fossil functions removed (abs/sqrt/min/max/floor/ceil now native).
Only `exp_f64` / `log_f64` still need the workaround on NVK/RADV.

**Next steps**:
- **ACO (AMD)**: Contribute `fexp2(f64)` implementation to Mesa RADV/ACO for RDNA2/3
- **NAK (NVIDIA)**: Contribute `exp(f64)` lowering after Titan V hardware validation
- **Validate**: `bench_f64_builtins` on Titan V + RTX 4070 to complete capability matrix

---

### Sovereign Phase 4 — Full naga-IR Optimizer (Q3 2026)

**Next major architectural milestone**: drive naga as a library for full SSA-form
analysis and register pressure estimation.

```
WGSL text
  → naga::parse() → naga::Module (typed IR)
  → BarraCuda IR passes (reorder, unroll, software pipeline)
  → modified naga::Module
  → naga::back::spv::write() → SPIR-V bytes  (no WGSL round-trip)
  → wgpu device
```

Prerequisites:
- [ ] Understand naga IR shape for our Jacobi rotation kernel
- [ ] Prototype SSA dependency graph on a small kernel
- [ ] Register pressure estimation pass
- [ ] Inter-iteration loop pipelining (preload iteration i+1 during i's ops)

---

## Upcoming Infrastructure

### NPU Model Pipeline
- [ ] Train → compile → deploy from Rust (VFIO backend exists)
- [ ] Integrate Akida NPU with ToadStool job queue

### burn-inference Models
- [ ] Full BERT implementation (tokenizer + inference loop)
- [ ] Whisper (audio → text)
- [ ] YOLO (object detection)

### Test Coverage (target 90%)
- [ ] Current: 84.33% across 5 core crates (config 89%, server 86%, common 84%, toadstool 83%, distributed 82%) — 4,009 tests
- [ ] Gap: deep integration/network code (~16%) — async service calls, server lifecycle, protocol handlers
- [ ] Mock infrastructure exists (TCP/Unix socket mock servers) — needs expansion for full coverage
- [ ] Remaining pending test suites: `e2e`, `fhe`, `comprehensive` (require future APIs)

### Conv2D/MaxPool2D GPU Executor Wiring (neuralSpring P0)
- [ ] Refactor `ops::Conv2D` to handle batched multi-channel inputs matching `MathOp::Conv2D` params
- [ ] Wire `GpuExecutor::execute` Conv2D/MaxPool2D/AvgPool2D branches to WGSL ops (currently CPU fallback)
- [ ] Enables full LeNet-5 GPU validation for neuralSpring

### PCoA BatchedEighGpu naga Fix
- [ ] `BatchedEighGpu` fails naga shader validation (wetSpring currently uses `catch_unwind`)
- [ ] Investigate naga "invalid function call" error in eigensolve shaders

### Cross-Repo Debt (neuralSpring + wetSpring)
- [ ] **D-S20-003**: neuralSpring `evolved/` migration (~2075 lines) — API mapping table in `DEBT.md`; awaiting neuralSpring team effort
- [ ] **D-S21-003 partial**: wetSpring `GemmCached` → `GemmCachedF64` session-cached type (semantically blocked: B matrix changes per-sample in streaming_gpu.rs)
- [ ] **D-S18-002**: cubecl transitive `dirs-sys` — needs upstream PR replacing `dirs` with `etcetera`

---

## Completed (All Sessions)

### Sessions 28–29 (Feb 21, 2026) ✅ — Deep Debt Sprint: Dependencies, Refactoring, Hardcoding

- [x] 8 external deps removed: `once_cell`, `lazy_static`, `tempdir`, `term_size`, `mdns`, `dashmap`, `which`; `base64` unified to 0.22
- [x] 5 files refactored under 1000 lines: `session/mod.rs`, `svd_gpu.rs`, `tensor/mod.rs`, `lu_gpu.rs`, `math_f64.wgsl`
- [x] All `/tmp` and `/etc` hardcoded paths → XDG + env var + `std::env::temp_dir()` fallback
- [x] `RwLock` poison recovery: `expect("poisoned")` → `unwrap_or_else(|e| e.into_inner())`
- [x] Production `unwrap()` elimination: `try_into().unwrap()` → explicit array indexing in `gpu_executor.rs`
- [x] ML model honesty: BERT/Whisper/YOLO return `Error::NotImplemented` (were fake empty results)
- [x] Unsafe documentation: comprehensive SAFETY comments for `NonNull`, `Send`/`Sync` impls
- [x] GPU capability magic numbers → `capability_defaults` module with named constants
- [x] Fallback port numbers → named `const` values in `primal_discovery_complete.rs`
- [x] Pre-existing test assertion bugs fixed in `toadstool-common` error types

### Sessions 25–27 (Feb 20–21, 2026) ✅ — Coverage + Spring Absorption

- [x] 172 new unit tests across 11 modules (~65% line coverage)
- [x] hotSpring/wetSpring/neuralSpring shader absorption (16 new WGSL shaders)
- [x] CPU-based f64 Householder+QR eigensolver
- [x] NVVM Ada Lovelace f64 transcendental bug fix

### hotSpring Absorption (Feb 19, 2026) ✅ — Unidirectional Pipeline Feedback + NAK Universal Solution

- [x] `batched_eigh_nak_optimized_f64.wgsl` — NAK-optimized eigensolve shader (5 workarounds:
      manual 4× unroll, hoisted locals, load-before-compute, explicit fma(), select() branchless).
      Drop-in for `batched_eigh_single_dispatch_f64.wgsl`. 2–4× speedup on NVK.
- [x] `StatefulPipeline` — iterative simulation abstraction for MD/HFB/PDE; GPU-resident state,
      scalar-only readback, single `queue.submit()` per N iterations.  `run_until_converged()` variant.
- [x] `ReduceScalarPipeline` — first-class two-pass `sum/max/min` f64 reduction (8 bytes readback
      vs N×8 previously); `scalar_buffer()` for zero-copy pipeline chaining.  At N=10,000: 10,000× 
      reduction in energy readback bandwidth.
- [x] `atomic_cell_bin.wgsl` + `cell_list_scatter.wgsl` + `CellListGpu` — 3-pass GPU-resident
      cell-list rebuild (bin + prefix-sum + scatter); eliminates 240 KB readback + 240 KB re-upload
      every 20 MD steps at N=10,000.  Entirely GPU-resident; `sorted_indices` / `cell_start` bind
      directly to force kernel.
- [x] `contrib/mesa-nak/NAK_DEFICIENCIES.md` — formal decomposition of 5 NAK deficiencies for
      f64 loop-heavy kernels with Mesa Rust patch locations, priority table, validation strategy.

### Sessions 19–24 (Feb 20, 2026) ✅ — Debt Sprint + Test Graduation + ML ops
- [x] `TensorSession` extended: `matmul`, `relu`, `gelu`, `softmax`, `layer_norm`, `reshape`, `head_split`, `attention`, `head_concat` (all 11 neuralSpring handoff items)
- [x] `GemmCachedF64` (`ops/linalg/gemm_f64.rs`): pre-compiled pipeline + GPU-resident B matrix; `GemmF64::WGSL` as `pub const`
- [x] `capabilities.rs` → `driver_profile.rs` split (D-S17-002): 929 lines → 505 + 424; backward compat via re-exports
- [x] `ParallelFilter` two-level scan (D-S16-003): `apply_l1_offsets` WGSL pass; 4-pass/6-pass auto-select up to 16 M elements
- [x] `error_paths_discovery_tests.rs` graduated (10 tests): `self_identity` API correct, `SelfIdentity::new()` sync
- [x] `fault_tests.rs` graduated (19 tests): `chaos/fault_injection.rs` + `chaos/resilience_tests.rs`; `FaultType` fields corrected
- [x] `security_tests.rs` graduated (13 tests): `security/penetration_tests.rs`; `IsolationLevel::Enhanced`; empty-caps → `Err`
- [x] 8 stale `pending/` test copies removed
- [x] `wetSpring/barracuda/Cargo.toml` path case fixed (`toadstool` → `toadStool`)
- [x] `wetSpring gemm_cached.rs`: `include_str!` → `barracuda::ops::linalg::GemmF64::WGSL`
- [x] neuralSpring `evolved/` retirement plan documented in DEBT.md (API mapping table)
- [x] Sessions 22–23: error\_handling, resource\_requirements, security\_context, config\_management, evolution\_fault/chaos, runtime\_execution tests graduated

### Sessions 9–11 (Feb 19, 2026) ✅ — Concurrency + Zero-Copy + Coverage
- [x] `bytes::Bytes` on all binary RPC/execution payloads (7 types migrated) — O(1) clone
- [x] 27 `sleep` calls eliminated across 11 files (advance, Barrier, Notify, arithmetic, removal)
- [x] `MemoryTracker` → `tokio::time::Instant` (leak detection test uses `advance()`)
- [x] `PerformanceTestManager::benchmark()` → `tokio::time::Instant` (benchmark tests use `advance()`)
- [x] `CircuitBreaker` + `metrics_middleware` → `tokio::time::Instant`
- [x] `AsyncBatcher` queue-full test → `Barrier` + `timeout` (eliminates 5ms ordering hack)
- [x] `DistributedCoordinator` → `tokio::spawn` fan-out with `Notify` (no 50ms sleep)
- [x] Hardcoded DNS servers removed — containers inherit from host/orchestrator
- [x] `TelemetryConfig.enabled: false` default (opt-in)
- [x] `DnsConfig` derives `Default`
- [x] `pure_jsonrpc.rs` (979 lines) → `pure_jsonrpc/` module (4 focused files)
- [x] `SemanticMethodRegistry` wired into `JsonRpcHandler`
- [x] `storage_backend/mod.rs` (987 lines) → 4 files (mod, nestgate, inmemory, tests)
- [x] `DualChipEnsemble` → `rayon::join` parallel ensemble state
- [x] `UnifiedBuffer::drop()` bug fixed (both metrics fields decremented)
- [x] CLI executor inline tests: `display.rs` (6), `signals.rs` (4), `resources.rs` (5) = 15 tests
- [x] `llvm-cov` workspace SIGSEGV resolved (exit 0 consistently)
- [x] Coverage: 61.35% → **63.02%** lines, 66.47% → **68.58%** functions

### Session 18 (Feb 20, 2026) ✅ — Phase 3 Activated + Apple GPU + Zero-Copy + Integration Tests
- [x] `WgpuDevice::compile_shader_f64()` now runs `WgslOptimizer::optimize()` — Phase 3 live in the hot path
- [x] `GpuArch::AppleM` + `AppleMLatencyModel` (software f64 ~16cy) — cross-vendor arch matrix complete
- [x] `Tensor::from_arc_buffer()` + `Tensor::try_arc_buffer()` — zero-copy construction from existing Arc
- [x] `GpuTensorStorage.buffer: Arc<wgpu::Buffer>` + `from_tensor()` — GPU→CPU→GPU round-trip eliminated (D-S16-001)
- [x] `crates/integration-tests/` created, workspace `tests/*.rs` cleared (D-S16-004)
- [x] 3 integration test suites active (13 pass, 7 ignored); 12 pending tests quarantined with `README.md` tracking

### Session 8 (Feb 19, 2026) ✅ — Sovereign Phase 3 + Mesa NAK patches
- [x] `WgslDependencyGraph` — let-binding DAG parser, `classify_op()` heuristic
- [x] `IlpReorderer` — ASAP list scheduling (BinaryHeap, release_cycle), 5 tests
- [x] `WgslLoopUnroller` — `@unroll_hint N` bounded loop unrolling ≤ 32 iters, 6 tests
- [x] `WgslOptimizer` — top-level API, `for_arch()`, `Default` (Conservative), 6 tests
- [x] `ShaderTemplate::for_driver_auto()` wired — every shader passes through optimizer
- [x] `ShaderTemplate::for_driver_profile()` — hardware-accurate scheduling variant
- [x] `contrib/mesa-nak/sm70_instr_latencies.rs` — SM70–SM89 Mesa MR patch, validation harness
- [x] `contrib/mesa-nak/rdna2_instr_latencies.rs` — RDNA2/RDNA3 ACO contribution

### Session 7 (Feb 19, 2026) ✅ — Sovereign Phase 2 + migration validation + display hardening
- [x] `crates/barracuda/src/device/latency.rs` — `LatencyModel` trait + `WgslOpClass` enum
- [x] `Sm70LatencyModel` (DFMA=8cy), `Rdna2LatencyModel` (VFMA64≈4cy), `ConservativeModel`, `MeasuredModel`
- [x] `model_for_arch()` dispatch, `GpuDriverProfile::latency_model()` wired
- [x] `workload_migration/validation.rs` — `ResourceRequirements`, `PreflightOutcome`, `validate_preflight()`, `PreMigrationSnapshot` rollback pattern, 11 tests
- [x] `display/input/events.rs` — full Linux keymap (nav, F1–F12, A–Z, 0–9)
- [x] `display/input/mod.rs` — focus TODO resolved: `Arc<RwLock<>>` shared focus state; `WindowUnfocused` bug fixed

### Session 6 (Feb 19, 2026) ✅ — Security providers, load balancer, coverage run
- [x] `SoftwareHsmProvider` — AES-256-GCM + ed25519-dalek in-process key store
- [x] `LocalKeyringProvider` — D-Bus Secret Service probe + SoftwareHsm fallback
- [x] `LoadBalancer` — Equal (round-robin), Weighted, Dynamic (least-loaded), 6 tests
- [x] RISC-V 'V' extension detection in `cpu_resource.rs` and `auto_config/hardware/cpu.rs`
- [x] `llvm-cov` run: **61.35%** line coverage (non-GPU crates)
- [x] Multiple test isolation fixes (TempDir, race conditions, threshold adjustments)

### Session 5 (Feb 19, 2026) ✅ — F-001 through F-009 audit resolutions
- [x] F-001: Test compilation — universal_scheduler 5 failures fixed
- [x] F-003: Policy LRU cache touch on hit, evaluator full implementation verified
- [x] F-004: `storage.rs` hardcoded endpoint deprecated, `Default` impl added
- [x] F-005: `factory.rs` TODOs — LocalKeyringProvider + SoftwareHsmProvider wired
- [x] F-007: `compute.*` vs `toadstool.*` documented in `docs/reference/SERVER_METHODS.md`
- [x] `hosting/resources.rs` `can_allocate` bug fixed (undeclared totals treated as unlimited)

### Session 4 (Feb 18–19, 2026) ✅ — Warp-packing + deep audit
- [x] Phase 1 ILP: Jacobi kernel `@ilp_region` restructured, `cc/ss/two_cs` hoisted
- [x] `@workgroup_size(32,1,1)` warp-packing (2.2× NVK speedup on Titan V)
- [x] `GpuDriverProfile`, `EigensolveStrategy`, `bench_wgsize_nvk.rs`
- [x] All files ≤ 1000 lines (21 large files smart-refactored)
- [x] Zero clippy warnings across workspace

### Session 3 (Feb 18, 2026) ✅
- [x] `NetworkDistributor::distribute_job` — least-loaded node selection, local fallback
- [x] `LocalCapacityManager` — real sysinfo, live capacity tracking
- [x] `ToadStoolSongbirdIntegration::submit_job` — full dispatch flow, all helpers wired
- [x] f64 fossil functions — `abs_f64`, `sqrt_f64`, etc. → native WGSL builtins
- [x] NAK SM70 latency tables — DFMA=8cy, FFMA=4cy, WAR/WAW per-category
- [x] Service discovery — mDNS, config-file, HTTP registry all live
- [x] Health dashboard — WebSocket JS → /health polling
- [x] `discover_beardog_at` / `discover_nestgate_at` — wrong defaults fixed (12 tests fixed)

### Previous Sessions (Feb 14–17, 2026) ✅
- [x] GPU-Resident Pipeline complete (Phases 1–3)
- [x] Unidirectional Pipeline complete (Phases 0–4)
- [x] MD pipeline complete (thermostats + PPPM + observables)
- [x] cudarc 0.11 → 0.19 upgrade
- [x] Clippy -D warnings clean across workspace
- [x] Three Springs validated: 313+ Rust checks
- [x] f64 math library: 27+ transcendentals via WGSL

---

*From the ToadStool evolution desk — sovereign compute, pure Rust, any GPU.*
