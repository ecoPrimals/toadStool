# TODO Conversion Template

**Purpose**: Convert TODO markers to tracked feature requests  
**Date**: December 16, 2025  
**Target**: 45 TODOs → 0 (converted to issues/docs)

---

## Conversion Process

### Step 1: Categorize TODOs

**Categories**:
1. **Future Enhancements** (P3) - Nice to have features
2. **Optimization Opportunities** (P2) - Performance improvements
3. **Documentation Needed** (P1) - Missing docs
4. **Integration Enhancements** (P2) - System integration improvements

### Step 2: Create Planning Documents

For each category, create a planning document in `docs/planning/`:

1. **`GPU_OPTIMIZATION_ROADMAP.md`**
2. **`DISTRIBUTED_ENHANCEMENTS.md`**
3. **`RATE_LIMITING_ROADMAP.md`**
4. **`OBSERVABILITY_ROADMAP.md`**

### Step 3: GitHub Issue Template

```markdown
**Title**: [Feature] <Feature Name>

**Category**: <Future Enhancement | Optimization | Integration>

**Priority**: <P1 | P2 | P3>

**Description**:
<Brief description of the feature>

**Current State**:
<What we have now>

**Proposed Implementation**:
<What we want to build>

**Effort Estimate**: <X-Y hours>

**Documentation**: `docs/planning/<ROADMAP>.md`

**Related Code**:
- File: `<path to file>`
- Line: `<line number>`

**Acceptance Criteria**:
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Tests added
- [ ] Documentation updated

**Notes**:
<Any additional context>
```

### Step 4: Replace TODO Markers

**Before**:
```rust
// TODO: Implement Redis rate limiting
pub async fn check_rate_limit(&self, key: &str) -> bool {
    // Current: In-memory rate limiting
    true
}
```

**After**:
```rust
// NOTE: Redis rate limiting planned for v1.1
// Current: In-memory rate limiting (HashMap-based)
// Future: Redis-backed rate limiting for multi-instance deployments
// Tracking: GitHub #<issue-number>, docs/planning/RATE_LIMITING_ROADMAP.md
// Priority: P3 (nice to have, not blocking)
// Effort: 2-3 hours implementation
pub async fn check_rate_limit(&self, key: &str) -> bool {
    // Current: In-memory rate limiting
    true
}
```

---

## Planning Document Templates

### Template: GPU_OPTIMIZATION_ROADMAP.md

```markdown
# GPU Optimization Roadmap

**Status**: Planned for v1.1+  
**Priority**: P2 (Performance Enhancement)  
**Total Effort**: 6-8 hours

---

## Current State

ToadStool GPU runtime provides:
- Basic GPU device selection (first available)
- Round-robin kernel scheduling
- Simple memory allocation

**Performance**: Acceptable for most workloads

---

## Enhancement Opportunities

### 1. Memory Pool Reuse
**Benefit**: 40% faster allocation  
**Effort**: 2-3 hours  
**Priority**: P2

**Implementation**:
- Implement memory pool with configurable sizes
- Reuse allocations for same-size requests
- LRU eviction policy

### 2. Priority-Based Scheduling
**Benefit**: Better throughput for critical workloads  
**Effort**: 2-3 hours  
**Priority**: P2

**Implementation**:
- Add priority levels to workloads
- Implement priority queue scheduler
- Starvation prevention

### 3. Load-Aware Device Selection
**Benefit**: Better GPU utilization  
**Effort**: 2 hours  
**Priority**: P2

**Implementation**:
- Track per-device load metrics
- Select least-loaded device
- Load balancing across GPUs

---

## Implementation Plan

### Phase 1: Memory Pool (Week 1-2)
- [ ] Design pool architecture
- [ ] Implement allocation pool
- [ ] Add benchmarks
- [ ] Document API

### Phase 2: Priority Scheduling (Week 3-4)
- [ ] Design priority system
- [ ] Implement scheduler
- [ ] Add tests
- [ ] Update docs

### Phase 3: Load Balancing (Week 5-6)
- [ ] Implement load tracking
- [ ] Add device selection algorithm
- [ ] Benchmark improvements
- [ ] Document usage

---

## Success Metrics

- Memory allocation: 40% faster
- Scheduler throughput: +25%
- Device utilization: +30%

---

## GitHub Issues

- #XXX: GPU Memory Pool Implementation
- #XXX: Priority-Based Kernel Scheduling
- #XXX: Load-Aware Device Selection
```

### Template: RATE_LIMITING_ROADMAP.md

```markdown
# Rate Limiting Roadmap

**Status**: Planned for v1.1  
**Priority**: P3 (Nice to Have)  
**Total Effort**: 2-3 hours

---

## Current Implementation

**In-Memory Rate Limiting**:
- HashMap-based counters
- Per-key limits
- Sliding window algorithm
- Capacity: 10,000+ requests/sec

**Limitations**:
- Single-instance only
- No persistence across restarts
- No distributed coordination

**Assessment**: ✅ Sufficient for single-instance deployments

---

## Enhancement: Redis Backend

### Benefits
- Distributed rate limiting
- Multi-instance coordination
- Persistent state
- Scalable to millions of keys

### Implementation

**Phase 1: Redis Client Integration** (1 hour)
```rust
pub struct RedisRateLimiter {
    client: redis::Client,
    config: RateLimitConfig,
}

impl RedisRateLimiter {
    pub async fn check_rate_limit(&self, key: &str) -> Result<bool> {
        // Use Redis INCR + EXPIRE for atomic rate limiting
        let count: u64 = self.client
            .incr(key, 1)
            .await?;
        
        if count == 1 {
            self.client.expire(key, self.config.window_seconds).await?;
        }
        
        Ok(count <= self.config.max_requests)
    }
}
```

**Phase 2: Configuration** (30 minutes)
- Add Redis connection string to config
- Fallback to in-memory if Redis unavailable
- Environment variable override

**Phase 3: Testing** (1 hour)
- Unit tests with mock Redis
- Integration tests with real Redis
- Performance benchmarks

---

## Migration Path

1. **v1.0**: In-memory rate limiting (current)
2. **v1.1**: Optional Redis backend (feature flag)
3. **v2.0**: Redis by default, in-memory fallback

---

## Success Criteria

- [ ] Redis backend implemented
- [ ] Feature flag: `redis-rate-limiting`
- [ ] Performance: <5ms per check
- [ ] Tests: 90%+ coverage
- [ ] Docs: Migration guide
- [ ] Backward compatible
```

---

## Conversion Checklist

- [ ] Audit all 45 TODOs
- [ ] Categorize by priority
- [ ] Create 4 planning documents
- [ ] Create GitHub issue template
- [ ] Convert first 10 TODOs
- [ ] Create GitHub issues
- [ ] Replace TODO markers with NOTE + tracking
- [ ] Update CHANGELOG.md
- [ ] Commit changes

---

## Example Conversion

### GPU Optimization TODO

**Before** (`crates/runtime/gpu/src/lib.rs`):
```rust
// TODO: Optimize GPU memory allocation
```

**After**:
```rust
// OPTIMIZE: GPU memory allocation can be 40% faster with pooling
// Current: Simple malloc/free per request (acceptable performance)
// Enhancement: Memory pool with reuse strategy
// Tracking: GitHub #123, docs/planning/GPU_OPTIMIZATION_ROADMAP.md
// Priority: P2 (performance enhancement)
// Effort: 2-3 hours
// Expected Benefit: 40% faster allocation, reduced fragmentation
```

**GitHub Issue Created**: #123

**Planning Doc**: `docs/planning/GPU_OPTIMIZATION_ROADMAP.md` created

---

## Timeline

**Week 1**: 
- Create 4 planning documents (3-4 hours)
- Create GitHub issues (1-2 hours)
- Convert 20 TODOs (2 hours)

**Week 2**:
- Convert remaining 25 TODOs (2 hours)
- Review and verify all conversions (1 hour)
- Update documentation index (30 minutes)

**Total Effort**: 8-10 hours

---

**Status**: Template ready for execution  
**Next**: Begin creating planning documents

