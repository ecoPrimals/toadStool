# Science Dispatch

ToadStool showcase demo: GPU compute job submission via science.* methods.

## What It Demonstrates

- **GPU Capabilities Query**: `science.gpu.capabilities` with wgpu backend details
- **Compute Job Submission**: `science.compute.submit` for matrix multiply
- **GPU Dispatch**: `science.gpu.dispatch` with operation, dimensions, precision
- **NPU Dispatch**: `science.npu.dispatch` for inference (Akida fallback to CPU)
- **Substrate Discovery**: `science.substrate.discover` listing CPU, GPU, NPU

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

- Banner: "ToadStool Showcase: Science Dispatch"
- GPU capabilities request/response
- Compute job submission request
- GPU dispatch request/response with timing
- NPU dispatch request/response (CPU fallback when Akida absent)
- Substrate discovery request/response
- Summary: "Science dispatch demonstrated — GPU, NPU, and substrate discovery"

## Prerequisites

- Rust 1.82+
- Primals optional (demo shows simulated responses when sockets unavailable)
