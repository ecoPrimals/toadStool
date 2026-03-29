# ToadStool Quick Reference

**March 29, 2026 — S164**

---

## Quality Gates

```bash
# All gates (run these before any commit)
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic
cargo doc --workspace --no-deps
cargo test --workspace
cargo llvm-cov --lib -p toadstool-common -p toadstool-config -p toadstool -p toadstool-server -p toadstool-distributed --summary-only
```

---

## Build

```bash
# Full workspace
cargo build --release

# Specific crate
cargo build --release -p toadstool-common
cargo build --release -p toadstool-server
cargo build --release -p toadstool-cli
cargo build --release -p toadstool-core
```

---

## Test

```bash
# Full workspace (runs concurrently)
cargo test --workspace -- --test-threads=8

# Specific crate
cargo test -p toadstool-common
cargo test -p toadstool-server
cargo test -p toadstool-config
cargo test -p toadstool-cli
cargo test -p toadstool-distributed
cargo test -p toadstool-core

# Coverage (requires cargo-llvm-cov)
cargo llvm-cov --workspace --ignore-run-fail

# Coverage (core crates combined)
cargo llvm-cov --lib -p toadstool-common -p toadstool-config -p toadstool -p toadstool-server -p toadstool-distributed --summary-only
```

---

## Showcases

```bash
# Local primal — hello compute
cd showcase/00-local-primal/01-hello-compute && ./demo.sh

# Shader pipeline — naga fallback
cd showcase/01-shader-pipeline/01-naga-fallback && ./demo.sh

# Compute patterns — capability discovery
cd showcase/02-compute-patterns/01-capability-discovery && ./demo.sh

# Ecosystem integration — songbird registration
cd showcase/03-ecosystem-integration/01-songbird-registration && ./demo.sh
```

---

## JSON-RPC Methods (95+ total, dynamically built)

### Core (`toadstool.*`)

| Method | Description |
|--------|-------------|
| `toadstool.health` | Health check (includes `error_count`, `uptime_secs`) |
| `toadstool.version` | Version and protocol info |
| `toadstool.query_capabilities` | Executor capabilities |

### Resources (`toadstool.resources.*`)

| Method | Description |
|--------|-------------|
| `toadstool.resources.estimate` | Estimate resource requirements for a graph |
| `toadstool.resources.validate_availability` | Check system can execute graph |
| `toadstool.resources.suggest_optimizations` | Suggest graph optimizations |

### Compute (`compute.*`)

| Method | Description |
|--------|-------------|
| `compute.health` | Health check |
| `compute.version` | Version info |
| `compute.capabilities` | Capability listing |
| `compute.discover_capabilities` | List all available methods |
| `compute.submit` | Submit job (inference/transform/custom) with routing |
| `compute.status` | Check job status |
| `compute.result` | Get completed job result |
| `compute.cancel` | Cancel pending/running job |
| `compute.list` | List all jobs (optional state filter) |

### Resources (`resources.*`) — biomeOS neural API aliases

| Method | Description |
|--------|-------------|
| `resources.estimate` | Estimate resource requirements |
| `resources.validate_availability` | Validate system can execute graph |
| `resources.suggest_optimizations` | Suggest graph optimizations |

### AI (`ai.*`) — biomeOS ai_local capability

| Method | Description |
|--------|-------------|
| `ai.local_inference` | Local inference |
| `ai.local_execute` | Local execution |

### Nautilus (`ai.nautilus.*`) — Evolutionary Reservoir Computing (feature-gated)

| Method | Description |
|--------|-------------|
| `ai.nautilus.status` | Brain status (observation count, shell generation, drift) |
| `ai.nautilus.observe` | Feed physics observation (beta, observable values) |
| `ai.nautilus.train` | Evolve shell on accumulated observations |
| `ai.nautilus.predict` | Predict dynamical observables for a beta value |
| `ai.nautilus.screen` | Score candidate beta values by information content |
| `ai.nautilus.edges` | Detect concept edges via leave-one-out analysis |
| `ai.nautilus.shell.export` | Serialize brain state to JSON |
| `ai.nautilus.shell.import` | Restore brain from serialized JSON |

### GPU (`gpu.*`)

| Method | Description |
|--------|-------------|
| `gpu.info` | GPU device info (wgpu backends) |
| `gpu.memory` | GPU memory usage |

### Ollama (`ollama.*`)

| Method | Description |
|--------|-------------|
| `ollama.list_models` | List available models |
| `ollama.inference` | Run model inference |
| `ollama.load` | Preload model into VRAM |
| `ollama.unload` | Free VRAM by unloading model |

### Transport (`transport.*`) — Hardware Transport Layer

| Method | Description |
|--------|-------------|
| `transport.discover` | Discover available hardware transports (display, capture, serial) |
| `transport.list` | List transports with optional direction/medium filters |
| `transport.route` | Route data from one transport to another (any-to-any) |

### Cross-Gate (`gate.*`)

| Method | Description |
|--------|-------------|
| `gate.update` | Register/update remote gate GPU capabilities |
| `gate.remove` | Remove offline gate |
| `gate.list` | List all known gates |
| `gate.route` | Preview routing decision for a model |

### Science (`science.*`)

| Method | Description |
|--------|-------------|
| `science.compute` | Submit scientific compute workload |
| `science.status` | Check scientific compute status |
| `science.result` | Retrieve scientific compute result |
| `science.gpu.dispatch` | GPU scientific dispatch |
| `science.gpu.capabilities` | GPU capabilities for science (precision routing) |
| `science.npu.dispatch` | NPU scientific dispatch |
| `science.npu.capabilities` | NPU capabilities for science |
| `science.substrate.discover` | Discover compute substrates |
| `science.substrate.probe` | Probe substrate capabilities |
| `science.activations.list` | List available activation functions (barraCuda Sprint 2) |
| `science.rng.capabilities` | RNG backend capabilities (CPU LCG, GPU xoshiro128**) |
| `science.special.functions` | Special function catalog (tridiagonal QL, Anderson, plasma dispersion, Hill, PK Monte Carlo) |

### Shader Compilation (`shader.compile.*`) — coralReef Proxy

| Method | Description |
|--------|-------------|
| `shader.compile.wgsl` | Compile WGSL to native binary (proxy to coralReef, naga fallback) |
| `shader.compile.spirv` | Compile SPIR-V to native binary (proxy to coralReef, naga fallback) |
| `shader.compile.status` | Check compilation status |
| `shader.compile.capabilities` | Report available compilation pipelines and architectures |

### Provenance (`toadstool.provenance`)

| Method | Description |
|--------|-------------|
| `toadstool.provenance` | Cross-spring evolution flow matrix (17+ documented flows across 5 springs) |

---

## IPC Architecture

### Socket Paths (biomeOS Standard)

```
/run/user/$UID/biomeos/toadstool.sock         -- ToadStool (default)
/run/user/$UID/biomeos/toadstool-{family}.sock -- ToadStool (multi-family)
/run/user/$UID/biomeos/beardog.sock            -- BearDog (crypto)
/run/user/$UID/biomeos/songbird.sock           -- Songbird (coordination)
/run/user/$UID/biomeos/nestgate.sock           -- NestGate (storage)
/run/user/$UID/biomeos/nucleus.sock            -- NUCLEUS (orchestrator)
```

### JSON-RPC Method Naming

```
{domain}.{operation}[.{variant}]

Examples:
  compute.submit
  compute.discover_capabilities
  gpu.info
  ollama.inference
  gate.route
  toadstool.resources.estimate
```

### Discovery (Capability-Based)

```rust
use toadstool_common::primal_sockets::get_socket_path_for_capability;

// Discovers by capability name, not primal name (sovereignty-compliant)
let socket = get_socket_path_for_capability("crypto");
let socket = get_socket_path_for_capability("storage");
let socket = get_socket_path_for_capability("network");
```

> **Note**: `get_socket_path_for_service()` is deprecated since 0.92.0.
> Use `get_socket_path_for_capability()` for sovereignty-compliant discovery.

### Configuration (Parameter-Based)

```rust
use toadstool_common::primal_sockets::SocketPathEnv;

// Production: reads from environment once
let env = SocketPathEnv::from_env();
let dir = resolve_runtime_dir(&env);

// Testing: explicit values, zero env mutation
let env = SocketPathEnv {
    xdg_runtime_dir: Some("/run/user/1000".to_string()),
    ..Default::default()
};
```

---

## biomeOS Socket Standard

Socket path: `$XDG_RUNTIME_DIR/biomeos/toadstool.sock`

Environment variables:
| Variable | Default | Purpose |
|----------|---------|---------|
| `TOADSTOOL_SOCKET` | `$XDG_RUNTIME_DIR/biomeos/toadstool.sock` | Own socket path |
| `BEARDOG_SOCKET` | `$XDG_RUNTIME_DIR/biomeos/beardog.sock` | Security provider |
| `SONGBIRD_SOCKET` | `$XDG_RUNTIME_DIR/biomeos/songbird.sock` | Discovery |
| `FAMILY_ID` | (from `.family.seed`) | Deployment family |

Real-time events: WebSocket removed. Use `compute.status` polling.

---

## Port Configuration

```bash
# Environment overrides (all optional — defaults are 0 = OS-assigned)
TOADSTOOL_SERVER_PORT=9000
TOADSTOOL_GPU_PORT=9001
TOADSTOOL_DISTRIBUTED_PORT=9002
TOADSTOOL_METRICS_PORT=9090
```

Default ports: all 0 (OS-assigned at bind time). Override via env vars above.
Port constants: `toadstool_config::ports::toadstool::{SERVER, GPU_COMPUTE, DISTRIBUTED, METRICS}`
Port helpers: `toadstool_config::ports::{server_port(), gpu_compute_port(), distributed_port(), metrics_port()}`

---

## Key Crates

| Crate | Purpose |
|-------|---------|
| `toadstool-common` | Shared types, constants, discovery, IPC client |
| `toadstool-config` | Centralized config, ports, network |
| `toadstool` | Core runtime, IPC server/client, scheduler |
| `toadstool-server` | JSON-RPC server, GPU job queue, Ollama, cross-gate router |
| `toadstool-common` | Shared types, constants, universal adapter, primal discovery |
| `toadstool-cli` | UniBin CLI, daemon, ecosystem integration |
| `toadstool-core` | Generic hardware traits (NpuDispatch, NpuParameterController) |
| `toadstool-distributed` | Multi-gate coordination, crypto integration |
| `toadstool-testing` | Chaos, fault, property, performance testing |

---

## Scientific Computing Middleware API

> **Note**: Scientific computing middleware (linalg, numerical, special, stats, optimize, etc.) has moved to **barraCuda** (`ecoPrimals/barraCuda/`). See `cargo doc -p barracuda --open` in the barraCuda workspace. The examples below reference barraCuda APIs for convenience.

### Linear Algebra

```rust
use barracuda::linalg::solve_f64;
use barracuda::ops::linalg::{lu_decompose, qr_decompose, svd_decompose};

// Solve Ax = b (Gauss-Jordan with partial pivoting)
let a = vec![2.0, 1.0, 1.0, 3.0];  // Row-major 2×2
let b = vec![5.0, 8.0];
let x = solve_f64(&a, &b, 2)?;

// LU decomposition with pivoting (PA = LU)
let lu = lu_decompose(&a, 2)?;
let det = lu.det();           // Determinant
let inv = lu.inverse()?;      // Matrix inverse
let x = lu.solve(&b)?;        // Solve system

// QR decomposition (A = QR, Householder)
let qr = qr_decompose(&a, 2, 2)?;
let x = qr.solve_least_squares(&b)?;  // Least squares

// SVD decomposition (A = UΣVᵀ)
let svd = svd_decompose(&a, 2, 2)?;
let pinv = svd.pseudoinverse(1e-10);  // Moore-Penrose inverse
let rank = svd.rank(1e-10);           // Numerical rank
let cond = svd.condition_number();    // Condition number
```

### Numerical Methods

```rust
use barracuda::numerical::{gradient_1d, trapz};
use barracuda::numerical::rk45::{rk45, Rk45Config};

// Finite-difference gradient (3-point stencil)
let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];  // y = x²
let dy_dx = gradient_1d(&y, 1.0);  // dy/dx ≈ 2x

// Trapezoidal integration
let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
let integral = trapz(&y, &x)?;  // ∫ x² dx

// Adaptive RK45 ODE solver (dy/dt = f(t, y))
let f = |t: f64, y: &[f64]| vec![-0.5 * y[0]];  // Exponential decay
let config = Rk45Config::default();
let result = rk45(&f, 0.0, 10.0, &[1.0], &config)?;
```

### PDE Solvers

```rust
use barracuda::pde::crank_nicolson::{CrankNicolsonSolver, CrankNicolsonConfig, BoundaryCondition};

// 1D heat equation: ∂u/∂t = α ∂²u/∂x²
let config = CrankNicolsonConfig {
    nx: 100,
    dx: 0.01,
    dt: 0.0001,
    alpha: 0.01,
    theta: 0.5,  // Crank-Nicolson (0.5 = implicit/explicit blend)
    left_bc: BoundaryCondition::Dirichlet(0.0),
    right_bc: BoundaryCondition::Dirichlet(0.0),
};
let mut solver = CrankNicolsonSolver::new(config)?;
solver.step()?;  // Advance one timestep
```

### Special Functions

```rust
use barracuda::special::{gamma, lgamma, digamma, beta, factorial};
use barracuda::special::{erf, erfc, bessel_j0, bessel_i0};
use barracuda::special::{hermite, legendre, assoc_legendre, laguerre};

// Gamma and related
let g = gamma(5.0);       // Γ(5) = 24
let lg = lgamma(100.0);   // log Γ(100) (avoids overflow)
let psi = digamma(2.0);   // ψ(2) = 1 - γ ≈ 0.4227
let b = beta(2.0, 3.0);   // B(2,3) = Γ(2)Γ(3)/Γ(5)

// Error functions
let e = erf(1.0);         // erf(1) ≈ 0.8427
let ec = erfc(2.0);       // erfc(2) ≈ 0.0047

// Bessel functions
let j0 = bessel_j0(1.0);  // J₀(1) ≈ 0.7652
let i0 = bessel_i0(1.0);  // I₀(1) ≈ 1.2661

// Orthogonal polynomials (all WGSL shader-first)
let h5 = hermite(5, 1.0);           // H₅(1) = 41
let p3 = legendre(3, 0.5);          // P₃(0.5)
let p32 = assoc_legendre(3, 2, 0.5); // P₃²(0.5)
let l3 = laguerre(3, 0.0, 2.0);     // L₃(2) simple Laguerre
let l3a = laguerre(3, 0.5, 2.0);    // L₃^0.5(2) generalized
```

### Statistics

```rust
use barracuda::stats::{norm_cdf, norm_pdf, norm_ppf};
use barracuda::stats::{pearson_correlation, covariance, correlation_matrix};

// Normal distribution (standard N(0,1))
let p = norm_cdf(1.96);     // Φ(1.96) ≈ 0.975
let phi = norm_pdf(0.0);    // φ(0) ≈ 0.3989
let z = norm_ppf(0.975);    // Φ⁻¹(0.975) ≈ 1.96

// Correlation and covariance
let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
let r = pearson_correlation(&x, &y)?;  // r = 1.0 (perfect)
let cov = covariance(&x, &y)?;         // Cov(X,Y) = 5.0

// Correlation matrix (p×p for p variables)
let data = vec![
    vec![1.0, 2.0], vec![2.0, 4.0], vec![3.0, 6.0],
];
let corr = correlation_matrix(&data)?;  // 2×2 flattened
```

### Optimization

```rust
use barracuda::optimize::{nelder_mead, multi_start_nelder_mead, bisect};
use barracuda::optimize::bfgs::{bfgs, BfgsConfig};

// Local: Nelder-Mead simplex
let f = |x: &[f64]| (x[0] - 2.0).powi(2) + (x[1] - 3.0).powi(2);
let (x_best, f_best, n_evals) = nelder_mead(
    f, &[0.0, 0.0], &[(-10.0, 10.0), (-10.0, 10.0)], 1000, 1e-8,
)?;

// BFGS quasi-Newton (with gradient)
let grad = |x: &[f64]| vec![2.0 * (x[0] - 2.0), 2.0 * (x[1] - 3.0)];
let config = BfgsConfig::default();
let result = bfgs(&f, &grad, &[0.0, 0.0], &config)?;

// Global: Multi-start NM
let (best, cache, all_results) = multi_start_nelder_mead(
    f, &[(-10.0, 10.0), (-10.0, 10.0)], 16, 1000, 1e-8, 42,
)?;

// Root-finding: bisection
let root = bisect(|x| x * x - 2.0, 0.0, 2.0, 1e-10, 100)?;
```

### Surrogate Modeling

```rust
use barracuda::surrogate::{RBFSurrogate, RBFKernel};

// Train RBF surrogate
let x_train = vec![vec![0.0], vec![1.0], vec![2.0]];
let y_train = vec![0.0, 1.0, 4.0];

let surrogate = RBFSurrogate::train(
    &x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-12,
)?;

// Predict at new points
let y_pred = surrogate.predict(&[vec![1.5]])?;
```

### Sampling

```rust
use barracuda::sample::{latin_hypercube, random_uniform};
use barracuda::sample::sobol::sobol_sequence;

// Latin Hypercube: space-filling, one sample per interval
let bounds = vec![(-5.0, 5.0), (-5.0, 5.0)];
let lhs_points = latin_hypercube(1000, &bounds, 42)?;

// Sobol quasi-random: low-discrepancy sequence
let sobol_points = sobol_sequence(1000, 2)?;  // 1000 points in 2D

// Uniform random: simple baseline
let rng_points = random_uniform(1000, &bounds, 42);
```

### Mixing (SCF Solvers)

```rust
use barracuda::ops::mixing::{LinearMixer, BroydenMixer};

// Linear mixing: x_new = α*x_out + (1-α)*x_in
let mixer = LinearMixer::new(device, n_dim, 0.3)?;  // α = 0.3
let x_mixed = mixer.mix(&x_in, &x_out).await?;

// Modified Broyden II: accelerated SCF convergence
let mixer = BroydenMixer::new(device, n_dim, 0.3, 5)?;  // 5 history vectors
let residual = mixer.compute_residual(&x_in, &x_out).await?;
let x_next = mixer.update(&x_in, &residual).await?;
```

### Finite-Difference Gradients

```rust
use barracuda::ops::grid::{Gradient1D, Gradient2D, Laplacian2D, CylindricalGradient};

// 1D gradient (3-point central differences, 2nd-order boundaries)
let grad = Gradient1D::new(device, n_points)?;
let df_dx = grad.compute(&f_values, dx).await?;

// 2D gradient on structured grid
let grad_2d = Gradient2D::new(device, nx, ny)?;
let (df_dx, df_dy) = grad_2d.compute(&f_values, dx, dy).await?;

// Laplacian ∇²f
let lap = Laplacian2D::new(device, nx, ny)?;
let del2_f = lap.compute(&f_values, dx, dy).await?;

// Cylindrical coordinates (ρ, z)
let cyl = CylindricalGradient::new(device, n_rho, n_z)?;
let (df_drho, df_dz) = cyl.compute(&f_values, &rho_grid, d_rho, d_z).await?;
```

---

## Hardware Routing API

```rust
use barracuda::device::{Device, WorkloadHint};

// Auto-routing (barraCuda picks the best device via toadStool capabilities)
let device = Device::select_for_workload(&WorkloadHint::FFT);

// User override (force CPU even if GPU is available)
let device = Device::select_with_preference(
    Some(Device::CPU),
    &WorkloadHint::FFT,
);

// Explicit device context (bypasses routing entirely)
let ctx = DeviceContext::for_device(Device::NPU).await?;
```

**WorkloadHints**: `PhysicsForce`, `FFT`, `EigenDecomp`, `LinearSolve`, `Training`,
`Inference`, `PreScreen`, `SurrogateEval`, `MonteCarlo`, `SparseMath`, `Reservoir`,
`LargeMatrices`, `SparseEvents`, `EventProcessing`, `SmallWorkload`, `StringOps`, `General`.

---

## Constants

```rust
use toadstool_common::constants;

// JSON-RPC
constants::jsonrpc::VERSION          // "2.0"
constants::jsonrpc::error_codes::*   // PARSE_ERROR, METHOD_NOT_FOUND, etc.

// Network
constants::network::LOCALHOST_IPV4   // "127.0.0.1"
constants::network::BIND_ALL_IPV4    // "0.0.0.0"
constants::network::DEFAULT_HTTP_PORT // 8080

// Timeouts
constants::timeouts::*               // Connection, request, etc.
```

---

## Documentation

| File | What |
|------|------|
| [README.md](README.md) | Overview, architecture, status |
| [STATUS.md](STATUS.md) | Detailed technical status |
| [DEBT.md](DEBT.md) | Active debt register, evolution paths |
| [NEXT_STEPS.md](NEXT_STEPS.md) | Roadmap and upcoming work |
| [DOCUMENTATION.md](DOCUMENTATION.md) | Navigation hub |
| [QUICK_REFERENCE.md](QUICK_REFERENCE.md) | This file (API reference) |

### Scientific Middleware

Scientific middleware has moved to barraCuda. See `cargo doc -p barracuda --open` in the barraCuda workspace.

---

**Last Updated**: March 20, 2026 — S160
