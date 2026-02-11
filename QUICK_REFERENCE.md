# ToadStool Quick Reference

**February 11, 2026**

---

## Quality Gates

```bash
# All gates (run these before any commit)
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace
cargo test --workspace
```

---

## Build

```bash
# Full workspace
cargo build --release

# Specific crate
cargo build --release -p barracuda
cargo build --release -p toadstool-common
cargo build --release -p toadstool-server
cargo build --release -p toadstool-cli
```

---

## Test

```bash
# Full workspace (excludes hardware-dependent)
cargo test --workspace \
  --exclude ml-inference-showcase \
  --exclude homomorphic-computing \
  --exclude barracuda \
  --exclude toadstool-runtime-gpu \
  --exclude showcase-rbf-surrogate \
  --exclude akida-detection-demo

# Specific crate
cargo test -p toadstool-common
cargo test -p toadstool-server
cargo test -p toadstool-config
cargo test -p toadstool-cli

# BarraCUDA (lib tests work without GPU; shader tests require GPU)
cargo test -p barracuda --lib
cargo test -p barracuda --lib ops::linalg --release
cargo test -p barracuda --lib linalg numerical special optimize surrogate --release

# Coverage (requires cargo-llvm-cov)
cargo llvm-cov -p toadstool-server --lib
cargo llvm-cov -p toadstool-common --lib
cargo llvm-cov -p toadstool-config --lib
cargo llvm-cov --workspace --ignore-run-fail \
  --exclude ml-inference-showcase \
  --exclude homomorphic-computing \
  --exclude barracuda \
  --exclude toadstool-runtime-gpu
```

---

## Showcases

```bash
# RBF surrogate learning
cd showcase/rbf-surrogate && ./demo.sh

# NPU detection
cd showcase/neuromorphic/01-akida-detection && ./demo.sh
```

---

## JSON-RPC Methods (26 total)

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
| `compute.discover_capabilities` | List all available methods |
| `compute.submit` | Submit job (inference/transform/custom) with routing |
| `compute.status` | Check job status |
| `compute.result` | Get completed job result |
| `compute.cancel` | Cancel pending/running job |
| `compute.list` | List all jobs (optional state filter) |

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

### Cross-Gate (`gate.*`)

| Method | Description |
|--------|-------------|
| `gate.update` | Register/update remote gate GPU capabilities |
| `gate.remove` | Remove offline gate |
| `gate.list` | List all known gates |
| `gate.route` | Preview routing decision for a model |

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
use toadstool_common::primal_sockets::discover_crypto_socket;

// Discovers ANY crypto service (beardog, HSM, KMS, etc.)
let socket = discover_crypto_socket().await?;
```

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

## Port Configuration

```bash
# Environment overrides (all optional)
TOADSTOOL_SERVER_PORT=8084
TOADSTOOL_GPU_PORT=8085
TOADSTOOL_DISTRIBUTED_PORT=8086
TOADSTOOL_METRICS_PORT=9090
```

Default ports: 8084 (server), 8085 (GPU), 8086 (distributed), 9090 (metrics).
Named constant: `toadstool_common::constants::network::DEFAULT_HTTP_PORT`

---

## Key Crates

| Crate | Purpose |
|-------|---------|
| `toadstool-common` | Shared types, constants, discovery, IPC client |
| `toadstool-config` | Centralized config, ports, network |
| `toadstool` | Core runtime, IPC server/client, scheduler |
| `toadstool-server` | JSON-RPC server, GPU job queue, Ollama, cross-gate router |
| `toadstool-api` | REST API, middleware, WebSocket handlers |
| `toadstool-cli` | UniBin CLI, daemon, ecosystem integration |
| `barracuda` | 414 WGSL shaders, tensor ops, device management, hardware routing |
| `toadstool-distributed` | Multi-gate coordination, crypto integration |
| `toadstool-testing` | Chaos, fault, property, performance testing |

---

## Scientific Computing Middleware API

### Linear Algebra

```rust
use barracuda::linalg::solve_f64;

// Solve Ax = b (Gauss-Jordan with partial pivoting)
let a = vec![2.0, 1.0, 1.0, 3.0];  // Row-major 2×2
let b = vec![5.0, 8.0];
let x = solve_f64(&a, &b, 2)?;
```

### Numerical Methods

```rust
use barracuda::numerical::{gradient_1d, trapz};

// Finite-difference gradient (3-point stencil)
let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];  // y = x²
let dy_dx = gradient_1d(&y, 1.0);  // dy/dx ≈ 2x

// Trapezoidal integration
let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
let integral = trapz(&y, &x)?;  // ∫ x² dx
```

### Special Functions

```rust
use barracuda::special::{gamma, factorial};

// Gamma function (Lanczos approximation, 15 digits)
let g = gamma(5.0);  // Γ(5) = 4! = 24

// Factorial (exact + Stirling)
let f = factorial(10);  // 10! = 3628800
```

### Optimization

```rust
use barracuda::optimize::{nelder_mead, multi_start_nelder_mead, bisect};

// Local: Nelder-Mead simplex
let f = |x: &[f64]| (x[0] - 2.0).powi(2) + (x[1] - 3.0).powi(2);
let (x_best, f_best, n_evals) = nelder_mead(
    f,
    &[0.0, 0.0],
    &[(-10.0, 10.0), (-10.0, 10.0)],
    1000,
    1e-8,
)?;

// Global: Multi-start NM (like SparsitySampler)
// Returns best + ALL evaluations (for surrogate training)
let (best, cache, all_results) = multi_start_nelder_mead(
    f,
    &[(-10.0, 10.0), (-10.0, 10.0)],
    16,    // n_starts (LHS initial guesses)
    1000,  // max_iter per start
    1e-8,  // tolerance
    42,    // seed
)?;

// Use evaluation cache for RBF surrogate training
let (x_data, y_data) = cache.training_data();

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
    &x_train,
    &y_train,
    RBFKernel::ThinPlateSpline,
    1e-12,  // smoothing
)?;

// Predict at new points
let y_pred = surrogate.predict(&[vec![1.5]])?;
```

### Sampling

```rust
use barracuda::sample::{latin_hypercube, random_uniform};

// Latin Hypercube: space-filling, one sample per interval
let bounds = vec![(-5.0, 5.0), (-5.0, 5.0)];
let lhs_points = latin_hypercube(1000, &bounds, 42)?;

// Uniform random: simple baseline
let rng_points = random_uniform(1000, &bounds, 42);
```

---

## Hardware Routing API

```rust
use barracuda::device::{Device, WorkloadHint};

// Auto-routing (BarraCUDA picks the best device)
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
| [DOCUMENTATION.md](DOCUMENTATION.md) | Navigation hub |
| [QUICK_STATUS.md](QUICK_STATUS.md) | One-page summary |

---

## Documentation

| File | What |
|------|------|
| [README.md](README.md) | Overview, architecture, status |
| [STATUS.md](STATUS.md) | Detailed technical status |
| [DOCUMENTATION.md](DOCUMENTATION.md) | Navigation hub |
| [QUICK_STATUS.md](QUICK_STATUS.md) | One-page summary |
| [QUICK_REFERENCE.md](QUICK_REFERENCE.md) | This file (API reference) |

### Scientific Middleware Docs

| File | What |
|------|------|
| `docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md` | Full implementation guide |
| `docs/PHASE1_COMPLETION_REPORT.md` | Validation and metrics |
| `docs/MIDDLEWARE_COMPLETION_SUMMARY.md` | Technical summary |
| `DEEP_DEBT_STATUS.md` | Deep debt compliance |

---

**Last Updated**: February 11, 2026
