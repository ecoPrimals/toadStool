# ToadStool Pure Rust Evolution Handoff

**Date**: January 16, 2026  
**Primal**: ToadStool 🍄  
**Discovery**: ARM cross-compilation sprint (ecosystem-wide)  
**Priority**: **HIGH** - Blocking ARM deployment  
**Complexity**: **MEDIUM-HIGH** (dual evolution: ring + OpenSSL)  
**Effort**: 4-8 hours  
**Status**: 🎯 **READY TO EVOLVE**

---

## 🎯 Executive Summary

**Current State**: ❌ Cannot cross-compile to ARM64  
**Root Cause**: Two C dependencies (ring + openssl-sys)  
**Evolution Path**: Migrate to 100% pure Rust alternatives  
**Expected Result**: ✅ ARM64 cross-compilation works, pure Rust ecosystem

---

## 📊 Current Dependency Analysis

### **Dependency Tree Analysis** (via `cargo tree`)

#### **1. ring v0.17.14** (C/assembly crypto library)

**Path 1**: `jsonwebtoken v9.3.1`
```
ring v0.17.14
└── jsonwebtoken v9.3.1
    └── toadstool-api v0.1.0
        └── toadstool-runtime-container v0.1.0
```

**Issue**: JWT library uses ring for crypto operations

**Path 2**: `rustls v0.21.12` (older version)
```
ring v0.17.14
└── rustls v0.21.12  ← OLD VERSION, depends on ring!
    └── hyper-rustls v0.24.2
        └── reqwest v0.11.27
```

**Issue**: Old rustls (0.21) depends on ring. Modern rustls (0.23) is pure Rust!

---

#### **2. openssl-sys v0.9.111** (C OpenSSL binding)

**Path**: `native-tls` → `hyper-tls` → `reqwest`
```
openssl-sys v0.9.111
└── native-tls v0.2.14
    └── hyper-tls v0.5.0
        └── reqwest v0.11.27
            └── toadstool v0.1.0 (many crates depend on this)
```

**Issue**: reqwest defaults to native-tls (OpenSSL), not rustls

---

## 🔧 Evolution Plan

### **Two-Track Migration**

#### **Track 1: ring → Pure Rust** (2-4 hours)

**Option A**: Replace `jsonwebtoken` with pure Rust alternative
- **Replace**: `jsonwebtoken = "9.3"` (uses ring)
- **With**: `jwt-simple = "0.12"` (pure Rust, no ring!)
- **Or**: Update to `jsonwebtoken = "10+"` if it's ring-free

**Option B**: Fork/patch jsonwebtoken to use RustCrypto
- More work, but maintains same API

**Recommendation**: Option A (jwt-simple) - modern, pure Rust, actively maintained

---

#### **Track 2: OpenSSL → rustls** (2-4 hours)

**Primary Fix**: Configure reqwest for rustls

**Current** (implicit OpenSSL):
```toml
reqwest = { version = "0.11", features = ["json"] }
# Uses default features, includes native-tls (OpenSSL)
```

**Evolution** (pure Rust):
```toml
reqwest = { 
    version = "0.11", 
    features = ["json", "rustls-tls"], 
    default-features = false 
}
# Uses rustls (pure Rust TLS!)
```

**Side Benefit**: This will also upgrade rustls to 0.23+ (pure Rust, no ring!)

---

## 📋 Step-by-Step Migration

### **Phase 1: Audit Dependencies** (15 minutes)

```bash
# Find all Cargo.toml files with problematic dependencies
cd /home/strandgate/Development/ecoPrimals/phase1/toadStool

# Find jsonwebtoken usage
rg "jsonwebtoken" crates/*/Cargo.toml

# Find reqwest usage
rg "reqwest" crates/*/Cargo.toml

# Verify dependency tree
cargo tree -i ring
cargo tree -i openssl-sys
```

**Expected Findings**:
- `jsonwebtoken` in `crates/api/Cargo.toml`
- `reqwest` in root `Cargo.toml` (workspace dependencies)

---

### **Phase 2: Evolve JWT Library** (1-2 hours)

#### **Step 1**: Update `crates/api/Cargo.toml`

**Before**:
```toml
[dependencies]
jsonwebtoken = "9.3"
```

**After**:
```toml
[dependencies]
jwt-simple = "0.12"
```

#### **Step 2**: Update JWT code in `crates/api/src/*.rs`

**API Changes** (jsonwebtoken → jwt-simple):

**Old API** (jsonwebtoken):
```rust
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};

// Encoding
let token = encode(
    &Header::default(),
    &claims,
    &EncodingKey::from_secret(secret.as_ref())
)?;

// Decoding
let token_data = decode::<Claims>(
    &token,
    &DecodingKey::from_secret(secret.as_ref()),
    &Validation::default()
)?;
```

**New API** (jwt-simple):
```rust
use jwt_simple::prelude::*;

// Create key (one-time setup)
let key = HS256Key::from_bytes(secret.as_bytes());

// Encoding
let claims = Claims::create(Duration::from_hours(2))
    .with_custom_claims(my_claims);
let token = key.authenticate(claims)?;

// Decoding
let claims = key.verify_token::<MyCustomClaims>(&token, None)?;
```

**Migration Guide**:
1. Replace `use jsonwebtoken::*` with `use jwt_simple::prelude::*`
2. Update encoding calls to use `HS256Key::from_bytes()`
3. Update decoding calls to use `key.verify_token()`
4. Run tests to verify JWT functionality

**Effort**: 1-2 hours (API is similar but not identical)

---

### **Phase 3: Evolve TLS to rustls** (1-2 hours)

#### **Step 1**: Update workspace dependencies

**File**: Root `Cargo.toml` (workspace.dependencies section)

**Before**:
```toml
[workspace.dependencies]
reqwest = { version = "0.11", features = ["json"] }
```

**After**:
```toml
[workspace.dependencies]
reqwest = { 
    version = "0.11", 
    features = ["json", "rustls-tls", "rustls-tls-native-roots"], 
    default-features = false 
}
```

**Key Changes**:
- `default-features = false` - Disables native-tls (OpenSSL)
- `rustls-tls` - Enables rustls backend
- `rustls-tls-native-roots` - Uses system CA certs (optional but recommended)

#### **Step 2**: Update any crate-specific reqwest usage

**Search** for any crate-specific reqwest dependencies:
```bash
rg "reqwest.*=" crates/*/Cargo.toml
```

**Update** each to match workspace config or remove if using workspace dependency.

#### **Step 3**: Test HTTP/HTTPS functionality

```bash
# Run tests that make HTTP requests
cargo test --workspace -- http
cargo test --workspace -- https
cargo test --workspace -- tls
```

**Expected**: All tests pass (rustls is drop-in replacement for most uses)

**Effort**: 1-2 hours (mostly testing)

---

### **Phase 4: Verify Pure Rust** (30 minutes)

#### **Verify No C Dependencies**

```bash
# Should show NO ring dependency
cargo tree -i ring

# Should show NO openssl-sys dependency
cargo tree -i openssl-sys

# Check for any other C dependencies
cargo tree | grep -E "(ring|openssl|cc\s)"
```

**Expected**: Zero C crypto dependencies!

#### **Test ARM64 Cross-Compilation**

```bash
# Install ARM64 target (if not already)
rustup target add aarch64-linux-android

# Attempt cross-compilation (should succeed!)
cargo build --release --target aarch64-linux-android --workspace

# Expected: SUCCESS! (no C compiler needed)
```

**Success Criteria**:
- ✅ Build completes without errors
- ✅ No "aarch64-linux-android-clang not found" errors
- ✅ Binaries ready for ARM64 deployment

---

### **Phase 5: Test Functional Equivalence** (1-2 hours)

#### **Critical Tests**

```bash
# JWT functionality (toadstool-api)
cargo test --package toadstool-api -- jwt
cargo test --package toadstool-api -- auth
cargo test --package toadstool-api -- token

# HTTP/HTTPS functionality
cargo test --workspace -- http
cargo test --workspace -- https
cargo test --workspace -- request

# Full workspace test suite
cargo test --workspace

# Integration tests
cargo test --test e2e_tests
cargo test --test runtime_integration_tests
```

**Expected**: 100% test pass rate (same as before migration)

#### **Smoke Tests**

```bash
# Start ToadStool server
cargo run --release --bin toadstool-server

# Test API endpoints (JWT auth)
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"test"}'

# Test HTTPS connections (if applicable)
cargo run --example https_demo
```

**Expected**: Everything works identically to before

---

## 📊 Effort Breakdown

| Phase | Task | Effort | Complexity |
|-------|------|--------|------------|
| 1 | Audit Dependencies | 15 min | Low |
| 2 | Evolve JWT (ring → jwt-simple) | 1-2 hours | Medium |
| 3 | Evolve TLS (OpenSSL → rustls) | 1-2 hours | Low-Medium |
| 4 | Verify Pure Rust | 30 min | Low |
| 5 | Test Functional Equivalence | 1-2 hours | Medium |
| **Total** | **Complete Evolution** | **4-8 hours** | **Medium-High** |

---

## ✅ Success Criteria

### **Technical Success**

- [ ] Zero `ring` dependencies (`cargo tree -i ring` shows nothing)
- [ ] Zero `openssl-sys` dependencies (`cargo tree -i openssl-sys` shows nothing)
- [ ] ARM64 cross-compilation succeeds (no C compiler needed)
- [ ] All existing tests pass (100% functional equivalence)
- [ ] JWT authentication works identically
- [ ] HTTPS/TLS connections work identically

### **Philosophy Success**

- [ ] 100% Pure Rust codebase (no C dependencies)
- [ ] ARM deployment unblocked
- [ ] Aligned with ecoPrimals pure Rust philosophy
- [ ] Easier to audit (all Rust code)
- [ ] Better portability (Rust everywhere)

---

## 🎓 Technical Background

### **Why ring is C/Assembly**

**ring**: Crypto library forked from BoringSSL (Google's OpenSSL fork)
- ❌ Contains C code for crypto primitives
- ❌ Contains assembly for performance
- ❌ Requires C compiler for cross-compilation
- ✅ Well-audited (inherited from BoringSSL)
- ❌ Not aligned with pure Rust philosophy

### **Why RustCrypto/jwt-simple is Better**

**RustCrypto**: Modern pure Rust crypto ecosystem
- ✅ 100% Pure Rust (no C, no assembly)
- ✅ Constant-time implementations
- ✅ Well-audited
- ✅ Actively maintained
- ✅ Modular (use only what you need)
- ✅ Cross-compiles to any target (no C compiler!)

**jwt-simple**: Modern JWT library using RustCrypto
- ✅ Pure Rust (uses RustCrypto underneath)
- ✅ Simpler API than jsonwebtoken
- ✅ Modern design
- ✅ Actively maintained

---

### **Why OpenSSL is C**

**OpenSSL**: Industry-standard TLS library (C codebase)
- ❌ Large C codebase
- ❌ Complex build system
- ❌ Requires native OpenSSL installation
- ❌ Hard to cross-compile
- ✅ Widely used, battle-tested

### **Why rustls is Better**

**rustls**: Modern pure Rust TLS implementation
- ✅ 100% Pure Rust
- ✅ Modern, memory-safe design
- ✅ Well-audited
- ✅ Actively maintained by Rust security experts
- ✅ Easier to integrate
- ✅ Cross-compiles to any target
- ✅ Used by default in modern Rust projects

---

## 📚 References

### **RustCrypto Ecosystem**

- **Main**: https://github.com/RustCrypto
- **SHA-2**: https://docs.rs/sha2
- **HMAC**: https://docs.rs/hmac
- **AES-GCM**: https://docs.rs/aes-gcm

### **JWT Libraries**

- **jwt-simple**: https://docs.rs/jwt-simple
- **Alternative**: https://github.com/Keats/jsonwebtoken/issues (check if v10+ is ring-free)

### **rustls**

- **Main**: https://github.com/rustls/rustls
- **Docs**: https://docs.rs/rustls
- **reqwest integration**: https://docs.rs/reqwest

### **BearDog Reference**

- **Guide**: `../beardog/BEARDOG_CRYPTO_EVOLUTION_HANDOFF.md`
- **Similar migration**: ring → RustCrypto (can reuse patterns)

---

## 🚨 Potential Gotchas

### **1. JWT API Differences**

**Issue**: jwt-simple API is different from jsonwebtoken

**Solution**: 
- Budget 1-2 hours for API migration
- Test thoroughly (unit tests should catch issues)
- jwt-simple is actually simpler - net positive!

---

### **2. rustls Certificate Verification**

**Issue**: rustls validates certificates more strictly than OpenSSL

**Solution**:
- Use `rustls-tls-native-roots` feature (uses system CA store)
- For self-signed certs (dev only): Use `danger_accept_invalid_certs()` feature
- Production: rustls strict validation is GOOD (more secure!)

---

### **3. Performance Differences**

**Issue**: Crypto performance might differ (ring has assembly optimizations)

**Reality**: 
- RustCrypto is highly optimized pure Rust
- Performance difference is minimal (< 10% in most cases)
- Modern Rust LLVM optimizations are excellent
- Worth it for pure Rust benefits!

**Recommendation**: Benchmark if critical path, but don't worry preemptively

---

## 🎯 Next Steps

### **Immediate** (This Session)

1. [ ] Create branch: `git checkout -b pure-rust-evolution`
2. [ ] Phase 1: Audit dependencies (15 min)
3. [ ] Phase 2: Evolve JWT library (1-2 hours)
4. [ ] Phase 3: Evolve TLS to rustls (1-2 hours)
5. [ ] Phase 4: Verify pure Rust (30 min)
6. [ ] Phase 5: Test functional equivalence (1-2 hours)
7. [ ] Commit: `git commit -m "evolve: Pure Rust (ring → jwt-simple, OpenSSL → rustls)"`
8. [ ] Test ARM64: `cargo build --target aarch64-linux-android`
9. [ ] Merge: `git checkout master && git merge pure-rust-evolution`
10. [ ] Celebrate: 🎉 **Pure Rust achieved!**

---

### **Validation** (After Merge)

1. [ ] Run full test suite: `cargo test --workspace`
2. [ ] Test ARM64 binary on Pixel 8a
3. [ ] Document learnings in wateringHole/
4. [ ] Update ecosystem tracking doc
5. [ ] Help other primals with similar migrations

---

## 💬 Support & Coordination

### **Get Help**

- **wateringHole/**: Inter-primal discussions
- **BearDog team**: Similar migration (ring → RustCrypto)
- **biomeOS team**: OpenSSL → rustls expertise

### **Share Learnings**

- **Patterns**: JWT migration approach
- **Blockers**: Any issues encountered
- **Benchmarks**: Performance comparisons
- **Wins**: ARM64 deployment success!

---

## 🎊 Expected Outcome

### **After Evolution**:

```bash
# Cross-compile to ARM64 (pure Rust!)
cargo build --release --target aarch64-linux-android --workspace
# ✅ SUCCESS! (no C compiler needed)

# Verify pure Rust
cargo tree | grep -E "(ring|openssl)"
# (nothing) ← Pure Rust! 🦀

# Deploy to Pixel 8a
adb push target/aarch64-linux-android/release/toadstool-server /data/local/tmp/
adb shell /data/local/tmp/toadstool-server
# ✅ Running on ARM64! 📱
```

**Philosophy Aligned**: 100% Pure Rust, ARM deployment unblocked! 🎉

---

## 📊 Summary

| Metric | Current | After Evolution |
|--------|---------|-----------------|
| **C Dependencies** | 2 (ring, openssl-sys) | 0 |
| **Pure Rust** | ❌ No | ✅ Yes |
| **ARM64 Cross-Compilation** | ❌ Fails | ✅ Works |
| **Philosophy Alignment** | ❌ Violated | ✅ Aligned |
| **Effort** | N/A | 4-8 hours |
| **Complexity** | N/A | Medium-High |
| **Priority** | N/A | **HIGH** |

---

**Status**: 🎯 **READY TO EVOLVE**  
**Primal**: ToadStool 🍄  
**Owner**: ToadStool team  
**Coordinator**: biomeOS (this doc)  
**Timeline**: 4-8 hours (one focused session)  
**Result**: 100% Pure Rust + ARM deployment! 🦀🚀

---

**Let's evolve to pure Rust!** 🌱🦀🏆

---

**Created**: January 16, 2026  
**Last Updated**: January 16, 2026  
**Purpose**: ToadStool pure Rust evolution coordination  
**Contact**: wateringHole/ for questions/help
