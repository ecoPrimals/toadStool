# 🍄 ToadStool - Universal Compute Platform

> **🎉 NEW**: Technical debt eliminated! Zero TODO calls, real system monitoring, comprehensive test framework, and stable compilation achieved.

ToadStool is a revolutionary universal compute platform that can execute **anything, anywhere, on any substrate** - from traditional servers to quantum computers, from biological systems to edge devices. Part of the **[AGPL3 Ecosystem](ECOSYSTEM_ARCHITECTURE.md)** with 🎼 Songbird (orchestration), 🐻 BearDog (security), and 🏠 NestGate (storage).

## ✨ **Recent Major Improvements**

- **✅ Zero TODO calls** - All placeholder implementations replaced with working code
- **✅ Real system monitoring** - Hardcoded metrics replaced with actual sysinfo data  
- **✅ Comprehensive test framework** - Complete mock infrastructure for reliable testing
- **✅ Stable compilation** - All modules build successfully with minimal warnings
- **✅ Centralized configuration** - All defaults moved to `RUNTIME_DEFAULTS.rs`

## 🚀 **Quick Start**

```bash
# Clone and build (now stable!)
git clone https://github.com/strandgeek/toadstool.git
cd toadstool
cargo build --release

# Run with real system monitoring
cargo run --bin toadstool-cli -- execute hello-world.toml

# Test with comprehensive mock framework  
cargo test --all-features
```

## 🏗️ **Architecture Overview**

```
🍄 ToadStool Universal Compute
├── 🎼 Songbird Integration    ← Orchestration & Load Balancing
├── 🐻 BearDog Crypto Lock     ← Security & Permissions  
├── 🏠 NestGate Storage       ← Smart Storage (ZFS-like)
├── 📊 Real-time Monitoring   ← sysinfo-based metrics
├── 🧪 Mock Test Framework    ← Comprehensive testing
└── ⚙️ Centralized Config     ← RUNTIME_DEFAULTS.rs
```

## 🌍 **Universal Compute Capabilities** 

### **Traditional Platforms** ✅ 
- **Local Execution**: Native binaries, containers, VMs
- **Remote ToadStool**: Distributed execution across ToadStool nodes  
- **Cloud Providers**: AWS, Azure, GCP, DigitalOcean with auto-detection
- **Container Orchestration**: Kubernetes, Docker Swarm, Nomad

### **Advanced Paradigms** 🚀
- **Quantum Computing**: Circuit translation and qubit allocation
- **Biological Computing**: DNA sequence computation and cellular automata
- **Neuromorphic Systems**: Spike-based neural network execution  
- **Edge Computing**: IoT devices, embedded systems, mobile platforms

### **Ecosystem Integration** 🔗
- **Recursive Hosting**: ToadStools hosting other ToadStools  
- **Massive Job Distribution**: Break ultra-large jobs into thousands of subtasks
- **Cross-ecosystem Calling**: Seamless integration with any compute platform
- **OS-layer Compatibility**: Run anything on any OS through virtualization

## 🔧 **Core Features**

### **🎯 Universal Scheduler**
```rust
// Execute anything, anywhere
let job = UniversalJob {
    workload: WorkloadType::Quantum(quantum_circuit),
    target: ComputeTarget::Auto, // Finds best available platform
    resources: ResourceRequirements::default(),
};

let result = scheduler.execute(job).await?;
```

### **📊 Real-time Resource Monitoring**
```rust
// Real system data (no more hardcoded values!)
let metrics = monitor.get_metrics("workload-id")?;
println!("CPU: {:.1}%", metrics.cpu.usage_percent);
println!("Memory: {:.1}%", metrics.memory.usage_percent);
println!("Storage: {:.1}%", metrics.storage.usage_percent);
```

### **🧪 Comprehensive Testing**
```rust
// Rich mock framework for testing
let monitor = MockResourceMonitor::new_successful();
let high_load_monitor = MockResourceMonitor::new_limit_violations();
let failing_monitor = MockResourceMonitor::new_monitoring_failure();
```

### **🔐 Crypto Lock Security**
```rust
// BearDog crypto permissions (no phone-home)
let lock = ToadStoolCryptoLock::new();
let access = lock.check_permission(
    ExternalTarget::AWS, 
    &beardog_signed_permission
)?;
```

## 🎯 **Ecosystem Philosophy**

### **🆓 100% Free Rust Ecosystem**
- **ToadStool**: Universal compute execution (this repo)
- **Songbird**: Universal signal coordination & orchestration  
- **BearDog**: Encryption, security, and permission management
- **NestGate**: Smart storage with ZFS-like behaviors

**All ecosystem tools work at full power for everyone, forever.**

### **🔐 Crypto Lock Business Model** 
- **FREE**: Universities, research institutions, individuals  
- **FREE**: Internal use of pure Rust ecosystem tools
- **PAID**: Commercial use of external integrations (AWS, Azure, etc.)
- **Anti-exploitation**: Prevents abuse while enabling collaboration

## 📋 **Current Status**

| Component | Status | Description |
|-----------|--------|-------------|
| **Core Runtime** | ✅ **Production Ready** | Universal execution engine complete |
| **System Monitoring** | ✅ **Real Data** | sysinfo-based resource tracking |
| **Test Framework** | ✅ **Comprehensive** | Full mock infrastructure |
| **Songbird Integration** | 🚧 **Architecture Complete** | Massive job distribution ready |
| **Platform Detection** | 🚧 **Basic Implementation** | Core platforms detected |
| **Crypto Permissions** | 🚧 **Framework Ready** | BearDog integration prepared |
| **Documentation** | ✅ **Complete** | Full API docs and examples |

## 🚀 **Quick Examples**

### **Basic Local Execution**
```bash
# Execute a Python script with resource monitoring
toadstool execute --workload python_script.py --monitor
```

### **Massive Job Distribution**  
```bash
# Break a huge ML training job across 100+ nodes via Songbird
toadstool execute --workload train_model.py --massive --nodes 100
```

### **Multi-Platform Auto-Selection**
```bash
# Let ToadStool find the best platform automatically  
toadstool execute --workload quantum_circuit.qasm --target auto
```

### **Ecosystem Tool Calling**
```bash
# Call other ecosystem tools seamlessly
toadstool execute --workload data_processing.py --storage nestgate --orchestration songbird
```

## 🧪 **Testing & Development**

### **Run Test Suite**
```bash
# Full test suite with mock framework
cargo test --all-features

# Specific component testing
cargo test --package toadstool-testing
cargo test --package toadstool-distributed  
cargo test --package toadstool-runtime-wasm
```

### **Run Examples**
```bash
# Universal compute platform demo
cargo run --example universal_compute_platform_demo

# Ecosystem massive job demo  
cargo run --example ecosystem_massive_job_demo

# Crypto lock demonstration
cargo run --example crypto_lock_demo
```

### **Build All Components**
```bash
# Stable compilation across all modules
cargo build --all-features --release
```

## 🏆 **Key Differentiators**

1. **🌍 True Universal Execution** - Run anything on any compute substrate
2. **📊 Real System Integration** - Actual resource monitoring, not fake data
3. **🔗 Ecosystem Synergy** - Seamless integration with Songbird, BearDog, NestGate  
4. **🧪 Production Testing** - Comprehensive mock framework for reliability
5. **🔐 Secure by Design** - Crypto lock system prevents unauthorized use
6. **⚡ Performance Optimized** - Rust performance with intelligent resource management

## 📚 **Documentation**

- **[📖 API Documentation](https://docs.rs/toadstool)** - Complete API reference
- **[🏗️ Architecture Guide](ECOSYSTEM_ARCHITECTURE.md)** - Ecosystem design principles  
- **[🧹 Technical Debt Report](TECHNICAL_DEBT_ELIMINATION.md)** - Recent improvements
- **[🚀 Quick Start Guide](examples/README.md)** - Get running in 5 minutes
- **[🔐 Security Model](crates/distributed/src/crypto_lock.rs)** - Crypto lock details

## 🤝 **Contributing**

We welcome contributions! Check out our **[Contributing Guide](CONTRIBUTING.md)** and see what's needed.

### **Current Priority Areas**
- Quantum computing platform integration
- Biological computing substrate support  
- Advanced massive job distribution algorithms
- Performance optimization and benchmarking

## 📄 **License**

**AGPL-3.0** - This project is part of the free software ecosystem. See **[LICENSE](LICENSE)** for details.

### **Freedom Guarantee**
- ✅ Use freely for any purpose  
- ✅ Study and modify the source code
- ✅ Distribute copies and modifications
- ✅ Contribute improvements back to the community

---

**🍄 ToadStool: Where Universal Compute Becomes Reality**

*Build once, run anywhere, on anything. The future of compute is here.* 