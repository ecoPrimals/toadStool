# JSON-RPC Unix Socket + Distributed Coordinator Priority Implementation Plan

**Date**: January 10, 2026  
**Status**: IN PROGRESS  
**Priority**: HIGH (Both are fundamental to ToadStool architecture)

---

## 🎯 REASSESSMENT

The user is correct - both priorities are fundamental:

1. **JSON-RPC Unix Socket**: Other primals need this pattern (not just ToadStool)
2. **Distributed Coordinator**: Core to ToadStool's isomorphic/fractal architecture

---

## 📊 CURRENT ATTEMPT - JSON-RPC UNIX SOCKET

### **Approach Tried**:
Using `hyper-util` custom transport layer with `jsonrpsee`

### **Issues Encountered**:
- `jsonrpsee` API complexity (raw_json_request signature changes)
- `MethodResponse` serialization complications  
- Return type mismatches with `ServerHandle`

### **Lessons Learned**:
- `jsonrpsee` is designed for TCP/WebSocket, not custom transports
- Forcing Unix socket support requires fighting the library
- The tarpc pattern we already implemented is simpler and works

---

## 💡 ALTERNATIVE SOLUTION (RECOMMENDED)

### **Option 1: JSON over tarpc transport (Simple)**

Already working:
```rust
// We have:
tarpc over Unix socket ✅ (working)
JSON serialization ✅ (serde)

// Just add:
JSON-RPC wrapper over tarpc transport
```

**Benefits**:
- Reuses proven Unix socket infrastructure
- Simple JSON wrapper layer
- No fighting libraries
- Same pattern other primals can use

### **Option 2: Pure manual HTTP over Unix socket**

Skip jsonrpsee entirely:
```rust
// Manual HTTP/1.1 parser
// Direct JSON-RPC 2.0 implementation
// Full control, no library fights
```

**Benefits**:
- Complete control
- No library dependencies
- Pure Rust
- Educational for other primals

---

## 🔧 DISTRIBUTED COORDINATOR INTEGRATION

### **Current State**:
- `DistributedCoordinator` exists in `crates/distributed`
- Uses isomorphic/fractal pattern already
- Capability-based discovery via Songbird
- Standalone fallback mode

### **What's Needed**:
Integrate as `WorkloadExecutor` for `tarpc_server`:

```rust
// Instead of:
let executor = Arc::new(StandaloneExecutor::new());

// Use:
let coordinator = Arc::new(DistributedCoordinator::new(config).await?);
let executor = Arc::new(CoordinatorExecutor::new(coordinator));
```

### **Implementation**:
Create `CoordinatorExecutor` wrapper:
```rust
pub struct CoordinatorExecutor {
    coordinator: Arc<DistributedCoordinator>,
}

#[async_trait]
impl WorkloadExecutor for CoordinatorExecutor {
    async fn execute(&self, submission: WorkloadSubmission) -> Result<WorkloadResult, String> {
        // Convert WorkloadSubmission -> ExecutionRequest
        let request = convert_to_execution_request(submission);
        
        // Submit to coordinator (isomorphic/fractal routing)
        let execution_id = self.coordinator.submit_execution(request).await?;
        
        // Convert result back
        convert_to_workload_result(execution_id).await
    }
    
    // ... other methods
}
```

---

## 🎯 RECOMMENDED ACTION PLAN

### **Phase 1: Distributed Coordinator (4-6 hours)**
1. Create `CoordinatorExecutor` wrapper ✅ Simple adapter
2. Implement type conversions ✅ Straightforward mapping
3. Replace `StandaloneExecutor` in `main.rs` ✅ One line change
4. Test multi-instance coordination ✅ Already designed for this

**Why First**: 
- Simpler than JSON-RPC Unix socket fight
- Core to ToadStool architecture
- Enables isomorphic/fractal patterns immediately
- Other primals can learn from the pattern

### **Phase 2: JSON-RPC Unix Socket (6-8 hours)**
Choose Option 1 or 2 above:

**Option 1** (Recommended): JSON over tarpc
- Add JSON-RPC protocol layer
- Reuse Unix socket transport
- 4-6 hours

**Option 2**: Manual HTTP parser
- Full JSON-RPC 2.0 impl
- Direct Unix socket HTTP
- 6-8 hours
- More educational

---

## 🚧 CURRENT BLOCKERS

### **JSON-RPC Unix Socket**:
- `jsonrpsee` API doesn't fit Unix socket pattern well
- Fighting the library instead of solving the problem
- Need to pivot to simpler approach

### **Distributed Coordinator**:
- No blockers! Ready to implement
- Type conversions are straightforward
- Pattern already proven

---

## 💼 RECOMMENDATION TO USER

### **Short Term (Today)**:
1. **Complete Distributed Coordinator integration** (Phase 1)
   - Achieves isomorphic/fractal architecture
   - Unblocks multi-instance coordination
   - Clean, proven pattern

2. **Reassess JSON-RPC approach** (Phase 2)
   - Option 1: JSON over tarpc (simple, fast)
   - Option 2: Manual HTTP (pure Rust, educational)
   - Don't fight jsonrpsee library

### **Why This Order**:
- Distributed coordinator is unblocked and ready
- JSON-RPC needs architectural decision first
- Both are equally important, but one is ready to ship

---

## 📝 NEXT STEPS

**Immediate** (Next 1-2 hours):
1. Implement `CoordinatorExecutor` wrapper
2. Add type conversion helpers
3. Update `main.rs` to use coordinator
4. Test with multiple instances

**Following** (Next 2-4 hours):
1. Choose JSON-RPC approach (Option 1 or 2)
2. Implement chosen approach
3. Test with biomeOS patterns
4. Document for other primals

---

**Status**: Awaiting user decision on priority order  
**Updated**: January 10, 2026

*Isomorphic. Fractal. Self-knowledge. Runtime discovery.* 🍄🐸

