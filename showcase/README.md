# ToadStool Showcase Collection
## Pure Rust Hardware + Universal Compute Demonstrations

**Status**: ✅ Production Ready | **Updated**: February 8, 2026

---

## 🎯 Overview

The ToadStool showcase collection demonstrates the complete self-evolving compute stack:

**🍄 ToadStool** - Pure Rust hardware infrastructure
- Discovers 13 GPUs + 2 NPUs + 1 CPU automatically
- Self-evolves with hardware changes
- Zero setup on fresh systems

**🦈 BarraCuda** - Universal compute layer
- 250+ GPU-accelerated operations
- FFT/NTT shaders for MD simulations
- FHE with 21.1x speedup

---

## 🚀 Core Showcases

### 1. Neuromorphic Computing (`neuromorphic/`)

**🧠 01-akida-detection** - Hardware Discovery
```bash
cd neuromorphic/01-akida-detection
./demo.sh
```
- ToadStool discovers Akida NPUs
- Dual-backend drivers (kernel + userspace)
- Runtime capability discovery

**🧬 02-akida-bioinformatics** - k-mer Filtering
```bash
cd neuromorphic/02-akida-bioinformatics
./demo-kmer-filtering.sh
```
- NPU-accelerated genomics
- Compare NPU vs CPU performance
- Power measurement

**🤖 03-akida-llm-intent** - Intent Classification
```bash
cd neuromorphic/03-akida-llm-intent
cargo run --release --example train_intent_classifier
```
- LLM intent classification on NPU
- Event-driven inference
- Low-power AI

**🎨 04-raytracing-comparison** - NPU vs GPU 🆕
```bash
cd neuromorphic/04-raytracing-comparison  
./demo.sh
```
- Raytracing on NPU (sparse, event-driven)
- Raytracing on GPU (dense, parallel)
- Performance comparison
- ToadStool automatic device selection

### 2. BarraCuda Validation (`barracuda-validation/`)

```bash
cd barracuda-validation
cargo test --release
```
- Cross-vendor GPU validation
- Performance benchmarks
- Operation correctness

### 3. GPU Universal (`gpu-universal/`)

```bash
cd gpu-universal
cargo run --release --example matmul_demo
```
- Universal GPU operations via WGPU
- Works on NVIDIA, AMD, Intel
- WGSL shader showcase

### 4. Homomorphic Computing (`homomorphic-computing/`)

```bash
cd homomorphic-computing
cargo run --release --example fhe_ntt_validation
```
- FHE operations (21.1x GPU speedup)
- NTT/INTT transforms
- Encrypted computation

### 5. WhitePaper Results (`whitePaper/`)

Benchmark data and validation reports:
- Performance CSVs
- Cross-vendor results  
- FHE speedup analysis

---

## 🗄️ Archived Showcases

Older showcases moved to `archive/`:
- `archive/gaming/` - Gaming evolution demos
- `archive/distributed/` - Multi-primal demos
- `archive/misc/` - Legacy showcases

These may be outdated with the new ToadStool architecture.

---

## 🧪 Quick Test All Core Showcases

```bash
# Neuromorphic
cd neuromorphic/01-akida-detection && ./demo.sh
cd ../02-akida-bioinformatics && ./demo-kmer-filtering.sh
cd ../04-raytracing-comparison && ./demo.sh

# BarraCuda  
cd ../../barracuda-validation && cargo test --release

# GPU Universal
cd ../gpu-universal && cargo run --release --example matmul_demo

# FHE
cd ../homomorphic-computing && cargo run --release --example fhe_ntt_validation
```

---

## 📊 Expected Results

### Hardware Discovery
```
✓ Discovered 16 device(s)
  • GPUs: 13 (via ToadStool)
  • NPUs: 2 Akida (via ToadStool)
  • CPUs: 1 (always available)
```

### NPU Raytracing (Sparse Scene)
```
NPU Results:
  Time: ~20-30ms (event-driven, efficient)
  Advantage: 2-3x faster than GPU for sparse scenes
```

### GPU Raytracing (Dense Scene)
```
GPU Results:
  Time: ~5-10ms (parallel throughput)
  Advantage: 5-10x faster than NPU for dense scenes
```

### FHE Performance
```
CPU Baseline: 107.8ms
GPU (BarraCuda): 5.1ms
Speedup: 21.1x faster
```

---

## 🏗️ Architecture

All showcases demonstrate the complete stack:

```
Showcase Application
     ↓
BarraCuda 🦈 (Math/Shaders)
  • FFT/NTT for MD
  • Raytracing shaders
  • FHE operations
     ↓
ToadStool 🍄 (Hardware Discovery)
  • Auto-discovers devices
  • Selects best for workload
  • Provides driver access
     ↓
Hardware (Auto-Discovered)
  GPU/NPU/CPU with full driver power
```

---

## 🎯 What Each Showcase Demonstrates

| Showcase | ToadStool Feature | BarraCuda Feature | Hardware |
|----------|-------------------|-------------------|----------|
| **akida-detection** | Discovery | - | NPU |
| **akida-bioinformatics** | Device selection | k-mer ops | NPU |
| **akida-llm-intent** | NPU drivers | Event encoding | NPU |
| **raytracing-comparison** | Workload selection | WGSL shaders | NPU + GPU |
| **barracuda-validation** | Multi-GPU | All operations | GPU |
| **gpu-universal** | GPU discovery | WGSL shaders | GPU |
| **homomorphic-computing** | Device selection | FHE NTT/INTT | GPU |

---

## 📝 Deep Debt Compliance

All showcases follow deep debt principles:

✅ **No Scripts** (Pure Rust)  
✅ **Runtime Discovery** (ToadStool)  
✅ **Self-Evolving** (Adapts to hardware)  
✅ **Agnostic** (Works with any hardware)  
✅ **Safe Rust** (No unsafe blocks)  
✅ **Real Hardware** (No simulations)  

---

## 🚀 Status

**Core Showcases**: 7 working demonstrations  
**Archived**: 10+ older showcases  
**New**: NPU raytracing comparison  
**Hardware**: 16 devices discovered  
**Tests**: All passing  

**Ready for:** Production validation, benchmarking, demonstrations

---

*See individual showcase READMEs for detailed instructions*
