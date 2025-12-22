# Akida Detection & Integration Demo

## Overview

This demo showcases ToadStool's ability to detect and integrate BrainChip Akida PCIe boards into the universal mesh. It demonstrates:

1. PCIe device enumeration
2. Akida board identification
3. Capability registration with UniversalSubstrate
4. Health monitoring and diagnostics
5. Multi-board management (2x on Strandgate, 1x on Southgate)

## Architecture

### Detection Flow

```
System Boot
    ↓
PCIe Bus Scan (lspci)
    ↓
Identify Akida Devices (Vendor ID: 0x1E7C, Device ID: 0x0001)
    ↓
Query Board Capabilities (via Akida SDK)
    ↓
Register with UniversalSubstrate
    ↓
Available for Workload Scheduling
```

### Integration Points

1. **PCIe Layer**: Direct device communication via `/dev/akida*`
2. **UniversalSubstrate**: Neuromorphic platform registration
3. **Scheduler**: Workload routing based on capabilities
4. **Monitoring**: Health checks and performance metrics

## Hardware Detection

### PCIe Identification

Akida boards appear as PCIe devices:
```
00:01.0 Processing accelerators: BrainChip Inc. Akida AKD1000 PCIe Board
    Subsystem: BrainChip Inc. Akida Development Kit
    Vendor: 0x1E7C (BrainChip)
    Device: 0x0001 (Akida AKD1000)
```

### Board Capabilities

Each Akida board provides:
- **NPU Count**: 80 neural processing units
- **Memory**: 10MB on-chip SRAM
- **Power**: 1-10W TDP
- **PCIe**: Gen2 x4 (2 GB/s bandwidth)
- **Supported Models**: Spiking Neural Networks (SNNs)
- **Max Input Rate**: Event-driven (no fixed rate)

## Demo Structure

### Files

```
01-akida-detection/
├── README.md                    (this file)
├── demo.sh                      (run all detection demos)
├── examples/
│   ├── detect_akida.rs         (basic PCIe detection)
│   ├── enumerate_boards.rs     (multi-board management)
│   ├── query_capabilities.rs   (detailed board info)
│   └── health_check.rs         (diagnostics and monitoring)
├── src/
│   ├── lib.rs                  (detection library)
│   ├── pcie_scan.rs            (PCIe bus scanning)
│   ├── akida_device.rs         (device abstraction)
│   └── substrate_integration.rs (UniversalSubstrate binding)
└── Cargo.toml
```

## Running the Demo

### Prerequisites

```bash
# Install Akida SDK (when hardware arrives)
# wget https://shop.brainchipinc.com/downloads/akida-sdk-linux.tar.gz
# tar xzf akida-sdk-linux.tar.gz
# cd akida-sdk && ./install.sh

# For now, we'll use mock detection for showcase
```

### Basic Detection

```bash
cd showcase/neuromorphic/01-akida-detection
cargo run --example detect_akida
```

Expected output:
```
Scanning PCIe bus for Akida devices...
Found 3 Akida board(s):
  
  Board 0: Akida AKD1000 @ PCI 0000:01:00.0 (Strandgate)
    NPUs: 80
    Memory: 10MB
    Power: 1.2W (current)
    Status: Healthy
    
  Board 1: Akida AKD1000 @ PCI 0000:02:00.0 (Strandgate)
    NPUs: 80
    Memory: 10MB
    Power: 0.8W (current)
    Status: Healthy
    
  Board 2: Akida AKD1000 @ PCI 0000:03:00.0 (Southgate - via network)
    NPUs: 80
    Memory: 10MB
    Power: 1.5W (current)
    Status: Healthy

Registering with UniversalSubstrate...
✓ All boards registered successfully

Total neuromorphic compute capacity:
  - 240 NPUs
  - 30MB total SRAM
  - ~3W total power consumption
  - 3 independent boards for redundancy
```

### Multi-Board Enumeration

```bash
cargo run --example enumerate_boards
```

Expected output:
```
Enumerating all Akida boards across mesh...

Local boards (Strandgate):
  akida0: 0000:01:00.0
  akida1: 0000:02:00.0

Remote boards:
  southgate.local/akida0: 0000:03:00.0

Board topology:
  Strandgate (PCIe lanes: 128)
    ├── Slot 1: akida0 (PCIe Gen2 x4)
    └── Slot 2: akida1 (PCIe Gen2 x4)
  
  Southgate (PCIe lanes: 24)
    └── Slot 1: akida0 (PCIe Gen2 x4)

Optimal workload distribution:
  - Dense neuromorphic compute → Strandgate (2 boards, low latency)
  - Real-time classification → Southgate (1 board, near GPU)
  - Fault tolerance → All 3 boards (automatic failover)
```

### Capability Query

```bash
cargo run --example query_capabilities
```

### Health Check

```bash
cargo run --example health_check
```

Expected output:
```
Akida Health Check Report

Board 0 (Strandgate):
  ✓ PCIe link: Gen2 x4 (nominal)
  ✓ Memory test: PASSED
  ✓ NPU test: 80/80 operational
  ✓ Temperature: 42°C (nominal)
  ✓ Power: 1.2W (nominal)
  ⚠ Uptime: 2h 15m (recent boot)
  
Board 1 (Strandgate):
  ✓ PCIe link: Gen2 x4 (nominal)
  ✓ Memory test: PASSED
  ✓ NPU test: 80/80 operational
  ✓ Temperature: 38°C (nominal)
  ✓ Power: 0.8W (nominal)
  ✓ Uptime: 15d 8h (stable)
  
Board 2 (Southgate - remote):
  ✓ Network latency: 0.3ms (LAN)
  ✓ PCIe link: Gen2 x4 (nominal)
  ✓ Memory test: PASSED
  ✓ NPU test: 80/80 operational
  ✓ Temperature: 45°C (gaming active)
  ✓ Power: 1.5W (higher due to workload)
  ✓ Uptime: 3d 12h (stable)

Overall Status: HEALTHY
All boards operational and ready for workload scheduling.
```

## Integration with UniversalSubstrate

### Rust API

```rust
use toadstool_distributed::universal::{
    UniversalSubstrateCapabilities,
    NeuromorphicPlatform,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Detect all platforms including Akida
    let substrate = UniversalSubstrateCapabilities::detect_all().await?;
    
    // Filter for neuromorphic platforms
    let akida_boards: Vec<_> = substrate
        .neuromorphic_platforms
        .iter()
        .filter(|p| {
            matches!(p, NeuromorphicPlatform::NeuromorphicChip { 
                chip_name, .. 
            } if chip_name.contains("Akida"))
        })
        .collect();
    
    println!("Found {} Akida board(s)", akida_boards.len());
    
    for (i, board) in akida_boards.iter().enumerate() {
        if let NeuromorphicPlatform::NeuromorphicChip {
            chip_name,
            manufacturer,
            core_count,
            power_consumption_mw,
            ..
        } = board {
            println!("Board {}: {} by {}", i, chip_name, manufacturer);
            println!("  NPUs: {}", core_count);
            println!("  Power: {:.1}W", power_consumption_mw / 1000.0);
        }
    }
    
    Ok(())
}
```

### Workload Scheduling

```rust
use toadstool_runtime::scheduler::WorkloadScheduler;
use toadstool_runtime::workload::{Workload, WorkloadHints};

// Create a workload that prefers neuromorphic compute
let workload = Workload::builder()
    .name("kmer-filter")
    .executable("/usr/local/bin/kmer-filter")
    .hints(WorkloadHints {
        prefer_neuromorphic: true,
        max_latency_ms: Some(10),
        power_budget_watts: Some(2.0),
        ..Default::default()
    })
    .build()?;

// Scheduler will automatically route to Akida if available
let placement = scheduler.schedule(workload).await?;
println!("Workload placed on: {}", placement.node_name);
```

## Technical Details

### PCIe Communication

The Akida SDK provides:
- `/dev/akida0`, `/dev/akida1`, etc. device files
- IOCTL interface for board control
- Shared memory regions for model/data transfer
- Event notification for inference completion

### Memory Layout

```
Akida Board Memory:
├── Model Storage: Up to 9MB
│   └── Loaded SNN model weights
├── Input Buffer: ~512KB
│   └── Incoming event streams
├── Output Buffer: ~256KB
│   └── Classification results
└── System Reserved: ~256KB
    └── Firmware, control structures
```

### Power States

1. **Idle**: 0.1-0.3W (waiting for input)
2. **Active**: 1-3W (processing events)
3. **Peak**: Up to 10W (all NPUs active)
4. **Sleep**: <0.05W (can wake in <1ms)

## Troubleshooting

### Board Not Detected

```bash
# Check PCIe bus
lspci | grep -i brainchip

# Check device files
ls -l /dev/akida*

# Check kernel module
lsmod | grep akida

# Check SDK installation
akida-util --version
```

### Performance Issues

```bash
# Check PCIe link speed
lspci -vv -s 01:00.0 | grep LnkSta

# Should show: Speed 5GT/s, Width x4
# If degraded, check PCIe slot assignment
```

### Multi-Board Conflicts

```bash
# Ensure each board has unique device ID
akida-util list-devices

# Check for PCIe address conflicts
dmesg | grep akida
```

## Next Steps

After confirming detection:
1. Load a simple SNN model → `02-akida-bioinformatics/`
2. Run inference benchmarks → `03-akida-llm-intent/`
3. Test mesh orchestration → `04-akida-mesh/`

## References

- BrainChip Akida SDK Documentation
- ToadStool UniversalSubstrate API docs
- PCIe device programming guide
- Spiking Neural Network primer

---

**Status**: 🟡 Ready for hardware testing

