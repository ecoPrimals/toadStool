# 🍄 Level 1: Multi-Runtime Workflows

**Time**: 20 minutes  
**Prerequisites**: Completed Level 0  
**Goal**: Learn to compare and mix runtimes effectively

---

## 🎯 What You'll Learn

In this level, you'll learn advanced runtime usage:
- ✅ How to benchmark and compare runtimes
- ✅ When to use each runtime (decision guide)
- ✅ How to mix runtimes in a single workflow
- ✅ Performance vs security trade-offs in practice

**Building on Level 0** - Now you'll see them side-by-side!

---

## 🚀 Quick Start

```bash
# Run all demos in sequence
./run-all-demos.sh

# Or run individually
./demo-runtime-comparison.sh      # Compare all 3 runtimes
./demo-cross-runtime-workflow.sh  # Mix runtimes in workflow
./demo-runtime-selection.sh       # Decision guide
```

---

## 📋 Demos

### 1. Runtime Comparison ⭐ **START HERE**
**Script**: `demo-runtime-comparison.sh`  
**Time**: 7 minutes

**What it shows**:
- Same workload on Native, WASM, and Python
- Performance benchmarking
- Memory usage comparison
- Startup time differences
- Security level comparison

**Run it**:
```bash
./demo-runtime-comparison.sh
```

**Expected Output**:
```
Runtime Benchmark Results:
┌──────────┬──────────┬─────────┬──────────┬──────────┐
│ Runtime  │ Duration │ Memory  │ Startup  │ Security │
├──────────┼──────────┼─────────┼──────────┼──────────┤
│ Native   │  0.1s    │  5.2MB  │  0.01s   │ ⭐⭐     │
│ WASM     │  0.2s    │  2.8MB  │  0.05s   │ ⭐⭐⭐⭐⭐  │
│ Python   │  0.5s    │  45MB   │  0.2s    │ ⭐⭐⭐    │
└──────────┴──────────┴─────────┴──────────┴──────────┘
```

**Key Insight**:
> Each runtime has a sweet spot. Choose based on your needs, not just performance!

---

### 2. Cross-Runtime Workflow
**Script**: `demo-cross-runtime-workflow.sh`  
**Time**: 7 minutes

**What it shows**:
- Data processing pipeline using multiple runtimes
- Native for I/O (fast file processing)
- Python for ML inference (ecosystem)
- WASM for data validation (security)

**Run it**:
```bash
./demo-cross-runtime-workflow.sh
```

**Real-World Example**:
```
Step 1: Native   → Read large file (fast I/O)
Step 2: WASM     → Validate data (sandboxed)
Step 3: Python   → ML inference (ecosystem)
Step 4: Native   → Write results (fast I/O)
```

**Key Insight**:
> Use the right tool for each job. Mix runtimes to get the best of all worlds!

---

### 3. Runtime Selection Guide
**Script**: `demo-runtime-selection.sh`  
**Time**: 6 minutes

**What it shows**:
- Interactive decision tree
- Use case → runtime mapping
- Common patterns and anti-patterns
- Performance vs security trade-offs

**Run it**:
```bash
./demo-runtime-selection.sh
```

**Decision Tree**:
```
Your workload is...
├─ Untrusted code? → WASM
├─ ML/AI task? → Python (+ GPU if available)
├─ Maximum performance? → Native
├─ Cross-platform? → WASM
└─ Rapid prototype? → Python
```

---

## 🎓 Key Concepts

### Runtime Characteristics

| Characteristic | Native | WASM | Python |
|---------------|--------|------|--------|
| **Startup** | ⚡ Fast | ⚡⚡ Medium | 🐢 Slow |
| **Performance** | 🚀 Fastest | 🚀 Fast | 🐌 Slower |
| **Memory** | 📦 Medium | 📦 Small | 📦📦 Large |
| **Security** | ⚠️ Low | 🔒 High | 🔒 Medium |
| **Portability** | ❌ No | ✅ Yes | ✅ Mostly |
| **Ecosystem** | 📚 Medium | 📚 Growing | 📚 Huge |

### When to Mix Runtimes

**✅ Good Reasons**:
- Different security requirements per step
- Optimize each task independently
- Leverage specific runtime features
- Gradual migration strategy

**⚠️ Watch Out For**:
- Communication overhead between runtimes
- Increased complexity
- Harder debugging
- Maintenance burden

---

## 💡 Real-World Patterns

### Pattern 1: Security Pipeline
```
Untrusted Input → WASM validation → Python processing → Native output
```
**Why**: Validate dangerous input safely, then process with full power

### Pattern 2: ML Pipeline
```
Native I/O → Python inference → Native serialization
```
**Why**: Fast I/O, ML ecosystem, fast results

### Pattern 3: Plugin Architecture
```
Native core → WASM plugins → Native integration
```
**Why**: Safe user plugins, fast core, seamless integration

### Pattern 4: Edge Computing
```
WASM edge → Python cloud → WASM edge
```
**Why**: Portable edge code, powerful cloud processing

---

## 🔍 Performance Deep Dive

### Benchmark: Factorial(20)

**Native** (C binary):
- Execution: 0.001s
- Memory: 1.2 MB
- Total: 0.011s (with startup)

**WASM** (compiled Rust):
- Execution: 0.003s
- Memory: 0.8 MB
- Total: 0.053s (with JIT compile)

**Python** (CPython):
- Execution: 0.015s
- Memory: 42 MB
- Total: 0.215s (with interpreter startup)

**Conclusion**: Native wins for pure compute, but WASM is close with better security!

---

## 🎯 Decision Matrix

Use this to choose your runtime:

```
┌────────────────────────┬──────────────────┐
│      Your Priority     │  Best Runtime    │
├────────────────────────┼──────────────────┤
│ Maximum performance    │ Native           │
│ Untrusted code         │ WASM             │
│ ML/AI workload         │ Python           │
│ Cross-platform         │ WASM             │
│ Large ecosystem        │ Python           │
│ System integration     │ Native           │
│ Minimal memory         │ WASM             │
│ Rapid development      │ Python           │
│ Real-time systems      │ Native           │
│ Web integration        │ WASM             │
└────────────────────────┴──────────────────┘
```

---

## 🏆 Level 1 Mastery

After completing this level, you'll be able to:

✅ **Benchmark** runtimes for your use case  
✅ **Choose** the right runtime based on requirements  
✅ **Mix** runtimes in complex workflows  
✅ **Optimize** for performance vs security trade-offs

---

## ➡️ Next Steps

**Mastered multi-runtime workflows?** Great! Now learn about:

### Level 2: Resource Management
**Path**: `../03-resource-management/`

**What's next**:
- Set CPU and memory limits
- Fair scheduling across workloads
- GPU quota management
- Priority-based execution

```bash
cd ../03-resource-management
cat README.md
```

---

## 📊 Demo Statistics

**Demos**: 3  
**Total Time**: 20 minutes  
**Difficulty**: Intermediate  
**Prerequisites**: Level 0 complete

---

## 🔧 Troubleshooting

### Demos Running Slowly?
All demos work in demo mode (simulated). For real benchmarks:
```bash
export DEMO_MODE=false
# Make sure ToadStool server is running
```

### Want Real Performance Data?
```bash
# Run with timing
time ./demo-runtime-comparison.sh
```

---

## 💡 Pro Tips

**Tip 1**: Start with Python for prototyping, migrate hot paths to Native/WASM

**Tip 2**: Use WASM for anything user-provided (plugins, scripts, configs)

**Tip 3**: Native for system integration, Python for ML, WASM for portable compute

**Tip 4**: Don't optimize prematurely - profile first, then choose runtime

---

**Ready to start?** Run your first comparison!

```bash
./demo-runtime-comparison.sh
```

🍄 **Master the runtimes!**
