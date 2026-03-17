# ToadStool Showcase -- Quick Start

**Get running in 5 minutes.**

---

## Step 1: Verify Rust toolchain

```bash
rustc --version    # Need 1.85+
cargo --version
```

## Step 2: Run hello-compute (no server needed)

```bash
cd showcase/00-local-primal/01-hello-compute
cargo run --release
```

Expected output: toadStool primal info, hardware capabilities, compute substrates.

## Step 3: Run hardware discovery

```bash
cd ../02-hardware-discovery
cargo run --release
```

Expected output: CPU cores/brand, GPU adapters (if wgpu available), NPU status.

## Step 4: Run workload lifecycle

```bash
cd ../03-workload-lifecycle
cargo run --release
```

Expected output: Workload submission, status polling, result retrieval, cancellation.

## Step 5: (Optional) Start toadStool server for inter-primal demos

```bash
# In a separate terminal:
cd /path/to/toadStool
cargo run --release --bin toadstool -- serve

# Then run compute-pattern demos:
cd showcase/02-compute-patterns/01-capability-discovery
cargo run --release
```

---

## What Next?

- Full local showcase: `showcase/00-local-primal/` (5 demos, 5 minutes)
- Shader pipeline: `showcase/01-shader-pipeline/` (requires toadStool server)
- Compute triangle: `showcase/02-compute-patterns/` (toadStool + barraCuda + coralReef)
- Full ecosystem: `showcase/03-ecosystem-integration/` (all phase1 primals)

See [00_SHOWCASE_INDEX.md](00_SHOWCASE_INDEX.md) for the complete learning path.
