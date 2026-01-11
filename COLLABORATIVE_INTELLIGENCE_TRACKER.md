# Collaborative Intelligence Integration - Tracker

**Initiative**: biomeOS Collaborative Intelligence  
**ToadStool Component**: Resource Planning API  
**Start Date**: January 11, 2026  
**Target Completion**: January 25, 2026 (2 weeks)  
**Status**: 🟡 Planning → Implementation

---

## Quick Links

- **Spec**: [specs/COLLABORATIVE_INTELLIGENCE_RESOURCE_PLANNING.md](specs/COLLABORATIVE_INTELLIGENCE_RESOURCE_PLANNING.md)
- **Implementation Plan**: [COLLABORATIVE_INTELLIGENCE_PLAN.md](COLLABORATIVE_INTELLIGENCE_PLAN.md)
- **biomeOS Request**: (See "Context" section below)
- **Coordination**: Slack #collaborative-intelligence, Wednesdays 2pm UTC

---

## Overview

biomeOS team requested ToadStool support for their "Collaborative Intelligence" system - enabling human-AI collaboration through interactive graph execution and real-time resource planning.

### ToadStool's Role

Provide **3 JSON-RPC methods** for resource intelligence:
1. `resources.estimate(graph)` - Estimate resource needs
2. `resources.validate_availability(graph)` - Check availability
3. `resources.suggest_optimizations(graph)` - Suggest improvements

**Priority**: Medium (nice to have)  
**Timeline**: 2 weeks (40 hours)  
**Grade Impact**: A+ (95/100) → A+ (97/100) (+2 points)

---

## Progress Tracker

### Week 1: Core Implementation (20 hours)

**Days 1-2: Graph Types + Estimation** (8 hours)
- [ ] Create `crates/server/src/graph_types.rs`
  - [ ] `ExecutionGraph` type
  - [ ] `GraphNode` type
  - [ ] `GraphEdge` type
  - [ ] JSON serialization/deserialization
  - [ ] Graph validation (detect cycles, validate structure)
- [ ] Create `crates/server/src/resource_estimator.rs`
  - [ ] Topological sort implementation
  - [ ] Parallel stage identification
  - [ ] Per-node resource estimation
  - [ ] Resource aggregation logic
  - [ ] Coordination overhead calculation
- [ ] Unit tests for graph parsing
- [ ] Unit tests for estimation logic

**Days 3-4: Validation + Optimization** (10 hours)
- [ ] Create `crates/server/src/resource_validator.rs`
  - [ ] System capacity query integration
  - [ ] Resource comparison logic
  - [ ] Gap calculation
  - [ ] Warning generation (>80% utilization)
- [ ] Create `crates/server/src/resource_optimizer.rs`
  - [ ] Bottleneck detection
  - [ ] Parallelization opportunity analysis
  - [ ] GPU acceleration suggestions
  - [ ] Memory optimization suggestions
  - [ ] Confidence scoring
- [ ] Unit tests for validation
- [ ] Unit tests for optimization

**Day 5: JSON-RPC Integration** (2 hours)
- [ ] Extend `crates/server/src/manual_jsonrpc.rs`
  - [ ] Add `handle_estimate_resources()` method
  - [ ] Add `handle_validate_availability()` method
  - [ ] Add `handle_suggest_optimizations()` method
  - [ ] Error handling for all methods
- [ ] Update method routing
- [ ] Basic integration test

### Week 2: Testing + Documentation (20 hours)

**Days 6-7: Comprehensive Testing** (8 hours)
- [ ] Complex graph scenarios
  - [ ] Sequential workflows (5+ nodes)
  - [ ] Parallel workflows (10+ nodes)
  - [ ] Mixed topologies
  - [ ] Large graphs (100+ nodes)
- [ ] E2E JSON-RPC tests
  - [ ] Full request/response cycle
  - [ ] Error cases (invalid graphs)
  - [ ] Performance testing (<100ms target)
- [ ] Integration with existing components
  - [ ] `StandaloneExecutor` integration
  - [ ] `DistributedCoordinator` integration
  - [ ] Real system resource queries

**Days 8-9: Documentation** (8 hours)
- [ ] API documentation
  - [ ] Method descriptions
  - [ ] Request/response examples
  - [ ] Error codes
- [ ] Usage examples
  - [ ] Simple graph estimation
  - [ ] Complex workflow validation
  - [ ] Optimization workflow
- [ ] Integration guide for biomeOS
  - [ ] Data format specifications
  - [ ] Example workflows
  - [ ] Troubleshooting guide
- [ ] Update `DOCUMENTATION_INDEX.md`

**Day 10: Polish + Handoff** (4 hours)
- [ ] Code review (self-review checklist)
- [ ] Linter/fmt check (`cargo fmt`, `cargo clippy`)
- [ ] Final testing pass
- [ ] Performance profiling
- [ ] Create handoff document for biomeOS
- [ ] Demo preparation

---

## Implementation Checklist

### Core Components

**Graph Types** (`graph_types.rs`)
- [ ] `ExecutionGraph` struct
- [ ] `GraphNode` struct
- [ ] `GraphEdge` struct
- [ ] `ResourceGap` struct
- [ ] JSON serde implementations
- [ ] Graph validation logic
- [ ] Cycle detection

**Resource Estimator** (`resource_estimator.rs`)
- [ ] Topological sort algorithm
- [ ] Stage identification (parallel analysis)
- [ ] Node resource estimation
- [ ] Resource aggregation (sequential/parallel)
- [ ] Overhead calculation
- [ ] Duration estimation
- [ ] Confidence scoring

**Resource Validator** (`resource_validator.rs`)
- [ ] System capacity query
- [ ] Resource comparison
- [ ] Gap calculation
- [ ] Warning generation
- [ ] Availability result construction

**Resource Optimizer** (`resource_optimizer.rs`)
- [ ] Bottleneck detection
- [ ] Parallelization analysis
- [ ] GPU acceleration detection
- [ ] Memory optimization detection
- [ ] Suggestion generation
- [ ] Priority ranking
- [ ] Confidence scoring

**JSON-RPC Integration** (`manual_jsonrpc.rs`)
- [ ] `resources.estimate` handler
- [ ] `resources.validate_availability` handler
- [ ] `resources.suggest_optimizations` handler
- [ ] Error response formatting
- [ ] Method routing update

### Testing

**Unit Tests**
- [ ] Graph parsing tests (valid/invalid cases)
- [ ] Estimation algorithm tests
- [ ] Validation logic tests
- [ ] Optimization logic tests
- [ ] Edge case handling

**Integration Tests**
- [ ] JSON-RPC E2E tests
- [ ] System resource query tests
- [ ] Multi-component integration
- [ ] Performance tests

**Test Coverage Target**: 80%+

### Documentation

- [ ] API specification (complete)
- [ ] Implementation plan (complete)
- [ ] Usage examples
- [ ] Integration guide
- [ ] Troubleshooting guide
- [ ] Update root docs

---

## Dependencies

### Internal (ToadStool)

✅ **Already Available**:
- `toadstool::resources::ResourceRequirements` - Type system
- `ManualJsonRpcServer` - JSON-RPC server
- `StandaloneExecutor` - System resource queries
- `DistributedCoordinator` - Multi-instance coordination

⚠️ **Need to Create**:
- Graph types and parsing
- Estimation algorithms
- Validation logic
- Optimization logic

### External (Ecosystem)

- **biomeOS**: Graph structure format (needs confirmation)
- **petalTongue**: UI integration (handled by biomeOS)
- **Squirrel**: AI learning from feedback (future)
- **Songbird**: Resource discovery (already integrated)

---

## Integration Points

### biomeOS → ToadStool

**Data Flow**:
1. User creates/modifies graph in petalTongue
2. biomeOS validates graph structure
3. biomeOS calls `resources.estimate(graph)`
4. ToadStool analyzes and returns estimate
5. biomeOS calls `resources.validate_availability(graph)`
6. ToadStool checks system and returns availability
7. biomeOS calls `resources.suggest_optimizations(graph)`
8. ToadStool analyzes and returns suggestions
9. User adjusts graph based on feedback
10. biomeOS deploys graph for execution

**Communication Protocol**: JSON-RPC 2.0 over Unix sockets

**Socket Path**: `$XDG_RUNTIME_DIR/toadstool-$TOADSTOOL_FAMILY.jsonrpc.sock`

### ToadStool → Other Primals

ToadStool needs to understand which primals are involved in the graph:
- **Validation**: Check if target primals are available (via Songbird)
- **Estimation**: Consider inter-primal communication overhead
- **Optimization**: Suggest primal substitutions if appropriate

**Note**: Initial implementation focuses on ToadStool-only graphs. Multi-primal support is future enhancement.

---

## Testing Strategy

### Test Scenarios

**Scenario 1: Simple Sequential Workflow**
- 3 nodes, sequential (A → B → C)
- All ToadStool CPU compute
- Verify: Sequential resource aggregation

**Scenario 2: Parallel Workflow**
- 5 nodes, 2 parallel stages (A,B → C → D,E)
- Mixed CPU/GPU compute
- Verify: Parallel resource optimization, MAX not SUM

**Scenario 3: Complex DAG**
- 20 nodes, complex dependencies
- Multiple parallel opportunities
- Verify: Correct topological sort, optimal parallelization

**Scenario 4: Resource Constrained**
- Graph requires 32 cores, 64GB RAM, 2 GPUs
- System has 16 cores, 32GB RAM, 1 GPU
- Verify: Correct gap reporting, actionable warnings

**Scenario 5: Optimization Opportunities**
- CPU-heavy node that could use GPU
- Over-allocated memory
- Sequential nodes that could be parallel
- Verify: Relevant suggestions with good confidence

### Performance Targets

- Estimation: <50ms for <50 nodes, <200ms for <200 nodes
- Validation: <30ms (includes system query)
- Optimization: <100ms for <50 nodes, <500ms for <200 nodes

---

## Success Criteria

### Minimum Viable Product (MVP)

1. ✅ All 3 methods implemented and working
2. ✅ JSON-RPC integration complete
3. ✅ Basic graph parsing and validation
4. ✅ Resource estimation for sequential workflows
5. ✅ Availability validation against real system
6. ✅ At least 1-2 optimization suggestions
7. ✅ Graceful error handling
8. ✅ Unit tests passing
9. ✅ Integration tests passing
10. ✅ Documentation complete

### Full Feature Set

11. ✅ Parallelism analysis (identify parallel stages)
12. ✅ Confidence scoring for all estimates
13. ✅ Per-node resource breakdowns
14. ✅ Multiple optimization types (GPU, memory, parallelization)
15. ✅ Performance meets targets
16. ✅ 80%+ test coverage
17. ✅ E2E tests with biomeOS integration
18. ✅ Comprehensive troubleshooting guide

---

## Risk Management

### Technical Risks

**Risk**: Graph complexity explosion (large DAGs)  
**Mitigation**: Implement size limits (1000 nodes max), timeout protection  
**Status**: Addressed in spec (resource limits section)

**Risk**: Inaccurate resource estimates  
**Mitigation**: Confidence scoring, conservative estimates, user feedback loop  
**Status**: Confidence scoring planned

**Risk**: System capacity queries are slow  
**Mitigation**: Caching (5 second TTL), async queries  
**Status**: Caching strategy defined

### Integration Risks

**Risk**: Graph format mismatch with biomeOS  
**Mitigation**: Early validation with biomeOS team, flexible parsing  
**Status**: Needs confirmation with biomeOS

**Risk**: Performance insufficient for large graphs  
**Mitigation**: Performance testing, optimization, progressive enhancement  
**Status**: Performance targets defined

### Schedule Risks

**Risk**: Estimation algorithm more complex than expected  
**Mitigation**: Start with simple heuristics, iterate  
**Contingency**: +3 days buffer in timeline

**Risk**: Integration issues with existing components  
**Mitigation**: Early integration testing, modular design  
**Contingency**: Fallback to simpler implementations

---

## Open Questions

### For biomeOS Team

1. **Graph Format**: Exact JSON schema for ExecutionGraph?
   - Status: Using ToadStool-defined format, needs validation
   
2. **Metadata Fields**: What metadata fields are available in nodes?
   - Status: Assumed generic HashMap, needs confirmation
   
3. **Multi-Primal Support**: Should initial version support multi-primal graphs?
   - Status: Deferred to future, starting with ToadStool-only
   
4. **Confidence Scores**: How will AI learn from confidence scores?
   - Status: API provides scores, learning is Squirrel's responsibility
   
5. **Update Frequency**: How often are graphs re-estimated during editing?
   - Status: On-demand via JSON-RPC, no automatic re-estimation

### For ToadStool Team

1. **Caching Strategy**: Cache system capacity queries?
   - Decision: Yes, 5 second TTL (defined in spec)
   
2. **GPU Detection**: Use existing Songbird GPU detection?
   - Decision: Yes, integrate with existing system
   
3. **Distributed Coordination**: Consider multi-instance ToadStool?
   - Decision: Initial version single-instance, future enhancement

---

## Communication

### Weekly Sync

**When**: Wednesdays, 2pm UTC  
**Who**: biomeOS team + ToadStool team  
**Agenda**:
- Progress update
- Blockers discussion
- Integration questions
- Next week planning

### Async Communication

**Slack**: #collaborative-intelligence  
**GitHub Issues**: Tag with `collaborative-intelligence`  
**Questions**: @biomeos-team or @toadstool-team

---

## Timeline Milestones

| Date | Milestone | Status |
|------|-----------|--------|
| 2026-01-11 | Plan created | ✅ Done |
| 2026-01-13 | Graph types complete | 🟡 Pending |
| 2026-01-15 | Estimation logic complete | 🟡 Pending |
| 2026-01-17 | Validation + optimization complete | 🟡 Pending |
| 2026-01-18 | JSON-RPC integration complete | 🟡 Pending |
| 2026-01-22 | Testing complete | 🟡 Pending |
| 2026-01-24 | Documentation complete | 🟡 Pending |
| 2026-01-25 | Handoff to biomeOS | 🟡 Pending |

---

## Handoff Checklist

### For biomeOS Team

- [ ] API specification reviewed and approved
- [ ] Example requests/responses validated
- [ ] Integration guide provided
- [ ] Demo completed
- [ ] Known issues documented
- [ ] Support contact established
- [ ] Feedback mechanism defined

### For ToadStool Team

- [ ] All code merged to master
- [ ] Tests passing (80%+ coverage)
- [ ] Documentation complete
- [ ] Performance validated
- [ ] Security reviewed
- [ ] Monitoring/logging in place
- [ ] Rollback plan documented

---

## Notes & Decisions

### 2026-01-11: Initial Planning

- **Decision**: Use manual JSON-RPC server (Unix sockets, not TCP)
- **Decision**: Start with ToadStool-only graphs, defer multi-primal
- **Decision**: Conservative estimates (over-estimate resources)
- **Decision**: 5 second cache TTL for system capacity
- **Decision**: 1000 node maximum, 10 second timeout for optimization

### Future Sessions

(Add notes as implementation progresses)

---

## Status Legend

- 🟢 **Complete**: Done and verified
- 🟡 **In Progress**: Currently working on
- 🔴 **Blocked**: Waiting on dependency
- ⚪ **Pending**: Not started yet
- ⚠️ **At Risk**: May slip schedule

---

**Current Status**: 🟡 Planning → Implementation  
**Next Action**: Create `graph_types.rs` and begin Phase 1  
**Updated**: 2026-01-11

Different orders of the same architecture. 🍄🐸

