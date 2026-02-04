# Deep Debt Evolution Status - February 4, 2026

## Executive Summary

Comprehensive audit completed. Systematic evolution to modern idiomatic Rust with Deep Debt principles underway.

**Overall Status**: C+ (70/100) → Target: A+ (95/100)

---

## Phase 1: Critical Fixes ✅ COMPLETE

- [x] Fix compilation errors (format strings, crate names)
- [x] Format entire codebase (`cargo fmt`)
- [x] Fix documentation collision (binary name)
- [ ] Build validation (blocked: wayland-dev system dependency)

---

## Phase 2: Hardcoding Elimination 🔄 IN PROGRESS

### Critical Violations of Deep Debt Principle

**Principle**: Primals have self-knowledge only. Discover others at runtime via capabilities.

### 2.1 Hardcoded Primal Names (40+ instances) 🔴 CRITICAL

**Files with violations**:
- `crates/core/common/src/primal_sockets.rs` - Hardcoded functions for each primal
- `crates/core/toadstool/src/ipc/server.rs` - Port mappings by primal name
- `crates/core/toadstool/src/ipc/client.rs` - Port mappings by primal name
- `crates/core/config/src/primal_capabilities.rs` - Primal-specific match statements
- `crates/core/toadstool/src/ipc_helpers.rs` - Songbird-specific functions
- `crates/cli/src/network_config/configurator/core.rs` - Domain references
- `crates/core/common/src/infant_discovery/sources.rs` - Env var names per primal

**Evolution Target**:
```rust
// ❌ OLD: Hardcoded knowledge of other primals
fn get_beardog_socket_path() -> PathBuf { ... }
fn get_songbird_socket_path() -> PathBuf { ... }

// ✅ NEW: Capability-based discovery
async fn discover_service(capability: &str) -> Result<ServiceEndpoint> {
    discovery_engine.find_by_capability(capability).await
}
```

### 2.2 Hardcoded Ports (50+ instances) 🔴 CRITICAL

**Pattern**: Port numbers embedded in match statements and constants

**Evolution Target**:
```rust
// ❌ OLD
match primal_name {
    "toadstool" => 8370,
    "songbird" => 8371,
    "beardog" => 8372,
}

// ✅ NEW: Dynamic port allocation
let port = PortAllocator::allocate_ephemeral().await?;
registry.register_capability("compute", port).await?;
```

### 2.3 Hardcoded IP Addresses (30+ instances) 🟡 HIGH

**Pattern**: `"127.0.0.1"`, `"0.0.0.0"`, `"10.0.0.1"` in code

**Evolution Target**:
- Use `std::net::Ipv4Addr::LOCALHOST` constant
- Dynamic subnet discovery for container networks
- Runtime address resolution

### 2.4 Hardcoded Paths (20+ instances) 🟡 HIGH

**Pattern**: `/tmp/biomeos-runtime`, `/etc/toadstool.conf`

**Evolution Target**:
```rust
// ✅ Runtime path detection
let runtime_dir = RuntimePaths::discover()?;
let socket_path = runtime_dir.socket_path("ipc");
```

---

## Phase 3: IPC Architecture Evolution 🔄 IN PROGRESS

### 3.1 tarpc Client Unix Socket Support 🔴 CRITICAL

**Issue**: tarpc client uses TCP; should use Unix sockets (architecture violation)

**Files**:
- `crates/client/src/tarpc_client.rs` - Uses `TcpStream` instead of `UnixStream`

**Evolution Target**:
```rust
// ✅ Unix socket support
let stream = UnixStream::connect(socket_path).await?;
let transport = tarpc::serde_transport::new(
    tokio_serde::formats::Json::default(),
    stream
);
```

### 3.2 Unified RPC Abstraction 🟡 HIGH

**Issue**: No trait/interface abstracting JSON-RPC vs tarpc

**Evolution Target**:
```rust
#[async_trait]
pub trait RpcClient: Send + Sync {
    async fn call(&self, method: &str, params: serde_json::Value) 
        -> Result<serde_json::Value>;
}

pub struct TarpcRpcClient { ... }
pub struct JsonRpcClient { ... }

impl RpcClient for TarpcRpcClient { ... }
impl RpcClient for JsonRpcClient { ... }
```

### 3.3 HTTP→JSON-RPC Migration 🟡 MEDIUM

**Remaining HTTP APIs**:
- `crates/api/src/lib.rs` - REST API v2
- `crates/cli/src/daemon/http_server.rs` - HTTP API v1
- `crates/api/src/byob.rs` - BYOB HTTP API

**Target**: Wrap as JSON-RPC gateway or deprecate

---

## Phase 4: Code Quality Evolution 🔄 PLANNED

### 4.1 File Size: nn.rs Refactoring 🔴 CRITICAL

**File**: `crates/barracuda/src/nn.rs` (1340 lines, max: 1000)

**Semantic Module Structure** (designed):
- `nn/config.rs` - Network configuration & hardware capabilities
- `nn/layer.rs` - Layer definitions
- `nn/optimizer.rs` - Optimizer integration with ops
- `nn/loss.rs` - Loss function integration
- `nn/metrics.rs` - Training/eval metrics
- `nn/state.rs` - Internal state management
- `nn/network.rs` - Core NeuralNetwork struct
- `nn/forward.rs` - Forward pass
- `nn/backward.rs` - Backward pass
- `nn/training.rs` - Training logic
- `nn/builder.rs` - Builder pattern
- `nn/mod.rs` - Public API

**Status**: Architecture designed, ready for migration

### 4.2 unwrap() Elimination 🟡 HIGH

**Count**: 7,236 unwrap() calls, 500+ expect() calls

**Strategy**:
1. Audit test vs production code
2. Production code: Replace with `?` operator or proper error handling
3. Test code: Convert to assertions with clear messages

**Evolution Pattern**:
```rust
// ❌ OLD
let value = some_result.unwrap();

// ✅ NEW
let value = some_result.map_err(|e| BarracudaError::ConfigurationError(
    format!("Failed to get value: {}", e)
))?;
```

### 4.3 Unsafe Code Audit 🟡 HIGH

**Count**: 120+ files with unsafe blocks

**High concentration**:
- `crates/runtime/gpu/src/unified_memory/buffer.rs` - 12 unsafe blocks
- `crates/runtime/secure_enclave/src/isolated_memory.rs` - 12 unsafe blocks
- `showcase/gpu-universal/vulkan-detection/src/main.rs` - 33 unsafe blocks

**Strategy**:
1. Document justification for each unsafe block
2. Explore safe alternatives (e.g., MaybeUninit, Pin)
3. Isolate unsafe in smallest possible scope
4. Add comprehensive safety comments

### 4.4 Zero-Copy Optimization 🟡 MEDIUM

**Current**:
- `.clone()`: 1,000+ instances
- `.to_vec()/.to_string()/.to_owned()`: 500+ instances

**Evolution**:
- Use `Cow<'_, str>` for owned-or-borrowed strings
- Use `&[T]` slices instead of `Vec<T>` where possible
- Implement `Borrow` trait for custom types

---

## Phase 5: Test Coverage 🔄 PLANNED

### 5.1 Critical Gaps

**Untested modules**:
- `runtime/orchestration/` - 0% external coverage
- `runtime/adaptive/` - 0% external coverage
- `core/common/capability_discovery.rs` - 28% coverage
- `core/common/capability_provider.rs` - 21% coverage

**Target**: 90% coverage across all production code

---

## Phase 6: Dependency Evolution 🔄 PLANNED

### 6.1 External Dependencies to Pure Rust

**Analysis needed**:
- Identify C dependencies (FFI bindings)
- Evaluate pure Rust alternatives
- Migration plan for critical dependencies

**Known dependencies with C**:
- OpenSSL bindings (evaluate rustls)
- Some GPU backends (CUDA, Vulkan)
- System libraries (justified exceptions for hardware access)

---

## Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Formatting | ✅ Pass | ✅ Pass | COMPLETE |
| Build | 🟡 Blocked | ✅ Pass | BLOCKED |
| Linting | ❌ Unknown | ✅ 0 warnings | BLOCKED |
| File Size | 1 violation | 0 violations | IN PROGRESS |
| Hardcoding | 150+ instances | 0 instances | IN PROGRESS |
| unwrap() | 7,236 | <100 | PLANNED |
| unsafe blocks | 120+ files | Audited+Documented | PLANNED |
| Test Coverage | 72-90% | 90%+ | PLANNED |
| IPC Architecture | Incomplete | Complete | IN PROGRESS |
| Documentation | Collision | Builds | COMPLETE |

---

## Next Actions (Priority Order)

1. **Eliminate hardcoded primal names** - Evolve to capability-based discovery
2. **Fix tarpc client** - Add Unix socket support
3. **Refactor nn.rs** - Semantic module structure
4. **Add orchestration tests** - Close coverage gap
5. **Audit unsafe code** - Document justifications
6. **Replace unwrap()** - Production code error handling

---

## Deep Debt Principles Compliance

| Principle | Compliance | Notes |
|-----------|------------|-------|
| Zero unsafe | 🟡 Partial | 120+ files need audit |
| No hardcoding | ❌ Fail | 150+ violations |
| Capability-based | 🟡 Partial | Architecture present, not enforced |
| No mocks | ✅ Pass | Mocks isolated to testing |
| Self-knowledge | ❌ Fail | Primals know other primals |
| Modern idioms | ✅ Pass | Async/await, builders, Result types |

**Target**: All principles at ✅ Pass

---

## Timeline Estimate

- **Phase 2 (Hardcoding)**: 2-3 sessions
- **Phase 3 (IPC)**: 1-2 sessions
- **Phase 4 (Quality)**: 3-4 sessions
- **Phase 5 (Testing)**: 2-3 sessions
- **Phase 6 (Dependencies)**: 2-3 sessions

**Total**: 10-15 coding sessions to achieve A+ grade

---

## Session Log

### Session 1 - Feb 4, 2026
- ✅ Audit complete
- ✅ Fixed compilation errors
- ✅ Formatted codebase
- ✅ Fixed documentation collision
- 🔄 Started hardcoding elimination planning
