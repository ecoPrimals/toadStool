# Next Session Plan - Deep Debt Evolution Session 2

## Priority 1: Complete Hardcoded Primal Name Elimination

### Task 1.1: Deprecate Old Functions (30 min)
**Files**: `crates/core/common/src/primal_sockets.rs`

Add deprecation to:
- `get_beardog_socket_path()`
- `get_songbird_socket_path()`
- `get_nestgate_socket_path()`
- `get_squirrel_socket_path()`

**Pattern**:
```rust
#[deprecated(since = "0.2.0", note = "Use discover_crypto_socket() for capability-based discovery")]
pub fn get_beardog_socket_path() -> PathBuf {
    // Refactor to use new discovery internally
    tokio::runtime::Handle::current()
        .block_on(discover_crypto_socket())
        .unwrap_or_else(|_| get_biomeos_dir().join("beardog.sock"))
}
```

### Task 1.2: Migrate BearDog Integration (45 min)
**Files**: 
- `crates/integration/beardog/src/discovery.rs`
- `crates/core/toadstool/src/biomeos_integration/storage_backend.rs`
- Others that call `get_beardog_socket_path()`

**Find all usages**:
```bash
rg "get_beardog_socket_path" --type rust
```

**Replace pattern**:
```rust
// OLD
let socket = get_beardog_socket_path();

// NEW
let socket = discover_crypto_socket().await?;
```

### Task 1.3: Migrate NestGate Integration (30 min)
Same pattern as BearDog for `get_nestgate_socket_path()` → `discover_storage_socket()`

## Priority 2: Fix tarpc Client Unix Socket Support (45 min)

**File**: `crates/client/src/tarpc_client.rs`

**Current Issue**: Uses `TcpStream` instead of `UnixStream`

**Fix**:
```rust
// OLD
let stream = TcpStream::connect(addr).await?;

// NEW  
let stream = UnixStream::connect(socket_path).await?;
let transport = tarpc::serde_transport::new(
    tokio_serde::formats::Json::default(),
    stream
);
```

## Priority 3: Begin nn.rs Refactoring (60 min)

**Create module structure**:
```bash
mkdir -p crates/barracuda/src/nn
```

**Phase 1**: Extract config module
- Create `nn/config.rs` (~100 lines)
- Move `NetworkConfig`, `HardwarePreference`, `HardwareCapabilities`
- Update `nn.rs` to use `mod config;`

**Test after each step!**

## Time Budget

| Task | Estimated | Priority |
|------|-----------|----------|
| Deprecate functions | 30 min | P1 |
| Migrate BearDog | 45 min | P1 |
| Migrate NestGate | 30 min | P1 |
| Fix tarpc client | 45 min | P2 |
| Begin nn.rs refactoring | 60 min | P3 |
| **Total** | **210 min (3.5 hours)** | |

## Success Criteria

- [ ] All deprecated functions have clear migration path
- [ ] At least 2 modules migrated to capability discovery
- [ ] tarpc client supports Unix sockets
- [ ] nn/config.rs extracted and working
- [ ] All tests still pass
- [ ] Code compiles cleanly

## Commands to Run

```bash
# Find all usages of hardcoded functions
rg "get_beardog_socket_path|get_nestgate_socket_path|get_songbird_socket_path" --type rust

# Test after changes
cargo test -p toadstool-common
cargo test -p toadstool-integration-beardog
cargo test -p toadstool-client

# Check compilation
cargo check --workspace
```

## Notes

- Maintain backward compatibility throughout
- Test incrementally (one module at a time)
- Document any issues encountered
- Update DEEP_DEBT_PROGRESS document
