# Toadstool Distributed GPU Compute Handoff

**Date**: February 9, 2026
**From**: biomeOS NUCLEUS Integration Team
**To**: Toadstool / barraCUDA Development Team
**Priority**: HIGH -- First successful cross-vendor distributed GPU compute
**Status**: Proof-of-concept validated, evolution roadmap defined

---

## Executive Summary

Today we achieved the **first successful distributed AI inference and GPU compute across multiple machines and GPU vendors** using the ecoPrimal stack. We demonstrated:

1. **Pipeline-parallel LLM inference** (TinyLlama-1.1B) split across an RTX 4070 (Tower) and RTX 3090 (gate2) over LAN TCP -- 39.85 tok/s
2. **BearDog-encrypted tensor transport** -- ChaCha20-Poly1305 encryption of hidden states between gates
3. **Vendor-agnostic GPU compute via barraCUDA** -- the same WGSL shader produced **bit-identical results** on NVIDIA RTX 4070, NVIDIA RTX 3090, and **AMD Radeon RX 6950 XT** -- zero CUDA, zero ROCm, pure Vulkan

---

## Hardware Inventory

### Tower (gate)
| Component | Details |
|-----------|---------|
| CPU | 24 cores |
| GPU | NVIDIA GeForce RTX 4070 (12 GB VRAM) |
| Vulkan | 1.3+ via nvidia_icd |
| OS | Pop!_OS 22.04 |
| Role | NUCLEUS primary, layers 0-10 |

### gate2 (strandgate)
| Component | Details |
|-----------|---------|
| CPU | AMD EPYC 7452 32-Core (64 threads) |
| RAM | 252 GB |
| GPU 0 | NVIDIA GeForce RTX 3090 (24 GB VRAM) |
| GPU 1 | AMD Radeon RX 6950 XT (16 GB VRAM) |
| Vulkan 0 | 1.4.312 via NVIDIA driver 580.119.02 |
| Vulkan 1 | 1.4.311 via RADV NAVI21 (Mesa 25.1.5) |
| OS | Pop!_OS 22.04 |
| Role | NUCLEUS secondary, layers 11-21 |

### Total Compute Available
- **3 discrete GPUs** across 2 machines (52 GB combined VRAM)
- **2 GPU vendors** (NVIDIA + AMD)
- **88 CPU threads** combined
- All connected via Gigabit LAN

---

## What Worked

### 1. barraCUDA Vendor-Agnostic Compute (VALIDATED)

**Test**: 1024x1024 matrix multiplication using a single WGSL shader, compiled to a single Rust binary, deployed to both machines.

| GPU | Vendor | Backend | GFLOPS | Checksum |
|-----|--------|---------|--------|----------|
| RTX 4070 (Tower) | NVIDIA | Vulkan | 388.7 | **5.128010** |
| RTX 3090 (gate2) | NVIDIA | Vulkan | 481.0 | **5.128010** |
| RX 6950 XT (gate2) | AMD | Vulkan (RADV) | 222.7 | **5.128010** |

**Identical checksums across all 3 GPUs.** Same binary. Same shader. Zero vendor SDK.

### 2. Pipeline-Parallel LLM Inference Across LAN

- **Tower (RTX 4070)**: Embedding + layers 0-10 (1.03 GB VRAM)
- **gate2 (RTX 3090)**: Layers 11-21 + RMSNorm + lm_head (1.03 GB VRAM)
- **Transport**: Direct TCP over LAN
- **Performance**: 80 tokens in 2.01s (**39.85 tok/s**)

### 3. BearDog-Encrypted Tensor Transport

- **Input**: 89,057 bytes raw tensor data
- **Output**: 118,744 bytes ciphertext + nonce + auth tag
- **Encryption time**: 20ms via BearDog Unix socket JSON-RPC

### 4. Songbird Native TCP JSON-RPC

Full `mesh.init` / `mesh.announce` / `mesh.peers` for peer discovery over native Songbird TCP.

---

## Gaps Found (Evolution Targets)

### 1. No Safetensors/GGUF Weight Loader in barraCUDA
barraCUDA has all transformer ops but cannot load pre-trained HuggingFace weights.

### 2. Tensor Serialization for Network Transfer
Need efficient binary format: shape metadata + raw f16/f32 buffer with zero-copy support.

### 3. Multi-GPU Device Selection Within a Process
`WgpuDevice::new()` picks one device. Need a `DevicePool` for multi-GPU orchestration.

### 4. Toadstool as RPC Service for Workload Dispatch
Needs to evolve from biome runner to JSON-RPC workload service.

### 5. INT4/INT8 Quantization in WGSL
Required for larger models. WGSL supports i32 for packed INT4.

---

## Evolution Roadmap

| Phase | Goal | Status |
|-------|------|--------|
| 1 | Safetensors/GGUF weight loader | Not started |
| 2 | Toadstool distributed executor (JSON-RPC) | Not started |
| 3 | Intelligent workload partitioning | Not started |
| 4 | Tensor parallelism | Not started |
| 5 | WGSL quantization (INT4/INT8) | Not started |

---

*Written after the first successful distributed AI compute across GPU vendors in the ecoPrimal ecosystem. Feb 9, 2026.*
