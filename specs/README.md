# Toadstool-Compute Specifications

## ✅ **CURRENT STATUS** (December 2025)

**ToadStool is PRODUCTION READY with A- grade (90/100).**

**For current status, read these first**:
1. **`../STATUS.md`** ⭐ Start here! - Current status (A- 90/100)
2. **`../HANDOFF_DEC_19_2025.md`** - Complete handoff document
3. **`../ULTIMATE_STATUS_DEC_19_2025.md`** - Comprehensive status report
4. **`../COMPREHENSIVE_CODE_AUDIT_DEC_20_2025.md`** - Latest audit (Dec 20, 2025)

**Latest Achievement**: A- grade (90/100), 787/787 tests passing (100%), world-class unsafe code (A+ 98/100), production ready NOW!

The specs below provide architectural context and remain valid.

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

### ⭐ **Current Status Reports** (December 2025)
- **[../STATUS.md](../STATUS.md)** ⭐ Current status dashboard (A- 90/100)
- **[../COMPREHENSIVE_CODE_AUDIT_DEC_20_2025.md](../COMPREHENSIVE_CODE_AUDIT_DEC_20_2025.md)** - Complete audit (Dec 20, 2025)
- **[../HANDOFF_DEC_19_2025.md](../HANDOFF_DEC_19_2025.md)** - Session handoff and architecture
- **[../ULTIMATE_STATUS_DEC_19_2025.md](../ULTIMATE_STATUS_DEC_19_2025.md)** - Comprehensive progress report
- **[../SONGBIRD_INTEGRATION_PLAN_DEC_19_2025.md](../SONGBIRD_INTEGRATION_PLAN_DEC_19_2025.md)** - Integration roadmap

### 📊 **Architectural Specifications** (Valid & Current)
- [PRIMAL_CAPABILITY_SYSTEM.md](./PRIMAL_CAPABILITY_SYSTEM.md) - Capability system (implemented)
- [PRODUCTION_READINESS_SUMMARY.md](./PRODUCTION_READINESS_SUMMARY.md) - Production overview
- [UNIVERSAL_COMPUTE_PLATFORM.md](./UNIVERSAL_COMPUTE_PLATFORM.md) - Platform architecture
- [SOVEREIGN_SCIENCE_GRADE_ACHIEVEMENT.md](./SOVEREIGN_SCIENCE_GRADE_ACHIEVEMENT.md) - Quality standards
- [UNIVERSAL_UNIFIED_MEMORY.md](./UNIVERSAL_UNIFIED_MEMORY.md) - ⭐ **NEW** Unified memory architecture (Jan 2026)

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

### In Progress 🟡
- [ ] Test coverage expansion (45% → 90%)
- [ ] Performance benchmarking and optimization
- [ ] Phase 4 auto-discovery (mDNS/DNS-SD)
- [ ] Inter-primal showcase demonstrations

### Future Enhancements 📋
- [ ] Horizontal scaling optimization
- [ ] Additional runtime backends
- [ ] Enhanced monitoring and observability
- [ ] Extended chaos engineering scenarios

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for contribution guidelines.

## Support

For questions and support:
- **Architecture**: Ecosystem integration team
- **Implementation**: Toadstool development team
- **Integration**: Songbird and Squirrel teams
