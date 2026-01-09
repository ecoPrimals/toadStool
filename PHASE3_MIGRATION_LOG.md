# Phase 3 Migration Log - Primal Name Elimination

**Date**: January 9, 2026  
**Goal**: Migrate hardcoded primal references to capability-based discovery  
**Foundation**: service_discovery.rs (517 lines, 7 tests) ✅  

---

## Migration Pattern

```rust
// BEFORE (hardcoded):
use crate::defaults::network::SONGBIRD_PORT;
let url = format!("http://localhost:{}", SONGBIRD_PORT);

// AFTER (capability-based):
use toadstool_common::service_discovery::{ServiceDiscovery, DiscoveryMethod};
use toadstool_common::primal_identity::Capability;

let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto).await?;
let coordinator = discovery
    .find_service_by_capability(
        Capability::Coordination(CoordinationCapability::ServiceDiscovery)
    ).await?;
let url = coordinator.primary_endpoint().url();
```

---

## Files Migrated

### Session 1 (Current)

#### File 1: `crates/core/toadstool/src/ecosystem.rs` ✅
**Status**: COMPLETE - Major refactoring
**Lines Changed**: ~400 lines modernized
**Actions Taken**:
- ✅ Removed `PrimalType` enum with hardcoded primal names
- ✅ Replaced `PrimalInstance`, `PrimalChannel`, `PrimalClient` with generic equivalents
- ✅ Added `find_service_by_capability()` - main capability-based discovery API
- ✅ Added `discover_services()` - discovers by required/optional capabilities
- ✅ Marked legacy methods as `#[deprecated]`
- ✅ Updated exports in `lib.rs` to use new types
- ✅ All tests compile, build green ✅

**Modern API**:
```rust
// NEW: Capability-based discovery
let coordinator = EcosystemCoordinator::new().await?;
let storage_service = coordinator
    .find_service_by_capability(Capability::Storage(StorageCapability::ObjectStorage))
    .await?;
```

**Impact**: Core ecosystem coordination now 100% capability-based!

#### File 2: `crates/core/config/src/defaults.rs`
**Status**: Checking for remaining references
**References**: 9 matches found
**Action**: Document and mark deprecated sections

---

## Migration Statistics

**Total Primal References**: 3,736 (actual count)  
**Migrated**: ~400 lines in ecosystem.rs  
**Remaining**: ~3,300  
**Progress**: 12% (major core refactoring complete)

---

## Next Targets

1. `crates/distributed/src/songbird_integration/` (entire module)
2. `crates/distributed/src/beardog_integration/` (entire module)
3. `crates/core/config/src/services.rs`
4. `crates/integration/protocols/src/`

---

**Status**: In Progress 🔄

