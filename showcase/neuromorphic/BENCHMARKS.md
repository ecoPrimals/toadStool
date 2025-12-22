# Neuromorphic Computing Benchmarks

## Overview

This document defines the benchmark suite for evaluating Akida neuromorphic boards against traditional CPU/GPU baselines, using industry-standard datasets and custom bioinformatics/LLM workloads.

## Benchmark Categories

### 1. Classic Vision Benchmarks

These establish baseline performance for pattern recognition:

#### MNIST (Handwritten Digits)
- **Dataset**: 60,000 training, 10,000 test images (28x28 grayscale)
- **Task**: 10-class digit classification
- **Why**: Standard ML baseline, well-understood
- **Akida Expected**: High accuracy (98%+), ultra-low power
- **Metrics**: Accuracy, latency, power/inference

#### N-MNIST (Neuromorphic MNIST)
- **Dataset**: Event-based version of MNIST (spiking camera)
- **Task**: Digit classification with temporal dynamics
- **Why**: Native neuromorphic representation
- **Akida Expected**: Superior to frame-based (lower latency, power)
- **Metrics**: Accuracy, event processing rate, energy/event

#### Fashion-MNIST
- **Dataset**: 60,000 clothing items (28x28 grayscale)
- **Task**: 10-class clothing classification
- **Why**: Harder than MNIST, tests generalization
- **Akida Expected**: Similar to MNIST but slightly lower accuracy
- **Metrics**: Accuracy, latency, power/inference

### 2. Neuromorphic-Specific Benchmarks

#### DVS Gesture (Dynamic Vision Sensor)
- **Dataset**: 11 hand gestures recorded with event camera
- **Task**: Real-time gesture classification
- **Why**: Tests temporal spiking dynamics
- **Akida Expected**: Near-real-time classification, <1ms latency
- **Metrics**: Accuracy, latency, power

#### N-Caltech101
- **Dataset**: Neuromorphic version of Caltech101 (101 object categories)
- **Task**: Object recognition from event streams
- **Why**: Complex visual recognition
- **Akida Expected**: Good accuracy, exceptional efficiency
- **Metrics**: Accuracy, events/sec, power

#### TIMIT Speech (Event-based)
- **Dataset**: Speech corpus converted to spike trains
- **Task**: Phoneme recognition
- **Why**: Tests audio processing on neuromorphic hardware
- **Akida Expected**: Competitive accuracy, low power
- **Metrics**: Accuracy, latency, power

### 3. Bioinformatics Benchmarks (ToadStool Custom)

#### K-mer Filtering (Kraken2 Pipeline)
- **Dataset**: Real Illumina/Nanopore sequencing data
- **Workloads**:
  - 1M sequences, k=31 (typical)
  - 10M sequences, k=31 (stress test)
  - Variable k sizes (21, 31, 51, 71)
- **Metrics**:
  - Throughput (sequences/sec)
  - Power efficiency (sequences/joule)
  - Accuracy (vs ground truth)
  - Latency (per sequence)

#### Sequence Quality Filtering
- **Dataset**: FASTQ files with known quality scores
- **Task**: Discard low-quality sequences
- **Why**: Another preprocessing step suitable for Akida
- **Metrics**: Throughput, power, accuracy

#### Adapter Detection
- **Dataset**: Sequences with/without common adapters
- **Task**: Identify and flag adapter contamination
- **Why**: Pattern matching, perfect for SNNs
- **Metrics**: Accuracy, latency, power

### 4. LLM Intent Classification (ToadStool Custom)

#### Intent Dataset
- **Size**: 50,000 labeled prompts across 8 intent categories
- **Categories**:
  - code_generation (15%)
  - simple_qa (40%)
  - complex_reasoning (10%)
  - creative_writing (12%)
  - translation (8%)
  - summarization (10%)
  - retrieval (4%)
  - moderation (1%)
- **Metrics**:
  - Classification accuracy
  - Latency (ms)
  - Power (W)
  - Cost savings (estimated)

#### Production Simulation
- **Workload**: 10,000 requests/hour for 24 hours
- **Metrics**:
  - Average routing latency
  - Total power consumption
  - Cloud API cost (actual vs with routing)
  - Quality score (end-to-end)

### 5. Edge AI Benchmarks

#### MLPerf Tiny
- **Subset**: Visual wake words, keyword spotting
- **Why**: Standard edge AI benchmark
- **Akida Expected**: Competitive accuracy, best power efficiency
- **Metrics**: Accuracy, latency, energy/inference

#### EEMBC ULPMark-ML
- **Benchmark**: Ultra-low-power ML benchmark
- **Why**: Industry standard for power efficiency
- **Akida Expected**: Top-tier scores
- **Metrics**: Score (ops/mW), absolute power

## Benchmark Execution Framework

### Test Harness Structure

```
showcase/neuromorphic/benchmarks/
├── README.md
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── mnist.rs              # MNIST benchmark
│   ├── n_mnist.rs            # N-MNIST benchmark
│   ├── fashion_mnist.rs      # Fashion-MNIST
│   ├── dvs_gesture.rs        # DVS Gesture
│   ├── bioinformatics.rs     # K-mer filtering
│   ├── llm_intent.rs         # LLM intent classification
│   └── runner.rs             # Unified benchmark runner
├── datasets/
│   ├── download.sh           # Download standard datasets
│   ├── mnist/
│   ├── fashion-mnist/
│   ├── n-mnist/
│   └── bioinformatics/
├── results/
│   ├── cpu_baseline.json
│   ├── gpu_baseline.json
│   └── akida_results.json
└── run-all-benchmarks.sh
```

### Benchmark Metrics

Every benchmark reports:

```json
{
  "benchmark": "mnist",
  "platform": "akida",
  "timestamp": "2025-01-15T10:30:00Z",
  "config": {
    "batch_size": 1,
    "model": "snn_lenet5",
    "board_count": 2
  },
  "results": {
    "accuracy": 0.987,
    "avg_latency_ms": 0.52,
    "p50_latency_ms": 0.48,
    "p95_latency_ms": 0.61,
    "p99_latency_ms": 0.73,
    "throughput_samples_per_sec": 1923,
    "power_watts": 1.2,
    "energy_per_inference_mj": 0.624,
    "total_samples": 10000,
    "total_time_sec": 5.2
  },
  "comparison_to_cpu": {
    "speedup": 2.3,
    "power_reduction": 20.8,
    "efficiency_gain": 47.8
  }
}
```

## Running Benchmarks

### Quick Start

```bash
cd showcase/neuromorphic/benchmarks

# Download datasets (one-time)
./datasets/download.sh

# Run all benchmarks
./run-all-benchmarks.sh

# Run specific benchmark
cargo run --release --bin mnist_benchmark
cargo run --release --bin bioinformatics_benchmark
cargo run --release --bin llm_intent_benchmark
```

### Individual Benchmarks

#### MNIST

```bash
# CPU baseline
cargo run --release --bin mnist_benchmark -- \
    --platform cpu \
    --batch-size 32

# GPU baseline
cargo run --release --bin mnist_benchmark -- \
    --platform gpu \
    --batch-size 128

# Akida
cargo run --release --bin mnist_benchmark -- \
    --platform akida \
    --batch-size 1 \
    --boards 2
```

#### N-MNIST (Neuromorphic)

```bash
cargo run --release --bin n_mnist_benchmark -- \
    --platform akida \
    --event-rate 1000000  # 1M events/sec
```

#### Bioinformatics

```bash
cargo run --release --bin bioinformatics_benchmark -- \
    --dataset data/illumina_sample.fastq \
    --kmer-size 31 \
    --platform akida \
    --boards 2
```

#### LLM Intent

```bash
cargo run --release --bin llm_intent_benchmark -- \
    --dataset data/intent_test_set.jsonl \
    --platform akida
```

## Expected Results

### MNIST Performance

| Platform | Accuracy | Latency (ms) | Power (W) | Energy/Inference (mJ) |
|----------|----------|--------------|-----------|----------------------|
| CPU (EPYC) | 98.9% | 2.1 | 15 | 31.5 |
| GPU (RTX 3090) | 99.1% | 0.8 | 50 | 40.0 |
| **Akida (2 boards)** | **98.7%** | **0.5** | **1.2** | **0.6** |

**Akida Advantage**: 53x more energy efficient than GPU, 4.2x faster than CPU

### N-MNIST Performance

| Platform | Accuracy | Latency (ms) | Power (W) | Events/Joule |
|----------|----------|--------------|-----------|--------------|
| CPU (frame conversion) | 97.2% | 5.8 | 15 | 2,400 |
| GPU (frame conversion) | 97.8% | 1.2 | 50 | 850 |
| **Akida (native events)** | **98.1%** | **0.3** | **1.0** | **326,000** |

**Akida Advantage**: 384x more energy efficient, native event processing

### K-mer Filtering Performance

| Platform | Throughput (seq/sec) | Power (W) | Efficiency (seq/J) |
|----------|---------------------|-----------|-------------------|
| CPU (8 cores) | 1,200,000 | 25 | 48,000 |
| **Akida (2 boards)** | **2,800,000** | **1.1** | **2,545,000** |

**Akida Advantage**: 2.3x throughput, 53x efficiency, frees CPU cores

### LLM Intent Classification Performance

| Platform | Latency (ms) | Power (W) | Cost Savings ($/year) |
|----------|--------------|-----------|----------------------|
| CPU | 12.5 | 10 | N/A |
| GPU (idle overhead) | 5.2 | 30 | N/A |
| **Akida** | **0.5** | **1.0** | **$575,000** |

**Akida Advantage**: 10x faster than GPU, 25x faster than CPU, massive cost savings

## Comparison to Published Results

### BrainChip Official Benchmarks

BrainChip published Akida AKD1000 results:
- MNIST: 98.4% accuracy @ 0.8mJ/inference
- Keyword spotting: 89.3% accuracy @ 0.6mJ/inference
- Power: 1-10W TDP, <1W typical

**ToadStool's Results**: Should match or slightly exceed (better software optimization)

### Academic Neuromorphic Benchmarks

Intel Loihi (competing neuromorphic chip):
- MNIST: 97.5% accuracy @ 1.2mJ/inference
- DVS Gesture: 94.1% accuracy @ 2.1mJ/inference

**Akida Advantage**: Higher accuracy, lower energy

## Validation Methodology

### Accuracy Validation

1. **Ground Truth**: Use labeled test sets
2. **Statistical Significance**: >10,000 samples per test
3. **Cross-validation**: 5-fold on training data
4. **Comparison**: CPU/GPU baselines must match published results

### Power Measurement

1. **System-level**: Measure total board power via PCIe
2. **Inference-level**: Power × time / samples
3. **Idle power**: Subtract from active power
4. **Validation**: External power meter for spot checks

### Latency Measurement

1. **End-to-end**: Data transfer + inference + result fetch
2. **Inference-only**: Exclude transfer overhead
3. **Percentiles**: Report p50, p95, p99
4. **Warmup**: Discard first 100 iterations

## BrainChip Partnership Demos

### Demo 1: MNIST Live Classification

```bash
./run-mnist-demo.sh
```

Shows:
- Real-time MNIST digit classification
- Akida vs GPU vs CPU comparison (side-by-side)
- Power meter display
- Speedup/efficiency calculations

### Demo 2: Bioinformatics Pipeline

```bash
./run-bioinformatics-demo.sh
```

Shows:
- Real DNA sequencing data
- Kraken2 pipeline with/without Akida
- Power savings calculation
- CPU cores freed

### Demo 3: LLM Routing Cost Savings

```bash
./run-llm-routing-demo.sh
```

Shows:
- Live LLM request routing
- Intent classification in <1ms
- Cost tracking (cloud API calls avoided)
- Annual savings projection

## Continuous Benchmarking

### Automated Testing

```yaml
# .github/workflows/neuromorphic-benchmarks.yml
name: Neuromorphic Benchmarks
on:
  push:
    branches: [main]
  schedule:
    - cron: '0 0 * * 0'  # Weekly

jobs:
  benchmark:
    runs-on: [self-hosted, akida]
    steps:
      - uses: actions/checkout@v3
      - name: Run benchmarks
        run: |
          cd showcase/neuromorphic/benchmarks
          ./run-all-benchmarks.sh
      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: results/*.json
```

### Performance Regression Detection

- Track metrics over time
- Alert on >5% performance degradation
- Compare against baseline (first run with hardware)

## References

### Datasets

- MNIST: http://yann.lecun.com/exdb/mnist/
- N-MNIST: https://www.garrickorchard.com/datasets/n-mnist
- Fashion-MNIST: https://github.com/zalandoresearch/fashion-mnist
- DVS Gesture: https://research.ibm.com/interactive/dvsgesture/
- N-Caltech101: https://www.garrickorchard.com/datasets/n-caltech101

### Benchmarks

- MLPerf Tiny: https://mlcommons.org/en/inference-tiny-11/
- EEMBC ULPMark: https://www.eembc.org/ulpmark/
- NeuroBench: https://neurobench.ai/

### Papers

- Akida: "Akida: A neuromorphic processor for edge AI" (2021)
- N-MNIST: "Converting Static Image Datasets to Spiking Neuromorphic Datasets" (2015)
- SNNs: "Deep Learning With Spiking Neurons" (2018)

---

**Status**: 🟡 Ready to implement upon hardware arrival

**Expected Timeline**: 
- Week 1: Run all benchmarks
- Week 2: Validate and document
- Week 3: Present to BrainChip

