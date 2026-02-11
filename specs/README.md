# ToadStool + BarraCUDA Specifications

## Current Status (February 11, 2026)

**For current status, read these first**:
1. **`../STATUS.md`** — Detailed technical status
2. **`../QUICK_STATUS.md`** — One-page summary
3. **`../README.md`** — Project overview and architecture

**Key Numbers**:
- **15,460+ tests passing**, 0 failing
- **414 WGSL shaders** (reorganized into categories)
- **5 scientific middleware modules** (linalg, numerical, special, optimize, surrogate)
- **0 clippy warnings**, 0 build warnings
- **0 unsafe blocks** in middleware
- Cross-vendor GPU compute validated (NVIDIA + AMD, bit-identical)
- 39.85 tok/s distributed LLM inference with encrypted transport

**Active Work**: Phase 2A — Sampling & Global Optimization (Latin Hypercube, Multi-start Nelder-Mead)

---

## Overview

**Toadstool-Compute** is the dedicated compute and environment management platform for the ecosystem. It provides secure, cross-platform execution environments for plugins, AI agents, and compute workloads.

## Project Mission

Toadstool-Compute serves as the **universal compute platform** that:
- Provides secure execution environments (Container, WASM, Native)
- Manages compute resources (CPU, memory, GPU)
- Implements cross-platform sandboxing and security
- Integrates with Songbird for capability discovery and request routing

## Core Responsibilities

### 🏗️ Execution Environments
- **Container Runtime**: Docker and containerd integration
- **WASM Runtime**: WebAssembly execution with Wasmtime
- **Native Runtime**: Secure native code execution
- **GPU Compute**: CUDA and OpenCL support

### 🔒 Security & Sandboxing
- **Cross-Platform Isolation**: Windows, macOS, Linux sandboxing
- **Resource Limits**: CPU, memory, network, filesystem controls
- **Permission System**: Fine-grained capability controls
- **Security Monitoring**: Real-time security event tracking

### 📊 Resource Management
- **Dynamic Allocation**: Intelligent resource allocation
- **Performance Monitoring**: Real-time performance metrics
- **Capacity Planning**: Resource usage prediction
- **Load Balancing**: Distribute workloads across instances

### 🔌 Ecosystem Integration
- **Songbird Integration**: Register capabilities, receive requests
- **Plugin Execution**: Execute plugins from Squirrel MCP platform
- **Storage Integration**: Coordinate with NestGate for data access
- **AI Workloads**: Execute AI agent computations

## Architecture Principles

### 🎯 Focused Responsibility
- **Pure Compute Platform**: Only handles execution and resource management
- **No Direct Communication**: All ecosystem communication via Songbird
- **Stateless Design**: Execution environments are ephemeral
- **Horizontal Scaling**: Multiple Toadstool instances for scale

### 🚀 Performance First
- **Rust Performance**: Zero-cost abstractions and memory safety
- **Efficient Scheduling**: Optimal workload scheduling
- **Resource Optimization**: Minimize overhead and maximize throughput
- **Hot Path Optimization**: Optimize critical execution paths

### 🔐 Security by Design
- **Principle of Least Privilege**: Minimal permissions by default
- **Defense in Depth**: Multiple security layers
- **Audit Trail**: Complete execution audit logs
- **Vulnerability Management**: Proactive security monitoring

## Specifications Index

### Active Roadmap
- **[BARRACUDA_EVOLUTION_ROADMAP.md](./BARRACUDA_EVOLUTION_ROADMAP.md)** ⭐ **Active** — Evolution roadmap, remaining work, cross-domain vision

### BarraCUDA Compute

- **[BARRACUDA_NPU_UNIVERSAL_COMPUTE_V2.md](./BARRACUDA_NPU_UNIVERSAL_COMPUTE_V2.md)** — Universal tensor operations (CPU, GPU, NPU)
- [BARRACUDA_PURE_RUST_TENSOR_OPS.md](./BARRACUDA_PURE_RUST_TENSOR_OPS.md) — Pure Rust tensor ops (v1.x, GPU focus)
- [BARRACUDA_SCIENTIFIC_COMPUTING_OPS.md](./BARRACUDA_SCIENTIFIC_COMPUTING_OPS.md) — Complex arithmetic, FFT, physics primitives
- [BARRACUDA_UNIVERSAL_COMPUTE_EVOLUTION.md](./BARRACUDA_UNIVERSAL_COMPUTE_EVOLUTION.md) — Operation coverage tracker
- [RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS.md](./RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS.md) — Neuromorphic extensions

### Platform & Architecture
- [PRIMAL_CAPABILITY_SYSTEM.md](./PRIMAL_CAPABILITY_SYSTEM.md) - Capability system (implemented)
- [UNIVERSAL_COMPUTE_PLATFORM.md](./UNIVERSAL_COMPUTE_PLATFORM.md) - Platform architecture
- [UNIVERSAL_UNIFIED_MEMORY.md](./UNIVERSAL_UNIFIED_MEMORY.md) - Unified memory architecture (Jan 2026)
- [SOVEREIGN_SCIENCE_GRADE_ACHIEVEMENT.md](./SOVEREIGN_SCIENCE_GRADE_ACHIEVEMENT.md) - Quality standards

### Core Specifications (Historical - Preserved for Context)
- [PROJECT_OVERVIEW.md](./PROJECT_OVERVIEW.md) - Project overview and architecture
- [EXECUTION_ENVIRONMENTS.md](./EXECUTION_ENVIRONMENTS.md) - Execution environment specifications
- [SECURITY_SANDBOXING.md](./SECURITY_SANDBOXING.md) - Security and sandboxing implementation
- [RESOURCE_MANAGEMENT.md](./RESOURCE_MANAGEMENT.md) - Resource allocation and management

### Integration Specifications
- [SONGBIRD_INTEGRATION.md](./SONGBIRD_INTEGRATION.md) - Songbird discovery and routing integration
- [ECOSYSTEM_COMMUNICATION.md](./ECOSYSTEM_COMMUNICATION.md) - Cross-project communication patterns
- [PLUGIN_EXECUTION.md](./PLUGIN_EXECUTION.md) - Plugin execution from Squirrel MCP

### Implementation Guides
- [DEVELOPMENT_SETUP.md](./DEVELOPMENT_SETUP.md) - Development environment setup
- [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md) - Production deployment guide
- [PERFORMANCE_TUNING.md](./PERFORMANCE_TUNING.md) - Performance optimization guide

### Project Management
- [ROADMAP.md](./ROADMAP.md) - Development roadmap and milestones
- [MIGRATION_PLAN.md](./MIGRATION_PLAN.md) - Migration from Squirrel compute infrastructure

## Quick Start

### For Developers
1. **Setup**: See [DEVELOPMENT_SETUP.md](./DEVELOPMENT_SETUP.md)
2. **Architecture**: Read [PROJECT_OVERVIEW.md](./PROJECT_OVERVIEW.md)
3. **Integration**: Review [SONGBIRD_INTEGRATION.md](./SONGBIRD_INTEGRATION.md)

### For Operations
1. **Deployment**: See [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md)
2. **Monitoring**: Review resource management specifications
3. **Security**: Read security and sandboxing guides

### For Ecosystem Integration
1. **Communication**: See [ECOSYSTEM_COMMUNICATION.md](./ECOSYSTEM_COMMUNICATION.md)
2. **Plugin Execution**: Review [PLUGIN_EXECUTION.md](./PLUGIN_EXECUTION.md)
3. **Capability Broadcasting**: See Songbird integration specs

## Project Status

### Current Phase: Production Ready ✅
- [x] Project structure and specifications
- [x] Core execution environment implementation
- [x] Songbird integration framework (Phase 3 complete)
- [x] Migration planning from Squirrel

### Completed Implementation ✅
- [x] Container runtime integration
- [x] WASM runtime implementation
- [x] Cross-platform sandboxing
- [x] Resource management system
- [x] GPU compute support (CUDA, OpenCL, WebGPU)
- [x] Advanced security features
- [x] Capability-based discovery
- [x] Self-knowledge architecture

### In Progress
- [ ] Phase 2A: Latin Hypercube Sampling + Multi-start Nelder-Mead
- [ ] Test coverage expansion (81% → 90%)
- [ ] hotSpring L2 accuracy parity (χ²/datum < 2)

### Remaining Work (see [BARRACUDA_EVOLUTION_ROADMAP.md](./BARRACUDA_EVOLUTION_ROADMAP.md))
- [ ] Phase 2B: Full SparsitySampler (if Phase 2A insufficient)
- [ ] Phase 2C: GPU-accelerated RBF surrogate training (14× speedup)
- [ ] Phase 3: Cross-domain shader evolution (ray tracing, audio, neural)
- [ ] Phase 4: VFIO NPU driver, multi-GPU DevicePool, quantization

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for contribution guidelines.

## Support

For questions and support:
- **Architecture**: Ecosystem integration team
- **Implementation**: Toadstool development team
- **Integration**: Songbird and Squirrel teams
