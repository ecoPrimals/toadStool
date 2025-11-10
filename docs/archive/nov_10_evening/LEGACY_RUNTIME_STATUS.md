# Legacy Runtime Status - November 10, 2025

## 🏭 **LEGACY RUNTIME** - Requires Dedicated Session

**Purpose**: Support for old/exotic hardware (mainframes, PLCs, embedded, industrial)  
**Status**: ⏳ **40% COMPLETE** - Needs dedicated 2-3 hour session  
**Priority**: MEDIUM (not blocking current functionality)

---

## ✅ **WHAT'S DONE** (40%)

### **1. Module Structure** ✅
- ✅ Module organization complete (`types/`, `mainframe/`, `embedded/`, etc.)
- ✅ `async-trait` dependency added
- ✅ Type imports fixed in most files

### **2. Core Types** ✅
- ✅ `LegacySystemType` - 20+ system types defined
- ✅ `LegacyArchitecture` - All architectures defined
- ✅ `LegacyJob` - Job structure complete
- ✅ `CompilationRequirements` - Fully specified
- ✅ `LegacyRuntimeRequirements` - All requirements defined

### **3. Main Engine** ✅
- ✅ `LegacyRuntimeEngine` struct defined
- ✅ RuntimeEngine trait implementation started
- ✅ Initialization logic written
- ✅ Job submission/status/cancellation logic written

---

## ⏳ **WHAT'S PENDING** (60%)

### **1. Missing Type Definitions** (Est. 30 minutes)

Need to define in `types/`:
- `CommunicationSession`
- `SystemEmulator`
- `EmbeddedConfig`, `IndustrialConfig`, `RealtimeConfig`, `EmulationConfig`
- `LegacyRuntimeMetrics`
- `JobStatus`, `JobOutput`

### **2. Adapter Implementations** (Est. 1 hour)

Need to implement in `mainframe/`, `embedded/`, `industrial/`, `realtime/`:
- `IBMMainframeAdapter`
- `VAXVMSAdapter`
- `AS400Adapter`
- `Microcontroller8BitAdapter`
- `System16BitAdapter`
- `PLCAdapter`
- `SCADAAdapter`
- `VxWorksAdapter`
- `QNXAdapter`

### **3. Cross-Compilation & Emulation** (Est. 30 minutes)

Need to implement in `cross_compilation/`, `emulation/`:
- `Toolchain6502`
- `ToolchainZ80`
- `Toolchain68000`
- `PDP11Emulator`
- `Apple2Emulator`

### **4. Type Fixes** (Est. 30 minutes)

Fix remaining import errors:
- Import `RuntimeCapabilities` and `WorkloadType` from toadstool core
- Fix trait/struct confusion for toolchains and emulators
- Add missing Default implementations

---

## 🎯 **COMPILATION ERRORS**

Current error count: **~20 unique errors** (down from 83+)

Main categories:
1. **Type not found** (10 errors) - Missing type definitions
2. **Expected trait, found struct** (4 errors) - Architecture issue with toolchains/emulators
3. **Unresolved imports** (6 errors) - Need to import from toadstool core

---

## 🔧 **HOW TO FIX** (Dedicated Session Plan)

### **Phase 1: Type Definitions** (30 min)

```rust
// In types/systems.rs
pub struct CommunicationSession {
    pub session_id: Uuid,
    pub system_type: LegacySystemType,
    pub connected_at: DateTime<Utc>,
    // ... more fields
}

pub struct SystemEmulator {
    pub emulator_id: Uuid,
    pub system_type: LegacySystemType,
    pub state: EmulatorState,
    // ... more fields
}
```

### **Phase 2: Config Types** (15 min)

```rust
// In types/configs.rs
pub struct EmbeddedConfig {
    pub system_id: String,
    pub architecture: LegacyArchitecture,
    // ... more fields
}

// Similar for IndustrialConfig, RealtimeConfig, EmulationConfig
```

### **Phase 3: Adapter Stubs** (45 min)

```rust
// In mainframe/ibm.rs
pub struct IBMMainframeAdapter {
    config: MainframeConfig,
}

impl IBMMainframeAdapter {
    pub fn new() -> Arc<dyn LegacyAdapter> {
        Arc::new(Self {
            config: MainframeConfig::default(),
        })
    }
}

#[async_trait]
impl LegacyAdapter for IBMMainframeAdapter {
    // Implement all methods
}
```

### **Phase 4: Testing** (30 min)

```bash
cargo check --package toadstool-runtime-legacy
cargo test --package toadstool-runtime-legacy
```

---

## 🚀 **WHY THIS MATTERS**

The legacy runtime is **critical** for ToadStool's "universal compute" claim:

### **Supported Systems** (when complete)

**Mainframes**:
- IBM System/360, System/370, System/390, z/OS
- DEC VAX/VMS
- IBM AS/400, IBM i
- HP 3000, HP-UX

**Embedded**:
- MOS 6502 (Apple II, Commodore 64, NES)
- Zilog Z80 (CP/M, ZX Spectrum)
- Intel 8080/8085 (early PCs)
- Intel 8086/8088 (IBM PC, MS-DOS)
- Motorola 68000 (Amiga, Atari ST, early Mac)

**Industrial**:
- PLC (Programmable Logic Controllers)
- SCADA systems
- Real-time industrial controllers
- Process control systems

**Real-Time**:
- VxWorks
- QNX
- RT-11
- RTOS variants

---

## 📊 **IMPACT ANALYSIS**

### **Without Legacy Runtime**

✅ ToadStool works fine for:
- Modern native execution
- Containers (Docker, containerd)
- WebAssembly
- GPU compute
- ML training
- Python workloads

### **With Legacy Runtime**

✅ ToadStool becomes **truly universal**:
- Run 1970s-1990s software
- Support industrial/embedded systems
- Enable mainframe migration
- Historical computing preservation
- Complete "any chip, any memory" claim

---

## 💡 **RECOMMENDATION**

### **For Now**

✅ **Keep legacy runtime disabled** in production
- Capability system is complete and working
- Other runtimes (Native, Container, WASM, GPU) are production-ready
- No blocking issues for current use cases

### **Next Sprint**

⏳ **Dedicate 2-3 hour focused session** to:
1. Define all missing types (30 min)
2. Implement adapter stubs (1 hour)
3. Fix compilation errors (30 min)
4. Add basic tests (30 min)
5. Enable in workspace (5 min)

### **Long-Term**

🚀 **Full implementation** (multiple sprints):
1. Real mainframe connectivity (TN3270, SSH, telnet)
2. Cross-compilation toolchains (GCC, SDCC, CC65)
3. System emulators (SIMH, MAME, custom)
4. Industrial protocols (Modbus, OPC UA, Profinet)
5. Real-time scheduling and guarantees

---

## 🎯 **CURRENT FOCUS**

**Priority 1** ✅: Capability System
- **Status**: COMPLETE
- **Deliverable**: Primal-agnostic registration & workload routing
- **Impact**: Enables distributed compute across ecosystem

**Priority 2** ⏳: Legacy Runtime
- **Status**: 40% COMPLETE
- **Deliverable**: Universal hardware support
- **Impact**: Completes "universal compute" claim

**Priority 3** 🔮: Additional Primals
- **Status**: READY TO START
- **Deliverable**: Squirrel, BearDog adapters
- **Impact**: Full ecosystem integration

---

## ✅ **BOTTOM LINE**

The legacy runtime is **important but not urgent**:

1. ✅ **Don't Block on It**: Capability system is complete and ready
2. ⏳ **Needs Dedicated Time**: 2-3 hours of focused work
3. 🎯 **Clear Path Forward**: Well-defined tasks and structure
4. 💪 **40% Done**: Significant progress already made

**Proceed with capability system deployment and testing. Schedule legacy runtime completion for next sprint.**

---

**Created**: November 10, 2025  
**Status**: ⏳ Awaiting Dedicated Session  
**Estimate**: 2-3 hours to completion  
**Priority**: MEDIUM

