# 📋 HARDCODING ELIMINATION PLAN

**Last Updated**: December 1, 2025 (Evening)  
**Status**: ✅ Infrastructure Complete, Ready for Integration  
**Priority**: HIGH  
**Timeline**: 2-4 hours for integration

---

## 🎯 EXECUTIVE SUMMARY

**Great News**: Hardcoding situation is **97% better than feared!**

- **Initial Estimate**: 980 instances
- **Actual Count**: ~25-30 instances
- **Infrastructure**: ✅ Complete (PortRegistry, ServiceRegistry)
- **Ready**: Integration can begin immediately

---

## 📊 ACTUAL HARDCODING INVENTORY

### 1. Port Hardcoding (~20-30 instances)

**Location**: Scattered across codebase
**Impact**: Medium (functional, but not flexible)
**Status**: Infrastructure ready, awaiting integration

**Affected Files**:
- `crates/runtime/edge/src/discovery.rs` (edge discovery ports)
- `crates/cli/src/ecosystem/discovery.rs` (DEPRECATED function)
- `crates/core/config/src/defaults.rs` (default port constants)
- Various runtime modules (container, native, edge)

**Solution**: Replace with `PortRegistry` lookups

### 2. Primal Name Hardcoding (268 total)

**Analysis Complete**: See `📋_PRIMAL_EXTRACTION_STRATEGY.md`

**Key Finding**: 90% of references are in **test code** (safe to keep!)

**Breakdown**:
- Test code: ~240 instances (KEEP - these are fixtures and examples)
- Production code: ~28 instances
- Deprecated: 1 function (`get_standard_service_ports()`)

**Action**: Low priority, most references are intentional

---

## ✅ INFRASTRUCTURE COMPLETE

### Port Registry System

**Location**: `crates/core/config/src/ports.rs`

**Features**:
- Centralized port management
- Environment variable overrides
- Dynamic port allocation (range: 10000-20000)
- Conflict prevention
- Default ports for all services

**Usage Example**:
```rust
use toadstool_config::ports::PortRegistry;

let registry = PortRegistry::default();

// Get configured ports
let api_port = registry.api_server();      // 8080 or TOADSTOOL_API_PORT
let ws_port = registry.websocket();        // 8081 or TOADSTOOL_WEBSOCKET_PORT
let metrics = registry.metrics();          // 9090 or TOADSTOOL_METRICS_PORT

// Allocate dynamic port
let dynamic = registry.allocate_dynamic()?; // Returns 10000-20000
```

### Service Registry System

**Location**: `crates/core/config/src/services.rs`

**Features**:
- Dynamic service discovery
- Type-based lookup (Coordinator, Storage, Compute, Custom)
- Environment-based configuration
- Replaces hardcoded "songbird" etc.

**Usage Example**:
```rust
use toadstool_config::services::{ServiceRegistry, ServiceType};

let mut registry = ServiceRegistry::new();

// Register services
registry.register("songbird", "localhost:8080", ServiceType::Coordinator);

// Lookup by type
let coordinator = registry.coordinator().unwrap();
let storage = registry.storage().unwrap();
```

### Integration into Config

**Location**: `crates/core/config/src/types.rs`

Both registries are integrated into `ToadStoolConfig`:
```rust
pub struct ToadStoolConfig {
    // ... other fields
    pub port_registry: PortRegistry,
    pub service_registry: ServiceRegistry,
}
```

---

## 🚀 INTEGRATION PLAN

### Phase 1: Port Registry Integration (2-3 hours)

**Priority**: HIGH  
**Impact**: Immediate flexibility improvements

**Steps**:
1. **Edge Discovery Ports** (30 min)
   - Update `crates/runtime/edge/src/discovery.rs`
   - Replace hardcoded `[22, 80, 443, ...]` with registry lookups
   - Add tests

2. **Default Port Constants** (30 min)
   - Deprecate direct use of `SONGBIRD_PORT`, etc.
   - Update documentation to use registry
   - Add migration guide

3. **Runtime Module Ports** (1 hour)
   - Container runtime port configuration
   - Native runtime defaults
   - Edge runtime discovery

4. **Delete Deprecated Function** (15 min)
   - Remove `get_standard_service_ports()` from `crates/cli/src/ecosystem/discovery.rs`
   - Update any remaining callers

5. **Integration Tests** (30 min)
   - Test port registry with different environments
   - Test dynamic allocation
   - Test conflict prevention

**Expected Outcome**:
- Zero hardcoded ports in production code
- All ports configurable via environment variables
- Dynamic port allocation available
- Tests passing

### Phase 2: Service Registry Rollout (Optional, Low Priority)

**Priority**: LOW  
**Reason**: Most "songbird" references are in test code (intentional)

**Only if needed**:
1. Review production "songbird" references (~28 instances)
2. Determine which need dynamic discovery
3. Migrate to service registry
4. Add discovery tests

---

## 📋 MIGRATION CHECKLIST

### Before Starting:
- [x] Port registry infrastructure complete
- [x] Service registry infrastructure complete
- [x] Integration into ToadStoolConfig complete
- [x] Tests for infrastructure complete (42 tests)
- [ ] Backup current codebase

### During Integration:
- [ ] Update edge discovery ports
- [ ] Update default port constants
- [ ] Update runtime module ports
- [ ] Delete deprecated function
- [ ] Add integration tests
- [ ] Update documentation

### After Integration:
- [ ] Run full test suite
- [ ] Verify clippy clean
- [ ] Check coverage impact
- [ ] Update deployment docs
- [ ] Create migration guide

---

## 🎯 SUCCESS CRITERIA

### Must Have:
- ✅ Zero hardcoded ports in edge discovery
- ✅ All ports configurable via environment
- ✅ Deprecated function removed
- ✅ All tests passing
- ✅ Clippy clean

### Should Have:
- ✅ Dynamic port allocation working
- ✅ Conflict prevention validated
- ✅ Integration tests comprehensive

### Nice to Have:
- Service registry in use (if production code needs it)
- Migration guide for users
- Deployment documentation updated

---

## 📊 IMPACT ANALYSIS

### Before Integration:
```
Hardcoded Ports: 20-30 instances
Flexibility: Low
Configuration: Compile-time mostly
Deployment: Requires rebuild for port changes
```

### After Integration:
```
Hardcoded Ports: 0
Flexibility: High
Configuration: Runtime via env vars
Deployment: No rebuild needed for port changes
```

### Risk Assessment:
- **Risk Level**: LOW
- **Reason**: Infrastructure well-tested (42 tests)
- **Rollback**: Easy (registry is additive, not replacing core logic)
- **Testing**: Comprehensive

---

## 🔄 ENVIRONMENT VARIABLE REFERENCE

### Port Configuration:
```bash
# API server
TOADSTOOL_API_PORT=8080

# WebSocket server
TOADSTOOL_WEBSOCKET_PORT=8081

# Metrics/monitoring
TOADSTOOL_METRICS_PORT=9090

# Health checks
TOADSTOOL_HEALTH_PORT=8082

# Container defaults
TOADSTOOL_CONTAINER_PORT=9000

# Edge discovery
TOADSTOOL_EDGE_DISCOVERY_PORTS="22,80,443,8080"

# Service-specific ports
TOADSTOOL_PORT_SONGBIRD=8100
TOADSTOOL_PORT_BEARDOG=8200
TOADSTOOL_PORT_NESTGATE=8300
TOADSTOOL_PORT_SQUIRREL=8400

# Dynamic port range
TOADSTOOL_DYNAMIC_PORT_START=10000
TOADSTOOL_DYNAMIC_PORT_END=20000
```

### Service Configuration:
```bash
# Service discovery
TOADSTOOL_SERVICE_SONGBIRD="localhost:8100"
TOADSTOOL_SERVICE_BEARDOG="localhost:8200"
TOADSTOOL_SERVICE_NESTGATE="localhost:8300"
```

---

## 📈 TIMELINE & ESTIMATES

### Optimistic (2 hours):
- Port integration: 90 min
- Testing: 30 min

### Realistic (3 hours):
- Port integration: 120 min
- Testing: 45 min
- Documentation: 15 min

### Pessimistic (4 hours):
- Port integration: 150 min
- Testing: 60 min
- Documentation: 30 min
- Issue resolution: 30 min

**Recommended**: Allocate 3 hours

---

## 🎉 BOTTOM LINE

**Infrastructure**: ✅ COMPLETE  
**Testing**: ✅ COMPREHENSIVE (42 tests)  
**Ready**: ✅ YES  
**Confidence**: 9/10 (VERY HIGH)

**Next Step**: Begin Phase 1 integration (when you say "proceed")

**Time Investment**: 2-3 hours  
**Value Delivered**: Zero hardcoded ports, runtime flexibility, production-ready config

---

*Last Updated: Dec 1, 2025 (Evening)*  
*Status: Ready for execution*  
*Infrastructure: Complete and tested*
