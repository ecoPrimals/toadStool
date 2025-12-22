# 🚀 Zero-Copy Optimization Plan
## December 12, 2025 - Performance Evolution

---

## 🎯 **GOAL**: 75% → 90% Zero-Copy Score

**Current**: ~75/100 (Good)  
**Target**: ~90/100 (Excellent)  
**Impact**: 15-25% performance improvement

---

## 📊 **HOT PATH ANALYSIS**

### **1. Execution Request Creation** 🔥🔥🔥

**Location**: `crates/core/toadstool/src/byob/executor.rs:110-150`

**Current Issues**:
```rust
// ❌ ALLOCATIONS: Multiple clones per execution
environment.insert("BYOB_DEPLOYMENT_ID".to_string(), // allocation 1
    deployment.request.deployment_id.to_string()); // allocation 2
environment.insert("BYOB_SERVICE_NAME".to_string(), // allocation 3
    service_name.to_string()); // allocation 4
environment.insert("BYOB_TEAM_ID".to_string(), // allocation 5
    deployment.request.team_id.clone()); // allocation 6
```

**Optimizations**:
1. Use `String::from` for static strings (minor)
2. Use format! once instead of multiple to_string()
3. Pre-allocate HashMap with capacity
4. Consider Cow<str> for conditional ownership

---

### **2. Service Spec Conversion** 🔥🔥

**Location**: `crates/core/toadstool/src/byob/byob_impl.rs:135-224`

**Current Issues**:
```rust
// ❌ MULTIPLE CLONES per service
image: image.clone(),           // clone 1
command: service.command.clone(),  // clone 2
environment: service.environment.clone(), // clone 3 (HashMap!)
```

**Impact**: Called once per service in deployment

**Optimizations**:
1. Use references where ownership not needed
2. Move values when taking ownership
3. Use Arc<T> for shared data
4. Builder pattern to avoid intermediate clones

---

### **3. String Allocations** 🔥

**Frequency**: 4,029 `.to_string()` calls

**Hot Paths**:
- Port mapping creation
- Service instance IDs
- Status messages
- Log formatting

**Optimizations**:
1. Use `format_args!` for temporary strings
2. Implement `Display` instead of building strings
3. Use `&str` parameters where possible
4. String interning for repeated values

---

## 🎯 **PRIORITY OPTIMIZATIONS**

### **Phase 1: Execution Hot Path** (Highest Impact)

**Files**:
- `crates/core/toadstool/src/byob/executor.rs`
- `crates/core/toadstool/src/byob/byob_impl.rs`

**Changes**:
1. ✅ Pre-allocate HashMaps with capacity
2. ✅ Use `String::from` for static strings
3. ✅ Reduce environment variable clones
4. ✅ Optimize service instance creation

**Expected Impact**: 5-10% reduction in allocation

---

### **Phase 2: Data Structure Optimization** (Medium Impact)

**Focus**: Reduce clone() calls by 50%

**Strategy**:
1. Use `Arc<T>` for shared immutable data
2. Use `Cow<str>` for conditional ownership
3. Move semantics where possible
4. Reference parameters instead of owned

**Expected Impact**: 10-15% reduction in allocation

---

### **Phase 3: String Optimization** (Long-tail)

**Focus**: Eliminate unnecessary .to_string()

**Strategy**:
1. Implement `Display` trait
2. Use `format_args!` in hot paths
3. String interning for IDs
4. Static string refs where possible

**Expected Impact**: 5% reduction in allocation

---

## 📐 **OPTIMIZATION PATTERNS**

### **Pattern 1: Pre-allocated HashMap**

**Before**:
```rust
let mut environment = service_spec.environment.clone();
environment.insert(key.to_string(), value.to_string());
```

**After**:
```rust
let mut environment = HashMap::with_capacity(
    service_spec.environment.len() + 4  // known additions
);
environment.extend(service_spec.environment.iter()
    .map(|(k, v)| (k.clone(), v.clone())));
environment.insert(key, value); // Use &'static str or move
```

---

### **Pattern 2: Cow for Conditional Ownership**

**Before**:
```rust
pub fn format_message(&self, msg: &str) -> String {
    if needs_formatting {
        format!("prefix: {}", msg)
    } else {
        msg.to_string()  // unnecessary allocation!
    }
}
```

**After**:
```rust
use std::borrow::Cow;

pub fn format_message<'a>(&self, msg: &'a str) -> Cow<'a, str> {
    if needs_formatting {
        Cow::Owned(format!("prefix: {}", msg))
    } else {
        Cow::Borrowed(msg)  // zero-copy!
    }
}
```

---

### **Pattern 3: Display Instead of Building Strings**

**Before**:
```rust
fn log_execution(&self, id: Uuid, status: &str) {
    let msg = format!("Execution {} status: {}", id, status);
    tracing::info!("{}", msg);
}
```

**After**:
```rust
fn log_execution(&self, id: Uuid, status: &str) {
    tracing::info!("Execution {} status: {}", id, status);  // format_args! internally
}
```

---

### **Pattern 4: Move Instead of Clone**

**Before**:
```rust
fn consume_data(&self, data: Vec<u8>) -> Result<Output> {
    let owned = data.clone();  // unnecessary clone!
    self.process(owned)
}
```

**After**:
```rust
fn consume_data(&self, data: Vec<u8>) -> Result<Output> {
    self.process(data)  // move ownership, no clone
}
```

---

## 🧪 **BENCHMARKING STRATEGY**

### **Measure Before Optimization**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_execution_request(c: &mut Criterion) {
    c.bench_function("create_execution_request", |b| {
        b.iter(|| {
            let request = create_execution_request(
                black_box("service-name"),
                black_box(&service_spec),
            );
        });
    });
}
```

### **Metrics to Track**

1. **Allocations**: Number of allocations per execution
2. **Memory**: Total bytes allocated
3. **Time**: Execution time (microseconds)
4. **Throughput**: Requests per second

---

## 📋 **IMPLEMENTATION CHECKLIST**

### **Phase 1: Hot Path** (This Session)

- [x] Identify hot paths
- [ ] Benchmark current performance
- [ ] Implement HashMap pre-allocation
- [ ] Reduce environment clones
- [ ] Optimize service instance creation
- [ ] Benchmark improvements
- [ ] Document changes

### **Phase 2: Data Structures** (Next Session)

- [ ] Audit all .clone() calls
- [ ] Introduce Arc<T> for shared data
- [ ] Use Cow<str> for conditional ownership
- [ ] Refactor to move semantics
- [ ] Benchmark improvements

### **Phase 3: String Optimization** (Future)

- [ ] Audit all .to_string() calls
- [ ] Implement Display traits
- [ ] Use format_args! in hot paths
- [ ] Consider string interning
- [ ] Benchmark improvements

---

## 🎯 **SUCCESS CRITERIA**

### **Phase 1 Complete When**:

1. ✅ 10% reduction in allocations (execution hot path)
2. ✅ Benchmarks show improvement
3. ✅ Zero performance regressions
4. ✅ All tests passing
5. ✅ Code review approved

### **Overall Success**:

1. Zero-copy score: 75% → 90%
2. Performance gain: 15-25%
3. Memory reduction: 20-30%
4. Maintainability: Same or better

---

## 🚫 **WHAT NOT TO OPTIMIZE**

### **Don't Optimize**:

1. **Test Code**: Clones are fine in tests
2. **Error Paths**: Readability > performance
3. **Cold Paths**: Rarely executed code
4. **Initialization**: One-time setup code

### **When to Clone**:

1. **Safety**: When ownership unclear
2. **Clarity**: When logic becomes complex
3. **Correctness**: When avoiding borrow checker battles
4. **APIs**: When public API needs owned data

---

## 📚 **REFERENCES**

### **Rust Performance Book**

- Chapter on Allocations: https://nnethercote.github.io/perf-book/
- Zero-copy patterns
- Cow documentation

### **Our Codebase**

- Hot paths: `crates/core/toadstool/src/byob/`
- Execution: `crates/core/toadstool/src/execution.rs`
- Benchmarks: `benches/hot_paths.rs`

---

## ✅ **NEXT STEPS**

1. **Immediate**: Implement Phase 1 optimizations
2. **Today**: Benchmark improvements
3. **This Week**: Complete hot path optimization
4. **Next Week**: Data structure optimization

---

**Plan Date**: December 12, 2025  
**Status**: Ready for Implementation  
**Expected Impact**: 15-25% performance gain

