# 🔧 Unwrap Elimination Guide
## Evolving to Modern Error Handling with `?` Operator

**Date**: December 3, 2025  
**Status**: In Progress  
**Target**: Replace ~175 production unwraps with proper error handling

---

## 🎯 Philosophy

> "Errors are data, not exceptions. Handle them explicitly, propagate them clearly."

**Modern Rust**: Use `?` operator for clean error propagation  
**Avoid**: `.unwrap()` in production code (panics on error)  
**Accept**: `.unwrap()` in test code (fail-fast is appropriate)

---

## 📊 CURRENT STATE (December 3, 2025)

### Unwrap Analysis
```
Total unwraps:        ~2,900+ instances
Production code:      ~128 instances (4.4%)  ⚠️ Needs migration
Test code:            ~2,772 instances (95.6%) ✅ ACCEPTABLE
```

### Distribution
- **Test code**: 95.6% (acceptable - tests should fail fast)
- **Doc comments**: Some (examples only, not real code)
- **Production code**: 4.4% (target: <1%)

### Recent Progress
- ✅ Specialty Runtime: Modern error handling with `?` operator throughout
- ✅ Zero new unwraps introduced in recent fixes
- 🎯 Next: Systematic migration of 128 production instances

---

## ✅ ACCEPTABLE UNWRAPS

### 1. Test Code (Fail-Fast is Good)
```rust
#[tokio::test]
async fn test_volume_lifecycle() {
    // ✅ ACCEPTABLE: Tests should fail fast
    let backend = StorageBackend::new().await.unwrap();
    backend.provision_volume(&config).await.unwrap();
    let status = backend.get_volume_status("test").await.unwrap();
    assert_eq!(status.state, VolumeState::Ready);
}
```

**Why OK**: 
- Tests failing with panic shows exactly where it broke
- No production impact
- Clearer than verbose error handling in tests

### 2. Static/Const Values (Compile-Time Guaranteed)
```rust
// ✅ ACCEPTABLE: Will never fail (hardcoded valid value)
const DEFAULT_PORT: u16 = 8080;
const LOCALHOST: IpAddr = "127.0.0.1".parse().unwrap(); // compile-time constant
```

**Why OK**:
- Value is known at compile time
- Will never fail in production
- If it fails, it's a development error caught immediately

### 3. Doc Comment Examples
```rust
/// # Example
/// ```
/// let value = map.get(key).unwrap();
/// ```
```

**Why OK**: Documentation examples, not real code

---

## ❌ PROBLEMATIC UNWRAPS

### 1. Production Code with User Input
```rust
// ❌ BAD: User input could be invalid
fn parse_address(input: &str) -> SocketAddr {
    input.parse().unwrap() // PANIC if invalid!
}
```

**Problem**: User provides invalid input → panic → crash

### 2. I/O Operations
```rust
// ❌ BAD: File might not exist, disk might be full
fn save_config(config: &Config) -> () {
    std::fs::write("config.json", serde_json::to_string(config).unwrap()).unwrap()
}
```

**Problem**: I/O can fail → panic → data loss

### 3. Network Operations
```rust
// ❌ BAD: Network can fail, endpoint might be down
async fn fetch_data(url: &str) -> Data {
    reqwest::get(url).await.unwrap().json().await.unwrap()
}
```

**Problem**: Network failure → panic → service crash

---

## 🔄 REFACTORING PATTERNS

### Pattern 1: Return Result
**Before**:
```rust
fn parse_config(path: &str) -> Config {
    let contents = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&contents).unwrap()
}
```

**After**:
```rust
fn parse_config(path: &str) -> Result<Config, ConfigError> {
    let contents = std::fs::read_to_string(path)?;
    let config = serde_json::from_str(&contents)?;
    Ok(config)
}
```

**Benefits**:
- Errors propagate cleanly with `?`
- Caller can handle or propagate
- No panics in production

### Pattern 2: Provide Default/Fallback
**Before**:
```rust
fn get_port() -> u16 {
    std::env::var("PORT").unwrap().parse().unwrap()
}
```

**After**:
```rust
fn get_port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080) // Safe default
}
```

**Benefits**:
- Graceful fallback
- No panic on missing env var
- Predictable behavior

### Pattern 3: Context-Rich Errors
**Before**:
```rust
fn connect_to_service(addr: &str) -> Client {
    Client::connect(addr.parse().unwrap()).unwrap()
}
```

**After**:
```rust
use anyhow::{Context, Result};

fn connect_to_service(addr: &str) -> Result<Client> {
    let socket_addr = addr
        .parse()
        .with_context(|| format!("Invalid address format: {addr}"))?;
    
    Client::connect(socket_addr)
        .with_context(|| format!("Failed to connect to {addr}"))
}
```

**Benefits**:
- Clear error messages
- Context shows what failed
- Easy debugging

### Pattern 4: Option Handling
**Before**:
```rust
fn get_user_name(id: u64) -> String {
    USERS.get(&id).unwrap().name.clone()
}
```

**After** (Option 1 - Propagate):
```rust
fn get_user_name(id: u64) -> Option<String> {
    USERS.get(&id).map(|user| user.name.clone())
}
```

**After** (Option 2 - Default):
```rust
fn get_user_name(id: u64) -> String {
    USERS
        .get(&id)
        .map(|user| user.name.clone())
        .unwrap_or_else(|| format!("Unknown user {id}"))
}
```

**Benefits**:
- Explicit handling of missing data
- Caller knows data might not exist
- Graceful degradation

---

## 🔨 SPECIFIC FIXES FOR TOADSTOOL

### Fix 1: Address Parsing in Tests
**File**: `crates/cli/src/ecosystem/mod.rs:138`

**Before**:
```rust
// In test code - this is actually acceptable,
// but we can make it more idiomatic
address: "127.0.0.1:8080".parse().unwrap(),
```

**After** (if production code):
```rust
address: "127.0.0.1:8080"
    .parse()
    .expect("Hardcoded address should be valid"),
```

**Rationale**: `expect()` is better than `unwrap()` because it provides context when it fails.

### Fix 2: JSON Serialization
**File**: `crates/cli/src/ecosystem/mod.rs:167`

**Before**:
```rust
let json = serde_json::to_string(&endpoint).unwrap();
let deserialized: ServiceEndpoint = serde_json::from_str(&json).unwrap();
```

**After** (if production code):
```rust
let json = serde_json::to_string(&endpoint)
    .expect("Serialization should not fail for valid types");
let deserialized: ServiceEndpoint = serde_json::from_str(&json)
    .expect("Deserialization should match serialized format");
```

Or better, in production:
```rust
let json = serde_json::to_string(&endpoint)?;
let deserialized: ServiceEndpoint = serde_json::from_str(&json)?;
```

### Fix 3: Result Handling
**File**: `crates/core/toadstool/src/biomeos_integration/agents.rs:184`

**Before**:
```rust
let agent_info = result.unwrap();
```

**After**:
```rust
let agent_info = result?; // Propagate error to caller
```

Or with context:
```rust
let agent_info = result
    .context("Failed to retrieve agent information")?;
```

---

## 🎯 MIGRATION STRATEGY

### Phase 1: Identify (DONE ✅)
- [x] Count unwraps (~2,891 total)
- [x] Separate test vs production (94% test, 6% production)
- [x] Identify critical paths

### Phase 2: Categorize (In Progress)
- [x] Acceptable unwraps (tests, constants, docs)
- [ ] Must-fix unwraps (production, user input, I/O)
- [ ] Should-fix unwraps (production, internal APIs)
- [ ] Nice-to-fix unwraps (production, rare paths)

### Phase 3: Fix Critical Paths (Week 1)
**Priority: HIGH**

Target files:
- [ ] `crates/cli/src/ecosystem/mod.rs` (production network code)
- [ ] `crates/core/toadstool/src/biomeos_integration/` (production integration)
- [ ] `crates/cli/src/ecosystem/capabilities/` (production discovery)

Pattern:
```rust
// Before: result.unwrap()
// After:  result?  (or .context("...")?)
```

### Phase 4: Fix Internal APIs (Week 2)
**Priority: MEDIUM**

- [ ] Replace unwraps in public API functions
- [ ] Add proper error types where missing
- [ ] Update function signatures to return `Result<T, E>`

### Phase 5: Polish (Week 3)
**Priority: LOW**

- [ ] Replace unwraps with `expect()` where panic is intentional
- [ ] Add error context with `.context()` for debugging
- [ ] Update documentation with error handling examples

---

## 📊 SUCCESS METRICS

### Code Quality
**Before**:
- Production unwraps: ~175 instances
- Error context: Minimal
- Panic risk: Moderate

**After** (Target):
- Production unwraps: <10 instances
- Error context: Comprehensive
- Panic risk: Minimal

### Specific Targets
- [x] Test code unwraps: Keep as-is (94% of total) ✅
- [ ] Production unwraps: Reduce to <10 (99% reduction)
- [ ] All remaining unwraps: Have `.expect()` with context
- [ ] All public APIs: Return `Result<T, E>`

---

## 🔍 DETECTION & PREVENTION

### CI/CD Check (Future)
```rust
// Add clippy lint in CI
#![deny(clippy::unwrap_used)] // in lib.rs

// Allow in test code
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    // Tests can use unwrap
}
```

### Code Review Checklist
- [ ] No `.unwrap()` in production code
- [ ] All `.unwrap()` have adjacent comment explaining why
- [ ] All public functions return `Result<T, E>` for fallible operations
- [ ] Error messages provide context for debugging

---

## 🎓 BEST PRACTICES

### Do's ✅
- ✅ Use `?` operator for error propagation
- ✅ Return `Result<T, E>` from fallible functions
- ✅ Use `.unwrap_or()` / `.unwrap_or_else()` for defaults
- ✅ Use `.expect("reason")` if unwrap is intentional
- ✅ Use `.context()` for rich error messages
- ✅ Use `.ok()` to convert Result → Option when appropriate

### Don'ts ❌
- ❌ Don't use `.unwrap()` on user input
- ❌ Don't use `.unwrap()` on I/O operations
- ❌ Don't use `.unwrap()` on network operations
- ❌ Don't hide errors that should be propagated
- ❌ Don't panic in libraries (let caller handle)

### Gray Area ⚠️
- ⚠️ `.unwrap()` in tests: Usually OK (fail-fast)
- ⚠️ `.unwrap()` on constants: OK if truly constant
- ⚠️ `.expect()` in production: OK with good message
- ⚠️ `.unwrap_or_default()`: OK for optional config

---

## 📚 EXAMPLES FROM TOADSTOOL

### Example 1: Network Address Parsing
```rust
// BEFORE (from ecosystem/mod.rs):
ServiceEndpoint {
    address: "127.0.0.1:8080".parse().unwrap(),
    // ...
}

// AFTER (production):
ServiceEndpoint {
    address: "127.0.0.1:8080"
        .parse()
        .context("Default address should be valid")?,
    // ...
}

// AFTER (test):  
ServiceEndpoint {
    address: "127.0.0.1:8080"
        .parse()
        .expect("Test address should be valid"),
    // ...
}
```

### Example 2: JSON Operations
```rust
// BEFORE:
let capabilities_json = serde_json::to_string(&permission.capabilities).unwrap();

// AFTER:
let capabilities_json = serde_json::to_string(&permission.capabilities)
    .context("Failed to serialize capabilities")?;
```

### Example 3: Async Result Handling
```rust
// BEFORE:
let resolved = resolver.resolve(capability).await.unwrap();

// AFTER:
let resolved = resolver
    .resolve(capability)
    .await
    .with_context(|| format!("Failed to resolve capability: {capability}"))?;
```

---

## 🚀 QUICK WINS

### Replace These Immediately (5 minutes each)
1. All `.unwrap()` after `.parse()` on user input
2. All `.unwrap()` after JSON serialization/deserialization
3. All `.unwrap()` in public API functions
4. All `.unwrap()` after network operations

### Tool-Assisted Replacement
```bash
# Find production unwraps (excluding tests)
rg "\.unwrap\(\)" --type rust --glob '!**/tests/**' --glob '!**/*test*.rs'

# Replace pattern (review each):
# Before: result.unwrap()
# After:  result?  (in functions returning Result)
# After:  result.expect("reason")  (if intentional panic)
```

---

## 🎯 COMPLETION CHECKLIST

### Critical (Week 1)
- [ ] Fix all unwraps on network operations
- [ ] Fix all unwraps on file I/O
- [ ] Fix all unwraps in public APIs
- [ ] Add Result returns where missing

### Important (Week 2)
- [ ] Replace unwraps with `?` operator throughout
- [ ] Add error context with `.context()`
- [ ] Update error types for better messages

### Polish (Week 3)
- [ ] Replace remaining unwraps with `.expect()`
- [ ] Add CI lint to prevent new unwraps
- [ ] Update documentation

---

**Status**: Guide Complete, Ready for Implementation  
**Timeline**: 2-3 weeks for full migration  
**Complexity**: Medium (mostly mechanical changes)

---

*"Handle errors explicitly. Your users will thank you."*

