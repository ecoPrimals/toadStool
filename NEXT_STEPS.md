# ToadStool/BarraCUDA — Next Steps

**Updated**: February 19, 2026 — Session 8
**Status**: Sovereign Phases 0–3 ✅ | LatencyModel ✅ | WgslOptimizer ✅ | Mesa NAK patches ✅

---

## Active Work

### W-003: NAK Compiler — Titan V Hardware Validation

**Phases 1–3 DONE** at source level (WGSL ILP reordering, loop unrolling, LatencyModel).
**Pending**: Run `bench_wgsize_nvk` on Titan V with the patched Mesa NVK driver to measure
the actual speedup and validate that the source-level ILP improvements eliminate scoreboard stalls.

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
  → BarraCUDA IR passes (reorder, unroll, software pipeline)
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
- [ ] Current: 61.35% line (non-GPU crates)
- [ ] Gap: async networking paths, F-003 placeholder modules now resolved
- [ ] Add test suites for security monitoring, migration coordinator, display input

---

## Completed (All Sessions)

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
