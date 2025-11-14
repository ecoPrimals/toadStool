# 🔨 Config Module Refactoring Plan

**Target**: `crates/core/config/src/lib.rs` (1556 lines → 3 focused modules)  
**Strategy**: Extract type definitions, keep helper modules inline  
**Approach**: Domain-driven split

---

## 📐 Current Structure Analysis

```
crates/core/config/src/lib.rs (1556 lines)
  Lines 1-573: Inline helper modules (network, app, testing, development, production)
  Lines 574-1556: 23 struct definitions (~982 lines)
  
Already has submodules:
  - config_utils.rs (24KB)
  - defaults.rs (20KB)
  - env_config.rs (22KB)
  - runtime_defaults.rs (37KB)
```

---

## 📊 New Structure (Smart & Modern)

```
crates/core/config/src/
  ├── lib.rs                (~300 lines) - Module declarations, helper modules, re-exports
  ├── types/
  │   ├── mod.rs           (~50 lines) - Type module coordinator
  │   ├── core.rs          (~250 lines) - Core config types (ToadStoolConfig, ApplicationConfig)
  │   ├── network.rs       (~200 lines) - Network types (NetworkConfig, EndpointConfig, etc.)
  │   ├── runtime.rs       (~200 lines) - Runtime types (RuntimeConfig, ContainerConfig, etc.)
  │   ├── security.rs      (~300 lines) - Security types (SecurityConfig, AuthConfig, etc.)
  │   └── observability.rs (~200 lines) - Logging, Metrics, Database types
  ├── config_utils.rs      (existing)
  ├── defaults.rs          (existing)
  ├── env_config.rs        (existing)
  └── runtime_defaults.rs  (existing)
```

**Total**: lib.rs (300) + types (1200) = 1500 lines  
**All files under 300 lines** ✅

---

## 🎯 Type Groupings

### `types/core.rs` (~250 lines)
```rust
- ToadStoolConfig (main config struct)
- ApplicationConfig
- FeatureFlags
```

### `types/network.rs` (~200 lines)
```rust
- NetworkConfig
- EndpointConfig
- ConnectionConfig
- TlsConfig
```

### `types/runtime.rs` (~200 lines)
```rust
- RuntimeConfig
- ResourceLimits
- ContainerConfig
- WasmConfig
- PythonConfig
- GpuConfig
```

### `types/security.rs` (~300 lines)
```rust
- SecurityConfig
- AuthConfig
- AuthzConfig
- EncryptionConfig
- AuditConfig
- SandboxConfig
```

### `types/observability.rs` (~200 lines)
```rust
- LoggingConfig
- DatabaseConfig
- BackendCacheConfig
- MetricsConfig
```

---

## 🚀 Implementation Steps

### Phase 1: Create Types Module Structure
1. Create `crates/core/config/src/types/` directory ✅
2. Create `types/mod.rs` with re-exports
3. Extract structs to domain files
4. Verify compilation

### Phase 2: Update lib.rs
1. Add `pub mod types;`
2. Re-export types: `pub use types::*;`
3. Keep helper modules inline
4. Verify tests pass

### Phase 3: Verify & Test
1. Run `cargo check --package toadstool-config`
2. Run `cargo test --package toadstool-config`
3. Verify no breaking changes
4. Check imports across workspace

---

## ✅ Benefits

1. **Clear Domain Separation** - Types grouped by responsibility
2. **Easy Navigation** - Find config by domain (security, network, runtime)
3. **Better Maintainability** - Changes isolated to relevant files
4. **Compilation Speed** - Smaller files = faster incremental compilation
5. **All Files Under 300 Lines** - Highly compliant! ✅

---

## 📝 Example: types/mod.rs

```rust
//! Configuration type definitions organized by domain

pub mod core;
pub mod network;
pub mod runtime;
pub mod security;
pub mod observability;

// Re-export all types for convenience
pub use core::*;
pub use network::*;
pub use runtime::*;
pub use security::*;
pub use observability::*;
```

---

## 📝 Example: lib.rs (after refactoring)

```rust
//! ToadStool Configuration System

pub mod config_utils;
pub mod defaults;
pub mod env_config;
pub mod runtime_defaults;
pub mod types;  // NEW

// Re-export types for backwards compatibility
pub use types::*;

// Inline helper modules stay here
pub mod network {
    // ~240 lines of helper functions
}

pub mod app {
    // ~120 lines of constants
}

// ... other inline modules
```

---

**Next**: Execute Phase 1 - Create types module

