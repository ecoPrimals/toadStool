# ToadStool/BarraCUDA — Next Steps

**Updated**: February 18, 2026 — Session 3
**Status**: GPU-Resident Pipeline ✅ | NAK Phase 1 ✅ | Fossil Functions ✅ | Distributed Routing ✅

---

## Active Work

### W-003: NAK Compiler — Phase 2 (f64 FMA Selection)

**Phase 1 DONE**: SM70/Volta latency tables written and wired. DFMA=8cy corrected.

**Phase 2 Next**: Verify NAK selects `DFMA` instead of `DMUL+DADD` for `a*b+c` patterns.

| Step | Action | Location |
|------|--------|----------|
| 1 | Run `bench_wgsize_nvk` on Titan V with patched Mesa NVK | hotSpring hardware |
| 2 | Measure baseline vs Phase 1 improvement | DEBT.md W-003 table |
| 3 | Dump NAK IR for Jacobi kernel (`MESA_SHADER_DUMP_PATH`) | Mesa environment |
| 4 | Check `from_nir.rs` for `OpFMul+OpFAdd` → `OpFma` fusion | `mesa-nak/nak/from_nir.rs` |
| 5 | If not fusing, add `FoldFmaPattern` pass | Phase 2 contribution |

**Expected impact**: ~1.3-1.5× additional speedup on Titan V.

---

### W-001: Upstream ACO/NAK Transcendental Fix

**Status**: Fossil functions removed (abs/sqrt/min/max/floor/ceil now native). Only exp/log still need workaround.

**Next steps**:
- **ACO (AMD)**: Contribute `fexp2(f64)` implementation to Mesa RADV/ACO for RDNA2/3
  - Track: https://gitlab.freedesktop.org/mesa/mesa
- **NAK (NVIDIA)**: Contribute `exp(f64)` lowering after Phase 1 hardware validation confirms benefit
- **Validate on Titan V + RTX 4070**: Run `bench_f64_builtins` binary to complete the capability matrix

---

## Upcoming Infrastructure

### NPU Model Pipeline
- [ ] Train → compile → deploy from Rust (VFIO backend exists)
- [ ] Integrate Akida NPU with ToadStool job queue

### burn-inference Models
- [ ] Full BERT implementation (tokenizer + inference loop)
- [ ] Whisper (audio → text)
- [ ] YOLO (object detection)

### Multi-GPU DevicePool
- [ ] Cross-device workload distribution
- [ ] f64 Tensor type with unified precision across vendors

---

## Completed (All Sessions)

### Session 3 (Feb 18, 2026) ✅
- [x] `NetworkDistributor::distribute_job` — least-loaded node selection, local fallback
- [x] `LocalCapacityManager` — real sysinfo, live capacity tracking
- [x] `ToadStoolSongbirdIntegration::submit_job` — full dispatch flow, all helpers wired
- [x] `MassiveJobDistributor` dead-code — `select_algorithm`, `plan_distribution` wired
- [x] f64 fossil functions — `abs_f64`, `sqrt_f64`, etc. → native WGSL builtins
- [x] `F64BuiltinCapabilities` probe — per-GPU matrix, crash-isolated
- [x] NAK SM70 latency tables — DFMA=8cy, FFMA=4cy, WAR/WAW per-category
- [x] `for_driver_auto` comment-aware — exp/log replacement skips comment lines
- [x] Service discovery — mDNS, config-file, HTTP registry all live
- [x] Auth self-knowledge — `env!("CARGO_PKG_NAME")`, audience from config/env
- [x] Health dashboard — WebSocket JS → /health polling
- [x] `discover_beardog_at` / `discover_nestgate_at` — wrong defaults fixed (12 tests fixed)

### Session 2 (Feb 18, 2026) ✅
- [x] Mutex poison recovery — `lock_cache` helper in `probe.rs`
- [x] GPU sampler panic — `sampler_gpu.rs` expect → `ok_or_else`
- [x] Auth audience hardcoding — `AuthManagerConfig::token_audience` + env var
- [x] Songbird stub types — `NodeCapacityTracker`, `PerformanceMetrics`, `SongbirdFeedbackSender`, `BroadcastChannel`, `MessageTypeRegistry`, `SubscriptionManager` all stateful
- [x] Unix socket health check — `probe_unix_socket` via tokio `UnixStream`
- [x] PPPM GPU physics validated — Cooley-Tukey butterfly bug fixed, 3 tests pass

### Session 1 (Feb 18, 2026) ✅
- [x] Warp-packed eigensolve (`@workgroup_size(32,1,1)`, 2.2x NVK speedup)
- [x] `GpuDriverProfile`, `EigensolveStrategy`, `bench_wgsize_nvk.rs`
- [x] `WgpuDevice::has_f64_shaders()`, SHADER_F64 feature request at creation
- [x] AMD wave32 empirical finding — RDNA2 ACO targets wave32 for compute
- [x] `bench_f64_builtins.rs` — per-GPU f64 capability survey binary
- [x] `F64BuiltinCapabilities` struct and `probe_f64_builtins()` function
- [x] WebSocket removed (tungstenite/ring C-FFI) — pure Rust JSON-RPC/tarpc
- [x] `reqwest` removed from client — Unix JSON-RPC
- [x] All files ≤ 1000 lines (smart refactor of 21 large files)

### Previous Sessions (Feb 14-17, 2026) ✅
- [x] GPU-Resident Pipeline complete (Phases 1-3)
- [x] Unidirectional Pipeline complete (Phases 0-4)
- [x] MD pipeline complete (thermostats + PPPM + observables)
- [x] cudarc 0.11 → 0.19 upgrade
- [x] Clippy -D warnings clean across workspace
- [x] Three Springs validated: 313+ Rust checks
- [x] f64 math library: 27+ transcendentals via WGSL

---

*From the ToadStool evolution desk — sovereign compute, pure Rust, any GPU.*
