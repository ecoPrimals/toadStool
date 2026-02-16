# 🍄 ToadStool Local Showcase Specification
**Date**: October 30, 2025  
**Status**: Planning → Implementation  
**Priority**: HIGH (Demonstrate core capabilities)

---

## 🎯 **OBJECTIVE**

Create a **compelling local demonstration** that showcases ToadStool's core superpower:
> **"Run anything, anywhere, with zero configuration"**

### **Success Criteria**:
- ✅ Demo runs on single machine (no cloud required)
- ✅ Shows SAME workload on multiple substrates
- ✅ Demonstrates live migration between substrates
- ✅ Proves zero-configuration intelligence
- ✅ Takes 15-30 minutes to run
- ✅ "Wow factor" is obvious and immediate

---

## 📋 **DEMO PHASES**

### **Phase 1: "Hello Universal"** (5 minutes)
**Goal**: Show ToadStool running same workload on 3 substrates

**What happens**:
1. Define ONE simple workload (hello world)
2. Run on native (local process)
3. Run on docker (container)
4. Run on python (python runtime)
5. Show identical output, different execution context

**Key Message**: "One definition, multiple substrates, zero changes"

---

### **Phase 2: "Intelligence"** (5 minutes)
**Goal**: Show ToadStool automatically choosing best substrate

**What happens**:
1. Define workload with resource requirements
2. ToadStool analyzes available substrates
3. Automatically selects optimal substrate
4. Runs workload
5. Shows why it chose that substrate

**Key Message**: "ToadStool thinks for you"

---

### **Phase 3: "Live Migration"** ⭐ (10 minutes)
**Goal**: Show workload moving between substrates while running

**What happens**:
1. Start long-running stateful workload (counter)
2. Shows running on substrate A (native)
3. Command: migrate to substrate B (docker)
4. Workload seamlessly continues on docker
5. No data loss, no interruption
6. Migrate back to native

**Key Message**: "Compute is liquid - it flows between substrates"

---

### **Phase 4: "Substrate Diversity"** (5 minutes)
**Goal**: Show wide range of supported substrates

**What happens**:
1. List all detected substrates
2. Show capabilities of each
3. Run simple test on each available substrate
4. Display substrate-specific features

**Key Message**: "If it computes, ToadStool runs on it"

---

### **Phase 5: "Failover"** (5 minutes)
**Goal**: Show automatic substrate failover on failure

**What happens**:
1. Start workload on docker
2. Simulate docker failure (stop daemon)
3. ToadStool detects failure
4. Automatically fails over to native
5. Workload continues running

**Key Message**: "ToadStool is resilient - your compute never dies"

---

## 🛠️ **TECHNICAL REQUIREMENTS**

### **Prerequisites** (User must have):
```
Required:
- ✅ Linux/macOS/Windows (any OS)
- ✅ Rust toolchain (for running ToadStool)
- ✅ 4GB RAM minimum
- ✅ 2 CPU cores minimum

Optional (enables more demos):
- ✅ Docker installed (for container substrate)
- ✅ Python 3.11+ (for python substrate)
- ✅ Second machine/VM (for distributed demo)
```

### **ToadStool Components Needed**:
```rust
Required Modules:
- ✅ toadstool-runtime-native     (native execution)
- ✅ toadstool-runtime-container  (docker execution)
- ✅ toadstool-runtime-python     (python execution)
- ✅ toadstool-cli                (demo CLI)

Optional Modules:
- ✅ toadstool-runtime-wasm       (WASM execution)
- ✅ toadstool-distributed        (multi-node)
```

---

## 📁 **DEMO STRUCTURE**

### **Directory Layout**:
```
demos/
├── toadstool-showcase/
│   ├── README.md                 # Quick start guide
│   ├── showcase.sh               # Main demo runner
│   ├── config/
│   │   └── showcase.toml         # Demo configuration
│   ├── workloads/
│   │   ├── hello.toml           # Phase 1: Hello world
│   │   ├── compute.toml         # Phase 2: Resource intensive
│   │   ├── counter.toml         # Phase 3: Stateful migration
│   │   └── test.toml            # Phase 4: Substrate test
│   ├── scripts/
│   │   ├── phase1-hello.sh      # Phase 1 runner
│   │   ├── phase2-intelligence.sh
│   │   ├── phase3-migration.sh  # ⭐ Killer demo
│   │   ├── phase4-diversity.sh
│   │   └── phase5-failover.sh
│   └── utils/
│       ├── setup.sh             # Environment setup
│       ├── cleanup.sh           # Demo cleanup
│       └── verify.sh            # Prerequisites check
```

---

## 🎬 **DEMO SCRIPT FLOW**

### **Main Runner**: `showcase.sh`

```bash
#!/bin/bash
# ToadStool Local Showcase - Main Runner

set -e

echo "🍄 ToadStool Local Showcase"
echo "================================"
echo ""

# Setup
./utils/setup.sh

# Verify prerequisites
./utils/verify.sh

# Run demo phases
echo "Phase 1: Hello Universal..."
./scripts/phase1-hello.sh

echo ""
read -p "Press Enter to continue to Phase 2..."

echo "Phase 2: Intelligence..."
./scripts/phase2-intelligence.sh

echo ""
read -p "Press Enter to continue to Phase 3 (LIVE MIGRATION)..."

echo "Phase 3: Live Migration ⭐"
./scripts/phase3-migration.sh

echo ""
read -p "Press Enter to continue to Phase 4..."

echo "Phase 4: Substrate Diversity..."
./scripts/phase4-diversity.sh

echo ""
read -p "Press Enter to continue to Phase 5..."

echo "Phase 5: Failover..."
./scripts/phase5-failover.sh

# Cleanup
./utils/cleanup.sh

echo ""
echo "✅ Showcase Complete!"
echo ""
echo "Key Takeaways:"
echo "  • One workload runs on ANY substrate"
echo "  • Live migration between substrates"
echo "  • Zero configuration required"
echo "  • Automatic failover on substrate failure"
echo "  • ToadStool: Universal Compute Platform 🍄"
```

---

## 🔧 **WORKLOAD SPECIFICATIONS**

### **1. Hello World** (`workloads/hello.toml`)

```toml
[workload]
id = "hello-universal"
name = "Hello Universal"
description = "Simple hello world for multi-substrate demo"
version = "1.0.0"

[execution]
type = "script"
entry_point = "main"
timeout_seconds = 30

[execution.script]
language = "bash"
code = """
#!/bin/bash
echo "🍄 Hello from ToadStool!"
echo "Hostname: $(hostname)"
echo "Substrate: ${TOADSTOOL_SUBSTRATE:-unknown}"
echo "Platform: $(uname -s)"
echo "Architecture: $(uname -m)"
echo "Date: $(date)"
"""

[resources]
cpu_cores = 1
memory_mb = 128
disk_mb = 10

[metadata]
tags = ["demo", "hello-world", "showcase"]
author = "ToadStool Team"
```

---

### **2. Compute Workload** (`workloads/compute.toml`)

```toml
[workload]
id = "compute-demo"
name = "Compute Intelligence Demo"
description = "Resource-intensive workload to showcase substrate selection"
version = "1.0.0"

[execution]
type = "script"
entry_point = "main"
timeout_seconds = 60

[execution.script]
language = "python"
code = """
#!/usr/bin/env python3
import time
import sys
import os

def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)

print(f"🍄 Running compute workload on: {os.environ.get('TOADSTOOL_SUBSTRATE', 'unknown')}")
print(f"Computing fibonacci(30)...")

start = time.time()
result = fibonacci(30)
elapsed = time.time() - start

print(f"Result: {result}")
print(f"Time: {elapsed:.2f} seconds")
print(f"Substrate: {os.environ.get('TOADSTOOL_SUBSTRATE', 'unknown')}")
"""

[resources]
cpu_cores = 2
memory_mb = 512
disk_mb = 50

[substrate_preferences]
prefer = ["native", "docker", "python"]
avoid = ["wasm"]  # WASM is slower for recursive compute

[metadata]
tags = ["demo", "compute", "intelligence"]
```

---

### **3. Stateful Counter** (`workloads/counter.toml`)

```toml
[workload]
id = "counter-migration"
name = "Stateful Counter (Migration Demo)"
description = "Long-running counter for live migration demo"
version = "1.0.0"

[execution]
type = "script"
entry_point = "main"
timeout_seconds = 300
stateful = true

[execution.script]
language = "python"
code = """
#!/usr/bin/env python3
import time
import os
import json
from pathlib import Path

STATE_FILE = Path(os.environ.get('TOADSTOOL_STATE_DIR', '/tmp')) / 'counter_state.json'

def load_state():
    if STATE_FILE.exists():
        with open(STATE_FILE) as f:
            return json.load(f)
    return {'count': 0}

def save_state(state):
    with open(STATE_FILE, 'w') as f:
        json.dump(state, f)

state = load_state()
substrate = os.environ.get('TOADSTOOL_SUBSTRATE', 'unknown')

print(f"🍄 Counter starting from: {state['count']}")
print(f"Running on substrate: {substrate}")

try:
    while True:
        state['count'] += 1
        save_state(state)
        
        print(f"Count: {state['count']:04d} | Substrate: {substrate} | Time: {time.strftime('%H:%M:%S')}")
        time.sleep(1)
        
except KeyboardInterrupt:
    print(f"\\nCounter stopped at: {state['count']}")
    save_state(state)
"""

[state]
persistent = true
checkpoint_interval_seconds = 5
state_directory = "/tmp/toadstool-state"

[resources]
cpu_cores = 1
memory_mb = 256
disk_mb = 100

[migration]
enabled = true
allow_live = true
checkpoint_on_migrate = true

[metadata]
tags = ["demo", "stateful", "migration"]
```

---

### **4. Substrate Test** (`workloads/test.toml`)

```toml
[workload]
id = "substrate-test"
name = "Substrate Capability Test"
description = "Tests substrate capabilities and features"
version = "1.0.0"

[execution]
type = "script"
entry_point = "main"
timeout_seconds = 30

[execution.script]
language = "bash"
code = """
#!/bin/bash
echo "🍄 Substrate Capability Test"
echo "=============================="
echo ""
echo "Substrate: ${TOADSTOOL_SUBSTRATE:-unknown}"
echo ""

echo "System Information:"
echo "  OS: $(uname -s)"
echo "  Kernel: $(uname -r)"
echo "  Architecture: $(uname -m)"
echo "  Hostname: $(hostname)"
echo ""

echo "Resource Availability:"
echo "  CPUs: $(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 'unknown')"
echo "  Memory: $(free -h 2>/dev/null | grep Mem | awk '{print $2}' || echo 'unknown')"
echo ""

echo "Substrate Features:"
case "${TOADSTOOL_SUBSTRATE}" in
  native)
    echo "  ✓ Native process execution"
    echo "  ✓ Direct system access"
    echo "  ✓ Maximum performance"
    ;;
  docker)
    echo "  ✓ Container isolation"
    echo "  ✓ Resource limits"
    echo "  ✓ Network isolation"
    ;;
  python)
    echo "  ✓ Python runtime"
    echo "  ✓ Managed environment"
    echo "  ✓ Package isolation"
    ;;
  wasm)
    echo "  ✓ WebAssembly sandbox"
    echo "  ✓ Maximum security"
    echo "  ✓ Cross-platform"
    ;;
  *)
    echo "  ✓ Universal compatibility"
    ;;
esac

echo ""
echo "✅ Test complete!"
"""

[resources]
cpu_cores = 1
memory_mb = 128
disk_mb = 10

[metadata]
tags = ["demo", "test", "substrate"]
```

---

## 🎯 **DEMO COMMANDS** (CLI Interface)

### **Core Commands**:

```bash
# Initialize showcase
toadstool showcase init

# Run specific phase
toadstool showcase run --phase 1   # Hello world
toadstool showcase run --phase 2   # Intelligence
toadstool showcase run --phase 3   # Migration
toadstool showcase run --phase 4   # Diversity
toadstool showcase run --phase 5   # Failover

# Run full showcase
toadstool showcase run --all

# List available substrates
toadstool substrate list

# Test substrate
toadstool substrate test --substrate native
toadstool substrate test --substrate docker

# Run workload on specific substrate
toadstool run workloads/hello.toml --substrate native
toadstool run workloads/hello.toml --substrate docker

# Start stateful workload
toadstool start workloads/counter.toml --substrate native

# Live migrate workload
toadstool migrate counter-migration --to docker

# Check workload status
toadstool status counter-migration

# Stop workload
toadstool stop counter-migration

# Cleanup showcase
toadstool showcase cleanup
```

---

## 📊 **SUCCESS METRICS**

### **Demo Quality**:
- ✅ All phases run without errors
- ✅ Live migration completes in <5 seconds
- ✅ No data loss during migration
- ✅ Substrate failover happens automatically
- ✅ All workloads execute correctly on all substrates

### **User Experience**:
- ✅ Setup takes <5 minutes
- ✅ Each phase is clear and obvious
- ✅ "Wow moment" happens in Phase 3 (migration)
- ✅ Total demo time: 15-30 minutes
- ✅ Viewer understands ToadStool's value

### **Technical Validation**:
- ✅ No hardcoded paths or assumptions
- ✅ Works on Linux, macOS, Windows
- ✅ Gracefully handles missing optional components
- ✅ Clear error messages if prerequisites missing
- ✅ Complete cleanup after demo

---

## 🔨 **IMPLEMENTATION PLAN**

### **Phase 1: Foundation** (This session)
```
Priority: HIGH
Estimated: 4-6 hours

Tasks:
1. ✅ Create specs/TOADSTOOL_LOCAL_SHOWCASE_SPEC.md (this file)
2. Create demos/toadstool-showcase/ directory structure
3. Implement showcase.sh main runner
4. Create workload definitions (4 TOML files)
5. Implement utils/verify.sh (prerequisite check)
6. Implement utils/setup.sh (environment setup)
7. Implement utils/cleanup.sh (cleanup)

Deliverables:
- Complete demo directory structure
- Working prerequisite verification
- All workload definitions ready
```

### **Phase 2: Core Demos** (Next session)
```
Priority: HIGH
Estimated: 6-8 hours

Tasks:
1. Implement phase1-hello.sh (multi-substrate hello)
2. Implement phase2-intelligence.sh (substrate selection)
3. Implement phase3-migration.sh (⭐ live migration)
4. Test on local machine
5. Fix bugs and edge cases

Deliverables:
- Working Phases 1-3
- Live migration demo functional
```

### **Phase 3: Advanced Features** (Future session)
```
Priority: MEDIUM
Estimated: 4-6 hours

Tasks:
1. Implement phase4-diversity.sh (substrate showcase)
2. Implement phase5-failover.sh (resilience)
3. Add CLI enhancements for better UX
4. Polish output formatting
5. Add demo recording capabilities

Deliverables:
- Complete showcase (all 5 phases)
- Production-ready demo
```

### **Phase 4: Documentation** (Future session)
```
Priority: MEDIUM
Estimated: 2-4 hours

Tasks:
1. Write demos/toadstool-showcase/README.md
2. Create quick start guide
3. Record demo video
4. Create troubleshooting guide
5. Document common issues

Deliverables:
- Complete documentation
- Video demo
- Troubleshooting guide
```

---

## 🎬 **DEMO SCRIPT** (What to Say)

### **Introduction** (30 seconds):
> "Hi, I'm going to show you ToadStool, a universal compute platform.
> The core idea is simple: **Write once, run anywhere**.
> Not just containers. Not just cloud. ANYWHERE.
> Let me show you."

### **Phase 1** (2 minutes):
> "Here's a simple hello world script. Watch what happens when I run it:
> 
> First, native: [run] - It runs as a local process.
> Second, Docker: [run] - Same script, now in a container.
> Third, Python: [run] - Same script, Python runtime.
> 
> **Same workload. Three different substrates. Zero code changes.**"

### **Phase 2** (2 minutes):
> "Now let's make ToadStool think. This workload needs 2 CPU cores and 512MB RAM.
> I'm NOT telling it where to run. Watch:
> 
> [run with --strategy optimal]
> 
> ToadStool analyzes available substrates and picks native because it has the best resources.
> **It's making intelligent decisions for you.**"

### **Phase 3** ⭐ (5 minutes):
> "Now the cool part. I'm going to start a counter - it counts from 1 to 1000.
> 
> [start counter on native]
> 
> See it counting? 10, 11, 12...
> 
> Now watch this: [migrate to docker]
> 
> Did you see that? 38, 39, 40... **It kept counting!**
> 
> The workload MOVED from native to Docker container **without stopping**.
> No data loss. No interruption. It just moved.
> 
> Let me move it back: [migrate to native]
> 
> 67, 68, 69... Still going!
> 
> **This is live migration of compute. Your workload flows between substrates like liquid.**"

### **Conclusion** (1 minute):
> "So that's ToadStool:
> - One workload definition
> - Runs on ANY substrate
> - Live migration between substrates
> - Zero configuration
> - Intelligent substrate selection
> 
> **If it computes, ToadStool runs it. Anywhere. Automatically.**"

---

## 🎯 **KEY MESSAGES**

### **For Technical Audience**:
- "Universal runtime abstraction layer"
- "Substrate-agnostic workload execution"
- "Live migration with state preservation"
- "Zero-config substrate detection"

### **For Business Audience**:
- "Run your code anywhere without rewriting"
- "Never locked into one platform"
- "Automatic optimization and failover"
- "Reduce infrastructure complexity"

### **For General Audience**:
- "Your code works everywhere"
- "Moves between systems automatically"
- "Always finds the best place to run"
- "Never goes down"

---

## ✅ **ACCEPTANCE CRITERIA**

### **Must Have**:
- ✅ Demo runs start-to-finish without errors
- ✅ Live migration works reliably
- ✅ All substrates execute workloads correctly
- ✅ Clear visual feedback at each step
- ✅ "Wow factor" is obvious

### **Should Have**:
- ✅ Colorful output (emojis, colors)
- ✅ Progress indicators
- ✅ Clear phase separation
- ✅ Pause between phases for explanation
- ✅ Cleanup is automatic

### **Nice to Have**:
- ⏳ Demo recording mode
- ⏳ Slow-motion mode for presentations
- ⏳ Screenshot generation
- ⏳ Metrics visualization
- ⏳ Performance comparison charts

---

## 🚀 **NEXT STEPS**

### **Right Now** (This Session):
1. Create demo directory structure
2. Write workload definitions
3. Implement utility scripts
4. Start on Phase 1 script

### **Next Session**:
1. Complete Phase 1-3 scripts
2. Test full showcase
3. Fix bugs
4. Polish UX

### **Future**:
1. Add Phases 4-5
2. Create documentation
3. Record demo video
4. Prepare for public showcase

---

## 📝 **NOTES**

### **Design Decisions**:
- **Why local only?**: Easier to demo, no cloud costs, faster iteration
- **Why 5 phases?**: Builds complexity gradually, clear progression
- **Why live migration is Phase 3?**: It's the "wow" moment, put it in middle
- **Why bash/python?**: Universal, everyone understands, easy to read

### **Technical Constraints**:
- Must work on single machine (no network required)
- Must handle missing optional dependencies (Docker, Python)
- Must cleanup completely after demo
- Must be reproducible (no random behavior)

### **Future Extensions**:
- Multi-machine tower demo
- Cloud extension demo
- BiomeOS integration demo
- Full ecosystem orchestration demo

---

**Status**: ✅ **SPEC COMPLETE - READY FOR IMPLEMENTATION**  
**Next**: Create demo directory structure and start implementation  
**Priority**: HIGH  
**Timeline**: Start now, complete foundation this session

**Let's build this! 🍄**

