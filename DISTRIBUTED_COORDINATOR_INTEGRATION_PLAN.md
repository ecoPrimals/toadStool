# 🎯 Distributed Coordinator Integration Plan

**Date**: January 10, 2026  
**Status**: 📋 **READY TO EXECUTE**  
**Priority**: **P0** (Next major milestone)

---

## 🔍 CURRENT STATE ANALYSIS

### **What We Have** ✅

#### **1. StandaloneExecutor (Server)**
Location: `crates/server/src/tarpc_server.rs:246`

```rust
pub struct StandaloneExecutor {
    capabilities: ComputeCapabilities,
}
```

**Purpose**: Simple in-memory executor for standalone mode  
**Status**: ✅ Production-ready (real system query, no hardcoding)  
**Limitation**: Single-instance only, no distributed coordination

#### **2. DistributedCoordinator (Distributed crate)**
Location: `crates/distributed/src/core/coordinator.rs:16`

```rust
pub struct DistributedCoordinator {
    config: DistributedConfig,
    capabilities: Arc<RwLock<ToadStoolCapabilities>>,
    coordination_client: Option<Arc<CoordinationClient>>,
    standalone_executor: Arc<StandaloneExecutor>,
}
```

**Purpose**: Multi-instance coordination with Songbird integration  
**Status**: ✅ Implemented, NOT WIRED to server  
**Features**:
- Capability-based discovery
- Coordination service integration (Songbird)
- Load balancing
- Graceful fallback to standalone

#### **3. Supporting Infrastructure** ✅

**Songbird Integration**:
- `crates/distributed/src/songbird_integration/` (complete)
- `crates/server/src/songbird_client.rs` (implemented)

**Coordination Integration**:
- `crates/distributed/src/coordination_integration/` (complete)

**Common Types**:
- `crates/distributed/src/types/` (execution, jobs, resources)

---

## 🎯 INTEGRATION GOALS

### **Phase 1: Wire DistributedCoordinator to Server** 🔴

**Goal**: Replace `StandaloneExecutor` with `DistributedCoordinator` in server daemon

**Why**:
- Enable multi-instance coordination
- Support distributed workload execution
- Maintain backward compatibility (standalone fallback)

**Impact**:
- Single-instance: Works as before (standalone mode)
- Multi-instance: Discovers peers via Songbird, distributes workloads
- Zero breaking changes (transparent upgrade)

---

## 📋 EXECUTION PLAN

### **Step 1: Align WorkloadExecutor Trait** ⚠️

**Issue**: `DistributedCoordinator` doesn't implement `WorkloadExecutor` trait

**Current Trait** (`crates/server/src/tarpc_server.rs:93`):
```rust
#[async_trait::async_trait]
pub trait WorkloadExecutor: Send + Sync + 'static {
    async fn execute(&self, submission: WorkloadSubmission) 
        -> Result<WorkloadResult, String>;
    
    async fn query_capabilities(&self) 
        -> Result<ComputeCapabilities, String>;
    
    async fn cancel(&self, workload_id: &str) 
        -> Result<(), String>;
}
```

**Solution**: Implement `WorkloadExecutor` for `DistributedCoordinator`

**File**: `crates/distributed/src/core/coordinator.rs`

**Changes**:
```rust
#[async_trait::async_trait]
impl toadstool_server::tarpc_server::WorkloadExecutor for DistributedCoordinator {
    async fn execute(&self, submission: WorkloadSubmission) 
        -> Result<WorkloadResult, String> 
    {
        // 1. Check if coordination client available
        if let Some(client) = &self.coordination_client {
            // Distributed mode: Use coordination for load balancing
            self.execute_distributed(client, submission).await
        } else {
            // Standalone mode: Execute locally
            self.standalone_executor.execute(submission).await
        }
    }
    
    async fn query_capabilities(&self) -> Result<ComputeCapabilities, String> {
        let caps = self.capabilities.read().await;
        // Convert ToadStoolCapabilities → ComputeCapabilities
        Ok(caps.into())
    }
    
    async fn cancel(&self, workload_id: &str) -> Result<(), String> {
        // Attempt cancel via coordination, fallback to local
        // ...
    }
}
```

**Type Conversions Needed**:
- `WorkloadSubmission` → `ExecutionRequest`
- `WorkloadResult` → `ExecutionResult`
- `ToadStoolCapabilities` → `ComputeCapabilities`

---

### **Step 2: Add Configuration Support** ⚠️

**File**: `crates/server/src/main.rs`

**Add Environment Variables**:
```bash
# Distributed mode configuration
TOADSTOOL_MODE=standalone|distributed  # Default: standalone
TOADSTOOL_COORDINATION_ENABLED=true|false  # Default: false

# Existing (already implemented)
TOADSTOOL_FAMILY=<family-id>
SONGBIRD_ENDPOINT=<endpoint>
```

**Configuration Structure**:
```rust
struct ServerConfig {
    mode: ServerMode,
    family_id: String,
    socket_path: PathBuf,
    coordination_enabled: bool,
}

enum ServerMode {
    Standalone,  // Single instance (default)
    Distributed, // Multi-instance with coordination
}
```

---

### **Step 3: Modify Server Startup** 🔴

**File**: `crates/server/src/main.rs:main()`

**Current** (Line 25-30):
```rust
// Create executor (workload handler)
info!("Initializing compute executor...");
let executor = create_executor().await?;
```

**New**:
```rust
// Create executor based on configuration
info!("Initializing compute executor...");
let config = ServerConfig::from_env();
let executor = match config.mode {
    ServerMode::Standalone => {
        info!("Starting in STANDALONE mode");
        Arc::new(StandaloneExecutor::new()) as Arc<dyn WorkloadExecutor>
    }
    ServerMode::Distributed => {
        info!("Starting in DISTRIBUTED mode");
        let dist_config = DistributedConfig::from_env()?;
        let coordinator = DistributedCoordinator::new(dist_config).await?;
        Arc::new(coordinator) as Arc<dyn WorkloadExecutor>
    }
};
```

**Benefits**:
- ✅ Backward compatible (default: standalone)
- ✅ Opt-in distributed mode (explicit configuration)
- ✅ Graceful degradation (distributed → standalone if no coordination)

---

### **Step 4: Update Dependencies** 📦

**File**: `crates/server/Cargo.toml`

**Add**:
```toml
[dependencies]
# Existing...
toadstool-distributed = { path = "../distributed" }
```

**Currently**: Not imported (that's why coordinator isn't used)

---

### **Step 5: Type Conversions** 🔄

**Create Adapter Module**: `crates/distributed/src/core/adapters.rs`

**Purpose**: Convert between server and distributed types

```rust
use toadstool_server::tarpc_server::*;
use crate::types::*;

impl From<WorkloadSubmission> for ExecutionRequest {
    fn from(submission: WorkloadSubmission) -> Self {
        ExecutionRequest {
            workload_id: submission.workload_id,
            workload_type: submission.workload_type,
            data: submission.data,
            // Map remaining fields...
        }
    }
}

impl From<ExecutionResult> for WorkloadResult {
    fn from(result: ExecutionResult) -> Self {
        WorkloadResult {
            workload_id: result.execution_id.to_string(),
            status: result.status.into(),
            data: result.output,
            // Map remaining fields...
        }
    }
}
```

---

## 🔬 TESTING STRATEGY

### **Unit Tests** ✅

1. **Standalone Mode**
   ```rust
   #[tokio::test]
   async fn test_standalone_mode_execution() {
       let config = ServerConfig {
           mode: ServerMode::Standalone,
           // ...
       };
       let executor = create_executor_from_config(config).await;
       let result = executor.execute(submission).await;
       assert!(result.is_ok());
   }
   ```

2. **Distributed Mode (No Coordination)**
   ```rust
   #[tokio::test]
   async fn test_distributed_fallback_to_standalone() {
       // Coordination unavailable
       let config = DistributedConfig {
           songbird_integration: None,
           // ...
       };
       let coordinator = DistributedCoordinator::new(config).await?;
       // Should execute locally
   }
   ```

3. **Type Conversions**
   ```rust
   #[test]
   fn test_workload_submission_conversion() {
       let submission = WorkloadSubmission { /* ... */ };
       let request: ExecutionRequest = submission.into();
       assert_eq!(request.workload_id, "test-id");
   }
   ```

### **Integration Tests** 🔄

1. **Single Instance (Standalone)**
   - Start server with `TOADSTOOL_MODE=standalone`
   - Submit workload via tarpc
   - Verify local execution

2. **Multi-Instance (Distributed - Simulated)**
   - Start 2 instances with same `SONGBIRD_ENDPOINT`
   - Instance 1 submits workload
   - Instance 2 executes (load balanced)

3. **Graceful Degradation**
   - Start with `TOADSTOOL_MODE=distributed`
   - Coordination unavailable
   - Verify fallback to standalone

---

## 📊 ROLLOUT PLAN

### **Phase 1: Development** (Week 1)
- [ ] Implement `WorkloadExecutor` for `DistributedCoordinator`
- [ ] Create type conversion adapters
- [ ] Add configuration support
- [ ] Unit tests

### **Phase 2: Integration** (Week 2)
- [ ] Wire to server daemon
- [ ] Integration tests
- [ ] Documentation
- [ ] Example configurations

### **Phase 3: Validation** (Week 3)
- [ ] Multi-instance testing
- [ ] Load balancing verification
- [ ] Fault tolerance testing
- [ ] Performance benchmarks

### **Phase 4: Production** (Week 4)
- [ ] Documentation updates
- [ ] Deployment guides
- [ ] Migration path from standalone
- [ ] Release notes

---

## 🎯 SUCCESS CRITERIA

### **Functional** ✅
- [x] Server starts in standalone mode (default)
- [ ] Server starts in distributed mode (opt-in)
- [ ] Standalone mode works as before (backward compatible)
- [ ] Distributed mode discovers peers via Songbird
- [ ] Workloads execute on appropriate instance (load balancing)
- [ ] Graceful fallback when coordination unavailable

### **Non-Functional** ✅
- [ ] Zero breaking changes
- [ ] Performance: <5% overhead for distributed mode
- [ ] Latency: <10ms additional for coordination overhead
- [ ] Test coverage: 85%+
- [ ] Documentation: Complete migration guide

---

## 🔐 DEEP DEBT COMPLIANCE

### **Principles** ✅
- ✅ **No Hardcoding**: Configuration via environment variables
- ✅ **Self-Knowledge**: Only local capabilities queried
- ✅ **Runtime Discovery**: Coordination discovered via Songbird
- ✅ **Graceful Degradation**: Falls back to standalone
- ✅ **Backward Compatible**: Default mode unchanged

---

## 📝 NEXT STEPS

### **Immediate** (Today)
1. Create type adapter module
2. Implement `WorkloadExecutor` trait
3. Add unit tests for adapters

### **This Week**
1. Wire to server daemon
2. Add configuration support
3. Integration tests

### **Next Week**
1. Multi-instance testing
2. Documentation
3. Production readiness verification

---

## 🏆 EXPECTED IMPACT

### **Before** (Current State)
```
ToadStool Server
  └─ StandaloneExecutor
      └─ Single instance only
      └─ No coordination
      └─ No load balancing
```

### **After** (Phase 1 Complete)
```
ToadStool Server (Backward Compatible)
  ├─ Mode: Standalone (DEFAULT)
  │   └─ StandaloneExecutor
  │       └─ Works as before
  │
  └─ Mode: Distributed (OPT-IN)
      └─ DistributedCoordinator
          ├─ Discovers peers via Songbird
          ├─ Load balances workloads
          └─ Graceful fallback to standalone
```

### **Benefits**
- ✅ **Horizontal Scaling**: Multi-instance support
- ✅ **Load Balancing**: Workload distribution
- ✅ **Fault Tolerance**: Peer discovery and failover
- ✅ **Backward Compatible**: Zero breaking changes
- ✅ **Deep Debt Compliant**: All principles satisfied

---

**Status**: 📋 **READY TO EXECUTE**  
**Estimated Time**: 2-3 weeks  
**Risk**: **LOW** (incremental, backward compatible)

---

*Distributed. Coordinated. Production ready.* 🍄🐸

