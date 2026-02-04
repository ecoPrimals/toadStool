# Hardware Discovery - ToadStool Universal Compute

**Purpose**: Detect and validate all compute substrates for heterogeneous validation.

## What It Does

1. **CPU Detection**: Detects dual-socket NUMA configuration
2. **GPU Detection**: Enumerates GPUs via WebGPU (NVIDIA, AMD)
3. **NPU Detection**: Detects BrainChip Akida neuromorphic processors
4. **Validation Check**: Determines readiness for cross-substrate validation

## Expected Output (Your System)

```
═══════════════════════════════════════════════════════════════
  ToadStool Universal Compute - Hardware Discovery
═══════════════════════════════════════════════════════════════

🔍 Detecting CPUs...
  ✅ CPU Socket 0: AMD EPYC 7452 32-Core Processor (32 cores, 64 threads)
  ✅ CPU Socket 1: AMD EPYC 7452 32-Core Processor (32 cores, 64 threads)

🔍 Detecting GPUs (via WebGPU)...
  ✅ GPU 0: NVIDIA GeForce RTX 3090 (Vulkan)
     Backend: Vulkan, Type: DiscreteGpu
  ✅ GPU 1: AMD Radeon RX 7xxx (Vulkan)
     Backend: Vulkan, Type: DiscreteGpu

🔍 Detecting NPUs (BrainChip Akida)...
  ✅ NPU 0: BrainChip Akida (PCI: a1:00.0)
  ✅ NPU 1: BrainChip Akida (PCI: e2:00.0)

═══════════════════════════════════════════════════════════════
  HARDWARE INVENTORY SUMMARY
═══════════════════════════════════════════════════════════════

  📊 CPUs: 2
     • Socket 0: 32 cores, 64 threads
     • Socket 1: 32 cores, 64 threads

  🎮 GPUs: 2
     • NVIDIA GeForce RTX 3090 (Vulkan)
     • AMD Radeon RX 7xxx (Vulkan)

  🧠 NPUs: 2
     • BrainChip Akida AKD1000 (PCI: a1:00.0)
     • BrainChip Akida AKD1000 (PCI: e2:00.0)

───────────────────────────────────────────────────────────────
  🔢 Total Substrates: 6
───────────────────────────────────────────────────────────────

  💾 Hardware inventory exported to: hardware_inventory.json

═══════════════════════════════════════════════════════════════
  VALIDATION READINESS
═══════════════════════════════════════════════════════════════

  ✅ Status: EXCELLENT (6+ substrates detected!)
     Heterogeneous validation ready:
     • 2 CPUs for reference baseline
     • 2 GPUs for cross-vendor comparison
     • 2 NPUs for neuromorphic validation

  🚀 Ready to validate "same math on any chip"!

═══════════════════════════════════════════════════════════════
```

## Usage

```bash
# Build
cargo build --release

# Run
cargo run --release

# Check JSON output
cat hardware_inventory.json
```

## Output Files

- `hardware_inventory.json` - Machine-readable inventory for validation scripts

## Next Steps

After confirming hardware detection:
1. Proceed to `02-validation/` for operation correctness tests
2. Then `03-performance/` for benchmarking suite
3. Finally `04-report/` for comprehensive validation report
