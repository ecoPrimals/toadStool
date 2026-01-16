# Discovery Files Assessment - January 16, 2026

**Files**: infant_discovery/sources.rs, detectors.rs  
**Finding**: Limited reqwest usage for external service registries  
**Decision**: These are legitimate external HTTP (Consul, etc.)

---

## 🔍 ANALYSIS

### **sources.rs** (4 usages)

**Line 211**: Port scanning verification
```rust
if reqwest::Client::new()
    .head(&endpoint)
    .timeout(std::time::Duration::from_millis(500))
    .send()
```
**Context**: Verifying a discovered port is actually responsive  
**Type**: External HTTP health check  
**Decision**: Keep or make optional

**Lines 293, 339, 366**: Consul service registry queries
```rust
reqwest::Client::new()
    .get(&url)  // Consul API query
    .send()
```
**Context**: Querying external Consul service registry  
**Type**: External service registry HTTP API  
**Decision**: These are genuinely external (Consul is external service)

---

### **detectors.rs** (1 usage)

**Line 172**: Consul availability detection
```rust
reqwest::Client::new()
    .get(format!("{consul_addr}/v1/status/leader"))
    .send()
```
**Context**: Detecting if Consul is available  
**Type**: External service registry detection  
**Decision**: Genuinely external

---

## 💡 DECISION

### **Keep These HTTP Usages**

**Why**:
1. ✅ **Genuinely External**: Consul is external service registry (not primal)
2. ✅ **Optional Discovery**: Only used if Consul is configured
3. ✅ **Graceful Degradation**: Falls back if unavailable
4. ✅ **Small Scope**: Only 5 HTTP calls total, all for external registries

**Impact on Pure Rust**:
- These will keep reqwest → rustls → ring in dependency tree
- BUT: Only if discovery features are enabled
- Can make this optional via feature flag

---

## 🎯 OPTIONS

### **Option A: Keep As-Is** ✅ RECOMMENDED

**Pros**:
- Discovery still works with Consul/external registries
- No functionality loss
- Small, isolated HTTP usage

**Cons**:
- Still has ring dependency (via reqwest)
- Not 100% pure Rust

**Mitigation**:
- Make discovery optional via feature flag
- Document that external registry support requires reqwest

---

### **Option B: Remove External Registry Support**

**Pros**:
- Achieves 100% pure Rust
- Simpler dependencies

**Cons**:
- Lose Consul integration
- Lose external registry support
- Breaks discovery in some deployments

**Risk**: Medium - some users may rely on Consul

---

### **Option C: Proxy Through Songbird**

**Pros**:
- 100% pure Rust for ToadStool
- Songbird handles external HTTP
- TRUE PRIMAL architecture

**Cons**:
- More complex
- Requires Songbird support for registry queries

**Timeline**: Would require Songbird changes

---

## 📋 RECOMMENDATION

### **Recommended: Option A (Keep As-Is) with Feature Flag**

**Implementation**:
```toml
[dependencies]
# Optional: External service registry support (requires reqwest)
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false, optional = true }

[features]
# External service discovery (Consul, etc.) - requires HTTP
external-registry = ["reqwest"]
```

**Result**:
- Pure Rust by default (no reqwest!)
- External registry support opt-in
- Best of both worlds

**Grade Impact**:
- Default build: 100% pure Rust! ✅
- With external-registry: 99% pure Rust (acceptable for optional feature)

---

## 🚀 IMMEDIATE ACTIONS

### **1. Make reqwest Optional in Common**

Update `crates/core/common/Cargo.toml`:
- reqwest = optional
- Feature flag: external-registry

### **2. Guard HTTP Code**

```rust
#[cfg(feature = "external-registry")]
if reqwest::Client::new()...
```

### **3. Document Feature**

- README: Note external-registry feature
- Docs: When to enable

---

**Status**: Assessment complete  
**Recommendation**: Feature-gate external registry support  
**Result**: 100% pure Rust by default, optional external support

