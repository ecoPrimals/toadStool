# Legacy Runtime Config Migration Plan

**Created**: November 9, 2025  
**Status**: Week 1, Day 1 - Analysis Complete  
**Target**: `crates/runtime/legacy/src/types/configs.rs` (918 lines)

---

## 📊 **ANALYSIS RESULTS**

### **Total Config Structs Found**: 13

```rust
1. SessionConfig              (lines 74-86)
2. CommunicationSettings      (lines 209-219) ← PRIMARY TARGET
3. AuthenticationSettings     (lines 239-251)
4. ToolchainConfig           (lines 269-289)
5. MainframeConfig           (lines 292-304)
6. ConnectionSettings         (lines 307-317) ← USES CommunicationSettings patterns
7. DatasetConfig             (lines 337-351)
8. SpaceAllocation           (lines 384-392)
9. JCLSettings               (lines 408-420) ← HAS time_limit: Duration
10. COBOLSettings            (lines 423-433)
11. EmbeddedConfig           (lines 436-446)
12. MemoryLayout             (lines 449-457)
13. MemoryRegion             (lines 460-472)

Plus 9 more supporting configs:
14. PeripheralConfig         (lines 501-513)
15. ProgrammingInterface     (lines 545-551)
16. IndustrialConfig         (lines 573-583)
17. IndustrialDevice         (lines 632-644)
18. SafetyConfig             (lines 668-676) ← HAS safety timing (domain-specific)
19. SafetyFunction           (lines 692-702) ← HAS Duration fields
20. EmergencyStopConfig      (lines 722-730) ← HAS Duration field
21. RealtimeConfig           (lines 746-756)
22. TaskConfig               (lines 799-813) ← HAS Duration fields
23. InterruptConfig          (lines 816-826)
24. EmulationConfig          (lines 844-856)
25. ROMFile                  (lines 873-886)
26. DiskImage                (lines 889-901)
```

---

## 🎯 **MIGRATION TARGETS**

### **Priority 1: CommunicationSettings** (HIGH IMPACT)

**Current Structure**:
```rust
pub struct CommunicationSettings {
    pub connection_type: ConnectionType,
    pub timeout: Duration,              // ← TimeoutConfig
    pub retry_count: u32,               // ← RetryConfig.max_retries
    pub authentication: Option<AuthenticationSettings>,
}
```

**Proposed Migration**:
```rust
use toadstool_common::config_bases::{TimeoutConfig, RetryConfig};

pub struct CommunicationSettings {
    pub connection_type: ConnectionType,
    
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,
    
    #[serde(flatten)]
    pub retries: RetryConfig,
    
    pub authentication: Option<AuthenticationSettings>,
}
```

**Impact**:
- Used by: MainframeConfig via ConnectionSettings
- Benefits: Consistent timeout/retry behavior
- Effort: 2-3 hours

---

### **Priority 2: JCLSettings** (MEDIUM IMPACT)

**Current Structure**:
```rust
pub struct JCLSettings {
    pub job_class: String,
    pub message_class: String,
    pub priority: u8,
    pub time_limit: Duration,           // ← Execution timeout
    pub region_size: u64,
}
```

**Proposed Migration**:
```rust
use toadstool_common::config_bases::TimeoutConfig;

pub struct JCLSettings {
    pub job_class: String,
    pub message_class: String,
    pub priority: u8,
    
    // Use execution_timeout from TimeoutConfig
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,
    
    pub region_size: u64,
}
```

**Alternative** (if only one timeout needed):
```rust
pub struct JCLSettings {
    pub job_class: String,
    pub message_class: String,
    pub priority: u8,
    pub execution_timeout: Duration,    // Renamed from time_limit
    pub region_size: u64,
}
```

**Impact**:
- Used by: MainframeConfig
- Benefits: Consistent timeout naming
- Effort: 1-2 hours
- **Recommendation**: Use alternative (single field) - domain-specific

---

### **Priority 3: Safety-Critical Timings** (KEEP DOMAIN-SPECIFIC)

**Configs with Duration fields**:
- `SafetyFunction` (response_time, test_interval)
- `EmergencyStopConfig` (response_time)
- `TaskConfig` (period, deadline)

**Decision**: **DO NOT MIGRATE**

**Rationale**:
- These are **safety-critical** or **real-time** constraints
- Different semantics than network timeouts
- Should remain explicit and domain-specific
- Mixing with general timeout configs could be dangerous

**Example of what NOT to do**:
```rust
// ❌ BAD: Don't do this!
pub struct SafetyFunction {
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,  // WRONG - not a network timeout!
    // ...
}
```

---

## 📋 **MIGRATION STRATEGY**

### **Phase 1A: CommunicationSettings Migration** (Day 2-3)

**Step 1**: Add imports
```rust
// At top of crates/runtime/legacy/src/types/configs.rs
use toadstool_common::config_bases::{TimeoutConfig, RetryConfig};
```

**Step 2**: Update CommunicationSettings
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationSettings {
    pub connection_type: ConnectionType,
    
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,
    
    #[serde(flatten)]
    pub retries: RetryConfig,
    
    pub authentication: Option<AuthenticationSettings>,
}
```

**Step 3**: Add Default impl
```rust
impl Default for CommunicationSettings {
    fn default() -> Self {
        Self {
            connection_type: ConnectionType::LocalEmulation,
            timeouts: TimeoutConfig::default(),
            retries: RetryConfig::default(),
            authentication: None,
        }
    }
}
```

**Step 4**: Find usage sites
```bash
grep -rn "\.timeout\|\.retry_count" crates/runtime/legacy/src/
```

**Step 5**: Update usage sites
```rust
// BEFORE:
let timeout = settings.timeout;
let retries = settings.retry_count;

// AFTER:
let timeout = settings.timeouts.connection_timeout;
let retries = settings.retries.max_retries;
```

**Step 6**: Test
```bash
cargo test -p toadstool-runtime-legacy
cargo check --workspace
```

---

### **Phase 1B: JCLSettings Review** (Day 4)

**Decision Point**: Rename or keep as-is?

**Option A: Keep as domain-specific** (RECOMMENDED)
```rust
pub struct JCLSettings {
    // ... existing fields ...
    pub job_execution_timeout: Duration,  // Renamed for clarity
    // ... existing fields ...
}
```

**Option B: Use TimeoutConfig**
```rust
pub struct JCLSettings {
    // ... existing fields ...
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,
    // ... existing fields ...
}
```

**Recommendation**: **Option A** (rename for clarity, keep domain-specific)

---

### **Phase 1C: Documentation** (Day 5)

**Create**: `LEGACY_RUNTIME_MIGRATION_GUIDE.md`

**Contents**:
1. What was migrated
2. Why (consistent timeout/retry behavior)
3. Before/after examples
4. Usage guide
5. Breaking changes (field name changes)

---

## 📊 **EXPECTED OUTCOMES**

### **After Phase 1A (CommunicationSettings)**:
```
✅ CommunicationSettings uses base configs
✅ Consistent timeout behavior with rest of codebase
✅ Easier to configure (standard timeout fields)
✅ All tests passing
```

### **After Phase 1B (JCLSettings)**:
```
✅ Clearer field naming (job_execution_timeout)
✅ Better documentation
✅ No unnecessary base config usage
```

### **After Phase 1C (Documentation)**:
```
✅ Migration guide complete
✅ Breaking changes documented
✅ Usage examples provided
```

---

## 🔍 **ANALYSIS: WHAT NOT TO MIGRATE**

### **Configs to KEEP Domain-Specific**:

1. **SessionConfig** - Terminal-specific settings
2. **AuthenticationSettings** - Security-specific
3. **ToolchainConfig** - Build toolchain paths
4. **DatasetConfig** - Mainframe dataset specs
5. **SpaceAllocation** - Storage allocation
6. **COBOLSettings** - COBOL compiler settings
7. **EmbeddedConfig** - Hardware-specific
8. **MemoryLayout** - Memory map
9. **MemoryRegion** - Memory region specs
10. **PeripheralConfig** - Hardware peripherals
11. **ProgrammingInterface** - ISP/JTAG/SWD
12. **IndustrialConfig** - PLC/SCADA/DCS
13. **IndustrialDevice** - Industrial devices
14. **SafetyConfig** - Safety-critical (DO NOT TOUCH)
15. **SafetyFunction** - Safety timing (DO NOT TOUCH)
16. **EmergencyStopConfig** - Safety (DO NOT TOUCH)
17. **RealtimeConfig** - RTOS-specific
18. **TaskConfig** - Real-time scheduling (domain-specific)
19. **InterruptConfig** - Hardware interrupts
20. **EmulationConfig** - Emulator settings
21. **ROMFile** - ROM file metadata
22. **DiskImage** - Disk image metadata

**Rationale**: All of these are **domain-specific** and don't have standard timeout/retry patterns.

---

## 🎯 **SUMMARY**

### **Configs to Migrate**: 1
- **CommunicationSettings** (timeout + retry patterns)

### **Configs to Rename** (optional): 1
- **JCLSettings** (time_limit → job_execution_timeout for clarity)

### **Configs to Keep As-Is**: 24+
- All domain-specific configs

### **Total Effort**:
- Phase 1A (CommunicationSettings): 2-3 hours
- Phase 1B (JCLSettings rename): 1 hour (optional)
- Phase 1C (Documentation): 1 hour
- **Total**: 4-5 hours (vs. original estimate of 40-60 hours)

### **Revised Impact**:
- **Performance**: Minimal direct impact (consolidation benefit)
- **Consistency**: High (timeout/retry behavior unified)
- **Maintainability**: High (fewer custom timeout implementations)

---

## 💡 **KEY INSIGHT**

**The legacy runtime configs are MOSTLY DOMAIN-SPECIFIC and SHOULD remain so.**

Only **CommunicationSettings** has clear timeout/retry patterns that benefit from base config migration.

The rest are correct as-is:
- Hardware-specific (embedded, peripherals)
- Safety-critical (do not touch!)
- Real-time constraints (domain-specific)
- Tool configuration (paths, options)
- Data structures (memory maps, datasets)

**This is GOOD NEWS**: The configs are already well-designed!

---

## 📅 **REVISED TIMELINE**

### **Week 1**:
- ✅ Day 1: Analysis (COMPLETE)
- Day 2: CommunicationSettings migration
- Day 3: Test and validate
- Day 4: JCLSettings review (optional rename)
- Day 5: Documentation

### **Result**:
- **1 week** instead of 2-3 weeks for config consolidation
- **4-5 hours** of actual work
- **Focus shifts to async_trait migration** (Week 2)

---

## 🚀 **NEXT STEPS**

### **Immediate** (Day 2):
1. Implement CommunicationSettings migration
2. Update usage sites
3. Run tests

### **Day 3**:
1. Validate all tests pass
2. Check for edge cases
3. Review with team

### **Day 4**:
1. Optional: Rename JCLSettings.time_limit
2. Update documentation

### **Day 5**:
1. Create migration guide
2. Document breaking changes
3. Prepare for async_trait migration (Week 2)

---

**Migration Plan Created**: November 9, 2025  
**Analysis Complete**: ✅  
**Ready to Proceed**: ✅  
**Estimated Effort**: 4-5 hours (significantly less than expected!)

🍄 **ToadStool - Smart migration focused on real opportunities!** 🎯

