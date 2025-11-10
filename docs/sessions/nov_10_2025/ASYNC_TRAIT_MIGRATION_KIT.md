# 🛠️ async_trait Migration Toolkit

**Created**: November 10, 2025  
**Status**: Complete migration guide + automated scripts  
**Progress**: 12/74 instances migrated (16.2%)  
**Remaining**: 62 instances across 34 files

---

## ✅ COMPLETED SO FAR (12/74 instances)

### **File 1**: `crates/core/toadstool/src/os_layer/compat.rs` ✅ **DONE** (5 instances)
- Migrated `CompatibilityLayer` trait + 4 implementations
- Status: ✅ Compiles, tested

### **File 2**: `crates/core/common/src/infant_discovery/capabilities.rs` ✅ **DONE** (1 trait)
- Migrated `EndpointSource` trait definition
- Status: ✅ Compiles

### **File 3**: `crates/core/common/src/infant_discovery/sources.rs` ✅ **DONE** (5 instances)
- Migrated 5 implementations: Environment, Fallback, MDNS, ServiceMesh, ConfigFile
- Status: ✅ Compiles, tested

### **File 4**: `crates/core/common/src/infant_discovery/capabilities.rs` ✅ **DONE** (1 trait)
- Migrated `SubstrateDetector` trait definition
- Status: ✅ Compiles

### **File 5**: `crates/core/common/src/infant_discovery/detectors.rs` 🔄 **IN PROGRESS** (1/5 done)
- ✅ Kubernetes

Detector (done)
- ⏳ DockerDetector (pending)
- ⏳ ConsulDetector (pending)
- ⏳ CloudDetector (pending)
- ⏳ BareMetalDetector (pending)

---

## 📋 REMAINING FILES (62 instances across 34 files)

### **High Priority** (15 instances remaining)

1. **detectors.rs** (4 remaining)
   - DockerDetector, ConsulDetector, CloudDetector, BareMetalDetector

2. **biomeos_integration/storage_backend.rs** (4 instances)
   - Storage backend traits

3. **execution.rs** (3 instances)
4. **biomeos_integration/auth_backend.rs** (3 instances)
5. **biomeos_integration/agent_backend.rs** (3 instances)

### **Medium Priority** (remaining ~47 instances across 29 files)

*See complete list in UNIFICATION_FILE_LOCATIONS.md*

---

## 🎯 MIGRATION PATTERN

All migrations follow this consistent pattern:

### **Pattern 1: Trait Definition**

```rust
// BEFORE:
#[async_trait]
pub trait MyTrait: Send + Sync {
    async fn method(&self, param: &str) -> Result<T, E>;
}

// AFTER:
pub trait MyTrait: Send + Sync {
    fn method(&self, param: &str) 
        -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + '_>>;
}
```

### **Pattern 2: Implementation (Simple)**

```rust
// BEFORE:
#[async_trait]
impl MyTrait for MyStruct {
    async fn method(&self, param: &str) -> Result<T, E> {
        // async code
        Ok(result)
    }
}

// AFTER:
impl MyTrait for MyStruct {
    fn method(&self, param: &str) 
        -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + '_>> 
    {
        let param = param.to_string();  // Capture owned data
        Box::pin(async move {
            // async code (same logic)
            Ok(result)
        })
    }
}
```

### **Pattern 3: Implementation (With Self Access)**

```rust
// BEFORE:
#[async_trait]
impl MyTrait for MyStruct {
    async fn method(&self, param: &str) -> Result<T, E> {
        let value = self.field;  // Access self
        // async code using value
    }
}

// AFTER:
impl MyTrait for MyStruct {
    fn method(&self, param: &str) 
        -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + '_>> 
    {
        let param = param.to_string();
        let value = self.field;  // Capture from self BEFORE async block
        
        Box::pin(async move {
            // async code using value
        })
    }
}
```

---

## 🚀 STEP-BY-STEP MIGRATION GUIDE

### **For Each File:**

#### **Step 1: Update Imports**

```rust
// REMOVE:
use async_trait::async_trait;

// ADD:
use std::pin::Pin;
use std::future::Future;
```

#### **Step 2: Migrate Trait Definition** (if applicable)

Find the trait and remove `#[async_trait]`, update method signatures.

#### **Step 3: Migrate Each Implementation**

1. Remove `#[async_trait]` above impl
2. Change method signature
3. Capture any data from `&self` or parameters
4. Wrap body in `Box::pin(async move { ... })`

#### **Step 4: Test**

```bash
# Check compilation
cargo check --package <package-name>

# Run tests
cargo test --package <package-name>
```

---

## 🔧 SEMI-AUTOMATED MIGRATION SCRIPT

Save this as `scripts/migrate_async_trait.sh`:

```bash
#!/bin/bash

# Semi-automated async_trait migration helper
# Usage: ./migrate_async_trait.sh <file_path>

FILE="$1"

if [ -z "$FILE" ]; then
    echo "Usage: $0 <file_path>"
    exit 1
fi

if [ ! -f "$FILE" ]; then
    echo "Error: File not found: $FILE"
    exit 1
fi

echo "🔄 Migrating $FILE"
echo "---"

# Step 1: Remove async_trait import
echo "Step 1: Removing async_trait import..."
sed -i 's/use async_trait::async_trait;//g' "$FILE"

# Step 2: Add required imports (if not present)
if ! grep -q "use std::pin::Pin;" "$FILE"; then
    echo "Step 2: Adding Pin import..."
    # Add after first 'use' statement
    sed -i '/^use /a use std::pin::Pin;' "$FILE"
fi

if ! grep -q "use std::future::Future;" "$FILE"; then
    echo "Step 3: Adding Future import..."
    sed -i '/^use std::pin::Pin;/a use std::future::Future;' "$FILE"
fi

# Step 3: Remove #[async_trait] attributes
echo "Step 4: Removing #[async_trait] attributes..."
sed -i '/#\[async_trait\]/d' "$FILE"

echo "---"
echo "✅ Automated steps complete!"
echo ""
echo "⚠️  MANUAL STEPS REQUIRED:"
echo "1. Update trait method signatures (async fn -> fn ... -> Pin<Box<...>>)"
echo "2. Update impl method signatures"
echo "3. Wrap async bodies in Box::pin(async move { ... })"
echo "4. Capture data from &self before async block"
echo ""
echo "📝 Open file to complete manual steps:"
echo "   \$EDITOR $FILE"
echo ""
echo "🧪 Test after completion:"
echo "   cargo check --package <package-name>"

```

Make it executable:
```bash
chmod +x scripts/migrate_async_trait.sh
```

---

## 📊 MIGRATION CHECKLIST

Use this checklist for each file:

```
[ ] Run migration script: ./scripts/migrate_async_trait.sh <file>
[ ] Open file in editor
[ ] Find trait definition (if any)
    [ ] Remove #[async_trait]
    [ ] Update method signature: async fn -> fn ... -> Pin<Box<...>>
[ ] Find all implementations
    For each implementation:
    [ ] Remove #[async_trait]
    [ ] Update method signature
    [ ] Capture data from &self or parameters
    [ ] Wrap body in Box::pin(async move { ... })
[ ] Save file
[ ] Run: cargo check --package <package>
[ ] Fix any compilation errors
[ ] Run: cargo test --package <package>
[ ] Mark file as complete
```

---

## 🎯 RECOMMENDED ORDER

### **Week 1: High-Priority Files** (19 instances)

1. ✅ `infant_discovery/detectors.rs` - finish remaining 4 (30 min)
2. ⏳ `biomeos_integration/storage_backend.rs` - 4 instances (30 min)
3. ⏳ `execution.rs` - 3 instances (30 min)
4. ⏳ `biomeos_integration/auth_backend.rs` - 3 instances (30 min)
5. ⏳ `biomeos_integration/agent_backend.rs` - 3 instances (30 min)
6. ⏳ `infant_discovery/engine.rs` - 3 instances (30 min)

**Total Time**: ~3 hours

### **Week 2: Medium-Priority Files** (~20 instances)

Process 4-5 files per day (1-2 hours per day).

### **Week 3: Remaining Files** (~23 instances)

Complete remaining files (1-2 hours per day).

---

## 🧪 TESTING STRATEGY

### **After Each File**

```bash
# Quick check
cargo check --package <package-name>

# Full test
cargo test --package <package-name> -- --nocapture
```

### **After Batch of Files**

```bash
# Check entire workspace
cargo check --workspace

# Run all tests
cargo test --workspace

# Check for regressions
cargo clippy --workspace
```

### **Before Committing**

```bash
# Full validation
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

---

## 📈 TRACKING PROGRESS

Update `ASYNC_TRAIT_MIGRATION_PROGRESS.md` after each file:

```markdown
### Completed
- [x] compat.rs (5/5) - 2025-11-10
- [x] sources.rs (5/5) - 2025-11-10
- [ ] detectors.rs (1/5) - IN PROGRESS

### Statistics
- Completed: 12/74 (16.2%)
- Remaining: 62/74 (83.8%)
```

---

## 💡 TIPS & TRICKS

### **Tip 1: Batch Similar Files**

Files in the same module often have similar patterns. Do them together.

### **Tip 2: Test Incrementally**

Don't migrate 10 files before testing. Test after each 1-2 files.

### **Tip 3: Use Pattern Search**

```bash
# Find all remaining async_trait instances
grep -rn "#\[async_trait\]" crates --include="*.rs"

# Count remaining
grep -r "#\[async_trait\]" crates --include="*.rs" | wc -l
```

### **Tip 4: Keep Examples Handy**

Refer to completed files (compat.rs, sources.rs) as templates.

---

## 🚨 COMMON PITFALLS

### **Pitfall 1: Forgetting to Capture Self Fields**

```rust
// ❌ WRONG:
fn method(&self) -> Pin<Box<...>> {
    Box::pin(async move {
        let val = self.field;  // ERROR: self moved
    })
}

// ✅ CORRECT:
fn method(&self) -> Pin<Box<...>> {
    let val = self.field;  // Capture BEFORE async
    Box::pin(async move {
        // Use val
    })
}
```

### **Pitfall 2: Not Cloning When Needed**

```rust
// If self.field is not Copy:
let val = self.field.clone();  // Clone before async block
```

### **Pitfall 3: Lifetime Issues**

```rust
// For &str parameters, convert to owned:
let param = param.to_string();
Box::pin(async move { /* use param */ })
```

---

## 📞 NEED HELP?

If you encounter issues:

1. **Check completed files** for similar patterns
2. **Run cargo check** to see specific errors
3. **Refer to this guide** for common solutions
4. **Test incrementally** - don't migrate too many at once

---

## 🎊 SUCCESS CRITERIA

You'll know Phase 1 is complete when:

- [ ] Zero `#[async_trait]` instances in codebase
- [ ] All packages compile: `cargo check --workspace`
- [ ] All tests pass: `cargo test --workspace`
- [ ] No clippy warnings: `cargo clippy --workspace -- -D warnings`
- [ ] Performance improvement measured (15-30% for async operations)

---

**Good luck with the migration!** 🚀

*This toolkit created based on 12 successfully migrated instances*  
*Estimated remaining time: 4-6 hours (distributed over 2-3 weeks)*

