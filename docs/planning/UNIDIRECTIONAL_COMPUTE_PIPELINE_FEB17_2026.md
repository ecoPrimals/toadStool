# Unidirectional Compute Pipeline Architecture

**Date**: February 17, 2026  
**Status**: Architectural Exploration  
**Origin**: Discussion on zero-roundtrip GPU compute pipelines

---

## The Insight

> "If we imagine a unidirectional structure where the CPU preps and streams to 2× GPU saturating the PCIe, with 0 roundtrip. Then the GPU over HDMI are input into another computer whose CPU intakes and processes from there. If we are using the GPU for the math and the work, and the output is just completed data, then the 10GB/sec becomes significant data streaming."

This reframes the problem: **10 GB/s of completed results is not the same as 10 GB/s of raw data.**

---

## Traditional vs Unidirectional

### Traditional (Round-Trip)

```
                    Round-trip bottleneck
                           ↓
CPU ──► GPU ──► CPU ──► GPU ──► CPU ──► GPU ──► CPU
    PCIe    PCIe    PCIe    PCIe    PCIe    PCIe
    
Problem: Every step waits for the previous
Throughput: Limited by slowest round-trip
```

### Unidirectional Pipeline

```
┌─────────────────────────────────────────────────────────────────────┐
│  PREP STATION (Computer A)                                          │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │  CPU: Prepare work units, stream continuously                  │ │
│  │       (no waiting for results — fire and forget)               │ │
│  └────────────────────────────────────────────────────────────────┘ │
│       │ PCIe (64 GB/s)              │ PCIe (64 GB/s)                │
│       ▼                             ▼                               │
│  ┌──────────┐                  ┌──────────┐                         │
│  │  GPU 1   │                  │  GPU 2   │                         │
│  │  (work)  │                  │  (work)  │                         │
│  └────┬─────┘                  └────┬─────┘                         │
│       │ HDMI (6 GB/s)               │ HDMI (6 GB/s)                 │
│       │ completed results           │ completed results             │
└───────┼─────────────────────────────┼───────────────────────────────┘
        │                             │
        │     Physical cables         │
        │     (one-way flow)          │
        ▼                             ▼
┌───────────────────────────────────────────────────────────────────────┐
│  COLLECTION STATION (Computer B)                                      │
│  ┌─────────────────────┐    ┌─────────────────────┐                   │
│  │  Capture Card 1     │    │  Capture Card 2     │                   │
│  │  (GPUDirect)        │    │  (GPUDirect)        │                   │
│  └─────────┬───────────┘    └─────────┬───────────┘                   │
│            │                          │                               │
│            ▼                          ▼                               │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │  CPU: Aggregate, store, analyze completed results              │  │
│  │       (receives finished data — no prep overhead)              │  │
│  └────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────────┘
```

---

## Why This Changes Everything

### 1. Zero Contention

```
Traditional:
  PCIe carries: input → output → input → output (interleaved)
  Effective bandwidth: ~50% of theoretical

Unidirectional:
  PCIe carries: input only (saturated)
  HDMI carries: output only (saturated)
  Effective bandwidth: 100% of both paths
```

### 2. Zero Round-Trip Latency

```
Traditional hotSpring problem:
  CPU → GPU (upload)     → wait
  GPU → CPU (eigenvals)  → wait
  CPU → GPU (next work)  → wait
  ... 70× slower than pure CPU

Unidirectional:
  CPU: continuously streaming work units
  GPU: continuously computing, outputting via HDMI
  No waiting. Ever.
```

### 3. Completed Data is Smaller

Consider an eigensolve workload:

```
INPUT (per matrix):
  - Matrix: 100×100 f64 = 80 KB
  - Parameters: ~100 bytes
  
OUTPUT (per matrix):
  - Eigenvalues: 100 f64 = 800 bytes
  - Optional eigenvectors: 80 KB (if needed)
  
Compression ratio: 100:1 if only eigenvalues needed!
```

At 10 GB/s output:
- **12.5 million eigenvalue sets per second** (if 100 eigenvalues each)
- Or **125,000 complete eigenpairs per second** (with vectors)

### 4. Bandwidth Math

```
Prep Station throughput:
  2× GPU × 64 GB/s PCIe input = 128 GB/s raw data in
  
Collection Station throughput:  
  2× GPU × 6 GB/s HDMI output = 12 GB/s completed results out

If compute reduces data 10:1 → perfectly balanced
If compute reduces data 100:1 → output path is never the bottleneck
```

---

## Concrete Example: Nuclear EOS Parameter Sweep

### Problem

Sweep 1 million parameter combinations for nuclear equation of state.
Each requires: eigensolve + BCS pairing + density functional.

### Traditional Approach

```
for each parameter set:
    upload to GPU           (10 μs)
    compute                 (100 μs)  
    download eigenvalues    (10 μs)
    CPU post-process        (50 μs)
    
Total: 170 μs × 1M = 170 seconds
Bottleneck: round-trip latency dominates
```

### Unidirectional Approach

```
PREP STATION:
  Stream 1M parameter sets to 2 GPUs
  Time: (1M × 80KB) / (128 GB/s) = 0.6 seconds

GPU COMPUTE:
  Each GPU processes 500K sets @ 100 μs each
  Time: 500K × 100 μs = 50 seconds (pipelined!)
  
COLLECTION STATION:
  Receive 1M × 800 bytes = 800 MB of eigenvalues
  Time: 800 MB / (12 GB/s) = 0.07 seconds

Total wall clock: ~50 seconds (dominated by compute, not I/O)
Speedup: 3.4× from eliminating round-trips
```

---

## MD Simulation Streaming

### Particle Trajectory Output

```
Particles: 10,000 atoms
Per frame: positions (3 × f64 × 10K) = 240 KB
Frame rate: 10 GB/s / 240 KB = 41,666 frames/second

That's 41K timesteps of MD trajectory streaming continuously!
```

### Real-Time Analysis

```
PREP STATION:
  - Initial conditions
  - Force field parameters
  - Integration parameters

GPU COMPUTE:
  - Forces (Lennard-Jones, Coulomb, bonded)
  - Integration (Verlet)
  - Thermostats
  
OUTPUT (via HDMI):
  - Positions every N steps
  - Energies
  - Order parameters
  - RDF bins

COLLECTION STATION:
  - Aggregate trajectories
  - Compute time averages
  - Detect phase transitions
  - Store to disk
```

---

## Implementation Architecture

### Prep Station (ToadStool A)

```rust
/// Unidirectional work streamer
pub struct PrepStation {
    gpus: Vec<WgpuDevice>,
    work_queue: mpsc::Sender<WorkUnit>,
}

impl PrepStation {
    /// Stream work units to GPUs (fire and forget)
    pub async fn stream_work(&self, work: impl Iterator<Item = WorkUnit>) {
        for (i, unit) in work.enumerate() {
            let gpu_idx = i % self.gpus.len();
            
            // Upload to GPU (no waiting for results!)
            self.gpus[gpu_idx].upload_async(&unit.data).await;
            self.gpus[gpu_idx].dispatch_compute(&unit.shader);
            
            // Result goes out HDMI — we never see it
        }
    }
}
```

### GPU Compute (BarraCUDA)

```rust
/// GPU compute with display output
pub struct DisplayOutputCompute {
    device: WgpuDevice,
    output_encoder: DisplayDataEncoder,
    output_texture: GpuTexture2D,
}

impl DisplayOutputCompute {
    /// Compute and output to display (no CPU readback)
    pub fn compute_and_output(&self, input: &GpuBuffer) {
        // 1. Run compute shader
        self.dispatch_compute(input);
        
        // 2. Encode result as pixels
        self.output_encoder.encode(&self.result_buffer, &self.output_texture);
        
        // 3. Present to display (goes out HDMI)
        self.present_texture(&self.output_texture);
        
        // No readback! Result flows to Collection Station via cable.
    }
}
```

### Collection Station (ToadStool B)

```rust
/// Collect completed results from display input
pub struct CollectionStation {
    capture_cards: Vec<CaptureDevice>,
    result_decoder: DisplayDataDecoder,
    storage: ResultStorage,
}

impl CollectionStation {
    /// Continuously receive completed results
    pub async fn collect_loop(&mut self) {
        loop {
            for card in &self.capture_cards {
                // Receive frame from GPU via capture card
                let frame = card.capture_frame().await;
                
                // Decode pixels back to results
                let results = self.result_decoder.decode(&frame);
                
                // Store/analyze (CPU work on completed data)
                self.storage.append(results);
                self.analyze_online(&results);
            }
        }
    }
}
```

---

## Hardware Configuration

### Minimum Setup

```
┌─────────────────────────────────────────────────────────┐
│  Computer A (Prep Station)                              │
│  ├── CPU: Streams work to GPUs                          │
│  ├── GPU 1: RTX 3090 (HDMI out)                         │
│  └── GPU 2: RTX 4070 (HDMI out)                         │
│                                                         │
│         HDMI cables (just regular cables!)              │
│                │                │                       │
└────────────────┼────────────────┼───────────────────────┘
                 │                │
                 ▼                ▼
┌────────────────────────────────────────────────────────┐
│  Computer B (Collection Station)                       │
│  ├── Capture Card 1: Magewell (GPUDirect)              │
│  ├── Capture Card 2: Magewell (GPUDirect)              │
│  └── CPU: Aggregates, stores, analyzes                 │
└────────────────────────────────────────────────────────┘
```

### Cost

| Component | Price | Notes |
|-----------|-------|-------|
| Magewell Pro Capture | ~$400 | GPUDirect support |
| HDMI 2.1 cable | ~$20 | Standard cable |
| Second computer | Variable | Can be modest (just collection) |

**Total additional cost**: ~$800 for bidirectional GPU pipeline without NVLink.

---

## Comparison with Alternatives

| Approach | Bandwidth | Round-trips | Cost | Complexity |
|----------|-----------|-------------|------|------------|
| Standard PCIe | 64 GB/s | Yes (50% loss) | $0 | Low |
| NVLink | 900 GB/s | No | $2000+ | Medium |
| **Unidirectional HDMI** | 12 GB/s | **No** | $800 | Medium |
| InfiniBand | 400 GB/s | No | $3000+ | High |

**Key advantage**: Unidirectional HDMI is the cheapest way to eliminate round-trips.

---

## When This Wins

### Good Fit

- Parameter sweeps (many independent computations)
- Monte Carlo simulations (embarrassingly parallel)
- MD trajectories (continuous output)
- Training data generation (GPU generates, CPU stores)
- Real-time analysis (continuous compute → continuous analysis)

### Not a Good Fit

- Iterative algorithms requiring convergence checks
- Interactive computation (need low-latency feedback)
- Small workloads (setup overhead dominates)

---

## Key Insight

**The 10 GB/s isn't the limit — the zero round-trips is the win.**

Traditional GPU computing is like a conversation:
> "Here's some data" → "Here's the result" → "Here's more data" → ...

Unidirectional computing is like a factory:
> Raw materials flow in one door → finished products flow out another door

The factory doesn't stop the assembly line to ship each product.

---

## Next Steps

1. **Prototype**: Single GPU → HDMI → Capture → decode
2. **Benchmark**: Measure actual throughput vs round-trip baseline
3. **Integrate**: Add to ToadStool Hardware Router
4. **Document**: Patterns for unidirectional workloads

---

*From the ToadStool evolution desk — unidirectional compute exploration*
