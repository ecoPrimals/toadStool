# 🧬 ToadStool Socket Configuration - Deep Debt Analysis

**Date**: January 11, 2026  
**Priority**: HIGH - Blocking biomeOS atomic deployment  
**Status**: 🔴 Requires immediate evolution  
**Effort**: 2-3 hours

---

## 🎯 Executive Summary

ToadStool's current socket configuration is **partially compliant** with biomeOS requirements but **missing critical features** that block atomic deployment. We need to add environment variable override support and robust path handling to enable biomeOS's Tower, Node, and Nest atomics.

**Impact**: HIGH - Blocking biomeOS production deployment  
**Grade Impact**: Maintains A+ (97/100) with improved compliance  
**Deep Debt**: Evolution opportunity to full agnostic capability-based design

---

## 📊 Current Implementation Analysis

### ✅ What ToadStool Does Well

1. **XDG-Compliant Socket Paths**
   - Uses `XDG_RUNTIME_DIR` environment variable
   - Falls back to `/run/user/<uid>/` (standard)
   - Format: `toadstool-<family>.sock`

2. **Family ID Support**
   - Respects `TOADSTOOL_FAMILY` environment variable
   - Defaults to `"default"` with warning if not set
   - Enables basic multi-instance support

3. **Secure Socket Handling**
   - Removes old socket files before binding (prevents "address in use")
   - Sets 0600 permissions (user-only, secure)
   - Proper graceful shutdown cleanup

4. **Modern Rust Patterns**
   - Proper error handling with `Result<T, E>`
   - No `unwrap()` in production
   - Clean async/await patterns

### ❌ Critical Missing Features (biomeOS Requirements)

#### Issue 1: NO `TOADSTOOL_SOCKET` Environment Variable Support

**Current Behavior**:
```rust
fn get_socket_path(family_id: &str) -> Result<PathBuf, ...> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", uid));
    
    let socket_path = PathBuf::from(runtime_dir)
        .join(format!("toadstool-{}.sock", family_id));
    
    Ok(socket_path)
}
```

**Problem**: Ignores `TOADSTOOL_SOCKET` if set by biomeOS launcher

**Required Behavior**:
```rust
fn get_socket_path(family_id: &str) -> Result<PathBuf, ...> {
    // 1. HIGHEST PRIORITY: Check TOADSTOOL_SOCKET env var
    if let Ok(socket) = std::env::var("TOADSTOOL_SOCKET") {
        return Ok(PathBuf::from(socket));
    }
    
    // 2. Fall through to XDG logic...
}
```

**Impact**: biomeOS cannot override socket paths for atomic deployment (Tower, Node, Nest)

---

#### Issue 2: NO Parent Directory Creation

**Current Behavior**:
- Assumes `/run/user/<uid>/` or `XDG_RUNTIME_DIR` exists
- Fails with "No such file or directory" if parent missing

**Required Behavior**:
```rust
// Ensure parent directory exists
if let Some(parent) = socket_path.parent() {
    std::fs::create_dir_all(parent)?;
}

// Remove old socket if exists
let _ = std::fs::remove_file(&socket_path);

// Now bind
let listener = UnixListener::bind(&socket_path)?;
```

**Impact**: May fail on edge systems or custom `TOADSTOOL_SOCKET` paths

---

#### Issue 3: NO 3-Tier Fallback to `/tmp`

**Current Behavior**:
- Only tries XDG paths
- Fails if XDG runtime directory doesn't exist

**Required Behavior** (3-tier priority):
1. `TOADSTOOL_SOCKET` env var (absolute path override)
2. `XDG_RUNTIME_DIR` or `/run/user/<uid>/`
3. `/tmp/toadstool-<family>-<node>.sock` (last resort)

**Impact**: Fails on systems without XDG runtime directory (containers, minimal systems)

---

#### Issue 4: NO `TOADSTOOL_NODE_ID` Support

**Current Behavior**:
- Only supports `TOADSTOOL_FAMILY`
- Cannot run multiple instances with same family ID

**Required Behavior**:
```rust
let family_id = std::env::var("TOADSTOOL_FAMILY").unwrap_or_else(|| "default".to_string());
let node_id = std::env::var("TOADSTOOL_NODE_ID").unwrap_or_else(|| "default".to_string());

// For /tmp fallback:
format!("/tmp/toadstool-{}-{}.sock", family_id, node_id)
```

**Impact**: Cannot run multiple ToadStool instances with same family (testing, redundancy)

---

## 🎯 Required Evolution

### Priority 1: Add `TOADSTOOL_SOCKET` Environment Variable (CRITICAL)

**Change**:
```rust
fn get_socket_path(family_id: &str, node_id: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // 1. HIGHEST PRIORITY: Explicit socket path override
    if let Ok(socket) = std::env::var("TOADSTOOL_SOCKET") {
        info!("Using socket path from TOADSTOOL_SOCKET: {}", socket);
        return Ok(PathBuf::from(socket));
    }
    
    // 2. XDG runtime directory (standard)
    // ... existing logic ...
}
```

**Rationale**: Enables biomeOS to control socket paths for atomic deployment

---

### Priority 2: Create Parent Directories (ROBUST)

**Change**:
```rust
// In serve_unix() and ManualJsonRpcServer::serve()
let socket_path = socket_path.as_ref();

// Ensure parent directory exists
if let Some(parent) = socket_path.parent() {
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Failed to create socket directory: {}", e))?;
    info!("Ensured socket directory exists: {:?}", parent);
}

// Remove old socket if exists
if socket_path.exists() {
    info!("Removing old socket file: {:?}", socket_path);
    std::fs::remove_file(socket_path)?;
}
```

**Rationale**: Prevents "No such file or directory" errors, enables custom paths

---

### Priority 3: Implement 3-Tier Fallback Logic (AGNOSTIC)

**Change**:
```rust
fn get_socket_path(family_id: &str, node_id: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // 1. TOADSTOOL_SOCKET (highest priority)
    if let Ok(socket) = std::env::var("TOADSTOOL_SOCKET") {
        info!("Using TOADSTOOL_SOCKET: {}", socket);
        return Ok(PathBuf::from(socket));
    }
    
    // 2. XDG runtime directory (standard)
    let uid = unsafe { libc::getuid() };
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", uid));
    
    let xdg_path = PathBuf::from(&runtime_dir)
        .join(format!("toadstool-{}.sock", family_id));
    
    if PathBuf::from(&runtime_dir).exists() {
        info!("Using XDG runtime directory: {:?}", xdg_path);
        return Ok(xdg_path);
    }
    
    // 3. /tmp fallback (last resort)
    let tmp_path = PathBuf::from("/tmp")
        .join(format!("toadstool-{}-{}.sock", family_id, node_id));
    
    warn!("XDG runtime directory not found, falling back to /tmp: {:?}", tmp_path);
    Ok(tmp_path)
}
```

**Rationale**: Works on all systems (containers, minimal, standard)

---

### Priority 4: Add `TOADSTOOL_NODE_ID` Support (MULTI-INSTANCE)

**Change**:
```rust
// In main()
let family_id = std::env::var("TOADSTOOL_FAMILY")
    .unwrap_or_else(|_| {
        warn!("TOADSTOOL_FAMILY not set, using 'default'");
        "default".to_string()
    });

let node_id = std::env::var("TOADSTOOL_NODE_ID")
    .unwrap_or_else(|_| {
        info!("TOADSTOOL_NODE_ID not set, using 'default'");
        "default".to_string()
    });

info!("Family ID: {}", family_id);
info!("Node ID: {}", node_id);

let socket_path = get_socket_path(&family_id, &node_id)?;
```

**Rationale**: Enables multiple instances with same family (testing, redundancy)

---

## 📚 biomeOS Standardization Compliance

### Environment Variables (All Primals)

| Variable | ToadStool Current | ToadStool Required | Priority |
|----------|-------------------|-------------------|----------|
| `TOADSTOOL_SOCKET` | ❌ Not checked | ✅ Priority 1 | HIGH |
| `TOADSTOOL_FAMILY_ID` | ✅ `TOADSTOOL_FAMILY` | ✅ Alias to `TOADSTOOL_FAMILY` | MEDIUM |
| `TOADSTOOL_NODE_ID` | ❌ Not supported | ✅ For multi-instance | HIGH |

### Fallback Logic (Priority Order)

| Priority | Current | Required |
|----------|---------|----------|
| 1. `TOADSTOOL_SOCKET` | ❌ Ignored | ✅ Check first |
| 2. XDG Runtime Directory | ✅ Implemented | ✅ Keep |
| 3. `/tmp` fallback | ❌ Not implemented | ✅ Add |

---

## 🧪 Testing Requirements (biomeOS Specified)

### Test 1: Environment Variable Override
```bash
export TOADSTOOL_SOCKET=/tmp/test-socket.sock
export TOADSTOOL_FAMILY_ID=test0
./target/release/toadstool

# Verify socket exists at /tmp/test-socket.sock
ls -lh /tmp/test-socket.sock
```

**Expected**: Socket created at exact path specified

---

### Test 2: XDG Runtime Directory
```bash
export TOADSTOOL_FAMILY_ID=xdg0
./target/release/toadstool

# Verify socket exists at /run/user/<uid>/toadstool-xdg0.sock
ls -lh /run/user/$(id -u)/toadstool-xdg0.sock
```

**Expected**: Socket created in XDG runtime directory

---

### Test 3: Fallback to /tmp
```bash
# Simulate missing XDG runtime directory
export XDG_RUNTIME_DIR=/nonexistent
export TOADSTOOL_FAMILY_ID=tmp0
export TOADSTOOL_NODE_ID=node1
./target/release/toadstool

# Verify socket exists in /tmp
ls -lh /tmp/toadstool-tmp0-node1.sock
```

**Expected**: Socket created in `/tmp` with family and node ID

---

### Test 4: Socket Cleanup
```bash
# Create old socket
touch /tmp/test-socket.sock

# Start ToadStool
export TOADSTOOL_SOCKET=/tmp/test-socket.sock
./target/release/toadstool

# Should remove old socket and create new one (no "address already in use")
```

**Expected**: Old socket removed, new socket created successfully

---

## 🎯 Deep Debt Compliance Analysis

### Current Compliance

✅ **Modern Idiomatic Rust**: Proper error handling, no unwrap()  
✅ **No TCP Hardcoding**: Unix sockets primary (v2.2.0)  
✅ **Capability-Based Discovery**: Songbird registration  
⚠️ **Agnostic Design**: Partial - missing TOADSTOOL_SOCKET override  
⚠️ **Runtime Discovery**: Partial - no /tmp fallback  
✅ **Self-Knowledge**: Uses TOADSTOOL_FAMILY  

### Post-Evolution Compliance

✅ **Agnostic Design**: Full - TOADSTOOL_SOCKET override  
✅ **Runtime Discovery**: Full - 3-tier fallback  
✅ **Multi-Instance**: TOADSTOOL_NODE_ID support  
✅ **Robust**: Parent directory creation  
✅ **Standardized**: Matches biomeOS primal requirements  

---

## 🚨 Impact Assessment

### Without These Fixes (Current State)

❌ **biomeOS BLOCKED**: Cannot deploy atomics (Tower, Node, Nest)  
❌ **Non-Standard**: Not compliant with primal socket standardization  
❌ **Edge Cases**: Fails on systems without XDG runtime directory  
❌ **Multi-Instance**: Cannot run multiple instances with same family  

### With These Fixes (Evolved State)

✅ **biomeOS UNBLOCKED**: Can deploy atomics immediately  
✅ **Standardized**: Compliant with primal socket configuration  
✅ **Robust**: Works on all systems (standard, minimal, containers)  
✅ **Multi-Instance**: Full support for redundancy and testing  
✅ **Deep Debt**: 100% agnostic, capability-based design  

---

## 📋 Implementation Checklist

- [ ] Add `TOADSTOOL_SOCKET` environment variable support (Priority 1)
- [ ] Add parent directory creation (`std::fs::create_dir_all`)
- [ ] Implement 3-tier fallback logic (env var → XDG → /tmp)
- [ ] Add `TOADSTOOL_NODE_ID` environment variable support
- [ ] Test all 4 biomeOS scenarios
- [ ] Update documentation (README, environment variables)
- [ ] Commit and push fixes
- [ ] Notify biomeOS team (ready for atomic deployment)

---

## 🏆 Summary

**Status**: 🔴 **Requires Immediate Evolution**

**Current**: Partially compliant, missing critical features  
**Required**: Full biomeOS primal socket standardization  
**Effort**: 2-3 hours  
**Priority**: HIGH - Blocking biomeOS production deployment  

**Recommendation**: **PROCEED WITH EVOLUTION IMMEDIATELY**

This evolution:
- ✅ Unblocks biomeOS atomic deployment
- ✅ Achieves 100% deep debt compliance
- ✅ Standardizes across all primals
- ✅ Maintains A+ grade (97/100)
- ✅ Demonstrates modern idiomatic Rust

---

**Different orders of the same architecture.** 🍄🐸

**ToadStool: Ready to Evolve for Atomic Deployment**  
**Priority**: HIGH  
**Status**: Analysis Complete, Ready to Implement

---

**Prepared by**: ToadStool Team  
**Date**: January 11, 2026  
**Version**: 2.2.0 → 2.2.1 (pending evolution)

