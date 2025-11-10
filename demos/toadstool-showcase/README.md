# 🍄 ToadStool Local Showcase

**The Universal Compute Platform Demonstration**

## 🎯 What This Demonstrates

This showcase proves ToadStool's core capability:
> **"Write once, run anywhere, with zero configuration"**

You'll see:
- ✅ Same workload running on multiple substrates (native, docker, python)
- ✅ Live migration of running workloads between substrates
- ✅ Intelligent substrate selection based on resources
- ✅ Automatic failover when substrates fail
- ✅ Zero configuration required

**Total Time**: 15-30 minutes  
**Difficulty**: Easy  
**Prerequisites**: Linux/macOS with Docker (optional)

---

## 🚀 Quick Start

```bash
# Run the complete showcase
./showcase.sh

# Or run individual phases
./scripts/phase1-hello.sh          # Hello World on 3 substrates
./scripts/phase2-intelligence.sh   # Intelligent substrate selection
./scripts/phase3-migration.sh      # Live migration (THE KILLER DEMO)
./scripts/phase4-diversity.sh      # Substrate diversity
./scripts/phase5-failover.sh       # Automatic failover
```

---

## 📋 Prerequisites

### **Required**:
- ✅ Linux or macOS (Windows support coming soon)
- ✅ Bash shell
- ✅ ToadStool installed (`cargo install --path .`)

### **Optional** (enables more demos):
- ✅ Docker installed (for container substrate demos)
- ✅ Python 3.11+ (for Python runtime demos)

### **Check Prerequisites**:
```bash
./utils/verify.sh
```

---

## 🎬 Demo Phases

### **Phase 1: Hello Universal** (5 min)
Shows the same "hello world" running on 3 different substrates with zero code changes.

**Key Takeaway**: One workload definition, multiple execution environments.

### **Phase 2: Intelligence** (5 min)
ToadStool analyzes available resources and automatically selects the optimal substrate.

**Key Takeaway**: ToadStool thinks for you.

### **Phase 3: Live Migration** ⭐ (10 min)
A running counter migrates between substrates without stopping or losing state.

**Key Takeaway**: Compute is liquid - it flows between substrates seamlessly.

**THIS IS THE KILLER DEMO** 🎯

### **Phase 4: Substrate Diversity** (5 min)
Shows all available substrates and their capabilities.

**Key Takeaway**: ToadStool runs on anything that computes.

### **Phase 5: Failover** (5 min)
Simulates substrate failure and shows automatic failover.

**Key Takeaway**: Your workloads never die.

---

## 🛠️ Setup

```bash
# 1. Verify prerequisites
./utils/verify.sh

# 2. Setup demo environment
./utils/setup.sh

# 3. Run showcase
./showcase.sh

# 4. Cleanup when done
./utils/cleanup.sh
```

---

## 📁 Structure

```
toadstool-showcase/
├── README.md              # This file
├── showcase.sh            # Main demo runner
├── config/
│   └── showcase.toml      # Demo configuration
├── workloads/
│   ├── hello.toml        # Phase 1: Hello world
│   ├── compute.toml      # Phase 2: Resource intensive
│   ├── counter.toml      # Phase 3: Stateful migration
│   └── test.toml         # Phase 4: Substrate test
├── scripts/
│   ├── phase1-hello.sh
│   ├── phase2-intelligence.sh
│   ├── phase3-migration.sh    # ⭐ The killer demo
│   ├── phase4-diversity.sh
│   └── phase5-failover.sh
└── utils/
    ├── setup.sh           # Environment setup
    ├── cleanup.sh         # Demo cleanup
    └── verify.sh          # Prerequisites check
```

---

## 🎯 Key Messages

### **Technical Audience**:
- Universal runtime abstraction layer
- Substrate-agnostic workload execution  
- Live migration with state preservation
- Zero-config substrate detection

### **Business Audience**:
- Run code anywhere without rewriting
- Never locked into one platform
- Automatic optimization and failover
- Reduced infrastructure complexity

### **Everyone**:
- Your code works everywhere
- Moves between systems automatically
- Always finds the best place to run
- Never goes down

---

## 🐛 Troubleshooting

### **"Docker not found"**
```bash
# Install Docker or skip container demos
./showcase.sh --skip-docker
```

### **"Python not found"**
```bash
# Install Python 3.11+ or skip python demos
./showcase.sh --skip-python
```

### **"Permission denied"**
```bash
# Make scripts executable
chmod +x showcase.sh
chmod +x scripts/*.sh
chmod +x utils/*.sh
```

### **"ToadStool command not found"**
```bash
# Install ToadStool
cd /home/eastgate/Development/ecoPrimals/toadstool
cargo install --path .
```

---

## 📊 Success Metrics

After running the showcase, you should have seen:
- ✅ Same workload on 3+ substrates
- ✅ Live migration with zero data loss
- ✅ Automatic substrate selection
- ✅ Resilient failover
- ✅ Zero configuration required

---

## 🚀 What's Next?

### **Extend the Demo**:
- Add WASM substrate
- Add multi-machine tower
- Connect to cloud (AWS/Azure/GCP)
- Integrate with BiomeOS

### **Dive Deeper**:
- Read: `../../specs/TOADSTOOL_LOCAL_SHOWCASE_SPEC.md`
- Explore: `../../examples/`
- Learn: `../../docs/`

---

## 📝 Notes

**This is a local demo** - no cloud or network required.  
**Works on single machine** - perfect for presentations.  
**Takes 15-30 minutes** - great for showcases.  
**Zero risk** - everything cleans up automatically.

---

## 💬 Feedback

Found a bug? Have suggestions?  
Open an issue or submit a PR!

---

**Built with 🍄 by the ToadStool Team**  
**Reality > Hype. Truth > Marketing. Excellence > Speed.** ✅

