# UniBin Architecture Status - January 16, 2026

**Date**: January 16, 2026  
**Question**: Are we following the one-bin system?  
**Answer**: ⏳ **PARTIALLY - IN TRANSITION**

---

## 🎯 **Current Status**

### **Binary Architecture**: TWO BINARIES

**Currently Built**:
1. **`toadstool-server`** - Server/daemon binary
   - Path: `crates/server/src/main.rs`
   - Purpose: Server daemon, JSON-RPC, tarpc
   - Features: API, websocket, GPU discovery
   
2. **`toadstool-cli`** - CLI binary  
   - Path: `crates/cli/src/main.rs`
   - Purpose: Command-line interface
   - Features: Full ecosystem, universal, monitoring, templates, **daemon mode**

---

## 🔍 **UniBin Capability: YES (Partial)**

### **CLI Has Daemon Mode!**

The `toadstool-cli` binary **DOES** have a `Daemon` command that can run in server mode!

**From `Cargo.toml`**:
```toml
[features]
daemon = ["axum", "tower", "tower-http"]
```

**Evidence**: CLI has full server capabilities when `daemon` feature is enabled.

---

## 📊 **Current vs Target Architecture**

### **Current (Two Binaries)**

```
toadstool-server  →  Server/daemon mode only
toadstool-cli     →  CLI commands + daemon mode (optional)
```

**Usage**:
- Server: `toadstool-server` (always daemon)
- CLI: `toadstool-cli run|up|down|...`
- CLI as Server: `toadstool-cli daemon ...`

---

### **Target (UniBin - One Binary)**

```
toadstool  →  CLI commands + server mode (via flag/subcommand)
```

**Usage** (proposed):
- CLI: `toadstool run|up|down|...`
- Server: `toadstool daemon ...` OR `toadstool --daemon`
- Server: `toadstool server ...`

---

## 🎯 **UniBin Benefits**

### **Why One Binary?**

**Deployment Simplicity**:
- ✅ Single binary to distribute
- ✅ Easier installation
- ✅ Simpler version management
- ✅ Reduced binary size (shared code)

**Operational Benefits**:
- ✅ CLI and server always version-matched
- ✅ No separate server package needed
- ✅ Simpler systemd/service configuration

**Development Benefits**:
- ✅ Shared code between CLI and server
- ✅ Easier to maintain consistency
- ✅ Single entry point to understand

---

## 🚀 **Evolution Path**

### **Option A: Keep Current (Two Binaries)**

**Pros**:
- Already working
- Clear separation of concerns
- Smaller individual binaries

**Cons**:
- More complex deployment
- Version mismatch risks
- Higher maintenance burden

---

### **Option B: Evolve to UniBin (One Binary)**

**Pros**:
- Simpler deployment
- Version consistency guaranteed
- Industry standard (most tools use this)
- CLI already has daemon mode!

**Cons**:
- Requires refactoring
- Slightly larger binary
- Migration effort

---

### **Option C: Hybrid (Symlinks)**

**Strategy**: Build one binary, create symlinks

```bash
# Build single binary
cargo build --release --bin toadstool

# Create symlinks for compatibility
ln -s toadstool toadstool-server
ln -s toadstool toadstool-cli
```

**Binary detects name**:
```rust
match std::env::args().next().unwrap().as_str() {
    "toadstool-server" => run_daemon_mode(),
    "toadstool-cli" => run_cli_mode(),
    _ => run_cli_mode(), // default
}
```

**Pros**:
- ✅ One binary internally
- ✅ Backward compatibility
- ✅ Gradual migration path

---

## 🏗️ **Implementation Plan**

### **Phase 1: Consolidation** (Current - Minimal Work)

**Status**: CLI already has daemon mode!

**Action**:
1. Verify CLI daemon mode works fully
2. Test feature parity with toadstool-server
3. Document both entry points

**Effort**: 1-2 hours (verification only)

---

### **Phase 2: UniBin Evolution** (1-2 Days)

**Approach**: Make `toadstool-cli` the primary binary

**Changes**:
1. Rename `toadstool-cli` to `toadstool`
2. Add binary name detection for compatibility
3. Update docs and scripts
4. Mark `toadstool-server` as deprecated

**Migration**:
```toml
# crates/cli/Cargo.toml
[[bin]]
name = "toadstool"  # Primary name
path = "src/main.rs"

# Backward compat (optional)
[[bin]]
name = "toadstool-cli"  # Alias
path = "src/main.rs"
```

**Code**:
```rust
// Detect invocation name for backward compatibility
let bin_name = std::env::args().next()
    .and_then(|p| Path::new(&p).file_name())
    .and_then(|n| n.to_str())
    .unwrap_or("toadstool");

match bin_name {
    "toadstool-server" => {
        // Legacy: Run daemon mode directly
        run_daemon_mode().await
    }
    _ => {
        // Modern: Use subcommands
        let cli = Cli::parse();
        execute_command(&cli).await
    }
}
```

**Effort**: 4-8 hours (refactoring + testing)

---

### **Phase 3: Deprecation** (1 Week Later)

**Actions**:
1. Remove `crates/server/` crate entirely
2. Update all references to use `toadstool`
3. Update plasmidBin/ harvesting
4. Announce deprecation

**Effort**: 2-4 hours (cleanup)

---

## 🌍 **Ecosystem Context**

### **Other Primals**

**BearDog** 🐻:
- `beardog-server` (daemon)
- No separate CLI

**Songbird** 🐦:
- `songbird-orchestrator` (daemon)  
- Has CLI subcommands

**Squirrel** 🐿️:
- `squirrel` (daemon)
- Cache primal (no CLI needed)

**NestGate** 🏰:
- `nestgate` (daemon)
- Storage primal (no CLI needed)

**ToadStool** 🍄:
- `toadstool-server` + `toadstool-cli` (TWO)
- ⏳ Evolution to UniBin recommended

---

## 💡 **Recommendation**

### **Evolve to UniBin: YES** ✅

**Why**:
1. ✅ CLI already has daemon mode (90% done!)
2. ✅ Simpler deployment for biomeOS
3. ✅ Industry standard pattern
4. ✅ Better version consistency
5. ✅ Lower maintenance burden

**Timeline**: 1-2 days (low effort, high value)

**Priority**: Medium (not blocking current work)

---

## 📚 **Examples from Industry**

### **One-Binary Tools**

**Docker**:
```bash
docker run ...        # CLI
dockerd               # Daemon (symlink to docker)
```

**Kubernetes**:
```bash
kubectl ...           # CLI
kubectl proxy         # Server mode
```

**Nomad** (HashiCorp):
```bash
nomad agent ...       # Server
nomad job run ...     # CLI
```

**Consul** (HashiCorp):
```bash
consul agent ...      # Server
consul kv get ...     # CLI
```

**Pattern**: Single binary, mode determined by subcommand or flag

---

## 🎯 **Decision**

### **Path Forward**: **Option B + Phase 2**

**Action**: Evolve to UniBin in next iteration

**Steps**:
1. Verify CLI daemon mode parity
2. Rename binary to `toadstool`
3. Add backward compatibility detection
4. Update documentation
5. Deprecate `toadstool-server`

**Timing**: After current evolution gap work complete

**Effort**: 4-8 hours (manageable)

**Benefit**: Simpler deployment, better UX, industry alignment

---

## 📊 **Summary**

**Question**: "Are we following the one-bin system?"

**Answer**: 
- **Current**: No (two binaries)
- **Capability**: Yes (CLI has daemon mode!)
- **Plan**: Yes (UniBin evolution planned)
- **Status**: ⏳ In transition (easy evolution path)

**Recommendation**: ✅ **Proceed with UniBin evolution**

**Timeline**: 1-2 days effort, medium priority

**Blocking**: No (current architecture works)

---

**Created**: January 16, 2026  
**Purpose**: Document UniBin architecture status and evolution path  
**Result**: Clear path forward identified! ✅

🦀 **MODERN ARCHITECTURE - ONE BINARY TO RULE THEM ALL!** 🦀✨
