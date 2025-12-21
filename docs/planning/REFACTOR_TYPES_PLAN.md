# Smart Refactoring Plan: types.rs → Domain-Driven Module

## Analysis

Current file: `crates/core/config/src/types.rs` (1,002 lines)

**Why it's large**: Contains ALL configuration types for ToadStool platform

**Is it bad?** No! It's actually well-organized by domain. The issue is discoverability and cognitive load.

## Domain Analysis (Behavioral, Not Mechanical)

### 1. **Application Lifecycle Domain** (86 lines)
- `ApplicationConfig`
- Handles: app identity, directories, threading, shutdown
- **Cohesion**: HIGH - these all relate to app lifecycle

### 2. **Network & Communication Domain** (125 lines)
- `NetworkConfig`, `EndpointConfig`, `ConnectionConfig`, `TlsConfig`
- Handles: networking, service endpoints, connections, TLS
- **Cohesion**: HIGH - all about network communication

### 3. **Runtime Execution Domain** (185 lines)
- `RuntimeConfig`, `ResourceLimits`
- `ContainerConfig`, `WasmConfig`, `PythonConfig`, `GpuConfig`
- Handles: workload execution across multiple runtimes
- **Cohesion**: HIGH - all about executing code

### 4. **Security & Access Control Domain** (176 lines)
- `SecurityConfig`, `AuthConfig`, `AuthzConfig`
- `EncryptionConfig`, `AuditConfig`, `SandboxConfig`
- Handles: security, authentication, authorization, auditing
- **Cohesion**: HIGH - all about security

### 5. **Observability Domain** (137 lines)
- `LoggingConfig`, `MetricsConfig`
- `DatabaseConfig`, `BackendCacheConfig`
- Handles: observability, persistence, caching
- **Cohesion**: MEDIUM - logging/metrics (high), DB/cache (medium)

### 6. **Feature Management Domain** (55 lines)
- `FeatureFlags`
- Handles: feature toggles, experimental features
- **Cohesion**: HIGH - all feature flags

### 7. **Configuration Orchestration** (138 lines)
- `ToadStoolConfig` (root orchestrator)
- Methods: load, validate, merge, override
- **Cohesion**: HIGH - configuration management

## Smart Refactoring Strategy

### Option A: Domain Modules (Recommended)
Create `types/` directory with domain-driven modules:

```
crates/core/config/src/
├── types/
│   ├── mod.rs                  (re-exports + ToadStoolConfig orchestrator)
│   ├── application.rs          (ApplicationConfig)
│   ├── network.rs              (Network* configs)
│   ├── runtime.rs              (Runtime* configs)
│   ├── security.rs             (Security* configs)
│   ├── observability.rs        (Logging, Metrics, DB, Cache)
│   └── features.rs             (FeatureFlags)
└── types.rs                    (DEPRECATED, re-export from types/)
```

### Option B: Keep Monolithic with Better Organization
If splitting hurts API ergonomics, keep single file but add:
- Clear section markers (80-char dividers)
- Module-level documentation per domain
- Table of contents at top

### Option C: Hybrid (Best of Both)
- Keep `types.rs` as facade (small, re-exports everything)
- Move implementations to domain modules
- Maintain backward compatibility
- Users import from `config::types::*` as before

## Recommendation: Option C (Hybrid)

### Why?
1. **Backward compatible**: Existing imports work
2. **Discoverability**: Domain modules for deep work
3. **Ergonomics**: Simple imports for common use
4. **Future-proof**: Can evolve modules independently

### Benefits
- ✅ Reduces cognitive load (each domain ~50-185 lines)
- ✅ Easier testing (domain-focused tests)
- ✅ Better parallel development (less merge conflicts)
- ✅ Clearer ownership (domain experts)
- ✅ Maintains API compatibility

### Implementation Plan

#### Step 1: Create domain modules (1 hour)
```bash
mkdir -p crates/core/config/src/types/
touch crates/core/config/src/types/{mod,application,network,runtime,security,observability,features}.rs
```

#### Step 2: Extract Application domain (10 min)
Move `ApplicationConfig` to `types/application.rs`

#### Step 3: Extract Network domain (15 min)
Move Network* types to `types/network.rs`

#### Step 4: Extract Runtime domain (20 min)
Move Runtime* types to `types/runtime.rs`

#### Step 5: Extract Security domain (20 min)
Move Security* types to `types/security.rs`

#### Step 6: Extract Observability domain (20 min)
Move Logging/Metrics/DB/Cache to `types/observability.rs`

#### Step 7: Extract Features domain (10 min)
Move `FeatureFlags` to `types/features.rs`

#### Step 8: Create facade mod.rs (20 min)
- Implement `ToadStoolConfig` orchestrator
- Re-export all types
- Add domain documentation

#### Step 9: Update types.rs as backward compat facade (10 min)
```rust
//! Configuration type definitions
//!
//! **New**: Types are organized in domain modules under `types/`.
//! This file provides backward compatibility.

mod types;
pub use types::*;
```

#### Step 10: Test & validate (30 min)
- Run all tests
- Verify imports work
- Check documentation
- Validate no breaking changes

### Total Time: ~3 hours (smart refactoring)

## Alternative: Keep Monolithic

If we decide file is fine as-is:

**Improvements**:
1. Add section dividers (5 min)
2. Add table of contents (5 min)
3. Add domain documentation (15 min)
4. Keep monitoring for future growth

**Total Time**: 25 minutes

## Decision Criteria

**Refactor if**:
- File will grow beyond 1500 lines
- Multiple developers working on different domains
- Want domain-focused testing
- Planning to add more config types

**Keep monolithic if**:
- File is stable (not growing)
- Single developer/small team
- Simple imports preferred
- No domain conflicts

## My Recommendation

**Hybrid refactoring (Option C)** because:
1. File is at the threshold (1,002 lines)
2. Clear domains exist
3. Future growth likely (GPU/Edge configs)
4. Improves maintainability
5. Zero breaking changes
6. Professional codebase standard

**Time investment**: 3 hours for long-term win


