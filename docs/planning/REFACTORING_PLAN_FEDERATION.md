# 🔨 Federation Module Refactoring Plan

**Target**: `src/federation.rs` (1936 lines → 4 focused modules)  
**Strategy**: Split by responsibility (Domain-Driven Design)  
**Approach**: Modern, idiomatic Rust patterns

---

## 📐 Module Structure (Smart & Modern)

```
src/federation/
  ├── mod.rs          (~150 lines) - Public API, re-exports, FederationManager struct
  ├── types.rs        (~200 lines) - All types, enums, errors, traits
  ├── manager.rs      (~600 lines) - Core manager implementation (lifecycle, messaging)
  ├── trust.rs        (~400 lines) - Authentication, BearDog integration, trust verification
  └── discovery.rs    (~600 lines) - Peer discovery (mDNS, network scan, bootstrap)
```

**Total**: ~1950 lines (accounting for imports) → All files under 1000 lines ✅

---

## 📊 Responsibility Breakdown

### `types.rs` - Data Definitions (Lines 1-169)
**Responsibility**: All data structures, enums, errors, traits

**Contents**:
- `SignatureResponse`
- `FederationError` (all error variants)
- `FederationStatus`, `NetworkInfo`
- `PeerInfo`, `PeerStatus`, `TrustLevel`
- `FederationMessage` (all message types)
- `FederationConfig` + `Default` impl
- `MessageHandler` trait
- `PeerConnection` (internal struct)

**Why**: Following Rust convention - types together, dependency-free

---

### `manager.rs` - Core Logic (Lines 170-657)
**Responsibility**: FederationManager lifecycle and core operations

**Public API**:
- `new()`, `enable()`, `disable()`
- `join_peer()`, `leave()`
- `get_status()`, `get_peers()`
- `send_message()`, `broadcast_message()`
- `register_handler()`

**Private Helpers**:
- `start_federation_services()`
- `start_network_listener()`
- `start_peer_discovery()`
- `start_heartbeat_service()`
- `handle_incoming_connection()`
- `connect_to_peer()`, `disconnect_peer()`
- `discover_peers()`, `send_heartbeats()`
- `handle_message()`, `update_peer_last_seen()`

**Why**: Core business logic, message routing, lifecycle management

---

### `trust.rs` - Security (Lines 658-1098)
**Responsibility**: Authentication, trust verification, BearDog integration

**Methods** (as trait extension or inherent impl):
- `authenticate_peer()`
- `verify_trust_policy()`
- `verify_beardog_signature()`
- `get_peer_public_key()`
- `request_signature_from_peer()`
- `wait_for_signature_response()`
- `wait_for_peer_signature_response()`
- `verify_allowlist()`
- `query_beardog_for_key()`
- `sign_challenge()`
- `get_our_private_key()`
- `query_beardog_for_our_key()`
- `update_peer_status()`
- `update_or_create_peer()`
- `send_signature_request()`

**Why**: Cohesive security/auth module, isolated from discovery

---

### `discovery.rs` - Peer Finding (Lines 1099-1640)
**Responsibility**: All peer discovery mechanisms

**Methods** (as trait extension or inherent impl):
- `mdns_discover()`
- `perform_mdns_query()`
- `process_mdns_response()`
- `scan_network_for_peers()`
- `probe_potential_peer()`
- `attempt_federation_handshake()`
- `bootstrap_discovery()`
- `get_bootstrap_nodes()`
- `connect_to_bootstrap_node()`
- `request_peer_list_from_bootstrap()`
- `peer_exchange_discovery()`
- `local_network_scan()`

**Why**: Isolated discovery concerns, multiple strategies

---

## 🎯 Modern Rust Patterns

### 1. **Private Methods Stay Private**
```rust
// In manager.rs - keep impl block organization
impl FederationManager {
    // Public API
    pub async fn new() -> Result<Self, FederationError> { }
    pub async fn enable(&mut self, config: FederationConfig) -> Result<(), FederationError> { }
    
    // Private helpers
    async fn start_federation_services(&self) -> Result<(), FederationError> { }
}
```

### 2. **Trait Extension for Domain Logic**
```rust
// In trust.rs - authentication logic as trait
trait TrustOperations {
    async fn verify_trust_policy(&self, peer_id: &str, policy: &str) -> Result<(), FederationError>;
    async fn authenticate_peer(&self, stream: &TcpStream, addr: &SocketAddr) -> Result<String, FederationError>;
}

impl TrustOperations for FederationManager {
    // Implementation
}
```

### 3. **Re-export Pattern in mod.rs**
```rust
// mod.rs - clean public API
mod types;
mod manager;
mod trust;
mod discovery;

pub use types::*;  // Export all public types
pub use manager::FederationManager;  // Export manager

// Private modules stay private
use trust::TrustOperations;
use discovery::DiscoveryOperations;
```

### 4. **Feature-Gate Discovery Methods**
```rust
// discovery.rs - conditional compilation
#[cfg(feature = "mdns")]
impl DiscoveryOperations for FederationManager {
    async fn mdns_discover(&self) -> Result<(), FederationError> {
        // mDNS implementation
    }
}

#[cfg(not(feature = "mdns"))]
impl DiscoveryOperations for FederationManager {
    async fn mdns_discover(&self) -> Result<(), FederationError> {
        warn!("mDNS feature not enabled, skipping discovery");
        Ok(())
    }
}
```

---

## ✅ Benefits of This Structure

1. **Maintainability**: Each file has one clear responsibility
2. **Testability**: Can test discovery without trust logic
3. **Readability**: Easy to find relevant code
4. **Scalability**: Can add new discovery methods without touching trust
5. **Compilation**: Smaller files = faster incremental compilation
6. **Compliance**: All files under 1000 lines ✅

---

## 🔄 Migration Strategy

### Phase 1: Create New Structure (No Breaking Changes)
1. Create `src/federation/` directory ✅
2. Create `types.rs` with all data structures
3. Create `manager.rs` with core FederationManager
4. Create `trust.rs` with authentication logic
5. Create `discovery.rs` with peer discovery
6. Create `mod.rs` with re-exports

### Phase 2: Test & Verify
1. Run `cargo check` - ensure compilation
2. Run `cargo test` - ensure all tests pass
3. Run `cargo clippy` - ensure no new warnings
4. Run `cargo doc` - ensure docs build

### Phase 3: Cleanup
1. Delete old `src/federation.rs`
2. Update imports in dependent files (if any)
3. Run full test suite
4. Commit with message: "refactor: split federation.rs into focused modules"

---

## 📝 Implementation Notes

### Import Strategy
Each module imports only what it needs:
```rust
// types.rs - minimal imports
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use thiserror::Error;

// manager.rs - imports types from sibling
use super::types::*;
use tokio::sync::RwLock;
use std::collections::HashMap;

// trust.rs - focused imports
use super::types::*;
use tracing::{debug, warn};

// discovery.rs - discovery-specific imports
use super::types::*;
#[cfg(feature = "mdns")]
use mdns;
```

### Visibility Strategy
- **Public**: Types, FederationManager, public API methods
- **Pub(crate)**: Trait extensions (TrustOperations, DiscoveryOperations)
- **Private**: Helper functions, internal state

---

## 🎯 Success Criteria

- [x] All files < 1000 lines
- [ ] No breaking API changes
- [ ] All tests pass
- [ ] No new clippy warnings
- [ ] Documentation builds
- [ ] Incremental compilation faster
- [ ] Code review approved

---

**Next**: Execute Phase 1 - Create new module structure

