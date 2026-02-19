# Status -- February 19, 2026 (Sessions 9–11: Concurrency + Zero-Copy + Coverage)

## Quality Gates

| Gate | Status | Notes |
|------|--------|-------|
| `cargo build --workspace` | PASS | Clean build |
| `cargo fmt --all -- --check` | PASS | Clean |
| `cargo clippy --workspace --tests -- -D warnings` | PASS | **Clean (including test code)** |
| `cargo doc --workspace --no-deps` | PASS | **Clean** |
| `cargo test --workspace` | PASS | **15,700+ tests passed** |
| `cargo llvm-cov` (non-GPU) | PASS | **Exit 0 — no SIGSEGV** |
| hotSpring validation | PASS | **195/195 acceptance checks** |
| Pure Rust syscalls | PASS | **mmap/mlock via rustix** |
| biomeOS networking | PASS | **No reqwest/hyper** |
| Sleep-free tests | PASS | **27 sleep calls removed** |
| Zero-copy hot paths | PASS | **bytes::Bytes on all binary RPC payloads** |
| Hardcoded IPs/DNS | PASS | **0 remaining — capability-based** |
| Line coverage (non-GPU) | PASS | **63.02% (+1.67 pp from 61.35%)** |

*All clippy warnings resolved. Workspace fully clean. Tested with `--tests` flag.*

Excludes hardware-dependent crates: `toadstool-runtime-gpu`, `ml-inference-showcase`, `homomorphic-computing`. Examples excluded (require GPU). `crates/client` excluded (pending reqwest migration to biomeOS tower).

---

## Sessions 9–11 Evolutions (Feb 19, 2026) ✅

### Zero-Copy Binary Payloads ✅

All hot-path binary types migrated from `Vec<u8>` → `bytes::Bytes`:

| Type | Location | Impact |
|------|----------|--------|
| `WorkloadSubmission.data` | `core/toadstool` | O(1) clone across RPC boundary |
| `WorkloadResult.data` | `core/toadstool` | O(1) result propagation |
| `ExecutionInput.data` | `core/toadstool` | O(1) dispatch to runtime |
| `ExecutionOutput.data` | `core/toadstool` | O(1) result collection |
| `ExecutableSource::Bytes` | `core/toadstool` | O(1) binary payload hand-off |
| `WasmModuleSource::Bytes` | `core/toadstool` | O(1) WASM module hand-off |
| `TarpcWorkloadSubmission.payload` | `server` | O(1) tarpc transport |

**Crates updated**: `core/toadstool`, `server`, `testing`, `runtime/native`, `runtime/wasm`, `distributed`.

### Sleep Elimination (27 calls) ✅

Systematic audit of all `tokio::time::sleep` and `std::thread::sleep` calls in non-hardware code:

| File | Fix | Count |
|------|-----|-------|
| `circuit_breaker.rs` | `tokio::time::Instant`, `start_paused + advance()` | 2 |
| `metrics_middleware.rs` | `tokio::time::Instant`, `start_paused + advance()` | 1 |
| `memory/tracker.rs` | `tokio::time::Instant`, `start_paused + advance()` | 2 |
| `performance/manager.rs` | `tokio::time::Instant`, `start_paused + advance()` | 4 |
| `performance_hardening/async_ops.rs` | `tokio::sync::Barrier` + `timeout` | 1 |
| `primal_discovery_complete.rs` | `cache_ttl: Duration::ZERO` | 1 |
| `capability_provider.rs` | Removed (socket bind is synchronous) | 1 |
| `integration/helpers.rs` | Removed (no behavioral assertions) | 5 |
| `multi_device_integration.rs` | Removed (`DeviceLease::drop()` is atomic) | 3 |
| `performance/mod.rs` tests | CPU-bound fold + `yield_now()` | 4 |
| `coordinator_executor.rs` | `Notify` + `AtomicBool` fan-out | 3 |

**Total removed**: 27 sleep calls across 11 files.

### Hardcoding Eliminated ✅

- **DNS servers** (`sandbox/src/types.rs`, container configs, CLI templates): removed `8.8.8.8`/`1.1.1.1` — containers inherit from host/orchestrator
- **Ollama IP**: reads `$OLLAMA_HOST` or discovers via Songbird capability
- **`TelemetryConfig.enabled`**: changed to `false` (opt-in, was always-on)
- **`DnsConfig`**: derives `Default` (empty by default)
- **Discovery DNS** (`configurator/core.rs`): reads system resolver via `system_dns_resolvers()`

### Code Structure Improvements ✅

- **`pure_jsonrpc.rs`** (979 lines) split into `pure_jsonrpc/` module:
  - `types.rs` — request/response types, traits
  - `handler.rs` — `JsonRpcHandler` with `SemanticMethodRegistry` wired
  - `mod.rs` — public API and re-exports
  - `tests.rs` — inline integration tests
- **`SemanticMethodRegistry`** wired into `JsonRpcHandler` — semantic routes (e.g. `runtime.workload.submit`) resolve to implementation names before dispatch
- **`biomeos_integration/storage_backend/mod.rs`** (987 lines) split:
  - `mod.rs` — trait + `VolumeStatus` enum + re-exports (64 lines)
  - `nestgate.rs` — `NestGateBackend` (306 lines)
  - `inmemory.rs` — `InMemoryBackend` (210 lines)
  - `tests.rs` — shared backend test suite (68 lines)

### Bug Fix: `UnifiedBuffer::drop()` ✅

`metrics.total_allocated` was not decremented on drop — only the outer `AtomicUsize` counter was decremented. Both the `RwLock<Metrics>` field and the atomic are now updated in a single write, ensuring metric consistency. This also eliminated 6 stale `sleep()` calls in GPU memory tests that had been masking the inconsistency.

### CLI Executor Coverage ✅

15 inline `#[cfg(test)]` tests added to previously untested executor sub-modules:

| Module | Tests Added |
|--------|-------------|
| `executor/display.rs` | `get_log_path`, `show_log_file` (tempfile), `tail_log_file` (tempfile) — 6 tests |
| `executor/signals.rs` | SIGCONT-to-self, invalid signal, dead-PID (spawn+wait), kill command — 4 tests |
| `executor/resources.rs` | `biome_exists`, `get_biome_info`, `find_process_pid`, error path, concurrent reads — 5 tests |

### `llvm-cov` SIGSEGV Resolved ✅

The `toadstool-server` SIGSEGV under `cargo llvm-cov` is resolved as a side-effect of the sleep
elimination and concurrency hardening work. Workspace-wide `llvm-cov` (excluding GPU crates) now
completes with exit 0 consistently.

**Coverage progression**:
- Session 8: 61.35% lines, 66.47% functions
- Session 11: **63.02% lines (+1.67 pp)**, **68.58% functions (+2.11 pp)**

---

## cudarc 0.11 → 0.19 Upgrade (Feb 17, 2026) ✅ COMPLETE

Major dependency upgrade for the CUDA backend, resolving long-standing TODOs for proper device queries:

| API | Before (0.11) | After (0.19) |
|-----|---------------|--------------|
| Device type | `CudaDevice` | `CudaContext` (Arc-wrapped for Clone) |
| Device name | Hardcoded "NVIDIA CUDA Device" | `ctx.name()` returns actual GPU name |
| Compute capability | Hardcoded (7, 5) | `ctx.compute_capability()` returns real values |
| Memory queries | Hardcoded defaults | `ctx.attribute(CUdevice_attribute::*)` |
| Memory allocation | `device.htod_copy()` | `stream.clone_htod()` (stream-based) |
| Kernel launch | `func.launch()` | `stream.launch_builder(&func).arg(...).launch(cfg)` |
| Module loading | `device.load_ptx()` | `context.load_module(Ptx::from_src())` |

**Files Modified**:
- `crates/runtime/gpu/Cargo.toml` — cudarc version bump
- `crates/runtime/gpu/src/backends/cuda_impl.rs` — Full API migration
- `crates/runtime/gpu/src/types.rs` — `FrameworkHandle::Cuda` now uses `Arc<CudaContext>`
- `showcase/cross-platform/Cargo.toml` — cudarc version bump

**Note**: WebGPU tests may fail when run in parallel due to resource exhaustion (too many concurrent device connections). Use `--test-threads=1` if needed.

---

## Clippy Cleanup (Feb 17, 2026) ✅ COMPLETE

Applied automatic and manual clippy fixes across the workspace:

| Crate | Warnings Fixed | Type |
|-------|---------------|------|
| barracuda | 43 | Auto-fix (`div_ceil`, `is_multiple_of`, slice calc) |
| barracuda | 1 | Manual (type alias for `CellSortResult`) |
| toadstool-server | 1 | Auto-fix (map iteration) |

**New Type Alias** (`crates/barracuda/src/ops/md/forces/yukawa_celllist_f64.rs`):
```rust
pub type CellSortResult = (Vec<f64>, Vec<usize>, Vec<u32>, Vec<u32>);
```

**Result**: Workspace clippy-clean. Only intentional deprecation warnings remain (for `BEARDOG`/`NESTGATE` migration helpers).

---

## Deep Debt Evolution — Pure Rust System Calls (Feb 17, 2026) ✅ COMPLETE

### Pure Rust Syscalls (akida-driver)

**Migrated from `libc` to `rustix`** for portable, pure Rust system calls:

| Syscall | Before | After | File |
|---------|--------|-------|------|
| `mmap` | `libc::mmap` | `rustix::mm::mmap` | `mmio.rs` |
| `munmap` | `libc::munmap` | `rustix::mm::munmap` | `mmio.rs` |
| `mlock` | `libc::mlock` | `rustix::mm::mlock` | `backends/vfio.rs` |
| `munlock` | `libc::munlock` | `rustix::mm::munlock` | `backends/vfio.rs` |
| VFIO ioctls | `libc::ioctl` | **Retained** | Kernel-specific |

**Note**: `libc` retained only for VFIO `ioctl()` calls — these are Linux kernel-specific and have no `rustix` equivalent.

### biomeOS Networking Policy

**NO reqwest / hyper / ring / openssl** — all have C dependencies:

| Component | Provider | Implementation |
|-----------|----------|----------------|
| TLS | **Songbird** | Pure Rust via `rustls` |
| Crypto | **Beardog** | Pure Rust (ChaCha20-Poly1305, etc.) |
| Transport | JSON-RPC 2.0 | Unix sockets (local) / TCP (remote) |

**Documentation updated**:
- `crates/runtime/gpu/src/distributed/mod.rs` — biomeOS tower pattern
- `crates/distributed/src/songbird_integration/capability_discovery.rs` — JSON-RPC example
- `crates/client/Cargo.toml` — workspace exclusion documented

### Broadcast Error Handling

Replaced `let _ = channel.send(...)` with explicit logging:

| File | Event | Log Level |
|------|-------|-----------|
| `server/handlers.rs` | `ExecutionStarted` | debug |
| `server/background.rs` | `ResourceUsageUpdate` | trace |
| `server/background.rs` | `HealthStatusChanged` | debug |
| `server/background.rs` | `ExecutionCompleted` | debug |
| `protocols/client.rs` | `ServiceHealthChanged` | debug |

No event receivers is normal when no clients are connected — now logged rather than silently ignored.

### Placeholder Documentation

| Placeholder | Location | Documentation Added |
|-------------|----------|---------------------|
| FPGA discovery | `core/substrate/discovery.rs` | Intel OPAE / Xilinx XRT evolution path |
| GPU remote execution | `runtime/gpu/distributed/mod.rs` | biomeOS tower pattern |
| GPU kernel compiler | `runtime/gpu/compiler.rs` | AOT vs JIT rationale |
| Akida model parsing | `neuromorphic/akida-models/model.rs` | FlatBuffers schema dependency |

---

## Deep Debt Evolution — Timeout Consolidation (Feb 17, 2026) ✅ COMPLETE

**Centralized timeout constants** in `toadstool_common::constants::timeouts`:

| File | Before | After |
|------|--------|-------|
| `server/handlers.rs` | `Duration::from_secs(300)` ×7 | `WORKLOAD_EXECUTION_TIMEOUT` |
| `server/background.rs` | `Duration::from_secs(300/30)` | `DEFAULT_CACHE_TTL` / `HEALTH_CHECK_INTERVAL` |
| `server/config/mod.rs` | `Duration::from_secs(300/30)` | Centralized constants |
| `toadstool/auth.rs` | `Duration::from_secs(3600/300)` | `TOKEN_REFRESH_INTERVAL` / `TIMESTAMP_VALIDATION_WINDOW` |
| `cli/monitoring.rs` | `Duration::from_secs(30)` | `HEALTH_CHECK_INTERVAL` |

**New auth constants added:**
- `TOKEN_REFRESH_INTERVAL` — 1 hour token refresh
- `TIMESTAMP_VALIDATION_WINDOW` — 5 minute replay protection window

---

## Deep Debt Evolution — SIMD Runtime Detection (Feb 17, 2026) ✅ COMPLETE

**Evolved compile-time `cfg!()` to runtime detection** in `unified_hardware.rs`:

| Architecture | Before | After |
|--------------|--------|-------|
| x86_64 | `cfg!(target_feature = "avx512f")` | `std::arch::is_x86_feature_detected!("avx512f")` |
| x86_64 | `cfg!(target_feature = "avx2")` | `std::arch::is_x86_feature_detected!("avx2")` |
| x86_64 | `cfg!(target_feature = "sse4.1")` | `std::arch::is_x86_feature_detected!("sse4.1")` |
| aarch64 | Compile-time assumption | Fixed NEON width (always 128-bit on aarch64) |

**Benefit**: Runtime detection accurately reflects actual CPU capabilities vs compile-time assumptions that depend on build flags.

---

## Critical Bug Fix: log_f64 (Feb 16, 2026 — wetSpring Discovery) ✅ FIXED

**Problem**: `log_f64()` in `math_f64.wgsl` produced ~1e-3 precision instead of ~1e-15.

**Root Cause**: The atanh series coefficients were doubled (`2/3, 2/5, 2/7...`) but the
formula `2 * s * (1 + s² * p)` already multiplies by 2. Result: polynomial terms were 2× too large.

**Discovery**: wetSpring life science validation — Shannon entropy `p * log(p)` on GPU
differed from CPU baseline by ~1e-3 instead of expected ~1e-10.

**Fix Applied**:
```wgsl
// BEFORE (wrong)
let c1 = f64_const(x, 0.6666666666666735130);  // 2/3

// AFTER (correct)
let zero = x - x;
let c1 = zero + 0.3333333333333367565;   // ≈ 1/3 (minimax)
```

**Additional Findings**:

| Finding | Description |
|---------|-------------|
| `zero + literal` pattern | `f64(0.333...)` truncates through f32; use `(x - x) + 0.333...` for full f64 |
| Native f64 builtins | `log(f64)`, `exp(f64)` REJECTED by NVVM — must use software implementations |
| Validated | Shannon entropy: `counts=[10,20,30,40]` → `1.27985422...` (GPU-CPU error ≤ 1e-10) |

**Impact**: All f64 GPU computations using `log_f64()` now achieve full precision.
This affects: Shannon entropy, statistical distributions, optimization algorithms.

---

## Bug Fixes from hotSpring (Feb 16, 2026) ✅ FIXED

### Bug 1: WGSL Reserved Keyword in BCS Bisection

**File**: `crates/barracuda/src/shaders/optimizer/batched_bisection_f64.wgsl`

**Problem**: `target` is a WGSL reserved keyword. naga rejects shader at compile time.

**Fix Applied**: Renamed `target` → `target_val` in `polynomial_test()` function.

### Bug 2: WgpuDevice Not Requesting SHADER_F64

**File**: `crates/barracuda/src/device/wgpu_device.rs`

**Problem**: `from_adapter_index()` created device with `Features::empty()` even when
adapter supports f64. All f64 WGSL shaders fail with "FLOAT64 flag" error.

**Fix Applied**: Inspect `adapter.features()` and request SHADER_F64/F16/TIMESTAMP_QUERY
when available. Now all device creation paths properly enable f64.

**Impact**: 
- BCS bisection GPU calls now work (previously all failed)
- All `WgpuDevice` creation paths now enable f64 when hardware supports it
- hotSpring validated: 195/195 acceptance checks now pass

---

## wetSpring Bray-Curtis Shader Absorbed (Feb 16, 2026) ✅ COMPLETE

The `bray_curtis_pairs_f64.wgsl` shader was missing from ToadStool and has been absorbed:

| Component | Location |
|-----------|----------|
| Shader | `shaders/math/bray_curtis_f64.wgsl` |
| Orchestrator | `ops::bray_curtis_f64::BrayCurtisF64` |
| Tests | 5 unit tests |

**API**: `BrayCurtisF64::condensed_distance_matrix(samples, n_samples, n_features)`

wetSpring can now wire this for GPU-accelerated diversity analysis.

---

## hotSpring v0.5.5 Quality Handoff (Feb 16, 2026 evening) ✅ ACKNOWLEDGED

hotSpring completed code quality hardening (no new physics, cleaner infrastructure):

| Metric | Before | After |
|--------|:------:|:-----:|
| Unit tests | 158 | **182** |
| Line coverage | 33% | **39%** |
| Inline magic numbers | 30+ | **0** |
| WGSL shaders extracted | 0 | **8** |

**Primitives Ready for hotSpring**:
| Primitive | Module | Status |
|-----------|--------|:------:|
| `SumReduceF64` | `barracuda::ops::sum_reduce_f64` | ✅ Ready |
| `SpinOrbitGpu` | `barracuda::ops::grid::spin_orbit_f64` | ✅ Ready |
| `FusedMapReduceF64` | `barracuda::ops::fused_map_reduce_f64` | ✅ Fixed (TS-004) |

hotSpring can now wire `SumReduceF64::sum()` to replace CPU `trapz` in HFB energy pipeline.

---

## airSpring ToadStool Issues Resolution (Feb 16, 2026) ✅ ALL RESOLVED

The airSpring team identified 4 ToadStool issues during their Phase 3 GPU integration.
All have been resolved:

| ID | Severity | Issue | Resolution |
|----|:--------:|-------|------------|
| TS-001 | **Critical** | `pow_f64` returns 0.0 for fractional exponents | Implemented `exp(exp * log(base))` for non-integer exponents |
| TS-002 | **Medium** | No Rust orchestrator for `batched_elementwise_f64` | Created `BatchedElementwiseF64` with FAO-56 and water balance support |
| TS-003 | **Medium** | `acos`/`sin` precision drift in f64 shaders | Extended Taylor series + new `asin_core` Padé approximation |
| TS-004 | **High** | `FusedMapReduceF64` buffer conflict for N>=1024 | Use separate input/output buffers in `reduce_partials_pass` |

**Files Modified**:
- `crates/barracuda/src/shaders/science/batched_elementwise_f64.wgsl` (TS-001, TS-003)
- `crates/barracuda/src/ops/batched_elementwise_f64.rs` (TS-002 — NEW)
- `crates/barracuda/src/ops/fused_map_reduce_f64.rs` (TS-004)
- `crates/barracuda/src/ops/mod.rs` (TS-002 — module registration)

**Validation**:
- `cargo clippy --workspace -- -D warnings` → **PASS** (0 warnings)
- `cargo test -p barracuda --lib batched_elementwise` → **3/3 PASS** (1 ignored, requires GPU)
- `cargo test -p barracuda --lib fused_map_reduce` → **2/2 PASS** (1 ignored, requires GPU)

airSpring can now proceed with GPU acceleration of FAO-56 ET₀ and water balance pipelines.

---

## Sibling Validation Projects (Feb 16, 2026)

Three domain-specific projects validate BarraCUDA's compute stack:

| Project | Domain | Rust Checks | Key Achievement |
|---------|--------|:-----------:|-----------------|
| **hotSpring** | Nuclear physics (HFB + MD) | 195/195 | GPU-resident HFB 15% faster than CPU |
| **wetSpring** | Life science + analytical chemistry | 48/48 | Shannon/Simpson/Bray-Curtis on GPU |
| **airSpring** | Precision agriculture (ET₀, soil, IoT) | 70/70 | FAO-56 validated; 918 real station-days |

**Combined**: 313+ Rust acceptance checks across physics, chemistry, biology, and agriculture.

**Cross-spring benefits**:
- All three share `serde`, `rayon`, f64 patterns
- hotSpring GPU patterns (batching, dispatch) inform airSpring and wetSpring
- airSpring spatial interpolation (kriging) can serve wetSpring sampling sites
- wetSpring IoT stream processing can serve airSpring real-time sensors

---

## Unified Math Library Evolution (Feb 16, 2026) ✅ COMPLETE

New cross-spring primitives absorbed into BarraCUDA:

### New Shaders

| Shader | Purpose | Springs Served |
|--------|---------|----------------|
| `fused_map_reduce_f64.wgsl` | Single-dispatch map+reduce | wetSpring (Shannon, Simpson) |
| `cosine_similarity_f64.wgsl` | All-pairs f64 similarity | wetSpring (MS2 matching) |
| `batched_elementwise_f64.wgsl` | Batched ET₀, water balance | airSpring |
| `kriging_f64.wgsl` | Spatial interpolation | airSpring + wetSpring |

### New Rust Orchestrators

| Module | Purpose | Features |
|--------|---------|----------|
| `FusedMapReduceF64` | Map+reduce primitive | Smart CPU/GPU routing, Shannon, Simpson, norms |
| `KrigingF64` | Spatial interpolation | 4 variogram models, variance estimation, simple kriging |

### Math f64 Precision Fixes

Applied `(zero + literal)` pattern for full f64 precision:
- `exp_f64()` - Full precision exponential
- `sin_f64()`, `cos_f64()` - Full precision Taylor series
- `sinh_f64()`, `cosh_f64()` - Via corrected exp_f64
- `erf_f64()` - Abramowitz & Stegun coefficients
- `gamma_f64()`, `lanczos_core_f64()` - Lanczos coefficients
- `bessel_j0_f64()` - Polynomial coefficients

### Comprehensive Test Suite: `three_springs_evolution_tests.rs`

| Category | Tests | Coverage |
|----------|:-----:|----------|
| Unit (Fused Map-Reduce) | 9 | Shannon, Simpson, sum, max, min |
| Unit (Kriging) | 7 | Variograms, interpolation, fitting |
| E2E | 3 | Biodiversity, soil mapping, combined |
| Chaos | 8 | Edge cases, stress, memory leak check |
| Fault | 8 | Error handling, invalid inputs |
| Precision | 3 | f64 accuracy, Kahan summation |
| **Total** | **37** | All passing (1 GPU-path ignored) |

---

## Deep Debt Evolution (Feb 16, 2026) ✅ COMPLETE

### ecoBin v2.0 Platform-Agnostic Compliance

| # | Item | Status | Description |
|:-:|------|:------:|-------------|
| 1 | Platform Paths Module | ✅ **DONE** | `platform_paths.rs` - XDG/temp_dir resolution |
| 2 | No Hardcoded `/run/user/` | ✅ **DONE** | Replaced with `XDG_RUNTIME_DIR` or temp_dir fallback |
| 3 | No Hardcoded `/tmp/` | ✅ **DONE** | Replaced with `std::env::temp_dir()` |
| 4 | IPC Path Evolution | ✅ **DONE** | client.rs, server.rs, unix.rs updated |
| 5 | Launcher Evolution | ✅ **DONE** | TCP/socket discovery uses temp_dir |
| 6 | Sandbox Evolution | ✅ **DONE** | SandboxConfig uses XDG_DATA_HOME |

**New Module**: `toadstool_common::platform_paths` provides:
- `runtime_dir()` - XDG_RUNTIME_DIR with temp_dir fallback
- `temp_dir()` - std::env::temp_dir() with TMPDIR override
- `toadstool_socket_dir()` - biomeOS standard paths
- Platform detection (Linux, macOS, Windows, Android, WASM)

### Semantic Method Naming (wateringHole Standard)

| # | Item | Status | Description |
|:-:|------|:------:|-------------|
| 1 | display.resizeWindow | ✅ **FIXED** | → `display.resize_window` |
| 2 | display.subscribeInput | ✅ **FIXED** | → `display.subscribe_input` |
| 3 | display.pollEvents | ✅ **FIXED** | → `display.poll_events` |
| 4 | display.inputEvent | ✅ **FIXED** | → `display.input_event` |

All display IPC methods now follow `domain.operation` snake_case standard.

### Unsafe Code Evolution

| # | Item | Status | Description |
|:-:|------|:------:|-------------|
| 1 | isolated_memory.rs wipe | ✅ **DONE** | slice.fill(0) instead of write_bytes |
| 2 | isolated_memory.rs Drop | ✅ **DONE** | Calls wipe() - no duplicate unsafe |
| 3 | cpu.rs zeroing | ✅ **DONE** | slice.fill(0) instead of write_bytes |
| 4 | SAFETY comments | ✅ **VERIFIED** | All unsafe blocks documented |

**Result**: Reduced unsafe surface while maintaining performance.

### NPU Executor Implementation

| # | Item | Status | Description |
|:-:|------|:------:|-------------|
| 1 | NpuExecutor | ✅ **DONE** | Implements ComputeExecutor trait |
| 2 | unified_hardware integration | ✅ **DONE** | discover_npus() returns NpuExecutor |
| 3 | Akida bridge | ✅ **DONE** | Wraps AkidaExecutor for scheduler |
| 4 | Capability detection | ✅ **DONE** | NPU-specific capabilities |

**New File**: `barracuda/src/npu_executor.rs` - bridges Akida to unified hardware.

### Pure Rust Dependency Evolution

| # | Item | Status | Description |
|:-:|------|:------:|-------------|
| 1 | CLI libc → rustix | ✅ **DONE** | SIGTERM signal uses rustix (ecoBin compliant) |
| 2 | Forbidden crypto audit | ✅ **PASS** | No openssl-sys, ring, aws-lc-sys |
| 3 | dirs-sys analysis | 📋 **TRACKED** | Via burn dependency - upstream fix recommended |
| 4 | unsafe-libyaml mitigation | ✅ **DONE** | TOML support added (preferred format) |
| 5 | akida-driver libc | 📋 **TRACKED** | VFIO ioctls complex - requires hardware testing |

**Quick wins applied**:
- CLI tests use rustix instead of libc for signal handling
- `load_biome_manifest()` now supports TOML (preferred) and YAML (legacy)
- `SecurityPolicyManager` loads/saves TOML (preferred) with YAML fallback
- `manifest_to_toml()` function added for template rendering

**Future work**: Upstream dirs-sys fix in Burn, evolve akida-driver VFIO to rustix.

### Large Files Analysis (Feb 16, 2026)

| File | Lines | Status | Notes |
|------|-------|--------|-------|
| batched_eigh_gpu.rs | 2054 | ✅ **OK** | Complex GPU kernel, shaders extracted |
| cg_gpu.rs | 2011 | ✅ **OK** | CG solver, well-structured |
| byob_impl.rs | 1653 | ✅ **OK** | Below 2000 threshold |

**Finding**: Large files are justified by algorithmic complexity. Shaders already extracted
to `.wgsl` files. Arbitrary splitting would reduce cohesion without improving maintainability.

### Pipeline Cache Status (Feb 16, 2026)

| # | Item | Status | Notes |
|:-:|------|:------:|-------|
| 1 | Per-device isolation | ✅ **DONE** | DeviceFingerprint in cache keys |
| 2 | Shader caching | ✅ **DONE** | GLOBAL_CACHE with dashmap |
| 3 | Bind group caching | ✅ **DONE** | 100% hit rate verified |
| 4 | Warmup system | ✅ **DONE** | Pre-compiles common ops |

**GPU-resident pipeline** (multi-kernel with zero CPU round-trips) tracked separately in
`BARRACUDA_PARITY_ROADMAP.md` - requires hotSpring integration work.

### Production Mocks Audit

| Category | Count | Status |
|----------|-------|--------|
| Mock* in production | 0 | ✅ All test-only |
| Stub implementations | 2 | TPU/FPGA (future work) |
| Fake patterns | 0 | ✅ Clean |

All mocks properly isolated to `#[cfg(test)]` modules.

### Capability-Based Discovery Evolution (Feb 16, 2026)

Evolved hardcoded primal names to capability-based discovery.

| # | File | Change | Description |
|:-:|------|--------|-------------|
| 1 | beardog_integration/client.rs | `new_async()` | Capability-based crypto discovery |
| 2 | crypto_integration/client.rs | Endpoint metadata | Uses actual endpoint, not hardcoded |
| 3 | ecosystem/communication.rs | `extract_socket_path()` | Helper for capability-based paths |
| 4 | beardog/discovery.rs | Generic fallback | `crypto.sock` instead of `beardog.sock` |
| 5 | auth_backend.rs | `new_async()` | `discover_crypto_socket()` |
| 6 | storage_backend.rs | `new_async()` | `discover_storage_socket()` |
| 7 | agent_backend.rs | `new_async()` | Custom ML capability |
| 8 | agents.rs | `with_ml_service()` | Async manager constructor |
| 9 | auth.rs | `with_crypto_service()` | Async manager constructor |
| 10 | storage.rs | `with_storage_service()` | Async manager constructor |

**Pattern**: All sync constructors deprecated with `#[deprecated]`. New async versions
use `toadstool_common::primal_sockets::discover_*_socket()`.

**Principle**: Self-knowledge only — each primal knows itself and discovers others at runtime.

### Health Check & Runtime Capabilities (Feb 16, 2026 — Continued)

| # | File | Change | Description |
|:-:|------|--------|-------------|
| 1 | beardog_integration/client.rs | `health_check()` | Actually probes endpoints via RPC |
| 2 | beardog_integration/client.rs | `query_capabilities_async()` | Runtime capability discovery |

**Before**: `health_check()` just returned discovered endpoints without probing.
**After**: `health_check()` calls `beardog.health` RPC and updates `healthy`/`latency_ms`.

**Before**: `capabilities()` returned hardcoded defaults (trait lifetime constraint).
**After**: `query_capabilities_async()` queries `beardog.capabilities` RPC at runtime.

### AlignedBuffer RAII Evolution (Feb 16, 2026)

Evolved `unified_memory/backends/cpu.rs` unsafe code:

| # | Item | Description |
|:-:|------|-------------|
| 1 | `AlignedBuffer` struct | RAII wrapper for aligned memory |
| 2 | `NonNull<u8>` | Compile-time null safety |
| 3 | `Drop` impl | Automatic cleanup via dealloc |
| 4 | `from_raw`/`into_raw` | Safe ownership transfer |

**Result**: Encapsulated unsafe operations in single audited location with automatic cleanup.

---

## Test Coverage

| Metric | Value | Change |
|--------|-------|--------|
| Line coverage (non-GPU, workspace) | **63.02%** | +1.67 pp from Session 8 |
| Function coverage (non-GPU, workspace) | **68.58%** | +2.11 pp from Session 8 |
| `toadstool-server` | ~85% | — |
| `toadstool-common` | ~84% | — |
| `toadstool-config` | ~85% | — |

Coverage tool: `cargo-llvm-cov`. Target: 90%.

**Highest coverage**: `state.rs` 100%, `graph_types.rs` 99%, `semantic_methods.rs` 99%, `self_identity.rs` 98%, `handlers.rs` 96%, `performance_hardening.rs` 96%, `cross_gate.rs` 95%, `layer_adaptation.rs` 94%.

**Lowest coverage** (inherently limited):
- `unibin.rs`: ~35% (server startup requires running server)
- `websocket.rs`: ~52% (requires live WebSocket connections)
- GPU execution paths: 0% (hardware required)

**Sessions 9–11 additions**: 15 new inline tests in `executor/display.rs`, `executor/signals.rs`, `executor/resources.rs`; concurrency hardening exposed and fixed several previously untested error paths.

**Coverage evolution**: 61.35% (Session 8) → **63.02%** (Session 11) via 15 new CLI executor tests + concurrency hardening surfacing new paths.

---

## Device Registry + F64 Reduce Suite (Feb 16, 2026) ✅ COMPLETE

### Physical Device Deduplication

**Problem solved**: Same GPU appearing multiple times via different backends (Vulkan, OpenCL).

| # | Item | Status | Description |
|:-:|------|:------:|-------------|
| 1 | DeviceRegistry | ✅ **DONE** | Physical device tracking with backend preference |
| 2 | Backend Preference | ✅ **DONE** | Vulkan > Metal > DX12 > GL (ecoPrimals uses Vulkan) |
| 3 | Name-based Dedup | ✅ **DONE** | Handles OpenGL device_id=0 quirk |
| 4 | Hardware Report | ✅ **DONE** | Deduplicated counts + raw adapter counts |

**Example**: RTX 3090 via Vulkan+GL now shows as **1 device** with **2 backends**, not 2 devices.

### F64 Reduce Operations Suite

| # | Item | Status | Description |
|:-:|------|:------:|-------------|
| 1 | ProdReduceF64 | ✅ **DONE** | GPU product reduction with log-domain variant |
| 2 | VarianceReduceF64 | ✅ **DONE** | Welford's algorithm for parallel variance |
| 3 | NormReduceF64 | ✅ **DONE** | L1, L2, Linf, Frobenius, p-norm |
| 4 | CumprodF64 | ✅ **DONE** | Cumulative product (inclusive/exclusive/reverse) |

**18 new tests** for f64 reduce operations (all passing).

---

## F64 Unified Math Language Suite (Feb 15, 2026) ✅ COMPLETE

### WGSL as Unified Math — Science-Grade Precision on Any GPU

| # | Item | Status | Description |
|:-:|------|:------:|-------------|
| 1 | CholeskyF64 | ✅ **DONE** | f64 Cholesky decomposition for SPD matrices |
| 2 | TriangularSolveF64 | ✅ **DONE** | Forward/backward solve + Cholesky pipeline |
| 3 | CyclicReductionF64 | ✅ **DONE** | O(log n) parallel tridiagonal solver |
| 4 | LennardJonesF64 | ✅ **DONE** | f64 van der Waals forces with Rust API |
| 5 | CoulombF64 | ✅ **DONE** | f64 electrostatics + Ewald real-space |
| 6 | MorseF64 | ✅ **DONE** | f64 bonded anharmonic interactions |

### Architecture: "Unified Math Language"

- **WGSL shaders** as the primary math implementation
- **f64 by default** — SPIR-V/Vulkan bypasses CUDA fp64 throttle
- **Native f64 builtins** — `sqrt()`, `exp()`, `log()` work at full speed
- **Any GPU hardware** — same shader runs NVIDIA, AMD, Intel

---

## ResourceQuota + MultiDevicePool (Feb 15, 2026) ✅ COMPLETE

### Multi-GPU with VRAM Budget Enforcement

| # | Item | Status | Description |
|:-:|------|:------:|-------------|
| 1 | ResourceQuota | ✅ **DONE** | Per-task VRAM budget with atomic tracking |
| 2 | QuotaTracker | ✅ **DONE** | Real-time usage monitoring and enforcement |
| 3 | MultiDevicePool | ✅ **DONE** | Heterogeneous GPU support (NVIDIA + AMD) |
| 4 | DeviceRequirements | ✅ **DONE** | Device selection by VRAM, vendor, capability |
| 5 | DeviceLease RAII | ✅ **DONE** | Automatic device release on drop |
| 6 | Vendor Detection Fix | ✅ **DONE** | NVIDIA OpenGL adapters correctly identified |
| 7 | Integration Tests | ✅ **DONE** | 13/13 tests pass (RTX 3090 + RX 6950 XT) |

### Test Environment

- **Hardware**: NVIDIA RTX 3090 (OpenGL) + AMD RX 6950 XT (Vulkan)
- **Tests**: `cargo test -p barracuda --test multi_device_integration`
- **Coverage**: Vendor preference, sequential/concurrent acquisition, quota enforcement, stress test

---

## GPU-Resident Pipeline (Feb 15, 2026) ✅ COMPLETE

### Implementation Complete

All components of the GPU-resident physics pipeline have been implemented:

| # | Item | Status | Description |
|:-:|------|:------:|-------------|
| 1 | Max Abs Diff Reduction | ✅ **DONE** | `max|a[i] - b[i]|` for SCF convergence |
| 2 | Persistent Buffer Management | ✅ **DONE** | `pin_solver_buffers()` / `release_solver_buffers()` |
| 3 | Batched Bisection | ✅ **DONE** | `BatchedBisectionGpu::solve_polynomial()`, `solve_bcs()` |
| 4 | Grid Quadrature GEMM | ✅ **DONE** | `H[b,i,j] = Σ_k φ[b,i,k] * W[b,k] * φ[b,j,k] * weights[k]` |
| 5 | Multi-Kernel Pipeline | ✅ **DONE** | `PipelineBuilder` with buffer chaining |
| **6** | **GPU-Resident Eigensolve** | ✅ **DONE** | **`BatchedEighGpu::execute_f64_buffers()` — hotSpring 4.1** |

### Item 6: GPU-Resident Eigensolve (Critical Path)

The original `BatchedEighGpu::execute_f64()` required CPU readback. This blocked
GPU-resident SCF because eigenvalues/eigenvectors round-tripped through CPU.

**New buffer-based API** enables zero-copy eigensolve:

```rust
// Create persistent buffers (once at solver start)
let (h_buf, eig_buf, vec_buf) = BatchedEighGpu::create_buffers(&device, n, batch)?;

for iteration in 0..max_iter {
    // H-build → eigensolve → BCS (all GPU→GPU)
    hamiltonian_kernel.execute_to_buffer(&h_buf)?;
    BatchedEighGpu::execute_f64_buffers(&device, &h_buf, &eig_buf, &vec_buf, n, batch, 30)?;
    bcs_kernel.execute_from_buffers(&eig_buf, &occupations_buf)?;
    
    // Minimal readback for convergence only
    let eigenvalues = BatchedEighGpu::read_eigenvalues(&device, &eig_buf, n, batch)?;
    if converged(&eigenvalues) { break; }
}
```

### Key Metrics Achieved

| Metric | Before | After |
|--------|:------:|:-----:|
| CPU↔GPU round-trips/iteration | ~10 | 1 |
| Buffer allocs/iteration | ~20 | 0 |
| Convergence check location | CPU | GPU |
| Hamiltonian construction | CPU | GPU |
| BCS root-finding | CPU | GPU |
| **Eigensolve readback** | **Required** | **Optional** |

### Background: hotSpring Experiment 005 Findings

The Amdahl's Law boundary identified in planning has been addressed:

- **Complexity boundary**: n<30 CPU wins, n>50 GPU wins
- **For n<30**: GPU wins ONLY with zero CPU↔GPU round-trips — **now achieved**
- **hotSpring item 4.1**: Dependent op chaining now possible through full SCF iteration

See: `docs/planning/GPU_RESIDENT_PIPELINE_FEB16_2026.md` and `NEXT_STEPS.md` for API usage

---

## Deep Debt Evolution (Feb 15, 2026 — Continued)

### Async-Safe Buffer Readback ✅

**Problem**: `AsyncReadback::read_*()` methods called `device.poll(Maintain::Wait)` BEFORE awaiting, blocking the async executor.

**Solution**:
- Added `poll_until_ready()` helper with cooperative yield points
- Uses `futures::FutureExt::now_or_never()` for non-blocking checks
- `tokio::task::yield_now()` between polls to avoid executor starvation
- Added explicit `read_*_blocking()` methods for synchronous contexts

### Cylindrical Grid Operators ✅

**Problem**: `CylindricalGradient` and `CylindricalLaplacian` had stubbed `compute()` methods.

**Solution**:
- Implemented `CylindricalGradient::compute()` returning `(grad_rho, grad_z)`
- Implemented `CylindricalLaplacian::compute()` with proper ∇²f = ∂²f/∂ρ² + (1/ρ)∂f/∂ρ + ∂²f/∂z²
- Tests validate against analytical derivatives

### Sobol Sequence Bug Fix ✅

**Problem**: `skip_to(n)` used incorrect Gray code formula causing wrong state computation.

**Solution**:
- Changed to sequential generation internally (O(n) but correct)
- Test removed from `#[ignore]` — all 14 Sobol tests now pass

### Documentation Cleanup ✅

- Fixed 7 rustdoc warnings for unclosed HTML tags (`Vec<f64>` escaping)
- `cargo doc` builds warning-free

---

## Deep Debt Evolution (Feb 15, 2026)

### hotSpring Math Primitives Absorption ✅

**Physics-agnostic GPU primitives** absorbed from hotSpring's nuclear EOS study:

- **f64 Special Functions**: `hermite_f64.wgsl`, `laguerre_f64.wgsl` with normalized variants
- **Broyden Mixing Module** (`ops/mixing/`): LinearMixer, BroydenMixer for SCF solvers
- **Finite-Difference Gradients** (`ops/grid/`): 1D/2D/cylindrical gradients, Laplacian
- **Weighted Inner Product**: `weighted_dot_f64.wgsl` with workgroup tree reduction
- **Science-Grade Buffer Limits**: `WgpuDevice::new()` defaults to 512 MiB / 1 GiB

**47 new tests** for evolution primitives:
- Unit tests: LinearMixer (5 α variants), BroydenMixer, Gradient1D (4 functions), 2D/cylindrical
- E2E tests: SCF convergence simulation, gradient-mixing pipeline
- Chaos tests: large/small values, alternating signs, pseudorandom, spikes
- Fault tests: dimension mismatch, NaN/infinity propagation, empty input
- Special functions: Hermite H_n(x), Laguerre L_n^α(x) CPU reference

All primitives validated by hotSpring's **169/169 nuclear EOS acceptance checks** on consumer GPU (RTX 4070, f64).

### Code Quality Hardening ✅

**Error Handling Evolution** — systematic elimination of panic paths:

- **unwrap/expect cleanup**: 50+ unwrap() calls in barracuda converted to proper error propagation
  - `receiver.recv().unwrap()` → `recv().map_err(|_| BarracudaError::execution_failed(...))?`
  - `chunk.try_into().unwrap()` → `expect("chunks_exact(N) yields N-byte chunks")` with SAFETY comments
  - Mutex/RwLock poisoning: `lock().unwrap()` → `lock().expect("mutex poisoned")`
- **panic!() to unreachable!()**: Internal invariant violations now use `unreachable!()` with clear messages
- Files updated: `cg_gpu.rs`, `bicgstab_gpu.rs`, `gpu_helpers.rs`, `svd_gpu.rs`, `qr_gpu.rs`, `lu_gpu.rs`, `batched_eigh_gpu.rs`, `vfio.rs`, `async_submit.rs`, `autotune.rs`, `tensor_context.rs`, + 15 more

### Large File Refactoring ✅

**Smart refactoring** — domain separation, not just splitting:

- **cg_gpu.rs**: 2556 → 2011 lines (-21%) by migrating buffer/BGL helpers to shared `gpu_helpers.rs`
- **gpu_helpers.rs**: Extended with `*_raw()` variants for device/queue overloads
- Reduced code duplication across all sparse linear algebra GPU solvers

### Clippy Compliance ✅

**Zero warnings** with `-D warnings`:

- Fixed `unnecessary_map_or` → `is_none_or` pattern
- Fixed `manual_range_contains` → `(0.0..1.0).contains(&x)` pattern
- Fixed format strings: `format!("{}", x)` → `format!("{x}")`
- Fixed identity operations: `1 * value` → `value`
- All 87+ clippy errors from previous session resolved

---

## Deep Debt Evolution (Feb 14, 2026 — Evening)

### Server Placeholder Evolution ✅

**Real system metrics** — no more hardcoded values in server monitoring:

- `SystemResources` extended with `cpu_usage_percent`, `memory_usage_percent`, `total_cpu_cores`, `total_memory_bytes`
- `resource_monitoring_task` now reports actual CPU/memory from sysinfo
- `perform_health_check` uses real values for threshold checks
- All mocks updated to include new fields

### GPU Self-Knowledge ✅

**Real GPU detection** — `query_gpu_devices()` now discovers actual hardware:

- **Linux NVIDIA**: Reads `/proc/driver/nvidia/gpus`, queries `nvidia-smi` for memory
- **Linux AMD/Intel**: Scans `/sys/class/drm` for vendor IDs (`0x1002`, `0x8086`)
- **macOS**: Parses `system_profiler SPDisplaysDataType -json`
- Logs detected GPUs at server startup

### Scheduler Primal Integration ✅

**Real primal routing** — scheduler now uses `primal_registry` for execution:

- `execute_executable()` returns proper `Failed` status with exit code 127 when no engine available
- `execute_wasm()` returns `Failed` with exit code 126 when no WASM engine
- `execute_primal()` routes via `primal_registry.route_request()` with proper `PrimalContext`
- `execute_biome_os()` looks up BiomeOS provider and routes or returns descriptive error

### Burn-Inference Placeholders ✅

**Explicit not-implemented errors** — clear guidance instead of dummy data:

- Added `Error::NotImplemented` variant
- `InferenceEngine::infer()` returns explicit error guiding to model-specific APIs
- Full model implementations deferred (requires ML architecture work)

---

## New Features (Feb 14, 2026)

### FP64-by-Default GPU Architecture ✅

**Design Philosophy**: Both CPU and GPU use **f64 by default**.

The WGSL/SPIR-V/Vulkan path bypasses CUDA's artificial fp64 throttle, achieving **1:2-3 FP64:FP32** performance (not 1:32 like CUDA consumer GPUs advertise).

**New f64 WGSL shaders**:
- `lu_decomp_f64.wgsl` — Full LU decomposition with partial pivoting
- `qr_decomp_f64.wgsl` — Householder QR via parallel norm reductions  
- `svd_f64.wgsl` — One-sided Jacobi SVD via eigendecomposition

**GPU Orchestrators (all f64)**:
- `LuGpu::execute_f64()` — Complete f64 GPU LU with buffer helpers
- `QrGpu::execute_f64()` — Full Householder QR on GPU
- `SvdGpu::execute_f64()` — One-sided Jacobi SVD with full sweep orchestration
- `CgGpu::solve()` — GPU sparse Conjugate Gradient for SPD systems
- `BiCgStabGpu::solve()` — GPU BiCGSTAB for non-symmetric sparse systems
- `Fft3DF64::forward()` / `inverse()` — GPU 3D FFT via 1D decomposition
- `PppmGpu::compute_with_kspace_gpu()` — Full PPPM with GPU FFT

### Bug Fix: Cell-List Index Wrapping

**CRITICAL** — Fixed `cell_idx` in `yukawa_celllist_f64.wgsl` (hotSpring ALERT):
- WGSL `i32 %` produces incorrect results for negative operands on NVIDIA/Naga/Vulkan
- Replaced modular arithmetic with branch-based wrapping
- Post-fix: cell-list PE matches all-pairs to machine precision (<1e-16)

### Native f64 Builtins Migration ✅

hotSpring found native f64 builtins work via Naga/wgpu (1.5-2.2× faster than software):
- `sqrt(f64)`, `exp(f64)`, `log(f64)`, `abs(f64)`, `floor(f64)`, `ceil(f64)`, `round(f64)`, `inverseSqrt(f64)`
- **Migrated MD kernels to native builtins**:
  - `yukawa_f64.wgsl` — sqrt, exp → native
  - `yukawa_celllist_f64.wgsl` — sqrt, exp → native
  - `erfc_forces.wgsl` — sqrt, exp → native (keeps erf_f64 for erfc)
  - `greens_apply.wgsl` — exp → native
  - `rdf_histogram.wgsl` — sqrt → native
- Expected 1.5-2.2× improvement in per-kernel transcendental performance

### Deep Debt Evolution

**Dependency Migration (Pure Rust):**
- `once_cell` / `lazy_static` → `std::sync::LazyLock` (Rust 1.80+)
- `num_cpus` → `std::thread::available_parallelism()` (Rust 1.59+)
- All legacy lazy initialization removed from production code

**Placeholder Implementations → Complete:**
- `lookahead.rs`: Implemented full slow weight EMA update using tensor ops
- `benchmark.rs`: Documented GPU simulation with empirical speedup factors

**Code Quality:**
- Zero unsafe blocks in barracuda and toadstool crates
- All mocks isolated to `#[cfg(test)]` modules
- Capability-based discovery (no hardcoded GPU/NPU identifiers)

### Remaining Evolution Work

**GPU Linear Algebra (f64) - COMPLETE:**
| Area | Status | Notes |
|------|--------|-------|
| LU decomposition | ✅ **COMPLETE** | `LuGpu::execute_f64()` — full GPU orchestration |
| QR decomposition | ✅ **COMPLETE** | `QrGpu::execute_f64()` — Householder via GPU |
| SVD | ✅ **COMPLETE** | `SvdGpu::execute_f64()` — Jacobi SVD on GPU |
| Sparse CG | ✅ **COMPLETE** | `CgGpu::solve()` — GPU sparse solver + `sparse_matvec_f64.wgsl` |
| Sparse BiCGSTAB | ✅ **COMPLETE** | `BiCgStabGpu::solve()` — non-symmetric systems |
| Eigenvalue (symmetric) | ✅ **COMPLETE** | `eigh_f64.wgsl` — Jacobi eigenvalue on GPU |
| Native f64 builtins | ✅ **MIGRATED** | MD kernels use native sqrt/exp (1.5-2.2× faster) |
| GPU FFT | ✅ **COMPLETE** | `Fft1DF64`, `Fft3DF64` — full Cooley-Tukey |
| PPPM GPU FFT | ✅ **COMPLETE** | `PppmGpu::compute_with_kspace_gpu()` |
| Prefix Sum | ✅ **COMPLETE** | `CumsumF64` — GPU f64 cumulative sum |
| Modular Preamble | ✅ **COMPLETE** | `with_math_f64_auto()` — 40-60% smaller shaders |
| Optimizers (Brent, Newton) | CPU only | Consider WGSL for batch |
| Stats (chi2, bootstrap) | CPU only | Low priority |
| Cubic spline | CPU only | Low priority |

**Performance Opportunities:**
- ✅ **FP64-by-default**: SPIR-V/Vulkan bypasses CUDA fp64 throttle (1:2-3 vs 1:32)
- ✅ **Native f64 builtins**: MD kernels migrated — 1.5-2.2× faster transcendentals
- ✅ **GPU FFT integrated**: Full PPPM with GPU FFT via `compute_with_kspace_gpu()`
- ✅ **Modular preamble**: Auto-detect needed math_f64 functions — faster shader compilation

---

## New Features (Feb 14, 2026)

### Molecular Dynamics Pipeline — COMPLETE ✅

**hotSpring MD integration fully absorbed** — all thermostat types + observables + neighbor search:

#### Thermostats (Complete Suite)
- `BerendsenThermostat` — Velocity rescaling for equilibration
- `NoseHooverChain` + `NoseHooverHalfKick` — Deterministic NVT production
- `LangevinParams` + `LangevinStep` — Stochastic dynamics with friction + noise

#### Observables
- `KineticEnergy` — GPU per-particle KE for temperature
- `compute_rdf()` — Radial distribution function (CPU)
- `compute_vacf()` — Velocity autocorrelation (CPU)
- `compute_ssf()` — Static structure factor (CPU)
- `compute_msd()` — Mean-squared displacement with PBC unwrapping for diffusion

#### Neighbor Search
- `CellList` — O(N) cell-list for large N-body simulations
- CPU-managed with GPU-ready exports (cell_start, cell_count)
- sort_array/unsort_array for coalesced memory access

#### PPPM/Ewald (Complete — CPU + GPU Universal)
- `Pppm` — CPU reference implementation
- `PppmGpu` — **Universal GPU implementation** via WGSL shaders:
  - `compute()` — Short-range erfc forces + self-energy (pure GPU)
  - `compute_with_kspace()` — Full PPPM: k-space + short-range forces (GPU particles, CPU FFT)
  - `bspline.wgsl` — B-spline M_p(x) evaluation
  - `charge_spread.wgsl` — Particle → mesh spreading
  - `greens_apply.wgsl` — K-space G(k) multiplication
  - `force_interp.wgsl` — Mesh → particle gradient
  - `erfc_forces.wgsl` — Real-space erfc-damped forces
- `PppmParams` — Automatic parameter tuning (Low/Medium/High accuracy)
- `BsplineCoeffs` — Cardinal B-spline charge spreading/force interpolation
- `ChargeMesh` / `PotentialMesh` — Mesh data structures
- `GreensFunction` — Precomputed G(k) with influence correction
- `spread_charges()` / `interpolate_forces()` — Particle-mesh operations
- `compute_short_range()` — erfc-damped real-space Coulomb
- `self_energy_correction()` / `dipole_correction()` — Energy corrections
- CPU FFT reference implementation (GPU integration ready)
- **38 electrostatics tests passing**

**Reference**: `docs/planning/HOTSPRING_MD_HANDOFF_FEB14_2026.md`

---

## New Features (Feb 13, 2026)

### Phase 5 Evolution — TIERS 1-3 COMPLETE

In response to hotSpring validation (129/129 tests passing, L1 χ²/datum = 1.19 — 82% better than scipy), all three tiers have been implemented.

#### Tier 3: Architecture ✅

**Sparse Linear Algebra** (`barracuda::linalg::sparse`):
- `CsrMatrix` — Compressed Sparse Row format with O(nnz) SpMV
- `CooMatrix` — Coordinate format for easy construction
- `cg_solve()` — Preconditioned Conjugate Gradient for SPD matrices
- `bicgstab_solve()` — BiCGSTAB for general non-symmetric matrices
- `jacobi_solve()` — Jacobi iteration for diagonally dominant systems
- Factory methods: `identity()`, `from_diagonal()`, `tridiagonal()`

**Pipeline Orchestration** (`barracuda::pipeline`):
- `Cascade` — Multi-stage filtering pipeline following hotSpring cascade pattern
- `Stage` — Filter and/or transform with target device selection
- `Target::Cpu`, `CpuParallel`, `Gpu`, `Npu`, `Auto`
- Per-stage statistics and overall savings metrics

**Benchmark Suite** (`barracuda::dispatch::benchmark`):
- `BenchmarkSuite` — Run benchmarks for all operations
- `BenchmarkConfig::quick()` / `default()` / `thorough()` presets
- Crossover detection with configurable speedup threshold
- Safety margin for threshold recommendations

#### Tier 2: New Algorithms ✅

**Direct Sampler** (`barracuda::sample::direct`):
- `direct_sampler()` — Round-based Nelder-Mead on true objective
- Warm-start from seeds or LHS
- Early stopping via convergence diagnostics

**Statistics** (`barracuda::stats`):
- `chi2_decomposed()` — Per-datum residuals, pulls, worst-N analysis
- `bootstrap_ci()` — Non-parametric confidence intervals for any statistic
- `bootstrap_mean()`, `bootstrap_median()`, `bootstrap_std()` convenience functions

**Optimization** (`barracuda::optimize`):
- `convergence_diagnostics()` — Detect improving/stagnant/oscillating/diverging states
- `should_stop_early()` — Simple early stopping predicate
- `adaptive_penalty()` — Data-driven penalty from feasible values
- `adaptive_penalty_mad()` — MAD-based robust variant

#### Tier 1: Critical Fixes ✅

**LOO-CV Hat Matrix Bug Fixed** (`barracuda::surrogate::rbf`):
- Bug: `compute_hat_diagonal()` used K_smooth for both system and RHS, giving H_ii = 1.0 always
- Fix: Use K_raw for RHS, K_smooth for system matrix

**Auto-Smoothing** (`barracuda::sample::sparsity`):
- `SparsitySamplerConfig::auto_smoothing` — Enable LOO-CV grid search per iteration
- `loo_cv_optimal_smoothing()` — Standalone function for finding optimal smoothing

**Penalty Filtering** (`barracuda::sample::sparsity`):
- `PenaltyFilter` enum — None, Threshold, Quantile, AdaptiveMAD
- `SparsitySamplerConfig::with_penalty_filter()` — Remove outliers before training

**Warm-Start Seeds** (`barracuda::sample::sparsity`):
- `SparsitySamplerConfig::with_warm_start()` — Pre-computed starting points
- Enables L1→L2 seeding pattern (2× better than random starts)

**Missing Special Functions** (`barracuda::special::gamma`):
- `digamma(x)`, `beta(a, b)`, `ln_beta(a, b)`

**New tests**: 62 additional tests for Phase 5 (all passing)

---

## New Features (Feb 12, 2026)

### Phase 3 Evolution — hotSpring Handoff Complete

All Phase A and Phase B priorities from the hotSpring handoff document have been implemented:

**Linear Algebra f64 Bridges** (`barracuda::linalg`):
- `cholesky_f64` — Cholesky-Banachiewicz decomposition with solve/det/log_det/inverse
- `eigh_f64` — Symmetric eigenvalue decomposition via Jacobi algorithm
- `gen_eigh_f64` — Generalized eigenvalue problem Ax = λBx via Cholesky reduction
- Re-exports for LU, QR, SVD, tridiagonal (already f64 in ops::linalg)

**Auto-Dispatch System** (`barracuda::dispatch`):
- `DispatchConfig` — Per-operation GPU thresholds with force_cpu/force_gpu overrides
- `DispatchTarget::Cpu | Gpu` — Runtime hardware routing
- GPU availability detection via wgpu
- Empirically-determined thresholds: erf (512), matmul (4096), convolution (8192)

**Scientific Functions** (`barracuda::special`, `barracuda::optimize`, `barracuda::interpolate`):
- `gamma.rs` — Incomplete gamma γ(a,x), regularized P/Q functions
- `chi_squared.rs` — Chi² distribution (CDF, PDF, quantile, goodness-of-fit test)
- `newton.rs` — Newton-Raphson, Secant methods with convergence info
- `brent.rs` — Brent root-finding and minimization
- `cubic_spline.rs` — Natural/clamped cubic spline with derivatives and integration

**Surrogate Quality** (`barracuda::surrogate::rbf`):
- `loo_cv_rmse()` — Leave-one-out cross-validation RMSE
- `loo_cv_errors()` — Per-point LOO residuals

**Cache Persistence** (`barracuda::optimize::eval_record`):
- `save()` / `load()` / `load_or_new()` — JSON serialization for warm-starting
- `from_training_data()` — Create cache from existing data

**Deep Debt Verification**:
- ✅ No unsafe code in linalg modules (all pure safe Rust)
- ✅ Mocks properly isolated (feature-gated `#[cfg(feature = "mock-tpu")]` or in test modules)

**Total new tests**: 96 tests across new modules (all passing)

### Deep Debt Resolution — Production Safety

**Mock Isolation** (`crates/core/toadstool/src/biomeos_integration/auth.rs`):
- Fixed mock signature path reachable in production
- Now feature-gated: `#[cfg(any(test, feature = "dev-mock-auth"))]`
- Production requires real signing key or returns error
- Added `dev-mock-auth` feature flag for development builds

**Akida Driver Evolution** (`crates/neuromorphic/akida-driver/`):
- Removed developer-specific driver path from search locations
- Added `AKIDA_DRIVER_PATH` environment variable for custom locations
- Created shared `pcie_ids` module for vendor/device constants
- Uses standard kernel module paths (`/lib/modules/{kver}/extra/`, `/usr/local/lib/akida/`)

**Primal Self-Knowledge Architecture**:
- Primal constants already deprecated with migration guidance
- `discover_socket_for_capability()` available for capability-based discovery
- Fallback constants maintained for backward compatibility during transition
- All new code should use `RuntimeDiscovery::discover_by_capability()`

---

## New Features (Feb 12, 2026)

### Runtime Evolution — Backend Implementations

**CPU Tensor Operations** (`crates/runtime/universal/src/backends/cpu/tensor_ops.rs`):
- Tiled matrix multiplication with 32x32 cache-blocking
- Direct 2D convolution with padding/stride/bias support
- Max/average pooling with sliding window implementation
- Comprehensive unit tests for dimension validation

**CUDA Backend** (`crates/runtime/gpu/src/backends/cuda_impl.rs`):
- Real PTX kernel execution via `cudarc`
- Matrix multiplication and reduction kernels embedded
- Proper grid/block dimension calculation
- Source kernel validation and dispatch

**Unified Memory Backends** (`crates/runtime/gpu/src/unified_memory/backends/`):
- OpenCL and Vulkan backends now use `wgpu` fallback (ecoBin-compliant)
- Pure Rust memory allocation via WebGPU abstractions
- Direct Vulkan/OpenCL available when specific extensions needed
- Full `BackendInitializer` trait implementation

**Security Providers** (`crates/distributed/src/security_provider/`):
- `UnixSocketSecurityProvider` for JSON-RPC 2.0 over Unix sockets
- Full `SecurityProvider` trait implementation (encrypt, decrypt, sign, verify)
- Factory updated to prefer Unix sockets over HTTP/TCP
- All RPC types derive `Serialize`/`Deserialize` for JSON transport

**Clippy Compliance** (barracuda crate):
- `legendre.rs`, `lu.rs` — `#[allow(clippy::manual_is_multiple_of)]` (nightly feature)
- `normal.rs` — `#[allow(clippy::excessive_precision)]` (intentional for Acklam's algorithm)
- `bessel.rs` — replaced approximate constant with `std::f64::consts::FRAC_2_PI`

### Deep Debt Resolution — hotSpring Audit Complete

All HIGH and MEDIUM priority items from the hotSpring science gaps audit have been implemented:

**Statistics Module** (`barracuda::stats`):
- `normal.rs` — Normal distribution CDF, PDF, inverse CDF (Acklam algorithm, |ε| < 1.15e-9)
- `correlation.rs` — Pearson/Spearman correlation, covariance, correlation/covariance matrices
- 27 tests covering critical values, symmetry, ties

**Matrix Decompositions** (`barracuda::ops::linalg`):
- `lu.rs` — LU decomposition with partial pivoting (Doolittle), determinant, inverse, solve
- `qr.rs` — QR decomposition (Householder reflections), least squares solver
- `svd.rs` — Singular Value Decomposition, pseudoinverse, rank, condition number, low-rank approximation
- 23 tests covering 2x2, 3x3, overdetermined, rank-deficient matrices

**Numerical Methods** (`barracuda::numerical`):
- `rk45.rs` — Adaptive Runge-Kutta-Fehlberg ODE solver with step size control (Cash-Karp coefficients)
- 8 tests: exponential decay/growth, harmonic oscillator, Lotka-Volterra

**PDE Solvers** (`barracuda::pde`):
- `crank_nicolson.rs` — Crank-Nicolson 1D heat equation solver (θ-method, boundary conditions)
- 7 tests including conservation verification, steady state

**Optimization** (`barracuda::optimize`):
- `bfgs.rs` — BFGS quasi-Newton optimizer with backtracking line search
- 7 tests including Rosenbrock function

**Sampling** (`barracuda::sample`):
- `sobol.rs` — Sobol quasi-random sequences (40 dimensions, Gray code generation)
- 11 tests for uniformity, scaling, high dimensions

**Special Functions** (`barracuda::special`):
- `hermite.rs` — Physicist's Hermite polynomials Hₙ(x) via recurrence
- `legendre.rs` — Legendre polynomials Pₙ(x) and associated Legendre Pₙᵐ(x)
- `laguerre.rs` — Generalized Laguerre polynomials Lₙ^α(x)
- `gamma.rs` — Extended with digamma ψ(x) and beta B(a,b) functions
- `erf.rs`, `bessel.rs` — CPU f64 implementations (erf, erfc, J0, J1, I0, K0)

**Shader-First Architecture** (Feb 12, 2026):
- ALL math is now WGSL shader-first — ToadStool dispatches to GPU/CPU
- 18 special function shaders (hermite, legendre, laguerre, digamma, beta, norm_cdf, norm_ppf, etc.)
- 3 sampling shaders (sobol, lhs, random_uniform)
- 5 statistics shaders (correlation, covariance, variance)
- When fp64 GPUs available, seamless transition

**GPU Acceleration**:
- SparsitySampler hybrid evaluation strategy with GPU-accelerated RBF surrogate training

**Total new tests**: 143 WGSL wrapper tests + 90+ middleware tests (all passing)

---

## Previous Features (Feb 11, 2026)

### BarraCUDA Scientific Computing Middleware

**6 production-grade library modules** for self-contained scientific computing:

- **`barracuda::linalg`** - Linear algebra (Gauss-Jordan solver with partial pivoting)
- **`barracuda::numerical`** - Numerical methods (gradient, trapezoidal integration)
- **`barracuda::special`** - Special functions (Lanczos gamma, factorial, Laguerre polynomials)
- **`barracuda::optimize`** - Optimization (Nelder-Mead, multi-start NM, bisection, evaluation cache, resumable solver) — Phase 2A/2B ✅
- **`barracuda::surrogate`** - Surrogate modeling (RBF with 6 kernel types, adaptive dual-precision dispatch) — Phase 2C ✅
- **`barracuda::sample`** - Sampling strategies (LHS, maximin LHS, SparsitySampler, uniform random) — Phase 2A/2B ✅

**Impact**: Self-contained scientific computing infrastructure. Same math serves physics (nuclear EOS), ML (hyperparameter tuning), graphics (camera calibration), audio (filter design). hotSpring tests inform evolution; algorithms are cross-domain.

**Tests**: 129 comprehensive tests (100% passing)
- 9 tests: linalg (2×2, 3×3, singular detection, large systems)
- 15 tests: numerical (gradient, trapz, edge cases)
- 21 tests: special (gamma, factorial, Laguerre polynomials)
- 37 tests: optimize (Nelder-Mead, multi-start global, eval cache, resumable solver, Rosenbrock, Rastrigin)
- 22 tests: surrogate (1D/2D interpolation, kernel variants, adaptive dispatch, f32 vs f64 validation)
- 31 tests: sample (LHS, maximin optimization, SparsitySampler, uniform random)

**New Phase 2C modules**: `train_adaptive` (dual-precision f32/f64 dispatch for surrogate training), `train_with_validation` (f32 vs f64 accuracy comparison), `AdaptiveConfig` (dispatch threshold configuration).

**New Phase 2B modules**: `maximin_lhs` (space-filling via CP algorithm), `sparsity_sampler` (Diaw et al. 2024 iterative surrogate-directed sampling), `ResumableNelderMead` (pausable solver), `laguerre` polynomials.

**Quality**: Zero unsafe, clippy clean, comprehensive docs, validated against scipy/numpy.

**Algorithms**: Gauss-Jordan (Golub & Van Loan), Nelder-Mead (Numerical Recipes), Lanczos gamma (1964), RBF interpolation (scipy pattern).

**Documentation**: 
- `docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md` (comprehensive guide)
- `docs/PHASE1_COMPLETION_REPORT.md` (validation report)
- `docs/MIDDLEWARE_COMPLETION_SUMMARY.md` (technical summary)
- `DEEP_DEBT_STATUS.md` (compliance verification)

**Usage Examples**: See `QUICK_REFERENCE.md#scientific-computing-middleware-api`

---

## Previous Features (Feb 9-10 Sessions)

### GPU Job Queue (`compute.*`)
- `compute.submit` -- Submit inference/transform/custom jobs with priority
- `compute.status` / `compute.result` -- Track and retrieve job results
- `compute.cancel` / `compute.list` -- Job lifecycle management
- Cross-gate routing integrated: submit response includes optimal gate selection

### Ollama Integration (`ollama.*`)
- `ollama.list_models` -- List available models
- `ollama.inference` -- Run model inference with parameters
- `ollama.load` / `ollama.unload` -- VRAM lifecycle management
- Pure Rust HTTP client (no reqwest dependency)

### Cross-Gate Compute Delegation (`gate.*`)
- `gate.update` -- Register remote gate GPU capabilities
- `gate.remove` -- Remove offline gates
- `gate.list` -- List all known gates
- `gate.route` -- Preview routing decision (model locality, VRAM, queue depth)
- Routing priority: ModelLoaded > MostVramAvailable > ShortestQueue > Local

### Multi-Family Socket Support
- `--family-id` CLI flag creates `toadstool-{family_id}.sock`
- Multiple ToadStool instances per machine for isolation

### Shared Error Tracking
- `Arc<AtomicU64>` error counter shared across tarpc and JSON-RPC servers
- Health endpoint reports real `error_count` and `uptime_secs`

---

## Code Quality Evolution (Feb 9-10 Sessions)

### Comprehensive Audit and Execution

**Test Coverage Evolution**: Server crate went from 60% to ~85% line coverage. Common crate at ~84%. Config crate at ~85%. Added 400+ new unit tests across server, common, config, and toadstool crates covering: JSON-RPC parsing and dispatch, handler error paths, builder patterns, validation logic, discovery integration, capability providers, error conversions, resource optimization, graph types, infrastructure detectors, BYOB types, auth, agents, jobs, requests.

**Test Concurrency Fixes**: All tests that modify environment variables now use scoped `ENV_MUTEX` to prevent race conditions during parallel execution. Eliminated nested Tokio runtime panics in `capabilities.rs` and `primal_sockets.rs`. Flaky performance assertions relaxed to realistic thresholds.

**Clippy/Fmt Compliance**: All new test code passes `cargo clippy -D warnings`. Fixed `await_holding_lock`, `redundant_closure`, `field_assignment_outside_of_initializer`, `needless_borrows_for_generic_args`, `clone_on_ref_ptr`, `clone_on_copy`, `assertions_on_constants` across the codebase.

### Deep Debt Fixes

**Unsafe Code**: 35 `unsafe` blocks, 3 `unsafe fn`, 11 `unsafe impl` -- all 100% documented with `// SAFETY:` comments.

**Production Mocks**: `MockExecutor` renamed to `TestExecutor`, isolated to `#[cfg(test)]`. `ServiceClient::Mock` feature-gated.

**Hardcoded Ports**: All magic number `8080` replaced with `DEFAULT_HTTP_PORT` constant. Songbird fallback uses `ports::fallback::SONGBIRD`.

**Doctests**: 9 barracuda doctests had `todo!()` -- replaced with real `Tensor` construction.

**TODOs**: High-priority production TODOs evolved to `tracing::debug!` with honest status messages (mDNS, K8s, Docker Compose, registry discovery). Hardware stubs (TPU/NPU) documented with integration requirements.

**External Dependencies**: Corrected misleading "100% Pure Rust" comment for `notify` (uses `inotify-sys` on Linux). Documented all C FFI deps: `drm-sys` (unavoidable), `renderdoc-sys` (optional via wgpu), `core-foundation-sys` (macOS only), `esp-idf-sys` (optional edge).

**Test Concurrency**:
- Replaced `#[serial]` in 2 config test files with scoped `Mutex` pattern
- Removed `serial_test` crate dependency
- Replaced `tokio::time::sleep` in 3 server test files with event-driven patterns (`yield_now`, `Notify`, `std::future::pending`)
- Added `ENV_MUTEX` across all test modules that mutate environment variables

**GPU Tests**: Barracuda tests skip gracefully on machines without real GPUs. `get_test_device_if_gpu_available()` returns `None` for software adapters. All 1,242 barracuda lib tests pass.

**CPU Backends**: Implemented all CPU compute backends (LayerNorm, BatchNorm, MatMul, Conv2d, Pooling, Vector ops, Transforms).

**Smart Refactoring**: `manual_jsonrpc.rs` extracted into core (713 lines) + handlers (429 lines), both under 1000-line limit.

---

## Cross-Vendor Distributed Compute

| GPU | Vendor | Machine | GFLOPS | Checksum |
|-----|--------|---------|--------|----------|
| RTX 4070 | NVIDIA | Tower | 388.7 | **5.128010** |
| RTX 3090 | NVIDIA | gate2 | 481.0 | **5.128010** |
| RX 6950 XT | AMD | gate2 | 222.7 | **5.128010** |

**Test**: 1024x1024 matmul, single WGSL shader, single Rust binary. Bit-identical checksums.

### Distributed LLM Inference

- TinyLlama-1.1B, 22 layers split across Tower + gate2
- **39.85 tok/s** over LAN TCP
- BearDog ChaCha20-Poly1305 encrypted tensor transport
- 20.4 MB total data transferred for 80 tokens

### Hardware Available

| Machine | GPU(s) | CPU | RAM |
|---------|--------|-----|-----|
| Tower | RTX 4070 (12 GB) + RX 6800 (16 GB AMD) | 24 cores | - |
| gate2 | RTX 3090 (24 GB) + RX 6950 XT (16 GB) | EPYC 7452 64-thread | 252 GB |

---

## BarraCUDA Shaders: 480+ WGSL Files (Shader-First Architecture)

**Organization**: Categorized directory structure for discoverability

| Category | Count | Location | Status |
|----------|-------|----------|--------|
| Activation | 37 | `shaders/activation/` | Complete |
| Attention | 8 | `shaders/attention/` | Complete |
| Audio/Signal | 9 | `shaders/audio/` | Complete |
| Augmentation | 10 | `shaders/augmentation/` | Complete |
| Convolution | 11 | `shaders/conv/` | Complete |
| Detection | 5 | `shaders/detection/` | Complete |
| Dropout | 2 | `shaders/dropout/` | Complete |
| GNN | 6 | `shaders/gnn/` | Complete |
| Gradient | 1 | `shaders/gradient/` | Complete |
| Interpolation | 2 | `shaders/interpolation/` | Complete |
| Linear Algebra | 11 | `shaders/linalg/` | Complete (cholesky, eigh, linsolve, triangular solve, inverse) |
| Loss | 31 | `shaders/loss/` | Complete (focal, dice, iou, bce, mse, kl, triplet, etc.) |
| Math | 68 | `shaders/math/` | Complete (trig, exp, log, floor, sqrt, etc.) |
| Normalization | 27 | `shaders/norm/` | Complete (batch, layer, group, instance, rms, spectral) |
| Optimizer | 13 | `shaders/optimizer/` | Complete (adam, adamw, sgd, lamb, rmsprop, etc.) |
| Pooling | 17 | `shaders/pooling/` | Complete (max, avg, adaptive, roi, global) |
| Reduce | 14 | `shaders/reduce/` | Complete (sum, mean, argmax, logsumexp, variance) |
| RNN | 4 | `shaders/rnn/` | Complete (lstm_cell, gru_cell, bi_lstm) |
| Special Functions | 7 | `shaders/special/` | Complete (Bessel, harmonics, Hermite f64, Laguerre f64) |
| Tensor/Shape | 41 | `shaders/tensor/` | Complete (concat, slice, reshape, transpose, gather, scatter) |
| Miscellaneous | 56 | `shaders/misc/` | Complete (matmul, embedding, quantize, utilities) |
| Complex | 10 | `ops/complex/` | Complete (add, sub, mul, div, exp, log, pow, sqrt, abs, conj) |
| FFT | 2 | `ops/fft/` | Complete (1D FFT, IFFT normalize) |
| FHE | 13 | `ops/` (fhe_*) | Complete (NTT, INTT, poly ops, key switch, boolean gates) |
| MD Forces | 5 | `ops/md/forces/` | Complete (Coulomb, Lennard-Jones, Yukawa, Morse, Born-Mayer) |
| MD Integrators | 3 | `ops/md/integrators/` | Complete (Velocity-Verlet, RK4, Laplacian) |
| MD PBC | 1 | `ops/md/` | Complete (Periodic boundary conditions) |
| Mixing | 1 | `shaders/mixing/` | Complete (Broyden f64 — hotSpring) |
| Grid | 1 | `shaders/grid/` | Complete (FD gradient f64 — hotSpring) |
| Reduce f64 | 1 | `shaders/reduce/` | Complete (weighted dot f64 — hotSpring) |
| **Total** | **480+** | **24+ categories** | **100% organized** |

**Documentation**: See `crates/barracuda/src/shaders/README.md` and `CATEGORIES.md` for detailed index.

### New Science Shaders (Feb 10, 2026)

| Shader | Purpose | Category |
|--------|---------|----------|
| `eigh.wgsl` | Jacobi eigenvalue decomposition | Linear algebra |
| `linsolve.wgsl` | Gaussian elimination with partial pivoting | Linear algebra |
| `bessel_j0.wgsl` | Bessel J0 (cylindrical coordinates) | Special functions |
| `bessel_j1.wgsl` | Bessel J1 | Special functions |
| `bessel_i0.wgsl` | Modified Bessel I0 | Special functions |
| `bessel_k0.wgsl` | Modified Bessel K0 | Special functions |
| `spherical_harmonics.wgsl` | Y_lm for multipole expansion (l=0..6) | Special functions |
| `prng_xoshiro.wgsl` | xoshiro128** PRNG for Monte Carlo | Numerical methods |
| `sparse_matvec.wgsl` | CSR sparse matrix-vector product | Numerical methods |
| `loo_cv.wgsl` | Leave-one-out cross-validation | Numerical methods |

### Shader TODOs: 0 Remaining (11/11 Evolved)

All shader TODOs resolved:
- `pow_simple.wgsl` ✅ general exponent via Params uniform
- `broadcast.wgsl` ✅ full NumPy-style shape/stride broadcasting
- `cast.wgsl` ✅ 7 modes (identity, f32↔i32, f32↔u32, clamp, bool)
- `determinant.wgsl` ✅ NxN via LU decomposition with pivoting (N≤16)
- `scatter_nd.wgsl` ✅ multi-dimensional scatter with trailing dims
- `gather_nd.wgsl` ✅ partial indexing with trailing dim slicing
- `edge_conv.wgsl` ✅ CSR-based real edge indices (replaced placeholder)
- `spectral_norm_1d.wgsl` ✅ proper σ computation via compute_sigma kernel
- `index_add.wgsl` ✅ atomic CAS-based f32 add for overlapping indices
- `u64_emu.wgsl` ✅ Barrett reduction via u64_mul_high (128-bit product)
- `fhe_key_switch.wgsl` ✅ documented Phase 3 path for FHE key infrastructure

---

## Deep Debt

### Clean (All Zero ✅)

- 0 clippy warnings (was 166)
- 0 build warnings
- 0 failed tests
- 0 production `todo!()` or `unimplemented!()`
- 0 `unsafe` blocks without `// SAFETY:` documentation
- All files ≤ 1000 lines (D-003 resolved Feb 18, 2026)
- 0 `#[serial]` test annotations (replaced with scoped Mutex)
- 0 sleep-based synchronization in server tests
- 0 misleading dependency comments
- 0 production `.unwrap()` on `Option` in hot paths (evolved to `Result`)
- 0 NaN-unsafe `partial_cmp().unwrap()` (7 sites fixed with `unwrap_or(Ordering::Equal)`)
- 0 shader TODOs remaining (11/11 evolved to complete implementations)
- 0 server placeholder metrics (evolved to real sysinfo values)
- 0 scheduler placeholder responses (proper error handling)
- Production mocks renamed and isolated to `#[cfg(test)]`
- All hardcoded ports replaced with named constants
- All env-mutating tests protected by `ENV_MUTEX`
- `num_cpus` FFI dependency removed from barracuda (evolved to `std::thread::available_parallelism()`)
- `validator` crate unified to 0.18 in config and toadstool (api pending 0.18 migration)

### Remaining

- W-001, W-002 (active debt)
- `unibin.rs` 18% coverage (socket helpers tested, server startup requires running server)
- `manual_jsonrpc.rs` 27% coverage (async I/O requires integration tests)
- `websocket.rs` needs integration tests (live WebSocket connections)
- PyTorch dependency for distributed LLM demo (solving with safetensors loader)
- mDNS/K8s/Docker Compose discovery (env vars work, other sources pending)
- FPGA discovery implementation
- TPU backend support

---

## Hardware Routing (WorkloadHint → Device)

BarraCUDA auto-routes workloads to the optimal device. Users can override
any decision via `Device::select_with_preference(Some(Device::CPU), &hint)`.
CPU is always available as a fallback.

| WorkloadHint | Auto-Route | Fallback | Notes |
|--------------|-----------|----------|-------|
| `PhysicsForce` | GPU | CPU | Arbitrary math via WGSL shaders |
| `FFT` | GPU | CPU | Parallel butterfly stages |
| `EigenDecomp` | GPU | CPU | Jacobi iteration on GPU |
| `LinearSolve` | GPU | CPU | Gaussian elimination on GPU |
| `Training` | GPU | CPU | Gradient shaders |
| `MonteCarlo` | GPU | CPU | Parallel xoshiro128** PRNG |
| `SparseMath` | GPU | CPU | CSR sparse matvec |
| `SurrogateEval` | GPU | CPU | RBF kernel evaluation |
| `LargeMatrices` | GPU | CPU | Dense matmul, batched ops |
| `SparseEvents` | NPU | CPU | Spiking neural network inference |
| `Inference` | NPU | GPU/CPU | Pre-compiled model inference |
| `PreScreen` | NPU | CPU | Binary classify at ultra-low power |
| `Reservoir` | NPU | CPU | ESN with fixed random weights |
| `EventProcessing` | NPU | CPU | Event-driven logic |
| `SmallWorkload` | CPU | -- | Avoids GPU dispatch overhead |
| `StringOps` | CPU | -- | Text processing |
| `General` | GPU→CPU | -- | Default fallback chain |

**NPU detection**: Scans `/dev/akida*` (C driver) and IOMMU groups for BrainChip
vendor `0x1e7c` (VFIO path). Returns false if no hardware found.

**CPU executor**: Supports ReLU, Sigmoid, Tanh, GELU, Add/Sub/Mul/Div/Pow,
ReduceSum/Mean/Max/Min/Prod, MatMul. AVX2/SSE2/NEON SIMD detection. Rayon
parallelism. Always accepts any workload as universal fallback.

---

## IPC Architecture

- **Protocol**: JSON-RPC 2.0 over Unix sockets (26 methods)
- **High-performance**: tarpc for typed RPC
- **Discovery**: Capability-based via `CapabilityDiscovery`
- **Socket standard**: `/run/user/$UID/biomeos/{primal}.sock`
- **Multi-family**: `toadstool-{family_id}.sock` via `--family-id`
- **Method naming**: `{domain}.{operation}[.{variant}]`
- **Error tracking**: Shared `AtomicU64` across transports
- **Constants**: Centralized in `toadstool_common::constants`

---

## Evolution Gaps

### Phase 5 Completed ✅ (Feb 13, 2026)

All hotSpring validation items from Tiers 1-3 have been implemented:
- ✅ LOO-CV hat matrix bug fixed
- ✅ Auto-smoothing via LOO-CV grid search
- ✅ Penalty filtering (Threshold, Quantile, AdaptiveMAD)
- ✅ Warm-start seeds for L1→L2 seeding
- ✅ digamma, beta, ln_beta special functions
- ✅ Direct sampler (round-based NM)
- ✅ Chi² decomposition with per-datum analysis
- ✅ Bootstrap confidence intervals
- ✅ Convergence diagnostics
- ✅ Adaptive penalty functions
- ✅ Sparse linear algebra (CSR, CG, BiCGSTAB, Jacobi)
- ✅ Pipeline orchestration (Cascade, Stage)
- ✅ Benchmark suite for auto-dispatch thresholds

### Phase 3 Completed ✅ (Feb 12, 2026)

- ✅ f64 linalg bridges (cholesky_f64, eigh_f64, gen_eigh_f64)
- ✅ Auto-dispatch system (CPU/GPU routing)
- ✅ EvaluationCache persistence (save/load/load_or_new)
- ✅ LOO-CV wiring for RBFSurrogate
- ✅ Incomplete gamma, chi-squared distribution
- ✅ Newton-Raphson, Brent root-finding
- ✅ Cubic spline interpolation
- ✅ Generalized eigenvalue problem

### Infrastructure (Feb 15, 2026 — Completed) ✅

| Gap | Priority | Status |
|-----|----------|--------|
| Safetensors/GGUF weight loader | HIGH | ✅ **COMPLETE** — Full GGUF v2/v3 + safetensors |
| INT4/INT8 quantized shaders | HIGH | ✅ **COMPLETE** — Q4_0/Q8_0 dequant + GEMV |
| Async batch GPU submission | MEDIUM | ✅ **COMPLETE** — `AsyncSubmitter`, `AsyncReadback` |
| Cache probing microbenchmarks | MEDIUM | ✅ **COMPLETE** — `cache_probe` CLI tool |

### Infrastructure Gaps (Remaining)

| Gap | Priority | Status |
|-----|----------|--------|
| Multi-GPU DevicePool | HIGH | Not started (awaiting Titan V) |
| mDNS/K8s/Docker discovery | HIGH | Env vars work, other sources pending |
| Cross-gate mesh relay | MEDIUM | Types defined, needs Songbird transport |
| f64 WGSL shaders (native Titan V) | MEDIUM | Awaiting hardware (Phase 5 Tier 4) |
| Generic precision support (f16/bf16/fp8) | MEDIUM | See specs/GENERIC_PRECISION_EVOLUTION.md |

### Generic Precision Evolution (Investigation)

The hotSpring team raised a key question: can we evolve to "any fp" instead of hardcoded f32/f64?

**Current State:**
- CPU code: hardcoded `f64` for precision-critical paths
- GPU WGSL: hardcoded `f32` (with f64 emulation in `matmul_fp64.wgsl`)
- No `num-traits` or generic `Float` abstraction

**Recommended Approach:**
1. Use `num-traits::Float` for CPU algorithms (supports f32/f64)
2. Keep WGSL shaders at f32 (hardware limitation)
3. Add `PrecisionMode` enum for runtime selection
4. Wait for Titan V hardware for native f64 GPU

**Why Not Full Generic:**
- WGSL fundamentally doesn't support generic types
- f16/bf16/fp8 have different numerical stability requirements
- Algorithms need precision-specific tolerances (e.g., 1e-14 for f64 vs 1e-6 for f32)

**Future Path:**
```rust
pub enum PrecisionMode {
    F32,                    // Standard GPU
    F64Emulated,            // Split hi/lo f32 pairs
    F64Native,              // Titan V / datacenter GPUs
    Mixed { threshold },    // f64 CPU small, f32 GPU large
}
```

See `specs/BARRACUDA_PHASE3_EVOLUTION_HOTSPRING.md` for full roadmap.

---

## Root Documentation

| File | Purpose |
|------|---------|
| `README.md` | Project overview, honest status |
| `STATUS.md` | This file -- detailed status |
| `DOCUMENTATION.md` | Navigation hub |
| `QUICK_STATUS.md` | One-page summary |
| `QUICK_REFERENCE.md` | Commands and API reference |

---

**Last Updated**: February 19, 2026 — Sessions 9–11: Zero-copy bytes::Bytes, 27 sleeps removed, hardcoding eliminated, pure_jsonrpc + storage_backend split, CLI executor coverage, llvm-cov SIGSEGV resolved, 63.02% coverage
