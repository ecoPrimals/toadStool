# Comprehensive Heterogeneous Compute Characterization
## A Complete Hardware Selection Framework for Modern Computing Workloads

**Authors**: ToadStool Research Team  
**Date**: February 1, 2026  
**Version**: 1.0  
**Status**: Publication Draft

---

## Abstract

We present the first comprehensive characterization of heterogeneous computing across three distinct hardware platforms: neuromorphic processors (NPU), graphics processors (GPU), and central processors (CPU). Through 85 validated benchmarks spanning five diverse workload categories—homomorphic encryption, machine learning inference, genomics, cryptography, and arithmetic operations—we establish quantified guidelines for optimal hardware selection.

Our key findings reveal: (1) NPU performance is workload-complexity-dependent, achieving 1,557× efficiency gains for homomorphic encryption while requiring >90% data sparsity for simple operations; (2) GPU throughput scales exponentially with data size, reaching 1,537× speedup for genomics and 96× for large-scale cryptography; (3) CPU dominates small-data scenarios with 2,857× better efficiency than GPU for sub-kilobyte workloads; (4) batch size critically determines ML inference efficiency, with a 25-image crossover point between CPU and GPU optimality.

All implementations utilize pure Rust with vendor-agnostic WGSL shaders, validated on actual hardware (BrainChip Akida AKD1000 NPU, NVIDIA RTX 3090 GPU, x86-64 multi-core CPU). Complete source code, datasets, and reproduction instructions are provided. This work enables evidence-based hardware selection, with immediate practical impact including 1,537× cost reduction for genomics and enabling real-time encrypted computation.

**Keywords**: Heterogeneous Computing, Neuromorphic Processors, GPU Acceleration, Hardware Characterization, Workload Analysis, Pure Rust, WGSL

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Background & Related Work](#2-background--related-work)
3. [Experimental Methodology](#3-experimental-methodology)
4. [Hardware Platforms](#4-hardware-platforms)
5. [Workload Characterization](#5-workload-characterization)
6. [Results & Analysis](#6-results--analysis)
7. [Hardware Selection Framework](#7-hardware-selection-framework)
8. [Discussion](#8-discussion)
9. [Conclusions](#9-conclusions)
10. [References](#10-references)
11. [Appendices](#11-appendices)

---

## Document Organization

This whitepaper is organized into the following sections:

### Core Content (`sections/`)
- `01_introduction.md` - Problem statement, motivation, contributions
- `02_background.md` - Related work, hardware overview
- `03_methodology.md` - Experimental design, validation approach
- `04_hardware_platforms.md` - NPU, GPU, CPU specifications
- `05_workload_characterization.md` - Benchmark descriptions
- `06_results_analysis.md` - Complete results for all 85 tests
- `07_selection_framework.md` - Decision trees, guidelines
- `08_discussion.md` - Implications, limitations, future work
- `09_conclusions.md` - Summary, impact statement

### Supporting Materials
- `data/` - All raw CSV/JSON results (85 tests)
- `figures/` - Performance charts, decision trees
- `code/` - Benchmark implementations, reproduction scripts
- `appendices/` - Detailed specifications, additional analyses

### Quick References
- `EXECUTIVE_SUMMARY.md` - 2-page overview for decision-makers
- `QUICK_START_GUIDE.md` - 5-minute guide to hardware selection
- `REPRODUCTION_GUIDE.md` - Complete instructions to replicate results

---

## Key Results Summary

### 85 Validated Tests Across 5 Workloads

| Workload | Tests | Best Hardware | Key Metric |
|----------|-------|---------------|------------|
| Homomorphic Encryption | 15 | **NPU** | 1,557× vs CPU |
| Dense/Sparse Operations | 48 | **Context-dependent** | Sparsity matters |
| ML Inference (MNIST) | 6 | **GPU** @ batch>32 | 4.2× vs CPU |
| Genomics (K-mer) | 8 | **GPU** | 1,537× vs CPU |
| Cryptography (AES) | 8 | **GPU** @ >1MB | 96× vs CPU |

### Hardware Selection Principles

**Use NPU When**:
- Complex sparse operations (homomorphic encryption)
- Ultra-low power critical (2W vs 15W CPU vs 250W GPU)
- Edge/mobile deployment

**Use GPU When**:
- Large parallel workloads (>1MB data)
- Genomics/bioinformatics (100-1,537× faster)
- Batched ML inference (>32 items)
- High throughput required (GB/s)

**Use CPU When**:
- Small data (<500KB) - 13× more efficient than GPU
- Single-item processing - 21× better than GPU
- Simple dense operations - 2,857× better than GPU
- Low latency critical (<1ms)

---

## Citation

```bibtex
@techreport{toadstool2026heterogeneous,
  title={Comprehensive Heterogeneous Compute Characterization: A Complete Hardware Selection Framework},
  author={ToadStool Research Team},
  institution={ToadStool Project},
  year={2026},
  month={February},
  note={85 validated benchmarks on actual hardware}
}
```

---

## License & Availability

**Code**: MIT License (Pure Rust, vendor-agnostic)  
**Data**: CC-BY-4.0 (All 85 test results)  
**Paper**: CC-BY-4.0 (Full reproduction rights)

**Repository**: https://github.com/ecoPrimals/toadStool  
**Data Archive**: https://doi.org/[TBD]

---

## Contact

For questions, collaborations, or access to hardware:
- **Project**: ToadStool Heterogeneous Compute Framework
- **Repository**: github.com/ecoPrimals/toadStool
- **Documentation**: toadstool.dev

---

**Status**: Publication Draft v1.0 - February 1, 2026  
**Next Steps**: Peer review submission, community release preparation
