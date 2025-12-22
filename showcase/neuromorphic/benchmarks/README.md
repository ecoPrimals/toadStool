# Neuromorphic Benchmarking Suite

## Quick Start

```bash
# Download standard datasets (MNIST, Fashion-MNIST, etc.)
./datasets/download.sh

# Run all benchmarks
./run-all-benchmarks.sh

# View results
cat results/summary.json | jq
```

## Available Benchmarks

### Vision

- **MNIST**: Handwritten digits (baseline)
- **N-MNIST**: Neuromorphic version with event streams
- **Fashion-MNIST**: Clothing classification
- **DVS Gesture**: Hand gestures from event camera

### Bioinformatics (ToadStool Custom)

- **K-mer Filtering**: DNA sequence preprocessing for Kraken2
- **Sequence Quality**: Quality score filtering
- **Adapter Detection**: Contamination identification

### LLM (ToadStool Custom)

- **Intent Classification**: Prompt categorization for routing
- **Production Simulation**: 24-hour realistic workload

## Individual Benchmarks

### MNIST

```bash
cargo run --release --bin mnist_benchmark -- \
    --platform akida \
    --dataset datasets/mnist \
    --boards 2
```

### Bioinformatics

```bash
cargo run --release --bin bioinformatics_benchmark -- \
    --platform akida \
    --dataset datasets/bioinformatics/illumina_sample.fastq \
    --kmer-size 31
```

### LLM Intent

```bash
cargo run --release --bin llm_intent_benchmark -- \
    --platform akida \
    --dataset datasets/llm/intent_test_set.jsonl
```

## Results

Benchmark results are saved to `results/` in JSON format:

```json
{
  "benchmark": "mnist",
  "platform": "akida",
  "accuracy": 0.987,
  "latency_ms": 0.52,
  "power_watts": 1.2,
  "efficiency_gain_vs_gpu": 47.8
}
```

## Comparison Charts

After running benchmarks, generate comparison charts:

```bash
cargo run --bin generate_charts
```

Outputs:
- `results/accuracy_comparison.png`
- `results/latency_comparison.png`
- `results/power_comparison.png`
- `results/efficiency_comparison.png`

## See Also

- `../BENCHMARKS.md` - Complete benchmark specification
- `../01-akida-detection/` - Hardware detection
- `../02-akida-bioinformatics/` - Bioinformatics demo
- `../03-akida-llm-intent/` - LLM intent demo

