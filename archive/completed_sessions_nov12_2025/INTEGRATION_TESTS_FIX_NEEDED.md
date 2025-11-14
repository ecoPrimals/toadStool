# Integration Protocol Tests - API Migration Needed
## Date: November 12, 2025

## 🔍 ISSUE SUMMARY

Pre-existing integration protocol tests are using deprecated `ServiceAuthConfig` API and need updating to match current `toadstool_common::auth` implementation.

## ⚠️ AFFECTED FILES

```
crates/integration/protocols/tests/
├── beardog_types_comprehensive_tests.rs  ✅ FIXED (duplicate import)
├── config_comprehensive_tests.rs         🔄 PARTIALLY FIXED (4/8 tests)
├── protocol_client_tests.rs              ❌ NEEDS FIXING
└── protocol_structures_comprehensive_tests.rs  ❌ NEEDS FIXING
```

## 📋 ROOT CAUSE

`ServiceAuthConfig` API changed from flat structure to nested credentials:

### Old API (tests use this):
```rust
ServiceAuthConfig {
    auth_type: AuthType::Bearer,
    token: Some("test".to_string()),
    cert_path: None,
    key_path: None,
    ca_path: None,
}
```

### New API (current implementation):
```rust
ServiceAuthConfig {
    auth_type: toadstool_common::AuthType::Bearer,
    credentials: AuthCredentials {
        token: Some("test".to_string()),
        cert_path: None,
        key_path: None,
        ca_path: None,
        ..Default::default()
    },
}
```

### Best Practice (helper methods):
```rust
// Bearer token
ServiceAuthConfig::bearer("test")

// API key
ServiceAuthConfig::api_key("key123")

// mTLS
ServiceAuthConfig::mtls("/cert", "/key", Some("/ca".into()))

// None
ServiceAuthConfig::none()
```

## 🔧 REQUIRED FIXES

### 1. Import Statement Fix
Replace protocol-local `AuthType` with `toadstool_common::AuthType`:

```rust
// Wrong (uses protocol's local type)
use toadstool_integration_protocols::AuthType;

// Correct (uses canonical type)
use toadstool_common::AuthType;
```

### 2. Constructor Pattern (Recommended)
```rust
// Before
let config = ServiceAuthConfig {
    auth_type: AuthType::Bearer,
    token: Some("test".to_string()),
    cert_path: None,
    key_path: None,
    ca_path: None,
};

// After
let config = ServiceAuthConfig::bearer("test");
```

### 3. Field Access Pattern
```rust
// Before
assert_eq!(config.token, Some("test".to_string()));

// After
assert_eq!(config.credentials.token, Some("test".to_string()));
```

## 📊 FIX PROGRESS

### Completed
- ✅ beardog_types_comprehensive_tests.rs (duplicate import removed)
- ✅ config_comprehensive_tests.rs (4 tests fixed)

### Remaining
- ❌ config_comprehensive_tests.rs (4 more tests need fixing)
- ❌ protocol_client_tests.rs (~5-10 instances)
- ❌ protocol_structures_comprehensive_tests.rs (~5-10 instances)

### Estimated Time
- **Total**: 1-2 hours to fix all remaining tests
- **Per file**: 15-30 minutes

## 🎯 IMPACT

### Current Status
- ❌ Integration protocol tests cannot compile
- ❌ Coverage analysis blocked (needs all tests to compile)
- ✅ All 316 NEW tests from this session compile and pass
- ✅ Main codebase is unaffected

### Priority
- **Priority**: Medium-High
- **Blocker for**: Coverage measurement
- **Not blocking**: New test functionality, staging deployment

## 🚀 RECOMMENDED ACTION

### Option A: Quick Fix (1-2 hours)
1. Fix remaining test files using helper methods
2. Run coverage analysis
3. Document actual coverage improvement

### Option B: Defer to Next Session
1. Document issue (this file)
2. Schedule focused 2-hour session
3. Continue with other priorities

### Option C: Team Assignment
1. Assign to team member familiar with auth
2. Use this document as guide
3. PR review before merge

## 📝 FIX TEMPLATE

For each occurrence, apply this pattern:

```rust
// Pattern 1: Constructor replacement
- ServiceAuthConfig {
-     auth_type: AuthType::Bearer,
-     token: Some("value".to_string()),
-     cert_path: None,
-     key_path: None,
-     ca_path: None,
- }
+ ServiceAuthConfig::bearer("value")

// Pattern 2: Field access update
- config.token
+ config.credentials.token

// Pattern 3: Type matching
- matches!(config.auth_type, AuthType::Bearer)
+ matches!(config.auth_type, toadstool_common::AuthType::Bearer)
```

## ✅ VERIFICATION CHECKLIST

After fixes:
- [ ] All integration protocol tests compile
- [ ] All integration protocol tests pass
- [ ] `cargo test -p toadstool-integration-protocols` succeeds
- [ ] Coverage analysis runs successfully
- [ ] No clippy warnings introduced
- [ ] Documentation updated if needed

## 🔗 RELATED FILES

- API Definition: `crates/core/common/src/auth.rs`
- Helper Methods: Lines 183-218 in auth.rs
- Test Files: `crates/integration/protocols/tests/*.rs`

---

**Created**: November 12, 2025 (end of session)
**Status**: Documented, ready for focused fix session
**Estimated Effort**: 1-2 hours
**Priority**: Medium-High (blocks coverage measurement)

