# 🎨 **MODERN RUST PATTERNS GUIDE**
## Applied to ToadStool Codebase

---

## 🎯 **PHILOSOPHY**

**Replace imperative code with declarative functional patterns**

- ✅ Iterator chains instead of manual loops
- ✅ Option/Result combinators instead of nested if/match
- ✅ Functional composition over procedural steps
- ✅ Zero-cost abstractions (no runtime overhead)

---

## 📋 **PATTERN CATALOG**

### **1. Iterator Chains**

#### **Before**:
```rust
let mut results = Vec::new();
for item in items {
    if item.is_valid() {
        results.push(item.process());
    }
}
```

#### **After**:
```rust
let results: Vec<_> = items
    .into_iter()
    .filter(|item| item.is_valid())
    .map(|item| item.process())
    .collect();
```

**Benefits**: More readable, composable, parallelizable

---

### **2. Option Combinators**

#### **Before**:
```rust
let value = if let Some(x) = maybe_value {
    if x > 10 {
        Some(x * 2)
    } else {
        None
    }
} else {
    None
};
```

#### **After**:
```rust
let value = maybe_value
    .filter(|&x| x > 10)
    .map(|x| x * 2);
```

**Combinators**: `map`, `and_then`, `filter`, `or_else`, `unwrap_or`, `map_or`

---

### **3. Result Combinators**

#### **Before**:
```rust
let result = do_something();
if result.is_err() {
    return Err(convert_error(result.unwrap_err()));
}
let value = result.unwrap();
process(value);
```

#### **After**:
```rust
do_something()
    .map_err(convert_error)?
    .and_then(process)?;
```

**Combinators**: `map`, `map_err`, `and_then`, `or_else`, `?` operator

---

### **4. Filtering with Combinators**

#### **Before**:
```rust
executions.values().filter(|exec| {
    if let Some(status) = &filter.status {
        if exec.status != *status {
            return false;
        }
    }
    if let Some(runtime) = &filter.runtime_type {
        if exec.runtime_type != *runtime {
            return false;
        }
    }
    true
})
```

#### **After**:
```rust
executions.values().filter(|exec| {
    filter.status.as_ref().map_or(true, |s| &exec.status == s)
        && filter.runtime_type.as_ref().map_or(true, |rt| &exec.runtime_type == rt)
})
```

**Pattern**: `map_or(default, predicate)` for optional filters

---

### **5. Find and Map (find_map)**

#### **Before**:
```rust
let mut result = None;
for exec in executions {
    if exec.status == ExecutionStatus::Running {
        result = Some(exec.execution_id);
        break;
    }
}
```

#### **After**:
```rust
let result = executions
    .iter()
    .find(|exec| matches!(exec.status, ExecutionStatus::Running))
    .map(|exec| exec.execution_id);
```

**Or even better**:
```rust
let result = executions
    .iter()
    .find_map(|exec| {
        matches!(exec.status, ExecutionStatus::Running)
            .then_some(exec.execution_id)
    });
```

---

### **6. Partition**

#### **Before**:
```rust
let mut completed = Vec::new();
let mut failed = Vec::new();
for exec in executions {
    if exec.status == ExecutionStatus::Completed {
        completed.push(exec);
    } else {
        failed.push(exec);
    }
}
```

#### **After**:
```rust
let (completed, failed): (Vec<_>, Vec<_>) = executions
    .into_iter()
    .partition(|exec| matches!(exec.status, ExecutionStatus::Completed));
```

---

### **7. Fold for Statistics**

#### **Before**:
```rust
let mut total = 0.0;
let mut count = 0;
for exec in executions {
    if let Some(duration) = exec.duration_ms {
        total += duration as f64;
        count += 1;
    }
}
let average = if count > 0 { total / count as f64 } else { 0.0 };
```

#### **After**:
```rust
let (total, count) = executions
    .iter()
    .filter_map(|exec| exec.duration_ms)
    .fold((0u64, 0usize), |(sum, count), duration| {
        (sum + duration, count + 1)
    });

let average = if count > 0 { total as f64 / count as f64 } else { 0.0 };
```

---

### **8. Collect into HashMap**

#### **Before**:
```rust
let mut map = HashMap::new();
for exec in executions {
    map.insert(exec.execution_id, exec.status);
}
```

#### **After**:
```rust
let map: HashMap<_, _> = executions
    .into_iter()
    .map(|exec| (exec.execution_id, exec.status))
    .collect();
```

---

### **9. Chaining Options**

#### **Before**:
```rust
let cpu_cores = if let Some(resources) = &request.resources {
    if let Some(cpu) = resources.cpu_cores {
        cpu
    } else {
        1.0
    }
} else {
    1.0
};
```

#### **After**:
```rust
let cpu_cores = request
    .resources
    .as_ref()
    .and_then(|r| r.cpu_cores)
    .unwrap_or(1.0);
```

---

### **10. Error Context**

#### **Before**:
```rust
let result = risky_operation();
if let Err(e) = result {
    return Err(ApiError::new("OPERATION_FAILED", &format!("Failed: {}", e)));
}
```

#### **After**:
```rust
risky_operation()
    .map_err(|e| ApiError::new("OPERATION_FAILED", &format!("Failed: {}", e)))?;
```

**Or with custom error trait**:
```rust
risky_operation()
    .context("Operation failed during execution")?;
```

---

## 🎯 **APPLIED TO TOADSTOOL**

### **Example 1: Execution Filtering** (crates/api/src/handlers/execution_modern.rs)

See `list_executions_modern()` for modern filtering patterns.

### **Example 2: Resource Extraction** (crates/api/src/handlers/execution_modern.rs)

See `extract_resource_value_modern()` for Option chaining.

### **Example 3: Statistics** (crates/api/src/handlers/execution_modern.rs)

See `calculate_average_duration_modern()` for fold patterns.

---

## 📊 **BENEFITS**

### **Readability**
- Declarative: "what" not "how"
- Self-documenting patterns
- Less boilerplate

### **Safety**
- Fewer manual index operations
- Clearer ownership
- Compiler-enforced correctness

### **Performance**
- Zero-cost abstractions
- Optimized by LLVM
- Often faster than manual loops

### **Composability**
- Chain operations easily
- Reusable patterns
- Easy to refactor

---

## 🔄 **MIGRATION STRATEGY**

### **Phase 1**: Identify Manual Loops
```bash
# Find manual loops
grep -r "for.*in" crates/*/src/*.rs
```

### **Phase 2**: Replace with Iterators
- Start with simple cases (map, filter)
- Move to complex cases (fold, partition)

### **Phase 3**: Apply Combinators
- Option chains
- Result chains
- Error handling

### **Phase 4**: Benchmark
- Ensure no performance regression
- Usually equal or better performance

---

## ✅ **CHECKLIST**

- [ ] Replace for loops with iterator chains
- [ ] Use Option combinators (`map`, `and_then`, `filter`)
- [ ] Use Result combinators (`map_err`, `and_then`, `?`)
- [ ] Apply `find_map` instead of find + map
- [ ] Use `partition` for splitting collections
- [ ] Use `fold` for aggregations
- [ ] Collect directly into target types
- [ ] Chain error handling
- [ ] Benchmark critical paths

---

## 📚 **FURTHER READING**

- [Rust Iterator Documentation](https://doc.rust-lang.org/std/iter/trait.Iterator.html)
- [Option Combinator Guide](https://doc.rust-lang.org/std/option/)
- [Result Combinator Guide](https://doc.rust-lang.org/std/result/)
- [Effective Rust Patterns](https://www.lurklurk.org/effective-rust/)

---

**Status**: ✅ Guide complete  
**Examples**: See `crates/api/src/handlers/execution_modern.rs`  
**Application**: Apply patterns systematically across codebase

