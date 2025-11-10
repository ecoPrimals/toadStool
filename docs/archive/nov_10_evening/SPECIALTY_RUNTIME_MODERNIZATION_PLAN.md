# 🏭 Specialty Hardware Runtime - Modernization Plan
**Date**: November 10, 2025 (Evening)  
**Status**: 🔧 IN PROGRESS  
**Goal**: Modernize and re-enable the specialty hardware runtime

---

## 🎯 MISSION

Transform the "legacy" runtime into a **competitive advantage** by:
1. ✅ Renaming to "specialty" (no deprecation stigma)
2. 🔧 Fixing compilation errors
3. 🔧 Modernizing to match core patterns
4. 🔧 Re-enabling in workspace build

---

## ✅ PHASE 1: RENAMING (COMPLETE)

### Actions Taken
- [x] Renamed `crates/runtime/legacy` → `crates/runtime/specialty`
- [x] Updated Cargo.toml package name
- [x] Created new README.md emphasizing competitive advantage
- [x] Removed "TEMPORARILY_DISABLED" marker

### Results
- **New name**: `toadstool-runtime-specialty`
- **New identity**: Specialty hardware support (not deprecated)
- **Status**: Ready for modernization

---

## 🔧 PHASE 2: MODERNIZATION (IN PROGRESS)

### 2.1 Fix Compilation Errors (83+ identified)

**Error Categories**:
1. **Type Resolution** (60+ errors)
   - Missing imports
   - Type definition mismatches
   - Namespace issues

2. **Trait Implementation** (8+ errors)
   - `RuntimeEngine` trait updates
   - Async pattern migration
   - Send/Sync bounds

3. **Missing Imports** (15+ errors)
   - Cross-module dependencies
   - Core type imports

### 2.2 Type System Modernization

**Current Issues**:
```rust
// OLD: Using outdated types
error[E0412]: cannot find type `LegacySystemType` in this scope
error[E0412]: cannot find type `LegacyArchitecture` in this scope
error[E0412]: cannot find type `SystemStatus` in this scope
```

**Modern Pattern**:
```rust
// NEW: Use core toadstool types
use toadstool::{RuntimeType, ExecutionRequest, ExecutionResponse};
use toadstool::resources::ResourceRequirements;
use toadstool::error::ToadStoolResult;
```

### 2.3 Config Pattern Adoption

**Target**: Use base config patterns

```rust
use toadstool_common::config_bases::{
    TimeoutConfig,
    RetryConfig,
    ConnectionPoolConfig,
};

#[derive(Debug, Clone)]
pub struct SpecialtyRuntimeConfig {
    // Platform-specific
    pub platform_type: SpecialtyPlatform,
    pub hardware_features: Vec<HardwareFeature>,
    
    // Base configs
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,
    
    #[serde(flatten)]
    pub retries: RetryConfig,
}
```

### 2.4 Async Pattern Migration

**Current**: Mix of async_trait and blocking code  
**Target**: Native `impl Future` patterns

```rust
// BEFORE
#[async_trait]
pub trait SpecialtyExecutor {
    async fn execute(&self, req: Request) -> Result<Response>;
}

// AFTER
pub trait SpecialtyExecutor {
    fn execute(&self, req: Request) -> impl Future<Output = Result<Response>>;
}
```

---

## 📋 PHASE 3: RE-ENABLE (PLANNED)

### 3.1 Workspace Integration

**Update root Cargo.toml**:
```toml
members = [
    # ... other members ...
    "crates/runtime/specialty",  # Re-enable!
]
```

### 3.2 Testing Strategy

1. **Unit Tests**: Per-module testing
2. **Integration Tests**: Platform simulation
3. **Mock Hardware**: Virtual device testing

### 3.3 Documentation

1. **API Docs**: Comprehensive rustdoc
2. **Examples**: Per-platform examples
3. **Migration Guide**: For existing users

---

## 🎯 SUCCESS CRITERIA

### Build Health
- [ ] Zero compilation errors
- [ ] Zero warnings (except dependency warnings)
- [ ] All tests passing

### Code Quality
- [ ] Modern type system (uses core types)
- [ ] Base config adoption
- [ ] Native async patterns
- [ ] Zero unsafe blocks

### Integration
- [ ] Re-enabled in workspace
- [ ] Compatible with other runtimes
- [ ] Example code working

---

## 📊 CURRENT STATUS

### Completed ✅
- [x] Phase 1: Renaming complete
- [x] README and documentation created
- [x] Cargo.toml updated

### In Progress 🔧
- [ ] Phase 2.1: Fix compilation errors
- [ ] Phase 2.2: Type system modernization
- [ ] Phase 2.3: Config pattern adoption
- [ ] Phase 2.4: Async pattern migration

### Planned 📅
- [ ] Phase 3.1: Workspace integration
- [ ] Phase 3.2: Testing strategy
- [ ] Phase 3.3: Documentation completion

---

## 🚀 NEXT STEPS

### Immediate (Tonight)
1. Fix critical type resolution errors
2. Update imports to use core types
3. Resolve trait implementation issues

### This Week
1. Complete compilation error fixes
2. Adopt base config patterns
3. Test basic functionality

### This Month
1. Re-enable in workspace
2. Comprehensive testing
3. Documentation polish
4. Customer validation

---

## 💡 KEY INSIGHTS

### Why This Matters
1. **Market Differentiation**: No other platform supports these systems
2. **Industrial Value**: Critical for manufacturing/infrastructure
3. **Strategic Asset**: Opens enterprise markets

### Technical Advantages
1. **Mainframe Access**: IBM z/OS, AS/400 integration
2. **Embedded Control**: Arduino, ESP32, STM32
3. **Industrial Protocols**: Modbus, Profinet, OPC UA
4. **Real-Time**: Deterministic scheduling

---

## 📚 REFERENCE

### Files Modified
- `crates/runtime/specialty/Cargo.toml` - Package rename
- `crates/runtime/specialty/README.md` - New documentation
- All source files TBD during modernization

### Related Documents
- `UNIFICATION_AUDIT_NOV_10_2025_EVENING.md` - Context
- `EXECUTIVE_SUMMARY_NOV_10_EVENING.md` - Overall status
- `specs/` - Architecture specifications

---

**Specialty Hardware Runtime** - *Universal Compute for Every Platform* 🏭

*Modernization in progress - transforming from "legacy" to "specialty" status*

