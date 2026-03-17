# GPU Job Queue

ToadStool showcase demo: GPU job queue — submission, priority, status tracking, and capabilities.

## What It Demonstrates

- **GPU Capabilities**: Example `science.gpu.capabilities` response (backends, shader_models, max_workgroup_size)
- **Job Submission**: Three simulated jobs with different priorities:
  - Job 1: matrix_multiply (high, gpu_hint=true)
  - Job 2: fft_transform (normal, gpu_hint=true)
  - Job 3: data_reduction (low, gpu_hint=false, CPU fallback)
- **Queue State**: Simulated queue status (3 jobs, 1 running, 2 queued, ordered by priority)
- **Job Completion**: Jobs completing in priority order with simulated timing
- **NPU Capabilities**: Example `science.npu.capabilities` (akida availability by platform)
- **Summary**: "GPU job queue demonstrated — 3 jobs processed"

## How to Run

```bash
./demo.sh
```

Or manually:

```bash
cargo build --release
cargo run --release
```

## Expected Output

- Banner: "ToadStool Showcase: GPU Job Queue"
- GPU Capabilities: JSON with backends, shader_models, max_workgroup_size
- Job Submission: Three JSON-RPC 2.0 `science.gpu.dispatch` requests
- Queue State: 3 jobs, 1 running, 2 queued
- Job Completion: Three jobs with completion times
- NPU Capabilities: JSON with akida status
- Summary: "GPU job queue demonstrated — 3 jobs processed"

## Prerequisites

- Rust 1.85+

## ToadStool Capabilities Shown

| Crate | Capability |
|-------|------------|
| toadstool-common | generate_id |
