# ToadStool Showcase Collection
## Pure Rust Hardware Discovery Demonstrations

**Status**: Active | **Updated**: March 3, 2026 — Session 94b

---

## Overview

ToadStool showcases demonstrate hardware discovery and compute orchestration.
Math/shader showcases (RBF surrogate, cross-platform GPU parity) have moved to
`ecoPrimals/fossil/toadStool/` — barraCuda is a separate primal.

---

## Showcases

### 1. Neuromorphic Computing (`neuromorphic/`)

- **01-akida-detection** — Hardware discovery for Akida NPUs
- **02-akida-bioinformatics** — NPU-accelerated genomics (k-mer filtering)
- **03-akida-llm-intent** — Intent classification on NPU
- **04-raytracing-comparison** — NPU vs GPU workload selection

### 2. GPU Universal (`gpu-universal/`)

Universal GPU operations via WGPU. Works on NVIDIA, AMD, Intel.

### 3. Homomorphic Computing (`homomorphic-computing/`)

FHE operations with GPU acceleration (NTT/INTT transforms).

### 4. Akida Characterization (`akida-characterization/`)

NPU performance characterization and benchmarks.

---

## Excluded from Workspace

All showcases are excluded from the main workspace build.
Build individually: `cd showcase/<name> && cargo build --release`

---

*See individual showcase READMEs for detailed instructions*
