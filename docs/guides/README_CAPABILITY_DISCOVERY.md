# 📌 Capability Discovery Reference Implementation

**File**: `crates/distributed/src/songbird_integration/capability_discovery.rs`  
**Status**: Reference Pattern Template (Not Currently Integrated)  
**Purpose**: Shows the target architecture pattern for capability-based discovery

---

## ⚠️ IMPORTANT

This file is **intentionally commented out** in `mod.rs`. It's a **reference implementation** showing the pattern to copy for integrations, not a working module yet.

**Think of it as**: Documentation through code - a template showing "this is what we're building toward."

---

## 🎯 HOW TO USE THIS

### Step 1: Read and Understand
Study `capability_discovery.rs` to understand:
- How to use `DiscoveryEngine`
- Capability-based service finding
- Automatic failover pattern
- Health-aware routing

### Step 2: Copy the Pattern
When integrating a primal (e.g., BearDog):
```rust
// Instead of modifying existing SongbirdConnection,
// apply this pattern to your integration:

pub async fn connect_to_crypto() -> Result<Connection> {
    let services = discovery
        .find_by_capability("cryptographic-operations")
        .await?;
    
    for service in services {
        match try_connect(&service).await {
            Ok(conn) => return Ok(conn),
            Err(_) => continue, // Automatic failover!
        }
    }
    Err("No crypto service available")
}
```

### Step 3: Adapt to Your Needs
- Change capabilities to match your use case
- Adjust error handling
- Add logging/metrics
- Keep the failover pattern!

---

## 📖 KEY PATTERNS TO COPY

### Pattern 1: Discovery-Based Connection
```rust
let services = discovery
    .find_by_capability("your-capability-here")
    .await?;
```

### Pattern 2: Automatic Failover
```rust
for service in services {
    match try_operation(&service).await {
        Ok(result) => return Ok(result),
        Err(e) => {
            warn!("Service {} failed: {}", service.endpoint, e);
            continue; // Try next!
        }
    }
}
```

### Pattern 3: Health-Aware Selection
```rust
// Services are already sorted by health/priority
let best_service = services.first()
    .ok_or("No services available")?;
```

---

## 🔧 WHEN TO INTEGRATE IT

### Phase 1: Learn the Pattern (NOW)
- ✅ Read `capability_discovery.rs`
- ✅ Understand the approach
- ✅ See how it differs from hardcoding

### Phase 2: Apply to BearDog (Week 1)
- Create `crates/cli/src/ecosystem/adapters/crypto_capability.rs`
- Copy relevant patterns from reference
- Integrate with existing BearDog adapter
- Test thoroughly

### Phase 3: Apply to All Primals (Week 2-3)
- NestGate (storage)
- Squirrel (AI agents)
- BiomeOS (orchestration)
- Eventually Songbird itself

### Phase 4: Replace Old Patterns (Week 3)
- Remove hardcoded endpoints
- Delete old connection code
- Update all usages
- Verify everything works

---

## 💡 WHY IT'S COMMENTED OUT

1. **Standalone Reference**: It defines a new struct that doesn't match existing `SongbirdConnection`
2. **Pattern Template**: Meant to be copied/adapted, not used directly
3. **Clean Builds**: Core codebase should build cleanly
4. **Learning Tool**: Read it to understand the target architecture

**It's like architectural blueprints** - you read them to understand what to build, you don't build the blueprint itself!

---

## 🎯 YOUR NEXT STEPS

1. **Read** `capability_discovery.rs` thoroughly
2. **Understand** the patterns it demonstrates
3. **Apply** those patterns to BearDog first
4. **Test** thoroughly
5. **Repeat** for other primals

---

## 📚 RELATED DOCS

- `CAPABILITY_DISCOVERY_INTEGRATION_GUIDE.md` - Step-by-step how-to
- `primal-capabilities.toml` - Capability registry
- `📍_START_HERE_MODERNIZATION.md` - Complete roadmap
- `MODERNIZATION_ROADMAP_DEC_3_2025.md` - Full vision

---

## ✅ VERIFICATION

When you've successfully applied the pattern, you should see:
- ✅ No hardcoded endpoints in your code
- ✅ Automatic failover working
- ✅ Health-aware service selection
- ✅ Tests passing with discovery mocks
- ✅ Logs showing "discovered service at..."

---

**Created**: December 3, 2025  
**Status**: Reference Implementation  
**Purpose**: Pattern Template  
**Next**: Apply to BearDog integration

**Remember**: This is a **guide**, not working code. Copy the patterns, adapt to your needs, test thoroughly!

