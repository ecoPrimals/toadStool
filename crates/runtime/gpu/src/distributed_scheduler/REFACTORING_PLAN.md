# Distributed Scheduler Smart Refactoring Plan

## Current State
- **File**: `distributed_scheduler.rs`
- **Size**: 1,250 lines (250 over 1000-line limit)
- **Status**: Monolithic but well-organized

## Smart Refactoring Strategy

### Principle: Domain-Driven, High Cohesion

This is NOT an arbitrary split - it's based on clear domain boundaries and responsibilities.

### Proposed Structure

```
distributed_scheduler/
├── mod.rs              (~200 lines) - Core types, main coordinator
├── partition.rs        (~300 lines) - Partition strategies
├── execution.rs        (~350 lines) - Execution engines  
├── tower.rs            (~250 lines) - Tower selection & management
└── tracking.rs         (~150 lines) - Job tracking & statistics
```

## Module Breakdown

### 1. `mod.rs` - Core Types & Coordinator (~200 lines)

**Responsibility**: Public API, core types, delegation

```rust
//! Distributed GPU Scheduler - Multi-Tower Coordination
//!
//! Smart refactored for high cohesion and clean interfaces

pub mod partition;
pub mod execution;
pub mod tower;
pub mod tracking;

// Re-export public types
pub use partition::PartitionStrategy;
pub use tower::RemoteTowerEndpoint;
pub use tracking::{DistributedJobState, JobStatus, DistributedSchedulerStats};

use crate::scheduler::UniversalComputeScheduler;
use crate::universal::{ComputeRequirements, UniversalWorkload, WorkloadResult};
use std::collections::HashMap;
use std::sync::Arc;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Distributed GPU scheduler - main coordinator
pub struct DistributedGpuScheduler {
    local_scheduler: Arc<UniversalComputeScheduler>,
    remote_towers: Arc<RwLock<Vec<RemoteTowerEndpoint>>>,
    job_tracker: Arc<RwLock<HashMap<String, DistributedJobState>>>,
    tower_id: String,
    
    // Delegate to specialized modules
    tower_manager: Arc<tower::TowerManager>,
    execution_engine: Arc<execution::ExecutionEngine>,
    job_tracker_impl: Arc<tracking::JobTracker>,
}

impl DistributedGpuScheduler {
    /// Main execution entry point - delegates to strategies
    pub async fn execute_distributed(
        &self,
        workload: UniversalWorkload,
        strategy: PartitionStrategy,
    ) -> ToadStoolResult<WorkloadResult> {
        // Delegate to execution engine
        self.execution_engine.execute(workload, strategy).await
    }
    
    // Other delegation methods...
}
```

**Why**: This becomes the thin coordinator that delegates to specialized modules.

---

### 2. `partition.rs` - Partition Strategies (~300 lines)

**Responsibility**: All partitioning logic and strategies

```rust
//! Workload Partitioning Strategies
//!
//! Implements different ways to split workloads across towers

use crate::universal::{UniversalWorkload, WorkloadBuffer};
use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Partitioning strategies for distributed execution
#[derive(Debug, Clone)]
pub enum PartitionStrategy {
    /// Single tower (no partitioning)
    Single,
    
    /// Replicate and race (use fastest)
    Redundant { replicas: usize },
    
    /// Split by data chunks
    DataParallel { chunk_size: usize },
    
    /// Pipeline stages across towers
    Pipeline { stages: Vec<String> },
}

/// Partitioning engine
pub struct Partitioner;

impl Partitioner {
    /// Partition inputs for data parallel execution
    pub fn partition_inputs(
        &self,
        inputs: &[WorkloadBuffer],
        chunk_size: usize,
        num_towers: usize,
    ) -> ToadStoolResult<Vec<Vec<WorkloadBuffer>>> {
        // Implementation of data partitioning logic
        // ~100 lines of chunking logic
    }
    
    /// Partition pipeline stages
    pub fn partition_pipeline(
        &self,
        workload: &UniversalWorkload,
        stages: &[String],
    ) -> ToadStoolResult<Vec<UniversalWorkload>> {
        // Implementation of pipeline partitioning
        // ~100 lines of stage separation logic
    }
    
    /// Validate partition strategy for workload
    pub fn validate_strategy(
        &self,
        workload: &UniversalWorkload,
        strategy: &PartitionStrategy,
    ) -> ToadStoolResult<()> {
        // Validation logic
        // ~50 lines
    }
}
```

**Why**: Partitioning is a distinct domain with complex logic that deserves its own module.

---

### 3. `execution.rs` - Execution Engine (~350 lines)

**Responsibility**: Actual execution of workloads on towers

```rust
//! Execution Engine for Distributed Workloads
//!
//! Handles the actual execution across local and remote towers

use crate::scheduler::UniversalComputeScheduler;
use crate::universal::{UniversalWorkload, WorkloadResult};
use toadstool::error::{ToadStoolError, ToadStoolResult};
use std::sync::Arc;

/// Execution engine for distributed workloads
pub struct ExecutionEngine {
    local_scheduler: Arc<UniversalComputeScheduler>,
    tower_id: String,
}

impl ExecutionEngine {
    /// Execute on single tower (local or remote)
    pub async fn execute_single(
        &self,
        tower_id: String,
        workload: UniversalWorkload,
    ) -> ToadStoolResult<WorkloadResult> {
        if tower_id == self.tower_id {
            self.execute_local(workload).await
        } else {
            self.execute_remote(tower_id, workload).await
        }
    }
    
    /// Execute redundantly (race multiple towers)
    pub async fn execute_redundant(
        &self,
        tower_ids: Vec<String>,
        workload: UniversalWorkload,
    ) -> ToadStoolResult<WorkloadResult> {
        // ~80 lines of concurrent execution + race logic
    }
    
    /// Execute with data parallelism
    pub async fn execute_data_parallel(
        &self,
        tower_chunks: Vec<(String, UniversalWorkload)>,
    ) -> ToadStoolResult<WorkloadResult> {
        // ~100 lines of parallel execution + aggregation
    }
    
    /// Execute as pipeline
    pub async fn execute_pipeline(
        &self,
        tower_stages: Vec<(String, UniversalWorkload)>,
    ) -> ToadStoolResult<WorkloadResult> {
        // ~80 lines of staged execution
    }
    
    /// Execute locally
    async fn execute_local(
        &self,
        workload: UniversalWorkload,
    ) -> ToadStoolResult<WorkloadResult> {
        self.local_scheduler.execute(workload).await
    }
    
    /// Execute on remote tower
    async fn execute_remote(
        &self,
        tower_id: String,
        workload: UniversalWorkload,
    ) -> ToadStoolResult<WorkloadResult> {
        // ~50 lines of HTTP/gRPC communication
    }
}
```

**Why**: Execution is the core responsibility - needs to be clearly separated and testable.

---

### 4. `tower.rs` - Tower Management (~250 lines)

**Responsibility**: Tower discovery, selection, and management

```rust
//! Tower Management and Selection
//!
//! Handles discovery and selection of compute towers

use crate::universal::{ComputeRequirements, ComputeCapabilities};
use toadstool::error::{ToadStoolError, ToadStoolResult};
use std::time::Instant;

/// Remote tower endpoint
#[derive(Debug, Clone)]
pub struct RemoteTowerEndpoint {
    pub tower_id: String,
    pub address: String,
    pub gpu_capabilities: Option<ComputeCapabilities>,
    pub last_seen: Instant,
    pub latency_ms: u64,
}

/// Tower selection and management
pub struct TowerManager {
    local_tower_id: String,
    remote_towers: Vec<RemoteTowerEndpoint>,
}

impl TowerManager {
    /// Select best single tower for requirements
    pub async fn select_best_tower(
        &self,
        requirements: &ComputeRequirements,
    ) -> ToadStoolResult<String> {
        // ~60 lines of selection logic
        // - Capability matching
        // - Latency optimization
        // - Load balancing
    }
    
    /// Select multiple towers for parallel execution
    pub async fn select_multiple_towers(
        &self,
        requirements: &ComputeRequirements,
        count: usize,
    ) -> ToadStoolResult<Vec<String>> {
        // ~60 lines of multi-tower selection
    }
    
    /// Register remote tower
    pub async fn register_tower(&mut self, endpoint: RemoteTowerEndpoint) {
        // ~30 lines
    }
    
    /// Remove stale towers
    pub async fn prune_stale_towers(&mut self, max_age_secs: u64) {
        // ~20 lines
    }
    
    /// Get all available towers
    pub fn available_towers(&self) -> Vec<String> {
        // ~10 lines
    }
}
```

**Why**: Tower management is a distinct concern with clear boundaries.

---

### 5. `tracking.rs` - Job Tracking & Statistics (~150 lines)

**Responsibility**: Job state tracking and statistics

```rust
//! Job Tracking and Statistics
//!
//! Tracks distributed job state and provides observability

use std::time::Instant;
use crate::universal::{UniversalWorkload, WorkloadResult};

/// Job status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Scheduled,
    Running,
    Completed,
    Failed,
}

/// Distributed job state
#[derive(Debug, Clone)]
pub struct DistributedJobState {
    pub job_id: String,
    pub workload: UniversalWorkload,
    pub status: JobStatus,
    pub assigned_tower: Option<String>,
    pub result: Option<WorkloadResult>,
    pub created_at: Instant,
    pub completed_at: Option<Instant>,
}

/// Job tracker
pub struct JobTracker {
    jobs: std::collections::HashMap<String, DistributedJobState>,
}

impl JobTracker {
    /// Get job by ID
    pub fn get_job(&self, job_id: &str) -> Option<&DistributedJobState> {
        self.jobs.get(job_id)
    }
    
    /// Get jobs by status
    pub fn get_jobs_by_status(&self, status: JobStatus) -> Vec<&DistributedJobState> {
        self.jobs.values().filter(|j| j.status == status).collect()
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> DistributedSchedulerStats {
        // ~40 lines of statistics aggregation
    }
}

/// Statistics
#[derive(Debug, Clone)]
pub struct DistributedSchedulerStats {
    pub total_jobs: usize,
    pub active_jobs: usize,
    pub completed_jobs: usize,
    pub failed_jobs: usize,
    pub remote_towers_available: usize,
    pub average_execution_time_ms: f64,
    pub success_rate: f64,
}
```

**Why**: Job tracking is orthogonal to execution - clean separation.

---

## Migration Plan

### Phase 1: Create Modules (1 hour)
1. Create directory: `distributed_scheduler/`
2. Create empty module files
3. Add mod declarations

### Phase 2: Extract Types (30 minutes)
1. Move shared types to appropriate modules
2. Add re-exports in `mod.rs`
3. Ensure compilation

### Phase 3: Extract Tower Management (30 minutes)
1. Move `RemoteTowerEndpoint` to `tower.rs`
2. Create `TowerManager` struct
3. Move selection methods

### Phase 4: Extract Partitioning (45 minutes)
1. Move `PartitionStrategy` to `partition.rs`
2. Create `Partitioner` struct
3. Move partitioning logic

### Phase 5: Extract Execution (1 hour)
1. Create `ExecutionEngine` in `execution.rs`
2. Move execution methods
3. Wire up delegation

### Phase 6: Extract Tracking (30 minutes)
1. Move job tracking types to `tracking.rs`
2. Create `JobTracker` struct
3. Move statistics methods

### Phase 7: Update Main Coordinator (30 minutes)
1. Update `DistributedGpuScheduler` to delegate
2. Clean up `mod.rs`
3. Update tests

### Phase 8: Testing (30 minutes)
1. Run all tests
2. Fix any integration issues
3. Verify behavior unchanged

**Total Time**: ~5 hours

---

## Benefits of This Approach

### 1. High Cohesion
- Each module has a single, clear responsibility
- Related functionality stays together
- Easy to understand and maintain

### 2. Low Coupling
- Clean interfaces between modules
- Minimal cross-module dependencies
- Easy to test in isolation

### 3. Extensibility
- Easy to add new partition strategies (just extend `partition.rs`)
- Easy to add new tower selection algorithms (just extend `tower.rs`)
- Easy to add execution methods (just extend `execution.rs`)

### 4. Testability
- Each module can be tested independently
- Mocking is straightforward
- Integration tests remain the same

### 5. Maintainability
- New developers can quickly understand each module
- Changes are localized to relevant modules
- Refactoring is safer

---

## Anti-Pattern Avoided

**What NOT to do**: Arbitrary line-count splits

```
// BAD: Split by arbitrary line counts
distributed_scheduler/
├── part1.rs  (lines 1-300)
├── part2.rs  (lines 301-600)
├── part3.rs  (lines 601-900)
└── part4.rs  (lines 901-1250)
```

**Why bad**: 
- Breaks logical boundaries
- Low cohesion (unrelated code together)
- High coupling (lots of cross-file references)
- Harder to understand and maintain

---

## Execution Status

- **Status**: PLANNED (not yet executed)
- **Priority**: P1 (quality issue, not blocking)
- **Time Estimate**: 5 hours
- **Dependencies**: None (can be done independently)

**Recommendation**: Execute this refactoring in dedicated session when no critical blockers remain.

---

**Document Created**: December 19, 2025  
**Purpose**: Guide for smart, domain-driven refactoring  
**Principle**: High cohesion, low coupling, clear responsibilities

