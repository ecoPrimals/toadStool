# Akida Neuromorphic Architecture Integration

## Technical Deep Dive

This document details the technical architecture for integrating BrainChip Akida PCIe boards with ToadStool's universal compute mesh.

---

## System Overview

### Hardware Stack

```
┌─────────────────────────────────────────────────────────────┐
│                      ToadStool Mesh                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Northgate   │  │  Strandgate  │  │  Southgate   │      │
│  │  RTX 5090    │  │  2x Akida    │  │  1x Akida    │      │
│  │  i9-14900K   │  │  RTX 3070    │  │  RTX 3090    │      │
│  │              │  │  Dual EPYC   │  │  5800X3D     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                           │                  │              │
│                     10GbE Ethernet                          │
└─────────────────────────────────────────────────────────────┘
```

### Software Stack

```
┌─────────────────────────────────────────────────────────────┐
│                   Application Layer                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Kraken2     │  │  LLM Server  │  │  Custom AI   │      │
│  │  Pipeline    │  │  (Ollama)    │  │  Workloads   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────────┐
│             ToadStool Universal Substrate                   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Workload Scheduler                                   │   │
│  │    • Capability-based routing                         │   │
│  │    • Multi-platform support                           │   │
│  │    • Fault tolerance                                  │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────────┐
│                Platform Abstraction Layer                   │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │    CPU     │  │    GPU     │  │ Neuromorphic│            │
│  │  (Native)  │  │  (CUDA/    │  │  (Akida)   │            │
│  │            │  │   WebGPU)  │  │            │            │
│  └────────────┘  └────────────┘  └────────────┘            │
└─────────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────────┐
│                   Hardware Layer                            │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │  x86_64    │  │  NVIDIA    │  │  BrainChip │            │
│  │  CPU       │  │  GPU       │  │  Akida     │            │
│  └────────────┘  └────────────┘  └────────────┘            │
└─────────────────────────────────────────────────────────────┘
```

---

## Akida Hardware Architecture

### AKD1000 Chip Specifications

**Neural Processing Units (NPUs)**: 80 cores  
**Neurons per NPU**: ~1,024  
**Synapses per NPU**: ~10,000  
**Total Neurons**: ~82,000  
**Total Synapses**: ~800,000  
**On-chip Memory**: 10MB SRAM  
**Interface**: PCIe Gen2 x4  
**Power**: 1-10W TDP (typically <2W)

### Memory Architecture

```
┌─────────────────────────────────────────────────────────┐
│             Akida Board Memory (10MB SRAM)              │
├─────────────────────────────────────────────────────────┤
│  Model Storage              │  ~9MB                     │
│  ─────────────────────────────────────────────────────  │
│  Input Buffer               │  ~512KB                   │
│  ─────────────────────────────────────────────────────  │
│  Output Buffer              │  ~256KB                   │
│  ─────────────────────────────────────────────────────  │
│  System/Firmware            │  ~256KB                   │
└─────────────────────────────────────────────────────────┘
```

### PCIe Communication

**Bus**: PCIe Gen2 x4  
**Bandwidth**: 2.0 GB/s (bidirectional)  
**Latency**: <100μs for small transfers  
**DMA**: Supported for bulk transfers  

---

## ToadStool Integration

### Detection Layer

**Location**: `showcase/neuromorphic/01-akida-detection/`

#### PCIe Enumeration

```rust
// Scan PCIe bus for Akida devices
pub fn scan_for_akida() -> Result<Vec<PcieDevice>> {
    // Use lspci or sysfs to enumerate devices
    let output = Command::new("lspci")
        .args(["-n", "-D"])
        .output()?;
    
    // Filter for BrainChip vendor ID (0x1E7C)
    // and Akida device ID (0x0001)
    parse_lspci_output(&output.stdout)
}
```

#### Device Management

```rust
pub struct AkidaBoard {
    pub index: usize,
    pub pcie_address: String,          // "0000:01:00.0"
    pub device_path: PathBuf,           // "/dev/akida0"
    pub npu_count: u32,                 // 80
    pub memory_bytes: u64,              // 10MB
    pub power_watts: f64,               // Current consumption
    pub temperature_celsius: f64,
    pub health: BoardHealth,
    pub node_name: Option<String>,      // For distributed mesh
}
```

### Registration with UniversalSubstrate

**Location**: `crates/distributed/src/universal/`

```rust
// Register Akida board as NeuromorphicPlatform
let platform = NeuromorphicPlatform::NeuromorphicChip {
    chip_name: "Akida AKD1000".to_string(),
    manufacturer: "BrainChip".to_string(),
    core_count: 80,
    neuron_count_per_core: 1024,
    synapse_count_per_core: 10_000,
    power_consumption_mw: 1200.0,
};

substrate.register_neuromorphic_platform(platform).await?;
```

### Workload Scheduling

**Location**: `crates/runtime/src/scheduler/`

#### Capability Matching

```rust
// Workload declares preferences
let workload = Workload::builder()
    .name("kmer-filter")
    .hints(WorkloadHints {
        prefer_neuromorphic: true,
        max_latency_ms: Some(10),
        power_budget_watts: Some(2.0),
        workload_type: WorkloadType::PatternMatching,
        ..Default::default()
    })
    .build()?;

// Scheduler routes to Akida if:
// 1. Workload is compatible (pattern matching, classification)
// 2. Akida boards are available and healthy
// 3. Performance requirements are met
// 4. Cost/power budget allows

let placement = scheduler.schedule(workload).await?;
```

#### Compatibility Rules

Akida is optimal for:
- ✅ Classification (image, text, intent)
- ✅ Pattern matching (k-mers, sequences, adapters)
- ✅ Event processing (DVS cameras, streaming data)
- ✅ Anomaly detection
- ✅ Small model inference (<9MB)
- ✅ Low-latency requirements (<1ms)
- ✅ Power-constrained environments

Akida is NOT suitable for:
- ❌ Matrix multiplication (use GPU)
- ❌ Large model inference (>10MB)
- ❌ Training (use GPU)
- ❌ Batch processing (GPU more efficient)
- ❌ General-purpose compute

---

## Akida Programming Model

### Spiking Neural Network (SNN)

Unlike traditional neural networks that use continuous activations, SNNs use discrete spikes (events) that propagate through the network.

#### Network Structure

```
Input Layer (Spike Encoding)
    ↓
Hidden Layers (Spiking Neurons)
    • Leaky Integrate-and-Fire (LIF)
    • Spike-Timing-Dependent Plasticity (STDP)
    ↓
Output Layer (Rate Coding)
    • Spike count = confidence
```

#### Training

1. **Convert Model**: Standard NN → SNN  
   Tools: Akida SDK, custom conversion scripts

2. **Quantize**: Float32 → Int8/Int4  
   Akida uses low-precision weights for efficiency

3. **Optimize**: Prune, compress, tune  
   Target: <9MB model size, >90% accuracy

4. **Export**: Save as `.akd` format  
   Binary format loadable to Akida board

### Inference Pipeline

```rust
// 1. Load model to board
let model = AkidaModel::load("kmer_filter.akd")?;
board.upload_model(&model)?;

// 2. Prepare input data
let input = prepare_input_spikes(&data);  // Convert to spike train

// 3. Run inference
let output = board.infer(&input)?;  // <1ms typical

// 4. Decode result
let classification = decode_spike_output(&output);
```

### Data Encoding

#### Temporal Encoding (for events)

```
DNA: A C G T
Time:  ▲ ▲ ▲ ▲
      t0 t1 t2 t3

Each base fires at a specific time
```

#### Rate Encoding (for static data)

```
Pixel value: 128
Spike rate: 64 Hz (proportional)

Higher value → More spikes per unit time
```

---

## Distributed Mesh Integration

### Multi-Board Coordination

#### Strandgate Configuration (2x Boards)

```
Akida 0: /dev/akida0 @ PCIe 0000:01:00.0
Akida 1: /dev/akida1 @ PCIe 0000:02:00.0

Allocation strategy:
  - Board 0: Primary workloads
  - Board 1: Redundancy + overflow
  - Load balancing: Round-robin or weighted
```

#### Cross-Node Communication

```
Strandgate (2x Akida)
    ↓ 10GbE
Southgate (1x Akida)

Remote board access:
  - gRPC API for inference requests
  - Sub-millisecond local, <5ms remote (LAN)
  - Automatic failover if local board busy
```

### Fault Tolerance

```rust
// If Akida board fails mid-inference
match board.infer(&input) {
    Ok(result) => Ok(result),
    Err(BoardError::Timeout) => {
        // Retry on different board
        retry_on_alternate_board(input)
    }
    Err(BoardError::HardwareFailure) => {
        // Mark board unhealthy, fallback to CPU
        scheduler.mark_board_unhealthy(board_id);
        fallback_to_cpu(input)
    }
}
```

---

## Performance Optimization

### Batching Strategy

Akida works best with single-sample inference (no batching overhead):

```
Traditional GPU:
  Batch size 128 → 10ms total → 78μs per sample
  (But 10ms latency for first result)

Akida:
  Batch size 1 → 0.5ms per sample
  (Immediate results, perfect for streaming)
```

### Memory Management

```rust
// Preload models to avoid loading overhead
board.preload_models(&[
    "kmer_filter.akd",
    "intent_classifier.akd",
    "motion_detector.akd",
])?;

// Models stay resident in 10MB SRAM
// Switch between models in <100μs
```

### Pipeline Optimization

#### Example: Bioinformatics

```
Traditional (CPU-only):
  Read FASTQ → CPU k-mer filter → CPU Kraken2 → GPU alignment
  Bottleneck: CPU doing too much

Optimized (with Akida):
  Read FASTQ → Akida k-mer filter → CPU Kraken2 → GPU alignment
  Result: CPU freed, 2x throughput, 50x power efficiency
```

---

## Monitoring & Diagnostics

### Health Checks

```rust
pub fn run_diagnostics(board: &AkidaBoard) -> DiagnosticReport {
    let mut tests = vec![];
    
    // PCIe link test
    tests.push(check_pcie_link(board));
    
    // Memory test
    tests.push(check_memory(board));
    
    // NPU test (run simple inference)
    tests.push(check_npus(board));
    
    // Temperature check
    tests.push(check_temperature(board));
    
    // Power check
    tests.push(check_power(board));
    
    DiagnosticReport { tests }
}
```

### Metrics Collection

```rust
pub struct AkidaMetrics {
    pub inferences_per_second: f64,
    pub avg_latency_ms: f64,
    pub power_watts: f64,
    pub utilization_percent: f64,
    pub error_count: u64,
    pub uptime_seconds: u64,
}

// Export to Prometheus, Grafana, etc.
```

---

## Code Organization

```
crates/
├── distributed/
│   └── src/universal/
│       ├── types/neuromorphic.rs    # Neuromorphic platform types
│       └── substrate.rs              # UniversalSubstrate with detection
│
└── runtime/
    └── src/scheduler/
        └── neuromorphic.rs           # Neuromorphic-specific scheduling

showcase/
└── neuromorphic/
    ├── 01-akida-detection/
    │   └── src/
    │       ├── pcie_scan.rs          # PCIe device discovery
    │       ├── akida_device.rs       # Board management
    │       └── substrate_integration.rs  # ToadStool integration
    │
    ├── 02-akida-bioinformatics/
    │   └── src/
    │       ├── akida_filter.rs       # Akida-accelerated k-mer filtering
    │       └── cpu_filter.rs         # CPU baseline
    │
    └── 03-akida-llm-intent/
        └── src/
            └── akida_classifier.rs   # Intent classification on Akida
```

---

## Future Enhancements

### Near-term (Q1 2025)

1. **Model Zoo**: Pre-trained models for common tasks
2. **Auto-tuning**: Automatic hyperparameter optimization
3. **Monitoring Dashboard**: Real-time metrics visualization
4. **CI/CD Integration**: Automated testing on Akida hardware

### Medium-term (Q2-Q3 2025)

1. **Multi-chip Scaling**: Distribute large models across multiple boards
2. **Hybrid Pipelines**: Akida → GPU → CPU automatic orchestration
3. **Dynamic Routing**: ML-based workload placement optimization
4. **Power Management**: Dynamic frequency/voltage scaling

### Long-term (Q4 2025+)

1. **Akida Gen2 Support**: Next-generation hardware
2. **Custom Neuron Models**: Beyond LIF neurons
3. **On-device Training**: STDP-based online learning
4. **Neuromorphic Sensors**: Direct DVS camera integration

---

## References

### BrainChip Documentation

- Akida AKD1000 Datasheet
- Akida SDK API Reference
- Akida MetaTF Documentation

### ToadStool Documentation

- UniversalSubstrate API
- Workload Scheduler Design
- GPU Runtime Architecture

### Academic Papers

- "Akida: A Neuromorphic Processor for Edge AI" (2021)
- "Spiking Neural Networks for Low-Power AI" (2020)
- "Neuromorphic Computing: The Next Generation" (2019)

---

**Version**: 1.0  
**Last Updated**: December 18, 2025  
**Status**: Ready for hardware integration

