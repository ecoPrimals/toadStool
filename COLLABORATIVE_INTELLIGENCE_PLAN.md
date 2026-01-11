# ToadStool Collaborative Intelligence Integration Plan

**Date**: January 11, 2026  
**Priority**: Medium (biomeOS requested)  
**Timeline**: 2 weeks  
**Status**: Planning → Implementation

---

## 🎯 Overview

biomeOS team has requested ToadStool support for **Collaborative Intelligence** - enabling human-AI collaboration through interactive graph execution and real-time resource planning.

### What ToadStool Needs to Provide

**3 New JSON-RPC Methods** for resource planning:
1. `resources.estimate(graph)` → Estimate resource needs for graph execution
2. `resources.validate_availability(graph)` → Check if resources available
3. `resources.suggest_optimizations(graph)` → Suggest resource improvements

**Priority**: Low (nice to have) but critical for ecosystem integration
**Timeline**: 2 weeks

---

## 📊 Current ToadStool Capabilities

### ✅ Already Have

1. **Resource Tracking**
   - `toadstool::resources::ResourceRequirements`
   - CPU, memory, storage, GPU, network requirements
   - Comprehensive type system

2. **System Resource Query**
   - `StandaloneExecutor` queries real system resources
   - CPU cores, memory (total/available)
   - GPU detection (NVIDIA, AMD via Songbird client)

3. **JSON-RPC Server**
   - `ManualJsonRpcServer` (pure Rust, Unix sockets)
   - Existing methods: health, version, query_capabilities
   - Easy to extend with new methods

4. **Distributed Coordination**
   - `DistributedCoordinator` for multi-instance coordination
   - `CoordinatorExecutor` wrapper
   - Capability-based resource allocation

### ⚠️ Need to Add

1. **Graph Types**
   - Define graph structure (nodes, edges, dependencies)
   - Map biomeOS graph format to ToadStool types

2. **Resource Estimation Logic**
   - Estimate total resources for graph execution
   - Consider parallelism and dependencies
   - Account for overhead and coordination

3. **Availability Validation**
   - Check current system capacity
   - Validate against available resources
   - Provide detailed feedback on what's missing

4. **Optimization Suggestions**
   - Suggest parallelization opportunities
   - Recommend resource adjustments
   - Identify bottlenecks

---

## 🏗️ Implementation Plan

### Phase 1: Define Graph Types (3 hours)

**File**: `crates/server/src/graph_types.rs`

```rust
/// Graph node representing a primal workload
pub struct GraphNode {
    pub id: String,
    pub primal: String,  // e.g., "toadstool", "squirrel", "nestgate"
    pub operation: String,  // e.g., "compute", "storage", "intelligence"
    pub requirements: ResourceRequirements,
    pub metadata: HashMap<String, String>,
}

/// Graph representing a complete workflow
pub struct ExecutionGraph {
    pub id: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,  // Dependencies between nodes
    pub metadata: HashMap<String, String>,
}

/// Dependency edge between nodes
pub struct GraphEdge {
    pub from: String,  // source node_id
    pub to: String,    // target node_id
    pub edge_type: EdgeType,
}

/// Resource estimate for graph execution
pub struct GraphResourceEstimate {
    pub total_cpu_cores: f64,
    pub total_memory_bytes: u64,
    pub total_storage_bytes: u64,
    pub total_gpu_count: u32,
    pub estimated_duration_secs: Option<f64>,
    pub parallelism_factor: f64,  // How much can run in parallel
    pub per_node_estimates: HashMap<String, NodeResourceEstimate>,
}
```

**Deep Debt Principles**:
- ✅ No hardcoding (generic graph structure)
- ✅ Self-knowledge (ToadStool reports own capabilities)
- ✅ Capability-based (estimates based on discovered resources)

### Phase 2: Resource Estimation Logic (6 hours)

**File**: `crates/server/src/resource_estimator.rs`

**Key Functions**:
1. `estimate_graph_resources(graph)` - Estimate total resource needs
2. `calculate_parallelism(graph)` - Analyze dependency graph for parallelism
3. `estimate_node_resources(node)` - Estimate single node requirements
4. `aggregate_resources(estimates)` - Combine node estimates considering parallelism

**Algorithm**:
```rust
// 1. Analyze graph topology
let topology = analyze_dependencies(&graph);

// 2. Identify parallel stages
let stages = identify_parallel_stages(&topology);

// 3. Estimate per-node resources
let node_estimates = graph.nodes.iter()
    .map(|n| estimate_node_resources(n))
    .collect();

// 4. Aggregate considering parallelism
let total = aggregate_by_stages(&stages, &node_estimates);

// 5. Add coordination overhead (10-20%)
let overhead_factor = 1.15;
total * overhead_factor
```

**Deep Debt Principles**:
- ✅ Graceful degradation (provide estimates even with missing info)
- ✅ Error handling (Result<T, E> throughout)
- ✅ No unwrap() in production code

### Phase 3: Availability Validation (4 hours)

**File**: `crates/server/src/resource_validator.rs`

**Key Functions**:
1. `validate_availability(graph, capabilities)` - Check if resources available
2. `check_cpu_availability(required, available)` - CPU validation
3. `check_memory_availability(required, available)` - Memory validation
4. `check_gpu_availability(required, available)` - GPU validation

**Logic**:
```rust
pub struct AvailabilityResult {
    pub available: bool,
    pub missing_resources: Vec<ResourceGap>,
    pub warnings: Vec<String>,
    pub confidence: f64,  // 0.0-1.0 confidence in estimate
}

pub struct ResourceGap {
    pub resource_type: String,
    pub required: f64,
    pub available: f64,
    pub shortfall: f64,
}
```

**Deep Debt Principles**:
- ✅ Self-knowledge (query real system state)
- ✅ Transparent (explain what's missing)
- ✅ Graceful (provide partial validation results)

### Phase 4: Optimization Suggestions (5 hours)

**File**: `crates/server/src/resource_optimizer.rs`

**Key Functions**:
1. `suggest_optimizations(graph, estimate)` - Generate optimization suggestions
2. `identify_bottlenecks(graph)` - Find resource bottlenecks
3. `suggest_parallelization(graph)` - Suggest parallel execution opportunities
4. `suggest_resource_adjustments(graph)` - Recommend resource changes

**Suggestions**:
```rust
pub struct OptimizationSuggestion {
    pub suggestion_type: SuggestionType,
    pub priority: Priority,
    pub description: String,
    pub expected_improvement: f64,  // % improvement estimate
    pub confidence: f64,
}

pub enum SuggestionType {
    IncreaseParallelism,
    ReduceMemory,
    AddGpu,
    ReorderNodes,
    SplitWorkload,
}
```

**Deep Debt Principles**:
- ✅ Transparent reasoning (explain why suggested)
- ✅ Confidence scores (AI learns from feedback)
- ✅ Non-prescriptive (suggestions, not commands)

### Phase 5: JSON-RPC Integration (2 hours)

**File**: `crates/server/src/manual_jsonrpc.rs` (extend)

**Add 3 Methods**:
```rust
// Method 1: Estimate resources
match request.method.as_str() {
    "resources.estimate" => self.handle_estimate_resources(request).await,
    "resources.validate_availability" => self.handle_validate_availability(request).await,
    "resources.suggest_optimizations" => self.handle_suggest_optimizations(request).await,
    // ... existing methods
}
```

**Deep Debt Principles**:
- ✅ Unix sockets (no TCP hardcoding)
- ✅ Graceful errors (JSON-RPC error responses)
- ✅ Type-safe (serde validation)

---

## 🧪 Testing Strategy

### Unit Tests (8 hours)

1. **Graph Types Tests**
   - Graph parsing
   - Edge validation
   - Cycle detection

2. **Estimation Tests**
   - Simple graphs (1-3 nodes)
   - Complex graphs (10+ nodes)
   - Parallel graphs
   - Sequential graphs

3. **Validation Tests**
   - Sufficient resources
   - Insufficient resources
   - Partial availability

4. **Optimization Tests**
   - Bottleneck detection
   - Parallelization suggestions
   - Resource adjustment recommendations

### Integration Tests (4 hours)

1. **JSON-RPC E2E Tests**
   - Full request/response cycle
   - Error handling
   - Invalid graphs

2. **Cross-Primal Tests**
   - Mock biomeOS graph
   - Multi-primal coordination
   - Resource pooling

---

## 📋 Implementation Checklist

### Week 1 (Core Implementation)

**Day 1-2: Graph Types + Estimation**
- [ ] Create `graph_types.rs` with all types
- [ ] Create `resource_estimator.rs`
- [ ] Implement graph parsing
- [ ] Implement basic estimation logic
- [ ] Unit tests for graph types
- [ ] Unit tests for estimation

**Day 3-4: Validation + Optimization**
- [ ] Create `resource_validator.rs`
- [ ] Implement availability checks
- [ ] Create `resource_optimizer.rs`
- [ ] Implement optimization suggestions
- [ ] Unit tests for validation
- [ ] Unit tests for optimization

**Day 5: JSON-RPC Integration**
- [ ] Extend `ManualJsonRpcServer`
- [ ] Add 3 new methods
- [ ] Integration tests
- [ ] Error handling

### Week 2 (Testing + Documentation)

**Day 6-7: Comprehensive Testing**
- [ ] E2E JSON-RPC tests
- [ ] Complex graph scenarios
- [ ] Error case coverage
- [ ] Performance testing

**Day 8-9: Documentation**
- [ ] API documentation
- [ ] Usage examples
- [ ] Integration guide for biomeOS
- [ ] Update DOCUMENTATION_INDEX.md

**Day 10: Polish + Handoff**
- [ ] Code review
- [ ] Linter/fmt check
- [ ] Final testing
- [ ] Create handoff document for biomeOS

---

## 🎯 Success Criteria

### Minimum Viable Product (MVP)

1. ✅ `resources.estimate(graph)` returns reasonable estimates
2. ✅ `resources.validate_availability(graph)` checks real system capacity
3. ✅ `resources.suggest_optimizations(graph)` provides 1-3 actionable suggestions
4. ✅ All methods work via JSON-RPC over Unix sockets
5. ✅ Graceful error handling for invalid graphs
6. ✅ 80%+ test coverage for new code

### Full Feature Set

7. ✅ Parallelism analysis (identify parallel execution opportunities)
8. ✅ Confidence scores (AI learning feedback loop)
9. ✅ Detailed per-node breakdowns
10. ✅ Resource optimization suggestions with expected improvements
11. ✅ Cross-GPU resource pooling support
12. ✅ Comprehensive documentation

---

## 🤝 Integration with biomeOS

### Data Flow

```
User (petalTongue) →
    Creates/modifies graph →
        biomeOS sends to ToadStool →
            resources.estimate(graph) →
                Returns ResourceEstimate →
                    User sees resource needs →
                        resources.validate_availability(graph) →
                            Returns AvailabilityResult →
                                User sees if resources sufficient →
                                    resources.suggest_optimizations(graph) →
                                        Returns OptimizationSuggestions →
                                            User adjusts graph →
                                                Deploys (execute workload)
```

### Example JSON-RPC Call

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "resources.estimate",
  "params": {
    "graph": {
      "id": "workflow-123",
      "nodes": [
        {
          "id": "node-1",
          "primal": "toadstool",
          "operation": "gpu_compute",
          "requirements": {
            "cpu_cores": 4,
            "memory_bytes": 8589934592,
            "gpu_count": 1
          }
        }
      ],
      "edges": []
    }
  },
  "id": 1
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "total_cpu_cores": 4.0,
    "total_memory_bytes": 8589934592,
    "total_gpu_count": 1,
    "estimated_duration_secs": 120.0,
    "parallelism_factor": 1.0,
    "per_node_estimates": {
      "node-1": {
        "cpu_cores": 4.0,
        "memory_bytes": 8589934592,
        "gpu_count": 1,
        "estimated_duration_secs": 120.0
      }
    }
  },
  "id": 1
}
```

---

## 💡 Deep Debt Compliance

### All Principles Met

✅ **No Hardcoding** - Graph structure generic, no primal names hardcoded  
✅ **Agnostic Discovery** - Discovers available resources at runtime  
✅ **Self-Knowledge Only** - ToadStool reports own capabilities  
✅ **Runtime Discovery** - Queries system state dynamically  
✅ **Capability-Based** - Estimates based on discovered capabilities  
✅ **Modern Idiomatic Rust** - Result<T, E>, no unwrap(), type-safe  
✅ **Graceful Degradation** - Provides partial results when possible  
✅ **Zero-Copy** - Arc, references where possible  
✅ **Unix Sockets** - ManualJsonRpcServer, no TCP hardcoding  
✅ **Comprehensive Tests** - Unit + integration + E2E

### Grade Impact

**Current**: A+ (95/100)  
**After Implementation**: A+ (97/100)  
**Improvement**: +2 points for ecosystem integration

---

## 📚 Resources

### Internal References

- `crates/server/src/manual_jsonrpc.rs` - Extend this
- `crates/core/toadstool/src/resources.rs` - Use these types
- `crates/server/src/tarpc_server.rs` - Reference StandaloneExecutor
- `crates/server/src/coordinator_executor.rs` - Distributed coordination

### External References

- biomeOS Collaborative Intelligence Spec
- JSON-RPC 2.0 Specification
- Graph theory (topological sort, parallelism analysis)

---

## 🚀 Next Steps

1. **Review this plan** - Ensure alignment with biomeOS requirements
2. **Create branch** - `feature/collaborative-intelligence`
3. **Start Phase 1** - Graph types implementation
4. **Weekly sync** - Wednesdays, 2pm UTC (biomeOS + ToadStool)
5. **Integration test** - Week 2, coordinate with biomeOS

**Status**: ✅ Plan Complete, Ready to Start  
**Timeline**: 2 weeks  
**Confidence**: High (building on existing capabilities)

---

Different orders of the same architecture. 🍄🐸

**ToadStool Team - January 11, 2026**

