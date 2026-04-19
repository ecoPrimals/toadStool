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

## Step 5: (Optional) Start toadStool server for ecosystem demos

```bash
# In a separate terminal:
cd /path/to/toadStool
cargo run --release --bin toadstool -- server

# Then run ecosystem-integration demos:
cd showcase/03-ecosystem-integration/01-coordination-registration
cargo run --release
```

---

## What Next?

- Full local showcase: `showcase/00-local-primal/` (5 demos, 5 minutes)
- Ecosystem integration: `showcase/03-ecosystem-integration/` (requires running primals)

> **Note**: `showcase/01-shader-pipeline/` and `showcase/02-compute-patterns/` are **archived (S169)** — the APIs they demonstrate have been moved to coralReef and barraCuda. They are preserved as fossil reference only.

See [00_SHOWCASE_INDEX.md](00_SHOWCASE_INDEX.md) for the complete learning path.
