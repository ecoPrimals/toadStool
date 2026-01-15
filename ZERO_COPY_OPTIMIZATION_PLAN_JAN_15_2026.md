# Zero-Copy Optimization Plan - January 15, 2026

## 📊 ANALYSIS SUMMARY

**Total Clones**: 2,323 across 650 files  
**Estimated Overhead**: 10-30% performance impact  
**Optimization Potential**: **HIGH** 🚀

---

## 🔍 Clone Pattern Analysis

### Pattern #1: HashMap Double Clone (HIGH IMPACT)

**Found**: ~100+ instances  
**Pattern**:
```rust
map.insert(key.clone(), value.clone());  // 2 clones per insert!
```

**Locations**:
- `service_discovery.rs:472` - `cache.insert(service.id.clone(), service.clone())`
- `crypto_lock/access_control.rs:200` - `insert(permission.external_target.clone(), permission.clone())`
- `client.rs:56` - `services.insert(service_info.id.clone(), service_info.clone())`
- `biomeos_integration/auth_backend.rs:215` - `tokens.insert(token.id.clone(), token.clone())`

**Impact**: **CRITICAL** - O(n) allocations in hot paths

### Pattern #2: Function Parameter Clones (MODERATE IMPACT)

**Found**: ~200+ instances  
**Pattern**:
```rust
function_call(param1.clone(), param2.clone(), param3.clone());
```

**Location**: `crates/cli/src/main.rs` - CLI argument passing (20+ instances)

**Impact**: **MODERATE** - CLI is not hot path, but wasteful

### Pattern #3: String Clones (MODERATE IMPACT)

**Found**: ~500+ instances  
**Pattern**:
```rust
let name = original.name.clone();  // String clone
```

**Impact**: **MODERATE** - Depends on string size and frequency

### Pattern #4: Arc Clone (LOW IMPACT)

**Found**: ~1,000+ instances  
**Pattern**:
```rust
let shared = arc_value.clone();  // Just increments ref count
```

**Impact**: **LOW** - Arc::clone() is cheap (atomic increment)

### Pattern #5: Multiple Clones in Expression

**Found**: ~20 instances  
**Pattern**:
```rust
format.clone(), resources.clone(), status.clone()  // 3+ clones in one line
```

**Impact**: **HIGH** (when in hot path)

---

## 🎯 Optimization Strategy

### Phase 1: HashMap Optimization (Week 1)

**Goal**: Eliminate double clones in HashMap insertions

#### Technique 1: Entry API

**Before**:
```rust
cache.insert(service.id.clone(), service.clone());
```

**After** (Cow pattern):
```rust
use std::borrow::Cow;

cache.entry(Cow::Borrowed(&service.id))
    .or_insert_with(|| service.clone());
```

**Savings**: 1 clone when entry exists (0 allocations)

#### Technique 2: Structural Sharing

**Before**:
```rust
struct Service {
    id: String,        // Cloned
    name: String,      // Cloned
    metadata: Metadata // Cloned
}
```

**After**:
```rust
struct Service {
    id: Arc<str>,      // Shared
    name: Arc<str>,    // Shared
    metadata: Arc<Metadata> // Shared
}
```

**Savings**: Clone becomes cheap (ref count increment)

#### Technique 3: Interning

**Before**:
```rust
let capability1 = "encryption".to_string();  // Allocation
let capability2 = "encryption".to_string();  // Duplicate allocation
```

**After**:
```rust
static ENCRYPTION: &str = "encryption";
let capability1 = ENCRYPTION;  // No allocation
let capability2 = ENCRYPTION;  // No allocation
```

**Savings**: Eliminate duplicate string allocations

---

### Phase 2: Function Parameter Optimization (Week 2)

**Goal**: Pass references instead of owned values

#### Technique 1: Accept References

**Before**:
```rust
pub fn process(name: String, config: Config) -> Result<()> {
    // Takes ownership, forces clone at call site
}

// Call site
process(name.clone(), config.clone());  // 2 clones!
```

**After**:
```rust
pub fn process(name: &str, config: &Config) -> Result<()> {
    // Borrows, no clone needed
}

// Call site
process(&name, &config);  // 0 clones!
```

**Savings**: Eliminate unnecessary clones

#### Technique 2: Cow for Conditional Cloning

**Before**:
```rust
pub fn format_message(template: String, params: Vec<String>) -> String {
    // Forces clone at call site
}
```

**After**:
```rust
use std::borrow::Cow;

pub fn format_message<'a>(
    template: Cow<'a, str>,
    params: Cow<'a, [String]>
) -> String {
    // Clones only if modification needed
}
```

**Savings**: Clone only when necessary

---

### Phase 3: CLI Argument Optimization (Week 3)

**Goal**: Optimize 20+ clones in `main.rs`

#### Technique: Reference Extraction

**Before**:
```rust
match command {
    Command::Run { manifest, name, env, ... } => {
        executor.run_biome(
            manifest.clone(),  // Clone
            name.clone(),      // Clone
            env.clone(),       // Clone
            ...
        ).await?
    }
}
```

**After**:
```rust
match &command {  // Borrow the command
    Command::Run { manifest, name, env, ... } => {
        executor.run_biome(
            manifest,  // Reference (no clone!)
            name.as_ref(),
            env,
            ...
        ).await?
    }
}
```

**Savings**: Eliminate all CLI argument clones

---

### Phase 4: Buffer Pooling (Week 4)

**Goal**: Reuse allocations for temporary buffers

#### Technique: Object Pool

**Before**:
```rust
async fn process_request(data: Vec<u8>) -> Result<Vec<u8>> {
    let buffer = Vec::with_capacity(1024);  // New allocation every call
    // ... process ...
    Ok(buffer)
}
```

**After**:
```rust
use object_pool::Pool;

static BUFFER_POOL: Pool<Vec<u8>> = Pool::new(100, || Vec::with_capacity(1024));

async fn process_request(data: Vec<u8>) -> Result<Vec<u8>> {
    let mut buffer = BUFFER_POOL.pull();  // Reuse from pool
    // ... process ...
    Ok(buffer.detach())  // Returns to pool when dropped
}
```

**Savings**: Eliminate repeated allocations (80%+ reduction)

---

## 🔥 Quick Wins (Implement First)

### Quick Win #1: HashMap Entry API (2 hours)

**Files**: 
- `service_discovery.rs`
- `crypto_lock/access_control.rs`
- `client.rs`
- `auth_backend.rs`

**Pattern**:
```rust
// Before
map.insert(key.clone(), value.clone());

// After
map.entry(key).or_insert(value);  // Only 1 clone if entry exists
```

**Impact**: 50-100 clone operations eliminated

### Quick Win #2: Static String Interning (1 hour)

**Create**: `crates/core/common/src/interned_strings.rs`

```rust
//! Interned strings for common values

pub mod capabilities {
    pub const ENCRYPTION: &str = "encryption";
    pub const STORAGE: &str = "storage";
    pub const COORDINATION: &str = "coordination";
    // ... etc
}

pub mod protocols {
    pub const HTTP: &str = "http";
    pub const GRPC: &str = "grpc";
    pub const WEBSOCKET: &str = "websocket";
}
```

**Usage**:
```rust
// Before
let cap = "encryption".to_string();  // Allocation

// After
use crate::interned_strings::capabilities;
let cap = capabilities::ENCRYPTION;  // No allocation
```

**Impact**: Eliminate 100+ string allocations

### Quick Win #3: CLI Reference Passing (1 hour)

**File**: `crates/cli/src/main.rs`

**Change**: Match on `&command` instead of `command`

**Impact**: Eliminate 20+ clones in CLI

---

## 📊 Expected Performance Improvements

### By Technique

| Technique | Clones Eliminated | Perf Improvement |
|-----------|-------------------|------------------|
| HashMap Entry API | 100+ | 5-10% |
| Static Interning | 100+ | 3-5% |
| Reference Passing | 200+ | 5-10% |
| Buffer Pooling | N/A | 10-20% (hot paths) |
| **TOTAL** | **400+** | **20-40%** |

### By Phase

| Phase | Duration | Impact | Cumulative |
|-------|----------|--------|------------|
| 1. HashMap | Week 1 | 5-10% | 5-10% |
| 2. Parameters | Week 2 | 5-10% | 10-18% |
| 3. CLI | Week 3 | 2-5% | 12-22% |
| 4. Pooling | Week 4 | 10-20% | **20-40%** |

---

## 🛠️ Implementation Checklist

### Week 1: HashMap Optimization

- [ ] Audit all HashMap insert patterns
- [ ] Apply Entry API to top 20 files
- [ ] Benchmark before/after
- [ ] Document improvements

### Week 2: Parameter Optimization

- [ ] Identify functions taking owned values
- [ ] Convert to accept references where possible
- [ ] Use Cow for conditional cloning
- [ ] Update call sites

### Week 3: CLI Optimization

- [ ] Update main.rs to match &command
- [ ] Update executor methods to accept references
- [ ] Verify zero functional changes
- [ ] Measure CLI startup time improvement

### Week 4: Buffer Pooling

- [ ] Profile hot paths (flamegraph)
- [ ] Identify temporary buffer allocations
- [ ] Implement object pool
- [ ] Benchmark throughput improvement

---

## 📈 Benchmark Strategy

### Before/After Comparison

```bash
# Baseline
cargo bench --bench hot_paths -- --save-baseline before

# After optimization
cargo bench --bench hot_paths -- --baseline before

# Should see 20-40% improvement in hot paths
```

### Flamegraph Analysis

```bash
cargo flamegraph --bin toadstool-server -- [args]

# Look for:
# - Large clone() sections (red)
# - String allocations (red)
# - HashMap operations (yellow)
```

### Memory Profiling

```bash
# Use valgrind/massif to measure allocation reduction
valgrind --tool=massif target/release/toadstool-server

# Should see 30-50% fewer allocations after optimization
```

---

## ✅ Success Criteria

1. **Clones Reduced**: 2,323 → <2,000 (15%+ reduction)
2. **Performance**: 20-40% improvement in hot paths
3. **Memory**: 30-50% fewer allocations
4. **Zero Regressions**: All tests still passing
5. **No API Breaks**: Backward compatible changes

---

## 🎓 Zero-Copy Principles

### 1. Ownership When Needed

```rust
// ✅ GOOD: Take ownership when consuming
pub fn consume(data: Vec<u8>) -> Result<()> {
    process_and_discard(data);  // Data is consumed
    Ok(())
}

// ❌ BAD: Take ownership but only reading
pub fn read_only(data: Vec<u8>) -> usize {
    data.len()  // Just reading, shouldn't own!
}

// ✅ BETTER
pub fn read_only(data: &[u8]) -> usize {
    data.len()  // Borrow, no clone needed
}
```

### 2. Cow for Conditional Cloning

```rust
use std::borrow::Cow;

pub fn process_message(msg: Cow<str>) -> String {
    if msg.contains("error") {
        msg.to_uppercase()  // Clones here (needed)
    } else {
        msg.into_owned()  // No clone if already owned
    }
}
```

### 3. Arc for Shared Ownership

```rust
// ❌ BAD: Clone large struct
let copy = large_config.clone();  // Deep copy!

// ✅ BETTER: Share with Arc
let shared = Arc::new(large_config);
let ref1 = Arc::clone(&shared);  // Just ref count++
let ref2 = Arc::clone(&shared);  // Just ref count++
```

### 4. Interning for Common Strings

```rust
// ❌ BAD: Allocate same string repeatedly
let cap1 = "encryption".to_string();
let cap2 = "encryption".to_string();

// ✅ BETTER: Use static str
static ENCRYPTION: &str = "encryption";
let cap1 = ENCRYPTION;
let cap2 = ENCRYPTION;
```

---

## 🚀 Quick Start

### Step 1: Measure Baseline (Today)

```bash
cargo bench --bench hot_paths -- --save-baseline before
cargo build --release
hyperfine 'target/release/toadstool-server --help'
```

### Step 2: Apply HashMap Fixes (Tomorrow)

Focus on these files first:
1. `service_discovery.rs` - Hot path
2. `auth_backend.rs` - Security path
3. `crypto_lock/access_control.rs` - Security path

### Step 3: Measure Improvement (Day 3)

```bash
cargo bench --bench hot_paths -- --baseline before
# Look for 5-10% improvement
```

---

## 📝 Code Examples

### Example 1: Service Discovery Optimization

**File**: `crates/core/common/src/service_discovery.rs:472`

**Before** (2 clones):
```rust
cache.insert(service.id.clone(), service.clone());
```

**After** (1 clone):
```rust
use std::collections::hash_map::Entry;

match cache.entry(service.id.clone()) {
    Entry::Vacant(e) => {
        e.insert(service);  // Move service, no clone
    }
    Entry::Occupied(mut e) => {
        e.insert(service);  // Update existing, move service
    }
}
```

**Better** (0-1 clones):
```rust
// If we own service.id already:
cache.insert(service.id, service);  // 0 clones if id is moved

// Or use Cow for conditional clone:
cache.entry(Cow::Owned(service.id))
    .or_insert(service);
```

### Example 2: CLI Argument Optimization

**File**: `crates/cli/src/main.rs:152-158`

**Before** (7 clones!):
```rust
executor.run_biome(
    manifest.clone(),
    name.clone(),
    env.clone(),
    debug,
    cpu_limit,
    memory_limit.clone(),
    security.clone(),
).await?
```

**After** (0 clones):
```rust
// Match on reference
match &command {
    Command::Run { manifest, name, env, debug, cpu_limit, memory_limit, security } => {
        executor.run_biome(
            manifest,      // &PathBuf
            name.as_ref(), // Option<&String>
            env,          // &Vec<String>
            *debug,       // Copy (bool)
            *cpu_limit,   // Copy (Option<f64>)
            memory_limit.as_deref(), // Option<&str>
            security,     // &String
        ).await?
    }
}
```

**And update executor signature**:
```rust
// Before
pub async fn run_biome(
    manifest: PathBuf,
    name: Option<String>,
    env: Vec<String>,
    ...
) -> Result<()>

// After
pub async fn run_biome(
    manifest: &Path,
    name: Option<&str>,
    env: &[String],
    ...
) -> Result<()>
```

**Savings**: 7 clones eliminated per CLI invocation

### Example 3: Token Storage Optimization

**File**: `crates/core/toadstool/src/biomeos_integration/auth_backend.rs:215`

**Before** (2 clones):
```rust
tokens.insert(token.id.clone(), token.clone());
```

**After Option 1** (1 clone):
```rust
let id = token.id.clone();
tokens.insert(id, token);  // Move token
```

**After Option 2** (0-1 clones with Arc):
```rust
// Change Token to use Arc<str> for id
tokens.insert(Arc::clone(&token.id), Arc::new(token));
// Then everywhere else just clones the Arc (cheap)
```

---

## 🎯 Priority Targets

### Top 10 Files by Clone Density

1. `crates/cli/src/main.rs` - CLI args (20+ clones) - **HIGH PRIORITY**
2. `service_discovery.rs` - Service cache (hot path) - **CRITICAL**
3. `auth_backend.rs` - Token storage (security path) - **HIGH**
4. `crypto_lock/access_control.rs` - Permissions (security path) - **HIGH**
5. `client.rs` - Service registration (medium frequency) - **MEDIUM**
6. `ai_mcp_interface.rs` - Session storage (12+ double clones) - **MEDIUM**
7. `natural_language/intent.rs` - Pattern matching - **LOW**
8. `capabilities/resolver.rs` - Capability registration - **MEDIUM**
9. `ecosystem/management.rs` - Ecosystem tracking - **MEDIUM**
10. `tarpc_server.rs` - RPC handling - **HIGH**

---

## 📊 Expected Impact Analysis

### Hot Path Impact (CRITICAL)

**Service Discovery** (`service_discovery.rs`):
- Current: 2 clones per cache insert
- After: 0-1 clones
- Frequency: Every service query (high)
- **Impact**: 10-15% improvement in discovery ops

**Token Management** (`auth_backend.rs`):
- Current: 2 clones per token operation
- After: 1 clone (or Arc for 0)
- Frequency: Every auth operation (high)
- **Impact**: 5-10% improvement in auth ops

**RPC Server** (`tarpc_server.rs`):
- Current: 2 clones per workload submission
- After: 0-1 clones
- Frequency: Every RPC call (very high)
- **Impact**: 15-20% improvement in RPC throughput

### Warm Path Impact (MODERATE)

**CLI Operations** (`main.rs`):
- Current: 7+ clones per command
- After: 0 clones
- Frequency: Every CLI invocation (medium)
- **Impact**: 50-100ms faster CLI startup

**Capability Resolution** (`resolver.rs`):
- Current: 2 clones per registration
- After: 0-1 clones
- Frequency: Service initialization (low-medium)
- **Impact**: 2-5% improvement in startup

---

## 🎓 Modern Rust Patterns

### Pattern: Entry API

```rust
// Idiomatic Rust - avoid double clone
map.entry(key).or_insert(value);
```

### Pattern: Cow (Clone on Write)

```rust
use std::borrow::Cow;

fn process<'a>(data: Cow<'a, str>) -> String {
    if needs_modification {
        data.to_uppercase()  // Clones if borrowed
    } else {
        data.into_owned()  // No clone if already owned
    }
}
```

### Pattern: Arc for Immutable Sharing

```rust
let config = Arc::new(expensive_config);
let ref1 = Arc::clone(&config);  // Cheap!
let ref2 = Arc::clone(&config);  // Cheap!
```

### Pattern: String Interning

```rust
// Global static strings for common values
pub mod interned {
    pub const ENCRYPTION: &str = "encryption";
    pub const STORAGE: &str = "storage";
}
```

---

## ✅ Implementation Plan

### This Week

1. ✅ Create optimization plan (this document)
2. ⏳ Implement Quick Win #1 (HashMap Entry API)
3. ⏳ Implement Quick Win #2 (String interning)
4. ⏳ Benchmark improvements

### Next Week

5. Implement function parameter optimization
6. Update top 10 clone-heavy files
7. Measure cumulative impact

### Weeks 3-4

8. Implement buffer pooling for hot paths
9. Profile with flamegraph
10. Final optimization pass

---

## 🎯 Success Metrics

| Metric | Before | Target | Improvement |
|--------|--------|--------|-------------|
| **Clone Count** | 2,323 | <2,000 | -15% |
| **Hot Path Perf** | Baseline | +20% | 20% faster |
| **Memory Alloc** | Baseline | -30% | 30% fewer |
| **CLI Startup** | Baseline | -50ms | Faster UX |
| **RPC Throughput** | Baseline | +15% | More scalable |

---

## 📝 Notes

### Clone is Not Always Bad

Some clones are necessary and acceptable:
- Arc::clone() is cheap (ref count++)
- Small Copy types (bool, u32, etc.)
- Needed for ownership transfer
- Low-frequency code paths

### Focus on Hot Paths

**80/20 Rule**: 20% of code runs 80% of the time

**Strategy**: Profile first, optimize hot paths, ignore cold paths

### Maintain Readability

**Goal**: Fast AND idiomatic

**Anti-pattern**: Micro-optimizations that hurt readability

---

**Status**: Strategy complete, ready to implement  
**Timeline**: 4 weeks for full optimization  
**Expected Impact**: 20-40% performance improvement  
**Priority**: **MEDIUM** (after hardcoding evolution)

---

*"Zero-copy is not zero-clone. It's smart cloning."*

**NEXT**: Implement Quick Wins (4 hours, 250+ clones eliminated) 🚀
