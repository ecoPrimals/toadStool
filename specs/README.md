# Toadstool-Compute Specifications

## ⚠️ **IMPORTANT: REALITY CHECK** (October 2025)

**The specs in this directory claimed 95-96% production ready. Reality is 65-75%.**

**For accurate current status, read these first**:
1. **`../AUDIT_QUICK_CHECKLIST_OCT_10.md`** ⭐ Start here!
2. **`../AUDIT_EXECUTIVE_SUMMARY_OCT_10.md`** - Management summary
3. **`../STATUS.md`** - Current accurate status
4. **`REALITY_CHECK_OCT_2025.md`** - Why the discrepancy

The specs below are preserved for historical context but contain outdated claims.

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

### ⭐ **Current Accurate Status** (Read These First!)
- **[../AUDIT_QUICK_CHECKLIST_OCT_10.md](../AUDIT_QUICK_CHECKLIST_OCT_10.md)** - Quick reference (all questions answered)
- **[../AUDIT_EXECUTIVE_SUMMARY_OCT_10.md](../AUDIT_EXECUTIVE_SUMMARY_OCT_10.md)** - Management summary
- **[../COMPREHENSIVE_AUDIT_REPORT_OCT_10_2025.md](../COMPREHENSIVE_AUDIT_REPORT_OCT_10_2025.md)** - Full audit (14,000 words)
- **[../STATUS.md](../STATUS.md)** - Current status dashboard
- **[REALITY_CHECK_OCT_2025.md](./REALITY_CHECK_OCT_2025.md)** - Why specs were incorrect

### 📊 **Reality Check Documents** (October 2025)
- [REALITY_CHECK_OCT_2025.md](./REALITY_CHECK_OCT_2025.md) - Explains discrepancy (95% → 65-75%)
- [FINAL_CODEBASE_REVIEW_2025.md](./FINAL_CODEBASE_REVIEW_2025.md) - ⚠️ Updated with reality check banner
- [PRODUCTION_READINESS_SUMMARY.md](./PRODUCTION_READINESS_SUMMARY.md) - ⚠️ Updated with reality check
- [CODEBASE_IMPROVEMENT_ROADMAP.md](./CODEBASE_IMPROVEMENT_ROADMAP.md) - Needs update

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

### Current Phase: Foundation
- [ ] Project structure and specifications
- [ ] Core execution environment implementation
- [ ] Songbird integration framework
- [ ] Migration planning from Squirrel

### Next Phase: Implementation
- [ ] Container runtime integration
- [ ] WASM runtime implementation
- [ ] Cross-platform sandboxing
- [ ] Resource management system

### Future Phases
- [ ] GPU compute support
- [ ] Advanced security features
- [ ] Performance optimization
- [ ] Horizontal scaling

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for contribution guidelines.

## Support

For questions and support:
- **Architecture**: Ecosystem integration team
- **Implementation**: Toadstool development team
- **Integration**: Songbird and Squirrel teams
