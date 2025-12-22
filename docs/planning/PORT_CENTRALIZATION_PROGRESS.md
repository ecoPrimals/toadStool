# Port Centralization Progress - Phase 1 Complete

## ✅ **Phase 1: Centralization** (COMPLETE)

**Created**: `crates/core/config/src/ports.rs` (180 lines)

### What We Built

#### 1. Centralized Port Definitions
```rust
pub mod toadstool {
    pub const SERVER: u16 = 8084;
    pub const GPU_COMPUTE: u16 = 8085;
    pub const DISTRIBUTED: u16 = 8086;
    pub const HEALTH: u16 = 8087;
    pub const METRICS: u16 = 9090;
}
```

#### 2. Fallback Ports (Temporary)
```rust
pub mod fallback {
    // Other primal ports - marked for removal after discovery
    pub const SONGBIRD: u16 = 8080;  // TODO: Remove after discovery
    pub const BEARDOG: u16 = 8081;   // TODO: Remove after discovery
    // etc...
}
```

#### 3. Environment Variable Support (Phase 2)
```rust
pub fn server_port() -> u16 {
    get_port_with_env(toadstool::SERVER, "TOADSTOOL_SERVER_PORT")
}
```

#### 4. Test Port Generation
```rust
pub mod test {
    pub fn unique_port(test_id: u16) -> u16 {
        BASE + (std::process::id() as u16 % 1000) + test_id
    }
}
```

### Philosophy Applied

**Self-Knowledge**: ToadStool defines only its own ports. Other primal ports are in `fallback` module marked for removal.

**Evolution Path**:
- ✅ Phase 1: Centralize (done)
- 📋 Phase 2: Environment overrides (implemented, needs adoption)
- 📋 Phase 3: Runtime discovery via Songbird
- 📋 Phase 4: Full mDNS + capability-based

### Next Steps

#### Immediate (Adoption)
1. Update all hardcoded `8084` → `config::ports::server_port()`
2. Update all hardcoded `8080` → discover via Songbird (not fallback)
3. Update tests to use `config::ports::test::unique_port()`

#### Short-term (Discovery Integration)
4. Implement runtime discovery in server startup
5. Remove `fallback` module once discovery works
6. Update all inter-primal communication to use discovery

#### Example Migration

**Before**:
```rust
let url = "http://localhost:8080"; // Hardcoded Songbird
```

**After (Phase 1 - Temporary)**:
```rust
use toadstool_config::ports;
let port = ports::fallback::SONGBIRD; // Centralized fallback
let url = format!("http://localhost:{}", port);
```

**After (Phase 3 - Target)**:
```rust
use toadstool::common::runtime_discovery::RuntimeDiscovery;
let discovery = RuntimeDiscovery::new(client);
let songbird = discovery.discover_capability(&Capability::Coordination).await?;
let url = songbird.endpoint; // Runtime discovered!
```

### Impact

**Before**:
- 755 hardcoded port references across 144 files
- No way to override ports
- Tests conflict with each other
- No runtime discovery

**After Phase 1**:
- 1 source of truth (`ports.rs`)
- Environment variable overrides available
- Test port generation prevents conflicts
- Foundation for runtime discovery

**After Full Evolution**:
- Zero hardcoded ports
- Full runtime discovery
- True self-knowledge principle
- Production-ready inter-primal communication

### Status

- ✅ **Created**: `ports.rs` module
- ✅ **Integrated**: Added to `config` crate
- ✅ **Tested**: Unit tests passing
- 📋 **Adoption**: Needs to replace 755 hardcoded instances
- 📋 **Discovery**: Needs integration with RuntimeDiscovery

---

**Phase 1 Complete**: December 19, 2025  
**Time**: 30 minutes  
**Next**: Adopt centralized ports across codebase

