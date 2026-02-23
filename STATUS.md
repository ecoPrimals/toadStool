# Status -- February 23, 2026 (Sessions 32-49: Shader-First Architecture + Deep Debt)

## Quality Gates

| Gate | Status | Notes |
|------|--------|-------|
| `cargo build --workspace` | PASS | Clean build |
| `cargo fmt --all -- --check` | PASS | 0 diffs |
| `cargo clippy --workspace --all-targets` | PASS | **0 warnings** |
| `cargo doc --workspace --no-deps` | PASS | 0 warnings |
| `cargo test --workspace --lib` | PASS | **14,000+ tests passing** |
| hotSpring validation | PASS | **195/195 acceptance checks** |
| wetSpring validation | PASS | **728 Rust tests, 95 experiments** |
| neuralSpring validation | PASS | **1,560+ checks, 115 binaries** |
| Pure Rust syscalls | PASS | mmap/mlock via rustix |
| Zero-copy hot paths | PASS | `Cow<'a, str>` + `#[serde(borrow)]`, `from_slice`, `bytes::Bytes` |
| Hardcoded primal names | PASS | **0 -- capability-based discovery** |
| Line coverage (common) | PASS | **87%** |
| Line coverage (config) | PASS | **89%** |
| Line coverage (core) | PASS | **~87%** |
| Line coverage (server) | PASS | **~85%** |
| Line coverage (distributed) | PASS | **55%** |
| `unsafe` blocks | PASS | **95+ blocks audited -- all SAFETY documented** |
| Production `Box<dyn Error>` | PASS | **0 in core crates -- all typed errors** |
| Production panics/unwraps | PASS | **Zero blind unwrap(); infallible expect() only** |
| TODOs/FIXMEs/HACKs | PASS | **Zero in production code** |
| File size limit | PASS | **All files under 1000 lines** |
| WGSL shaders | PASS | **645+ (zero orphans, all f64 shader-first)** |
| CPU-only math in prod | PASS | **Zero — all math dispatches GPU shaders** |

Excludes hardware-dependent crates: `toadstool-runtime-gpu`, `ml-inference-showcase`, `homomorphic-computing`. Examples excluded (require GPU).

---

## Sessions 46-49: Shader-First Architecture (Feb 23, 2026)

- **S49e-f: Zero CPU-only math** -- 27+ threshold-gated CPU fallbacks eliminated, 6 always-CPU ops wired to GPU, linalg (solve, cholesky) GPU-dispatched, RBF surrogate GPU pipeline (cdist + solve), PPPM electrostatics GPU FFT
- **S49c-d: Force field + MD GPU enforcement** -- Velocity-Verlet, MSD, cubic spline, RDF, cdist all GPU-first. Coulomb, Morse, Born-Mayer, Yukawa CPU fallbacks removed.
- **S49: Spring shader ingestion** -- 13 f32→f64 evolutions (bio, ESN, numerical). All 4 springs absorbed at f64.
- **S48: Lattice QCD GPU orchestration** -- CG solver + full HMC trajectory host loops
- **S47: Lattice QCD shaders** -- 14 WGSL shaders. CPU lattice code gated `#[cfg(test)]`.
- **S46: Cross-project absorption** -- hotSpring, neuralSpring, wetSpring shader absorption complete
- **f64 transcendental coverage** -- `compile_shader_f64()` auto-injects `math_f64.wgsl` polyfills on all drivers

## Session 45: Deep Debt Evolution (Feb 23, 2026)

- **Box<dyn Error> → typed errors**: 21 production usages eliminated (tarpc_server, manual_jsonrpc, unibin, resource_validator, production_hardening)
- **Barracuda fixes**: `atanh.wgsl` bind group layout, `batch_pair_reduce_f64.wgsl` fma→multiply+add, NPU test serialization (`SYNC_DEVICE_MUTEX`)
- **Coverage expansion**: 38 new tests (planner +9, ecosystem +8, detector +21)
- **Unsafe audit**: 95+ blocks documented; last `NonNull::new_unchecked` evolved to safe; 50+ SAFETY comments on env-var test blocks
- **Clippy pedantic**: 14 manual fixes (unnecessary_wraps, unused_async, match_same_arms) + 100+ auto-fixes across distributed/display/gpu crates
- **Event-driven**: Production polling loops → `tokio::time::interval` (launcher, client, health)
- **Clone reduction**: `Arc<str>` for version string, ref-based IPC params, borrow-based coordinator
- **Zero-copy**: `read_async` returns `bytes::Bytes`, `write_async` accepts `impl AsRef<[u8]>`
- **Hardcoding**: Primal integration and Consul/etcd endpoints configurable via env vars
- **WebSocket deprecation**: `WS_PROTOCOL_VERSION` and `ClientError::WebSocket` deprecated, `tokio-tungstenite` removed
- **Error conversion tests**: 5 pre-existing failures fixed (mismatched expected strings)
- **Test isolation**: ENV_MUTEX for all env-var-mutating detector tests
- **All quality gates green**: 0 clippy, 0 doc warnings, 0 fmt diffs, 14,000+ tests passing

## Session 41: f64 Shader Compile Fix + API Exposure (Feb 22, 2026)

- **Critical**: 6 f64 WGSL shaders used `compile_shader()` instead of `compile_shader_f64()`, missing f64 preamble injection for naga/Vulkan. Fixed: `batched_ode_rk4`, `batch_pair_reduce_f64`, `batch_tolerance_search_f64`, `kmd_grouping_f64`, `hill_f64`, `GemmCachedF64`
- **API**: `cpu_conv_pool::{conv2d, max_pool2d, avg_pool2d}` promoted from `pub(crate)` to `pub` (unblocks neuralSpring LeNet-5)
- **API**: All 25 bio ops re-exported at crate root (was 10)
- **Confirmed**: S-14/S-15 already resolved in S39; neuralSpring V8 recommendations stale for these

## Session 40: Richards PDE + Moving Window Stats + Dependency Audit (Feb 22, 2026)

- **Richards**: 1D unsaturated zone water flow solver (van Genuchten-Mualem, Picard iteration, Crank-Nicolson) with 4 tests (airSpring absorption)
- **Moving window stats**: WGSL GPU kernel computing mean/var/min/max over sliding windows for IoT sensor streams; always GPU dispatch
- **Dependency audit**: workspace already pure Rust; libc confined to akida VFIO ioctls
- **Dead code sweep**: 38 `#[allow(dead_code)]` all verified legitimate

## Sessions 39: Full Spring Absorption (Feb 22, 2026)

- Absorbed 7 neuralSpring bio ops + 3 wetSpring WGSL shaders + 11 hotSpring HFB physics shaders
- S-14 Naive matmul tier removed; S-15 matmul hang fix; S-16 transpose dispatch fix
- `GemmCachedF64::execute_to_buffer()`, `barracuda::math` module, `FlatTree` CSR, `sparse_eigh`, `quantize_affine_i8`
- `matmul_tiled.wgsl` barrier-safety fix for small matrix dispatch

---

## Session 38: Zero Warnings, Idiomatic Sweep, Test Coverage (Feb 22, 2026)

- **Zero clippy warnings**: Fixed `manual_div_ceil` in Yukawa GPU dispatch; added targeted `#[allow(clippy::expect_used)]` on infallible `Drop` in `AlignedBuffer` -- workspace now 0 clippy warnings
- **Blind unwrap() elimination**: Replaced 3 production `.unwrap()` calls with descriptive `.expect()` in `fused_map_reduce_f64.rs` and `batched_elementwise_f64.rs`; audited full workspace -- zero blind `unwrap()` in production code
- **Idiomatic match → if-let**: Simplified `deallocate_resources` in `hosting/resources.rs`
- **Test race condition fix**: 3 env-mutating tests in `toadstool-display` refactored from `std::env::set_var` to direct `PathEnv`/`PlatformPaths` construction -- eliminates parallel test races
- **Distributed test coverage**: 11 new behavioral tests for `NetworkLoadBalancer` (register, select, deregister, snapshot, least-loaded, unhealthy filtering) and `NetworkDistributor` (disabled fallback, deregister, accessor); distributed crate now 366 tests
- **Workspace verification**: 3,847+ tests passing across all non-GPU crates; barracuda targeted tests all passing

---

## Sessions 36-37: Precision, Deformed HFB, GPU Dispatch, Deep Debt (Feb 22, 2026)

- **TS-003**: Trig precision fix -- `sin_simple`/`cos_simple` upgraded to 7-term Taylor + Cody-Waite range reduction; `asin_core` extended from 5 to 8 polynomial terms
- **TS-001**: `pow_f64` fix -- f64 `exp_f64` extended to handle 2^k for |k| up to 1023; `log_f64` upgraded from 3 to 7 polynomial terms
- **TS-004**: `FusedMapReduceF64` buffer conflict -- both passes now encoded in single command encoder
- **S-13**: `PooledBuffer` drop race -- deferred return via pending queue with non-blocking device poll
- **Absorbed**: 5 deformed HFB shaders from hotSpring (Nilsson basis, density, Skyrme+Coulomb potential, cylindrical Laplacian Hamiltonian, BCS pairing)
- **Absorbed**: 4 neuralSpring shaders (`pairwise_l2`, `hill_gate`, `multi_obj_fitness`, `swarm_nn_forward`)
- **GPU dispatch**: Yukawa cell-list evolved from CPU-only to full GPU dispatch with sorted particles and result unsorting
- **LinuxEdgeDevice**: edge devices discovered via biomeOS runtime sockets get proper `EdgeDevice` impl
- **Bluetooth discovery**: sysfs-based adapter probe (`/sys/class/bluetooth`)
- **Federation discovery**: TCP probing of configured `discovery_endpoints`
- **29 new tests**: service discovery (17), federation (2), hosting resources (10)
- **ESN**: `export_weights()` + `import_weights()` for GPU-train → NPU-deploy pipeline
- **HFB spherical**: potentials, Hamiltonian, density, energy functional, BCS bisection -- 5 new f64 shaders
- **IPC v3.0**: abstract sockets, TCP fallback, tiered transport discovery confirmed
- **Code quality**: `cargo fmt` + `cargo clippy` clean; 589+ WGSL shaders (zero orphans)

---

## Sessions 32-35: Deep Debt Evolution (Feb 21-22, 2026)

### Capability-Based Discovery
- All hardcoded primal names (beardog, songbird, nestgate, squirrel) replaced with capability-based constants
- New `crates/core/common/src/constants/ecosystem.rs` with `well_known::*` identifiers for integration modules
- Auth modules: audience validation uses `PRIMAL_NAME` + `PLATFORM_AUDIENCE` only
- Config: self-knowledge only (no external primal port mappings)
- Doctor command: discovers running primals from socket files
- CLI zero-config: Unix socket capability-based discovery replacing HTTP placeholders

### Cloud Stubs Evolved to Real Implementations
- **Cost model**: Resource-based estimation with 6 pricing tiers, budget enforcement, structured breakdowns
- **Compliance**: Data sovereignty, security tier validation (Basic/Standard/High), resource isolation, structured reports
- **Federation**: Member management, heartbeats, capability exchange, configurable timeouts

### Zero-Copy Deepening
- `JsonRpcRequest<'a>` with `Cow<'a, str>` and `#[serde(borrow)]` for zero-copy deserialization
- `JsonRpcResponse<'a>` / `JsonRpcError<'a>` with borrowed fields
- Service discovery: `from_str` -> `from_slice` on hot paths

### Dependency Evolution
- thiserror 1.0 -> 2.0 workspace-wide (26 crates)
- async-trait retained (needed for `dyn Trait` async)
- FFI deps documented and justified

### Shader Completion
- Conv2D, MaxPool2D, AvgPool2D dedicated WGSL compute shaders (`ops/nn/`)
- RDF histogram GPU normalization (g(r) = histogram / (N_pairs * V_shell * rho))

### Testing & Coverage
- 200+ new unit tests across all crates
- FHE fault injection: GPU unavailable fallback, Barrett reduction, NTT twiddle factors
- WASM component-model: feature-gated stubs with skip messages
- Property-based testing with proptest for FHE operations

### Code Hardening
- Unsafe audit: all 62 blocks documented, none replaceable with safe Rust
- `#[allow]` audit: 5 unnecessary suppressions removed
- Production panic audit: 0 panics in core library code
- Error allocations reduced: useless `.into()` conversions eliminated
- Placeholder strings replaced with descriptive, actionable messages

### Architectural Evolution
- BYOB server merged into UniBin CLI (`toadstool byob-server` subcommand)
- `manual_jsonrpc` deprecated with MIGRATION.md guide to `pure_jsonrpc`
- Large files refactored: adaptive/mod.rs, config/lib.rs, primal_identity.rs, cpu_executor.rs
- Edge runtime: filesystem-based discovery + serial/TCP communication

---

## Session 31h: Deep Debt Polish (Feb 21, 2026)

### Clippy Clean Sweep
- **Barracuda**: 5 warnings → 0 (needless deref, manual div_ceil, manual is_multiple_of)
- **Akida-driver**: 2 warnings → 0 (map/unwrap_or_else → map_or_else, 8-arg fn → PollConfig struct)
- **Workspace**: Zero clippy warnings under `-W clippy::all` across all key crates

### Dead Code Audit (33 files)
- Removed 6 incorrect `#[allow(dead_code)]` from actually-used items (FheFastPolyMul, FhePointwiseMul, FheIntt, inv_n, Lookahead::alpha)
- Removed 2 dead functions (qr.rs::mat_approx_eq, nonzero::read_buffer_u32)
- Promoted view.rs::wgsl_shader() to pub const WGSL_VIEW
- 22 annotations confirmed legitimate (future GPU acceleration paths)

### Production Code Quality Verification
- All unwrap() calls in high-count files exclusively in #[cfg(test)] blocks
- Zero TODOs/FIXMEs/HACKs in production code (1 research TODO in akida-reservoir)

---

## Session 31g: Deep Debt Evolution (Feb 21, 2026) ✅

### Orphan Shader Integration
- **ESN GPU kernels**: `WGSL_RESERVOIR_UPDATE` + `WGSL_READOUT` constants
- **RF batch inference**: `RfBatchInferenceGpu` — full GPU wrapper (SoA f64, wetSpring v5)
- **HMM forward f32**: `WGSL_HMM_FORWARD_LOG_F32` — log-domain variant
- **SDPA single-kernel**: `WGSL_SDPA_SINGLE_KERNEL` — prototype alongside multi-pass
- **Optimizer shaders**: BFGS update, batch gradient, simplex ops wired as constants

### f64 Linear Algebra
- **`LinSolveF64`**: GPU Gaussian elimination (f64) for ill-conditioned systems
- **`InverseF64`**: GPU Gauss-Jordan inverse (f64, N ≤ 32)

### Safety & Quality Audit
- **Zero production panics**: All 50+ `panic!()` calls confirmed in test code only
- **Hardcoded IPs resolved**: All use env-var-with-defaults pattern
- **Unsafe audit clean**: All blocks minimal with SAFETY invariant docs
- **Extracted `PINNED_ALIGNMENT`**: De-duplicated constant in `pinned.rs`

---

## Session 31e: Deep Debt Evolution (Feb 21, 2026) ✅

### Executor Completeness ✅

- **GPU executor** — All MathOp variants now have dispatch paths: `Pow` (scalar pow), `Max`/`Min` (elementwise fallback), `Squeeze`, `Unsqueeze`, `Broadcast`, `Concat`, `Split`. Only Conv2D/MaxPool2D/AvgPool2D remain as honest `NotImplemented`.
- **CPU executor** — Full coverage: `Softmax`, `BatchMatMul`, `Reshape`, `Squeeze`, `Unsqueeze`, `Transpose`, `Broadcast`, `Concat`, `Split`. Only Conv ops remain `NotImplemented`.

### Orphan Shader Wiring ✅

- **6 new GPU op wrappers** connecting WGSL shaders to Rust APIs: `BatchIprGpu` (spectral/IPR), `LocusVarianceGpu` (bio/FST), `PairwiseHammingGpu`, `PairwiseJaccardGpu`, `SpatialPayoffGpu`, `BatchFitnessGpu`.
- Extended `elementwise_binary.wgsl` with Pow/Max/Min operations.
- Removed duplicate ODE shader (bio/ copy → numerical/ is canonical).
- Removed genuinely unused `read_buffer_u32()` from searchsorted.
- Fixed 3 lifetime elision warnings.

---

## Session 31d: Cross-Spring Absorption (Feb 21, 2026) ✅

### hotSpring Absorption ✅

- **Staggered Dirac operator** — `dirac_staggered_f64.wgsl` + `ops/lattice/dirac.rs`: Full GPU pipeline for Kogut-Susskind lattice QCD fermions. SU(3)×color multiplication, staggered phases, periodic boundaries. `DiracGpuLayout` for topology flattening.
- **CG lattice kernels** — `cg_kernels_f64.wgsl` + `ops/lattice/cg.rs`: Three BLAS-like GPU kernels (`complex_dot_re`, `axpy`, `xpay`) for CG solver on complex fermion fields. Also exported as standalone WGSL constants.
- **SubstrateCapability model** — `device/substrate.rs`: Capability-based dispatch enum (F64Compute, F32Compute, QuantizedInference, BatchInference, WeightMutation, ScalarReduce, SparseSpMV, Eigensolve, CG, ShaderDispatch, SimdVector, TimestampQuery). Runtime-probed from wgpu features. NPU discovery via `/dev/akida*`.

### wetSpring Absorption ✅

- **7 new bio GPU op wrappers** — Full `WgpuDevice` pipelines following `SmithWatermanGpu` pattern:
  - `HmmBatchForwardF64` — Batch HMM forward algorithm (log-domain, f64)
  - `AniBatchF64` — Pairwise Average Nucleotide Identity
  - `SnpCallingF64` — Position-parallel SNP calling
  - `DnDsBatchF64` — Batch Nei-Gojobori dN/dS with Jukes-Cantor
  - `PangenomeClassifyGpu` — Gene family classification (core/accessory/unique)
  - `QualityFilterGpu` — Per-read FASTQ quality trimming
  - `Dada2EStepGpu` — DADA2 E-step batch log-probability
- **ODE sweep shader** — `batched_qs_ode_rk4_f64.wgsl`: Full-GPU RK4 parameter sweep for QS/c-di-GMP ODE (5-variable system, 17 parameters per trajectory)

### neuralSpring Confirmation ✅

- **Householder+QR eigensolver** — Already absorbed as `ops/linalg/eigh_f64.rs`
- **7 domain shaders** — Already present as WGSL files (batch_ipr, spatial_payoff, pairwise_hamming, pairwise_jaccard, locus_variance, batch_fitness_eval, rk4_parallel)
- **GPU PRNG** — Already present as `shaders/misc/prng_xoshiro.wgsl`
- **CPU math** — Already present (`special/erf.rs`, `special/gamma.rs`)
- **NVVM Ada workaround** — Already complete (`NvvmAdaF64Transcendentals` in `driver_profile.rs`)

---

## Sessions 31–31c Evolutions (Feb 21, 2026) ✅

### Executor Wiring ✅

- **GpuExecutor** — 16 additional MathOps wired (total 31): Log, Sin, Cos, Tan, Reciprocal, Square, Div, BatchMatMul, ReduceMax/Min/Prod, Reshape, Transpose
- **CpuExecutor** — Full MathOp dispatch via `execute_unary_cpu`/`execute_binary_cpu`/`execute_reduce_cpu`/`execute_matmul_cpu` (was `NotImplemented`)
- **unified_hardware CpuExecutor** — Delegated to standalone CpuExecutor (was `NotImplemented`)
- **ProcessSpawner WASM loading** — Delegated to BiomeExecutor (was returning empty bytes)
- **Performance optimizer** — `get_recommendations()` and `update_model()` implemented

### Smart Refactoring ✅

| File | Before | After | Technique |
|------|--------|-------|-----------|
| `qr_gpu.rs` | 933 | 486 | `dispatch` closure + `make_bgl`/`make_bg` helpers (-48%) |
| `lu_gpu.rs` | 780 | 302 | Static `make_bgl`/`make_pipe`/`make_bg`/`dispatch` (-61%) |
| `svd_gpu.rs` | 764 | 305 | Same helpers; BGL via type-slice (-60%) |
| `esn_v2.rs` | 884 | 842 | `validate_config()` + `expect_size()` helpers |
| `cache_hierarchy.rs` | 638 | 607 | `bgl_entry` closure + table-driven substrate classification |

### GPU Path Completion ✅

- **MorseForceF64** — 2-pass GPU shader dispatch (per-bond + reduce-to-particle)
- **BornMayerForceF64** — N-body direct GPU shader dispatch
- **Unsafe evolution** — `NonNull::new_unchecked` → safe `NonNull::new().expect()`

---

## Session 29 Evolutions (Feb 21, 2026) ✅

### Code Size — Smart Module Extraction ✅

| File | Before | After | Technique |
|------|--------|-------|-----------|
| `svd_gpu.rs` | 973 lines | 842 lines | `make_pipeline` + `dispatch` closures deduplicate 7+7 blocks |
| `session/mod.rs` | 968 lines | 569 lines | Dispatch logic extracted to `session/dispatch.rs` (420 lines) |
| `tensor/mod.rs` | 948 lines | 799 lines | Scalar ops + random gen extracted to `tensor/ops.rs` (121 lines) |
| `math_f64.wgsl` | 1002 lines | 837 lines | Special functions (gamma/erf/bessel) → `math_f64_special.wgsl` (175 lines) |

All files safely under the 1000-line limit. No files over 950 lines remain.

### Production Safety ✅

- `gpu_executor.rs`: 3 `try_into().unwrap()` calls replaced with explicit array indexing
- Improved SAFETY documentation on `Send`/`Sync` impls in `AlignedBuffer` and `PinnedMemory`
- Removed unused `PhantomData<Arc<()>>` and dead `Arc` import from `PinnedMemory`

### Hardcoded Paths Evolved ✅

| File | Before | After |
|------|--------|-------|
| `server/capabilities/mod.rs` | `/tmp` fallback | `runtime_base_dir()` → `XDG_RUNTIME_DIR` / `std::env::temp_dir()` |
| `runtime/edge/src/lib.rs` | `/tmp/cache` | `std::env::temp_dir().join("toadstool-edge-cache")` |

### Dependency Evolution ✅

| Removed Dep | Crate(s) | Replacement |
|-------------|----------|-------------|
| `once_cell` | workspace root, `toadstool-config` | `std::sync::LazyLock` (Rust 1.80+) |
| `lazy_static` | `security-policies` | `std::sync::LazyLock` (Rust 1.80+) |
| `tempdir` | `toadstool-testing` | `tempfile` (already a dep) |
| `term_size` | `toadstool-cli` | Unused; `console` already in deps |
| `base64` 0.21 | `cli`, `client`, `nestgate` | Unified to 0.22; removed from `client`/`nestgate` (unused) |
| `mdns` | workspace root, `runtime-edge` | Standardized on `mdns-sd`; edge stub never used it |
| `dashmap` | `distributed`, `runtime-gpu` | Evolved to `std::sync::RwLock<HashMap>` (no source usage) |
| `which` | `toadstool-cli` | CLI uses shell `which`, not the crate |

---

## Session 28 Evolutions (Feb 21, 2026) ✅

### Production Safety — RwLock Poison Recovery ✅

`pipeline_cache.rs`: All 12 `expect("poisoned")` calls replaced with `read_or_recover()` / `write_or_recover()` helpers. Consistent with existing `probe.rs::lock_cache` pattern. Caches safely continue after previously panicked threads.

### Code Size — Smart Deduplication ✅

| File | Before | After | Technique |
|------|--------|-------|-----------|
| `lu_gpu.rs` | 996 lines | 854 lines | `build_lu_pipeline()` deduplicates 4 pipeline helpers |

### Hardcoded Values Evolved ✅

| Category | Files Changed | Evolution |
|----------|--------------|-----------|
| Fallback ports | `primal_discovery_complete.rs` | Raw `8080`/`8081`/`8082` → named constants |
| Runtime paths | `connection.rs`, `format.rs` | `/tmp` → `std::env::temp_dir()` + `BIOMEOS_RUNTIME_DIR` |
| Config paths | `service.rs` | `/etc/biomeos/` → XDG cascade (`XDG_CONFIG_HOME` → `HOME/.config`) |
| Hostname | `manual_jsonrpc/mod.rs` | `/etc/hostname` → `HOSTNAME` env var first |
| GPU estimates | `gpu_executor.rs` | Magic numbers → `capability_defaults` module |

### Placeholder Evolution ✅

ML model placeholders (`vision.rs`, `whisper.rs`, `bert.rs`) evolved from silently returning empty results to returning `Error::NotImplemented` with descriptive messages. API surface remains stable; honest error handling instead of fake success.

---

## Session 27 Evolutions (Feb 21, 2026) ✅

### wetSpring/neuralSpring Full Shader Absorption ✅

Absorbed all remaining WGSL shaders and Rust implementations from wetSpring v5 and neuralSpring metalForge handoffs:

| Component | Files | Tests | Provenance |
|-----------|-------|-------|------------|
| Bio/genomics shaders | 8 WGSL | — | wetSpring + neuralSpring |
| ML/evolution shaders | 3 WGSL | — | wetSpring + neuralSpring |
| Numerical shaders | 1 WGSL | — | neuralSpring |
| Math/distance shaders | 3 WGSL | — | neuralSpring |
| Reduce shaders | 1 WGSL | — | neuralSpring |
| Spectral shaders | 1 WGSL | — | neuralSpring |
| Householder+QR eigensolver | 1 Rust | 9 | neuralSpring (S-12) |
| **Total** | **18 files** | **9 tests** | |

### NVVM Ada Lovelace Driver Fix ✅

Fixed `needs_f64_exp_log_workaround()` to correctly return `true` for NVIDIA proprietary driver on Ada Lovelace (RTX 40xx). NVVM/PTXAS cannot compile native f64 transcendentals (exp, log, pow) on SM89. Added `NvvmAdaF64Transcendentals` workaround variant and `is_nvidia_ada_lovelace()` detection.

### Cumulative Shader Math Library (Sessions 26–27)

| Domain | Shaders | Source |
|--------|---------|--------|
| Lattice QCD | 5 | hotSpring |
| Spectral theory | 1 + 6 Rust | hotSpring |
| ESN (reservoir computing) | 2 | hotSpring |
| Bio/genomics | 14 | wetSpring + prior |
| ML/evolution | 3 | neuralSpring |
| Numerical ODE | 1 | neuralSpring |
| Math/distance | 3 | neuralSpring |
| Reduce | 1 | neuralSpring |
| Spectral (IPR) | 1 | neuralSpring |
| **Total new** | **31 WGSL + 7 Rust** | |

---

## Session 26 Evolutions (Feb 21, 2026) ✅

### hotSpring v0.6.0 Shader Math Absorption ✅

Absorbed spectral theory primitives from hotSpring v0.6.0 (commit `6bd0047`) to complete the pure shader math library:

| Component | Files | Tests | Description |
|-----------|-------|-------|-------------|
| Spectral theory module | 6 Rust files | 19 | Lanczos, Sturm, Anderson 1D/2D/3D, Hofstadter |
| ESN shaders | 2 WGSL shaders | — | Reservoir update + readout for reservoir computing |

**New modules in `barracuda/src/spectral/`**:
- `lanczos.rs` — Lanczos tridiagonalization with full reorthogonalization
- `tridiag.rs` — Sturm bisection eigensolve for symmetric tridiagonal matrices
- `anderson.rs` — Anderson localization (1D/2D/3D), Lyapunov exponent
- `hofstadter.rs` — Almost-Mathieu operator, Hofstadter butterfly
- `stats.rs` — Level spacing ratio (Poisson/GOE), band detection
- `sparse.rs` — `SpectralCsrMatrix` + GPU `WGSL_SPMV_CSR_F64` shader

**New shaders in `barracuda/src/shaders/ml/`**:
- `esn_reservoir_update.wgsl` — Fused matmul + leaky tanh for ESN
- `esn_readout.wgsl` — Readout matrix-vector product

**Previously absorbed from hotSpring**:
- `complex_f64.wgsl`, `su3.wgsl` — Complex f64 and SU(3) math
- `wilson_plaquette_f64.wgsl` — Wilson gauge plaquette
- `su3_hmc_force_f64.wgsl`, `higgs_u1_hmc_f64.wgsl` — Lattice QCD HMC
- CellListGpu fix (Session 25), GPU FFT f64

**Remaining in hotSpring** (domain-specific):
- Nuclear HFB shaders (`batched_hfb_*.wgsl`, `deformed_*.wgsl`)
- Physics validation suites (18 papers, 33/33 validation)

---

## Session 25 Evolutions (Feb 21, 2026) ✅

### Unit Test Coverage Expansion — 172 New Tests ✅

Comprehensive unit test additions across core modules to improve coverage toward 90% target:

| Module | Tests Added | Description |
|--------|-------------|-------------|
| `toadstool-common/service_discovery/endpoint.rs` | 13 | URL parsing for `ServiceEndpoint::from_url_string()` |
| `barracuda/ops/expand/compute.rs` | 19 | Broadcasting shape computation, stride calculations |
| `barracuda/dispatch/config.rs` | 14 | Dispatch thresholds, GPU routing, force CPU/GPU |
| `barracuda/workload.rs` | 27 | Workload classification, sparsity analysis, device selection |
| `barracuda/resource_quota.rs` | 22 | Quota tracking, VRAM limits, device requirements |
| `barracuda/numerical/rk45.rs` | 16 | ODE solver config builders, error paths, max steps |
| `toadstool/composition_constraints/constraint.rs` | 8 | Hard/soft constraint classification, serialization |
| `toadstool/composition_constraints/evaluation.rs` | 8 | Satisfaction scoring, constraint evaluation |
| `toadstool/composition_constraints/request.rs` | 13 | Composition requests, priorities, metadata |
| `toadstool/universal/types.rs` | 16 | `SecurityLevel`, `PrimalType`, `NetworkLocation`, `PrimalContext` |
| `toadstool/execution.rs` | 16 | `ExecutionStatus`, `RuntimeType`, `ExecutionInput/Output` |

**New builder methods added to `Rk45Config`**:
- `with_max_steps(usize)` — Set maximum number of integration steps
- `with_safety(f64)` — Set safety factor for step size adjustment

**Bug fixes**:
- Fixed unused import warning in `ipc/server.rs`
- Fixed case-sensitivity in `runtime.rs` test error message matching

All 172 new tests pass. Tests focus on pure CPU logic, serialization roundtrips, builder patterns, error handling paths, and boundary conditions.

---

## Session 24 Evolutions (Feb 20, 2026) ✅

### Integration Test Graduation — 3 More Suites ✅ (D-S18-003 continued)

**`error_paths_discovery_tests.rs`** (10 tests):
- Rewrote using `toadstool::self_identity::{Capability, DiscoveredService}` (no `primal_identity` module exists)
- `SelfIdentity::discover().await` → `SelfIdentity::new()` (sync constructor)
- `DiscoveredService` fields aligned: added `version`, `protocols`, `last_seen`; removed `metadata`
- `Capability::from("x")` → struct literal with `name`, `version`, `features`, `characteristics`

**`fault_tests.rs`** (19 tests via `chaos/fault_injection.rs` + `chaos/resilience_tests.rs`):
- Built against real `toadstool_testing::chaos::{ChaosScenario, FaultType, ResourceType, SystemState}`
- `FaultType` variants corrected: `node_id`, `consumption_percent`, `loss_rate: f64`, `duration_ms`

**`security_tests.rs`** (13 tests via `security/penetration_tests.rs`):
- Capability boundary enforcement, privilege escalation resistance, `IsolationLevel` correctness
- `IsolationLevel::Strict` → `IsolationLevel::Enhanced` (actual variant)
- Empty-capabilities context: asserts `validate().is_err()` (correct; ≥1 cap required)

**167 integration tests, 0 failures.** Stale `pending/` copies of 8 already-graduated suites removed.

### D-S21-003 — wetSpring `gemm_cached.rs` Path Fragility ✅

- `wetSpring/barracuda/Cargo.toml`: `../../phase1/toadstool` → `../../phase1/toadStool` (Linux case fix)
- `gemm_cached.rs`: `include_str!("../../../../phase1/toadstool/...")` → `barracuda::ops::linalg::GemmF64::WGSL`
- `cargo check --features gpu` passes cleanly in wetSpring

---

## Sessions 22–23 Evolutions (Feb 20, 2026) ✅

### D-S17-002 — `capabilities.rs` Semantic Split ✅

`GpuDriverProfile`, `DriverKind`, `CompilerKind`, `GpuArch`, `Fp64Rate`, `Workaround`,
`EigensolveStrategy` extracted from the 929-line `capabilities.rs` into new `driver_profile.rs`.
`capabilities.rs` (505 lines) now exclusively covers hardware limits + wgpu dispatch helpers.
`pub use driver_profile::*` in `capabilities.rs` preserves all callers without path changes.

### D-S16-003 — `ParallelFilter` Two-Level Scan ✅

New `apply_l1_offsets` WGSL entry point (Pass C) in `prefix_sum.wgsl`.
`filter.rs` `execute()` auto-selects:
- n ≤ 65,536: existing 4-pass single-level (unchanged)
- 65,536 < n ≤ 16,777,216: new 6-pass two-level (local scan → L1 scan → offsets → apply → scatter)
- n > 16M: `BarracudaError::InvalidInput` (three-level left for genome-scale)

### Integration Tests Graduated (Sessions 22–23)

| Suite | Tests |
|---|---|
| `runtime_execution_tests.rs` | 20 |
| `error_handling_tests.rs` | — |
| `resource_requirements_tests.rs` | — |
| `security_context_tests.rs` | — |
| `config_management_tests.rs` | — |
| `evolution_fault_tests.rs` + `evolution_chaos_tests.rs` | — |

---

## Sessions 19–21 Evolutions (Feb 20, 2026) ✅

### neuralSpring Absorption (`TensorSession` ML ops)

`TensorSession` extended with `matmul`, `relu`, `gelu`, `softmax`, `layer_norm`, `reshape`,
`head_split`, `attention`, `head_concat` — covers all 11 neuralSpring handoff shortcomings.
6 new fused MLP/transformer tests passing. Equivalent to the 46–78× fused pipeline in
`neuralSpring/src/evolved/`. All session ops encode in one `CommandEncoder` / `queue.submit()`.

### GPU Architecture + Dispatch Hardening

- `capabilities.rs::classify_substrate()`: vendor-ID-first (VENDOR\_NVIDIA/AMD/INTEL/APPLE/ARM/QUALCOMM),
  string-name fallback retained for zero-vendor-ID Mesa/software drivers.
- `dispatch/benchmark.rs::check_gpu()` + `dispatch/config.rs::check_gpu_available()`:
  duplicated raw wgpu adapter setup consolidated to `WgpuDevice::new()`.

### `GemmCachedF64` Absorbed from wetSpring

`ops/linalg/gemm_f64.rs`: pre-compiled GEMM pipeline with GPU-resident weight matrix B.
Pipeline compiled once at `new()`, B uploaded once; subsequent `multiply()` calls dispatch per-A only.
**Measured**: 60× speedup on taxonomy dispatch (first: 60 ms → subsequent: <1 ms).
`GemmF64::WGSL` published as `pub const` — eliminates wetSpring's `include_str!` path hack.

---

## Session 18 Evolutions (Feb 20, 2026) ✅

### Sovereign Compute Phase 3 — Now Live in Hot Path ✅

`WgpuDevice::compile_shader_f64()` now runs a two-stage pipeline:
1. `ShaderTemplate::for_driver_auto()` — NVK/RADV exp/log workaround (existing)
2. `WgslOptimizer::optimize()` — `@ilp_region` ILP reorder + `@unroll_hint` loop unroll (new)

Fast path: zero-overhead when no annotations present (single `contains()` guard). The Jacobi
eigensolve shader fires the reorderer automatically on every compile, pre-scheduling DFMA
pairs for the actual GPU's cycle count (`GpuDriverProfile::latency_model()`).

### Apple M-Series GPU Architecture ✅

- `GpuArch::AppleM` — detects `"apple m"` / `"apple paravirtual"` adapter names
- `AppleMLatencyModel` — software-emulated f64 FMA ~16 cy, f32 ~4 cy (all WGSL ILP annotations honour this)
- `Fp64Rate::Software` for AppleM (no native f64 silicon on M-series)
- Cross-vendor latency matrix now complete: SM70–SM89, RDNA2/3/CDNA2, AppleM, Conservative

### GpuExecutor Zero-Copy Output Path ✅ (D-S16-001)

- `GpuTensorStorage.buffer: Arc<wgpu::Buffer>` — shared ownership instead of owned buffer
- `Tensor::from_arc_buffer(Arc<wgpu::Buffer>, ...)` — zero-copy Tensor construction
- `Tensor::try_arc_buffer() -> Option<Arc<wgpu::Buffer>>` — bridge for storage code
- `GpuTensorStorage::from_tensor()` — Owned path: `Arc::clone()` (0 bytes); Pooled path: `copy_buffer_to_buffer()` (GPU-to-GPU, no CPU)
- `execute()` no longer calls `to_vec()` + `write_from_cpu()` — the GPU→CPU→GPU round-trip is gone

### Integration Tests Crate ✅ (D-S16-004)

- `crates/integration-tests/` created and added to workspace
- 21 orphan `tests/*.rs` files migrated from workspace root
- 3 active suites: `chaos_engineering_scenarios`, `error_paths_config_tests`, `pure_rust_validation_tests` (13 pass, 7 ignored with explanations)
- 12 files quarantined to `tests/pending/` with `README.md` tracking unimplemented APIs
- Workspace `tests/` directory is now free of bare `.rs` files

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
- Session 25: ~65% lines (+172 new unit tests across 11 modules)

---

## Previous Evolutions (Feb 14–17, 2026)

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session detail of earlier evolutions including:
- cudarc 0.11 → 0.19 upgrade
- Clippy cleanup (44 auto-fixes)
- Deep debt evolution (pure Rust syscalls, timeout consolidation, SIMD runtime detection)
- Bug fixes from hotSpring/wetSpring validation
- Device registry with physical deduplication
- F64 unified math language suite
- GPU-resident pipeline
- MD pipeline (thermostats + PPPM)
- Unidirectional compute pipeline
- ecoBin compliance evolution

---

## Root Documentation

| File | Purpose |
|------|---------|
| `README.md` | Project overview, honest status |
| `STATUS.md` | This file — detailed status |
| `DOCUMENTATION.md` | Navigation hub |
| `QUICK_STATUS.md` | One-page summary |
| `QUICK_REFERENCE.md` | Commands and API reference |
| `DEEP_DEBT_STATUS.md` | Deep debt evolution status |
| `DEBT.md` | Active workarounds and evolution paths |
| `CHANGELOG.md` | Full session-by-session evolution history |

---

**Last Updated**: February 22, 2026 — Session 38: Zero clippy warnings, blind unwrap() audit, test race fix, 11 new behavioral tests, 589+ WGSL shaders, 3,847+ workspace tests passing.
