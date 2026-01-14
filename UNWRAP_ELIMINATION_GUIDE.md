# Unwrap/Expect Elimination Guide

**Date**: January 13, 2026  
**Priority**: HIGH (Production Reliability)  
**Scope**: 3,536 unwraps + 930 expects = 4,466 potential panics

---

## 🚨 Why This Matters

**Current State**: Production code can panic unexpectedly
**Risk**: Service crashes, data loss, poor user experience  
**Goal**: Proper error handling with Result<T, E>

---

## 📊 Statistics

| Category | Count | Files | Priority |
|----------|-------|-------|----------|
| `.unwrap()` | 3,536 | 416 | HIGH |
| `.expect()` | 930 | 133 | MEDIUM |
| `panic!()` | 908 | 163 | HIGH |
| **Total** | **5,374** | **712** | **CRITICAL** |

---

## 🎯 Elimination Strategy

### Phase 1: Critical Production Code (This Week)
Focus on `crates/` directory (exclude `tests/`, `examples/`, `showcase/`):
- `crates/core/toadstool/src/` (core runtime)
- `crates/server/src/` (server code)
- `crates/runtime/*/src/` (runtime engines)
- `crates/distributed/src/` (coordination)

### Phase 2: Integration Code (Next Week)
- `crates/cli/src/`
- `crates/client/src/`
- `crates/api/src/`

### Phase 3: Showcase & Examples (Low Priority)
- `showcase/` - demo code, unwraps acceptable
- `examples/` - demo code, unwraps acceptable
- Document that these are demos

---

## 🔧 Replacement Patterns

### Pattern 1: Option.unwrap() → ok_or_else()

**Before (BAD - can panic!)**:
```rust
let value = config.get("key").unwrap();
```

**After (GOOD - returns Result)**:
```rust
let value = config
    .get("key")
    .ok_or_else(|| ToadStoolError::configuration("Missing configuration key 'key'"))?;
```

### Pattern 2: Result.unwrap() → Context + ?

**Before (BAD)**:
```rust
let data = read_file(path).unwrap();
```

**After (GOOD)**:
```rust
let data = read_file(path)
    .context(format!("Failed to read configuration file: {}", path))?;
```

### Pattern 3: expect() → Detailed Context

**Before (BAD)**:
```rust
let port = env::var("PORT").expect("PORT not set");
```

**After (BETTER - but still panics)**:
```rust
let port = env::var("PORT")
    .expect("PORT environment variable must be set for server startup. Set with: export PORT=8080");
```

**After (BEST - no panic)**:
```rust
let port = env::var("PORT")
    .context("PORT environment variable must be set for server startup")?;
```

### Pattern 4: panic!() → Result

**Before (BAD)**:
```rust
if !condition {
    panic!("Invalid state: condition not met");
}
```

**After (GOOD)**:
```rust
if !condition {
    return Err(ToadStoolError::runtime(
        "Invalid state: condition not met - expected X, got Y"
    ));
}
```

### Pattern 5: unwrap_or() → Use When Acceptable

**OK to keep (has fallback)**:
```rust
let timeout = config.timeout.unwrap_or(30); // Safe: has default
let name = primal_name.unwrap_or_else(|| "unknown".to_string()); // Safe: has fallback
```

### Pattern 6: Test Code unwraps → Keep

**OK to keep (test code)**:
```rust
#[test]
fn test_something() {
    let result = compute().unwrap(); // OK: tests should fail fast
    assert_eq!(result, expected);
}
```

---

## 🎓 Deep Debt Evolution Examples

### Hardcoded Port + Unwrap → Runtime Discovery + Result

**Before (MULTIPLE VIOLATIONS)**:
```rust
const DEFAULT_PORT: u16 = 8080;  // Hardcoded!

fn start_server() {
    let port = env::var("PORT")
        .unwrap()  // Panic!
        .parse::<u16>()
        .unwrap();  // Panic!
    
    bind_server(port).unwrap();  // Panic!
}
```

**After (DEEP DEBT + SAFE)**:
```rust
use crate::common::discovery::discover_available_port;

fn start_server() -> Result<ServerHandle> {
    // Deep Debt: Runtime discovery, no hardcoded ports
    let port = discover_available_port()
        .context("Failed to discover available port for server")?;
    
    // Safe error handling
    let server = bind_server(port)
        .context(format!("Failed to bind server to discovered port {}", port))?;
    
    Ok(ServerHandle { port, server })
}
```

---

## 📝 File-by-File Priority List

### Priority 1: Core Runtime (Highest Impact)
```bash
# Find unwraps in core runtime
grep -r "\.unwrap()" crates/core/toadstool/src/ | grep -v test

# Files to fix first:
crates/core/toadstool/src/execution.rs
crates/core/toadstool/src/composition_engine.rs
crates/core/toadstool/src/ecosystem/communication.rs
crates/core/toadstool/src/runtime_discovery.rs
```

### Priority 2: Server Code
```bash
crates/server/src/main.rs
crates/server/src/jsonrpc_server.rs
crates/server/src/tarpc_server.rs
crates/server/src/coordinator_executor.rs
```

### Priority 3: Runtime Engines
```bash
crates/runtime/gpu/src/engine.rs
crates/runtime/wasm/src/engine.rs
crates/runtime/native/src/lib.rs
```

---

## 🛠️ Semi-Automated Approach

### Step 1: Find All Unwraps in a File
```bash
grep -n "\.unwrap()" crates/core/toadstool/src/execution.rs
```

### Step 2: Review Each One
For each unwrap, ask:
1. **Is this test code?** → Keep it
2. **Is there a fallback?** → Use `unwrap_or()` or `unwrap_or_else()`
3. **Is this critical production code?** → Convert to `?` operator
4. **Is the error message clear?** → Use `.context()` or `ok_or_else()`

### Step 3: Replace with Pattern
Use the patterns above based on the answer.

### Step 4: Test
```bash
cargo test --package toadstool
```

### Step 5: Move to Next File

---

## 📊 Progress Tracking

Create an issue for each file/module:

```markdown
## Unwrap Elimination Progress

### Core Runtime
- [ ] execution.rs (23 unwraps)
- [ ] composition_engine.rs (3 unwraps)
- [ ] ecosystem/communication.rs (6 unwraps)
- [ ] runtime_discovery.rs (8 unwraps)

### Server
- [ ] main.rs (4 unwraps)
- [ ] jsonrpc_server.rs (6 unwraps)
- [ ] tarpc_server.rs (3 unwraps)

### Runtime Engines
- [ ] gpu/engine.rs (10 unwraps)
- [ ] wasm/engine.rs (3 unwraps)
- [ ] native/lib.rs (5 unwraps)

... (continue for all files)
```

---

## 🎯 Success Criteria

**Goal**: Zero unwraps/expects/panics in production code paths

**Metrics**:
- Production code (crates/*/src/): 0 unwraps
- Integration code (cli, client, api): < 10 unwraps (with justification)
- Test code: Unlimited unwraps (fast failure is good)
- Examples/showcase: Unlimited (demo code)

**Timeline**: 2-3 weeks for full elimination

---

## 💡 Tips

1. **Start with high-traffic code paths** (execution, server, runtime)
2. **Use ripgrep for batch finding**: `rg "\.unwrap\(\)" --type rust`
3. **Keep a list of "safe unwraps"** (with justification)
4. **Test after each file** (don't batch too much)
5. **Document why unwrap_or() is safe** when you use it
6. **Use clippy**: `cargo clippy -- -W clippy::unwrap_used`

---

## 🏆 Expected Impact

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| **Production Panics** | 5,374 potential | 0 | 100% ✓ |
| **Error Messages** | "thread panicked" | Contextual errors | ✓ |
| **Reliability** | Poor | Excellent | ✓ |
| **Debuggability** | Hard | Easy | ✓ |
| **User Experience** | Crashes | Graceful errors | ✓ |

---

**Status**: Guide ready, elimination can begin!  
**Priority**: HIGH - Critical for production readiness  
**Estimated Time**: 2-3 weeks for complete elimination
