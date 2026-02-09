# NPU vs GPU Raytracing Comparison

Demonstrates the strengths of NPU (sparse, event-driven) vs GPU (dense, parallel) for raytracing workloads using the complete ToadStool + BarraCUDA stack.

## Overview

This showcase demonstrates:
1. **ToadStool hardware discovery** - Automatically finds NPU and GPU
2. **BarraCUDA shader execution** - GPU raytracing via WGSL
3. **NPU event-driven processing** - Sparse scene raytracing
4. **Performance comparison** - NPU vs GPU for different scene types

## Architecture

```
Application (Raytracing)
     ↓
ToadStool (discovers NPU + GPU)
     ↓
    / \
   ↓   ↓
 NPU   GPU (BarraCUDA WGSL shader)
```

## Quick Start

```bash
# Run comparison benchmark
./demo.sh

# Or run individually
cargo run --release --example compare_raytracing

# GPU only
cargo run --release --example gpu_raytrace

# NPU only (if available)
cargo run --release --example npu_raytrace
```

## Expected Results

### Sparse Scene (Few objects, many empty rays)
- **NPU**: Faster (event-driven, skips empty space)
- **GPU**: Slower (processes all rays regardless)

### Dense Scene (Many objects, all rays hit)
- **NPU**: Slower (overhead from event processing)
- **GPU**: Faster (parallel throughput dominates)

### Hybrid (Mixed density)
- **ToadStool**: Automatically selects best device
- **BarraCUDA**: Executes on selected hardware

## Implementation

### GPU Raytracing (WGSL Shader)
- Parallel ray traversal
- BVH acceleration structure
- Runs on any GPU via BarraCUDA/WGPU

### NPU Raytracing (Event-Driven)
- Sparse ray representation
- Event-based traversal
- Runs on Akida via dual-backend driver

## Performance Expectations

| Scene Type | NPU Advantage | GPU Advantage |
|------------|---------------|---------------|
| **Sparse** | ✅ 2-3x faster | ❌ Wastes compute |
| **Dense** | ❌ Event overhead | ✅ 5-10x faster |
| **Hybrid** | ✅ Adaptive | ✅ Consistent |

## Hardware Requirements

- **GPU**: Any GPU (WGPU handles all vendors)
- **NPU**: Akida AKD1000/AKD1500 (optional)
- **CPU**: Fallback always available

## Deep Debt Compliance

✅ **No Hardcoding**: ToadStool discovers hardware at runtime  
✅ **No Scripts**: Pure Rust raytracing implementations  
✅ **Capability-Based**: Selects best device for scene type  
✅ **Self-Adapting**: Works with whatever hardware available  

## See Also

- [ToadStool Architecture](../../../TOADSTOOL_ARCHITECTURE_FEB08_2026.md)
- [NPU Driver Architecture](../../../specs/NPU_DRIVER_ARCHITECTURE.md)
- [BarraCUDA Integration](../../../ARCHITECTURE_COMPLETE.md)
