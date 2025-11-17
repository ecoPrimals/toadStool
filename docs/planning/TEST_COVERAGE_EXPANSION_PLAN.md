# 🧪 Test Coverage Expansion Plan

**Date**: November 13, 2025 (Updated Evening)  
**Current Coverage**: 42.99% (llvm-cov measured, Phase 3 complete)  
**Target Coverage**: 90% (production-ready standard)  
**Gap**: 47.01 percentage points  
**Status**: Phase 3 complete, Phase 4 ready to execute

---

## 📊 CURRENT STATE (Updated Nov 13, 2025)

### **Test Statistics** (Verified):
- **Total Tests**: 1,047+ (100% passing) ✅
- **Lines Covered**: 16,734 / 39,129 (library code only)
- **Coverage Rate**: 42.99%
- **Tests Added Nov 12-13**: +634 new tests (Phases 1, 2, 3)
- **Test Growth**: 154% increase over two days

### **Critical Finding**: Zero-Coverage Files (Priority 1)
- **cli/src/executor/executor_impl.rs**: 0% (938 lines) ❌ CRITICAL
- **core/toadstool/src/ecosystem.rs**: 0% (643 lines) ❌ CRITICAL
- **server/src/websocket.rs**: 0% (244 lines) ❌ HIGH
- **server/src/background.rs**: 0% (229 lines) ❌ HIGH
- **api/src/middleware.rs**: 0% (182 lines) ❌ MEDIUM
- **Total Uncovered**: 2,226 lines in critical production code

### **Coverage by Crate** (Measured Nov 13):
| Crate | Coverage | Status | Priority |
|-------|----------|--------|----------|
| `testing` | 96.34% | 🟢 Excellent | Low |
| `security/sandbox/manager` | 92.98% | 🟢 Excellent | Low |
| `runtime/native` | 81.07% | 🟢 Good | Low |
| `auto_config/natural_language` | 73.87% | 🟢 Good | Low |
| `integration/protocols/client` | 68.95% | 🟡 Fair | Medium |
| `runtime/wasm` | 48.62% | 🟡 Fair | Medium |
| `runtime/container` | 51.34% | 🟡 Fair | Medium |
| `distributed` | 43.17% | 🟡 Fair | High |
| `core/toadstool` | 42.77% | 🟡 Fair | **CRITICAL** |
| `api` | 44.83% | 🟡 Fair | High |
| `server` | 30.36% | 🔴 Low | **CRITICAL** |
| `cli` | 0% (executor) | 🔴 Zero | **CRITICAL** |

### **Uncovered Areas**:
1. **Error paths** (60% uncovered)
2. **Edge cases** (75% uncovered)
3. **Concurrent scenarios** (80% uncovered)
4. **Integration points** (70% uncovered)
5. **Configuration variants** (65% uncovered)

---

## 🎯 COVERAGE TARGETS (Updated Plan)

### **Phase 1-3: COMPLETE** ✅
- **Phase 1**: +199 tests (CLI, API, Distributed, Runtime) ✅
- **Phase 2**: +196 tests (Config, Production Hardening, State, Workload) ✅
- **Phase 3**: +113 tests (Integration scenarios) ✅
- **Total Added**: +508 tests
- **Current Coverage**: 42.99%

### **Phase 4a: Zero-Coverage Critical Files** (Target: 1-2 weeks)
**Focus**: Fix 5 critical zero-coverage files (Priority 1)
- CLI executor: 0% → 70% (120-150 tests)
- Core ecosystem: 0% → 70% (90-120 tests)
- Server websocket: 0% → 70% (30-40 tests)
- Server background: 0% → 70% (30-40 tests)
- API middleware: 0% → 70% (20-30 tests)

**Est. Tests Needed**: ~290-380 new tests  
**Coverage Gain**: +5-7% (42.99% → 50%)

### **Phase 4b: E2E & Integration Content** (Target: 3-6 weeks)
**Focus**: Complete E2E and integration scenarios
- E2E workflows: Add 20-30 scenarios
- Integration points: Add 30-40 scenarios
- Cross-service testing: Add 20-30 scenarios
- Distributed scenarios: Add 30-40 scenarios

**Est. Tests Needed**: ~100-140 new tests  
**Coverage Gain**: +15-20% (50% → 70%)

### **Phase 5: Chaos & Excellence** (Target: 4-6 weeks)
**Focus**: Chaos testing and remaining gaps
- Chaos scenarios: Add 20-30 tests
- Fault injection: Add 20-30 tests
- Edge cases: Add 40-50 tests
- Error paths: Add 30-40 tests
- Fill remaining gaps: Add 50-70 tests

**Est. Tests Needed**: ~160-220 new tests  
**Coverage Gain**: +20% (70% → 90%)

**Total New Tests Needed**: 550-740 tests over 6-10 weeks  
**Total Tests at 90%**: 1,597-1,787 tests

---

## 📋 PRIORITY TEST AREAS

### **Priority 1: API Handlers (Critical - Week 1-2)**

**File**: `crates/api/src/handlers.rs` (38% coverage)

**Missing Tests**:
1. **Error responses** (15 test cases)
   - Invalid JSON
   - Missing required fields
   - Type mismatches
   - Oversized payloads
   - Malformed requests

2. **Authentication edge cases** (10 test cases)
   - Expired tokens
   - Invalid signatures
   - Missing credentials
   - Concurrent auth

3. **Rate limiting** (8 test cases)
   - Burst traffic
   - Sustained load
   - Per-user limits
   - Global limits

4. **Content types** (6 test cases)
   - JSON variants
   - Binary payloads
   - Multipart forms
   - Streaming data

**Total**: ~40 new tests  
**Effort**: 15-20 hours  
**Impact**: Critical (user-facing)

---

### **Priority 2: Job Scheduling (Critical - Week 2-3)**

**File**: `crates/distributed/src/core/coordinator.rs` (55% coverage)

**Missing Tests**:
1. **Scheduling edge cases** (12 test cases)
   - Empty job queue
   - Full queue
   - Priority conflicts
   - Resource constraints

2. **Worker failures** (10 test cases)
   - Worker crashes mid-job
   - Network partitions
   - Timeout scenarios
   - Recovery flows

3. **Concurrent scheduling** (10 test cases)
   - Multiple coordinators
   - Race conditions
   - Job stealing
   - Load balancing

4. **Resource allocation** (8 test cases)
   - Over-subscription
   - Under-utilization
   - Dynamic scaling
   - Resource recovery

**Total**: ~40 new tests  
**Effort**: 20-25 hours  
**Impact**: Critical (core functionality)

---

### **Priority 3: Configuration Loading (High - Week 3-4)**

**Files**: `crates/core/config/src/*.rs` (40% coverage)

**Missing Tests**:
1. **Invalid configurations** (15 test cases)
   - Malformed TOML
   - Missing required fields
   - Type errors
   - Range violations

2. **Environment overrides** (10 test cases)
   - Each override var
   - Priority ordering
   - Invalid values
   - Partial overrides

3. **Default values** (8 test cases)
   - All default paths
   - Minimum config
   - Maximum config
   - Mixed configs

4. **Validation logic** (12 test cases)
   - Port ranges
   - Path validation
   - Dependency checks
   - Semantic validation

**Total**: ~45 new tests  
**Effort**: 15-20 hours  
**Impact**: High (affects all modules)

---

### **Priority 4: Client Library (High - Week 4-5)**

**File**: `crates/client/src/lib.rs` (31% coverage)

**Missing Tests**:
1. **Connection handling** (10 test cases)
   - Connect failures
   - Reconnect logic
   - Connection pooling
   - Timeout handling

2. **Request building** (12 test cases)
   - Each builder method
   - Validation
   - Serialization
   - Error paths

3. **Response parsing** (10 test cases)
   - Success responses
   - Error responses
   - Partial responses
   - Streaming

4. **Retry logic** (8 test cases)
   - Transient failures
   - Permanent failures
   - Backoff behavior
   - Max retries

**Total**: ~40 new tests  
**Effort**: 12-15 hours  
**Impact**: High (external-facing API)

---

### **Priority 5: Integration Protocols (High - Week 5-6)**

**Files**: `crates/integration/protocols/src/*.rs` (28% coverage)

**Missing Tests**:
1. **Protocol negotiation** (8 test cases)
   - Version compatibility
   - Feature detection
   - Fallback protocols
   - Upgrade paths

2. **Message encoding** (12 test cases)
   - Each message type
   - Serialization formats
   - Compression
   - Error encoding

3. **Transport layer** (10 test cases)
   - TCP transport
   - WebSocket transport
   - Unix sockets
   - Transport fallback

4. **Protocol errors** (10 test cases)
   - Malformed messages
   - Protocol violations
   - Timeout handling
   - Recovery

**Total**: ~40 new tests  
**Effort**: 15-20 hours  
**Impact**: High (ecosystem integration)

---

## 🧪 TEST CATEGORIES TO EXPAND

### **Error Path Testing** (Increase from 40% to 80%)

**Strategy**: For every success path, add 2-3 failure tests

**Example Pattern**:
```rust
// ✅ Existing: Happy path
#[tokio::test]
async fn test_execute_job_success() {
    let job = create_test_job();
    let result = executor.execute(job).await;
    assert!(result.is_ok());
}

// ❌ MISSING: Error paths
#[tokio::test]
async fn test_execute_job_invalid_spec() {
    let job = Job { spec: Invalid::default(), .. };
    let result = executor.execute(job).await;
    assert!(matches!(result, Err(ExecutorError::InvalidSpec(_))));
}

#[tokio::test]
async fn test_execute_job_resource_exhausted() {
    let job = create_huge_job(); // Exceeds limits
    let result = executor.execute(job).await;
    assert!(matches!(result, Err(ExecutorError::ResourceExhausted)));
}

#[tokio::test]
async fn test_execute_job_timeout() {
    let job = create_long_job();
    let result = executor.execute_with_timeout(job, Duration::from_millis(10)).await;
    assert!(matches!(result, Err(ExecutorError::Timeout)));
}
```

**Target**: +200 error path tests

---

### **Edge Case Testing** (Increase from 25% to 70%)

**Common Edge Cases**:
1. **Boundary values**: 0, MAX, MIN, -1
2. **Empty collections**: `vec![]`, `""`, `None`
3. **Single element**: `vec![x]`
4. **Large inputs**: 10k items, 10MB payloads
5. **Special characters**: Unicode, nulls, escapes
6. **Concurrent access**: Multiple threads/tasks
7. **Timing issues**: Race conditions, deadlocks

**Example**:
```rust
#[test]
fn test_queue_edge_cases() {
    let mut queue = JobQueue::new();
    
    // Edge: Empty queue
    assert_eq!(queue.pop(), None);
    
    // Edge: Single item
    queue.push(job1);
    assert_eq!(queue.len(), 1);
    
    // Edge: Capacity limit
    for i in 0..MAX_QUEUE_SIZE {
        assert!(queue.push(job()).is_ok());
    }
    assert!(queue.push(job()).is_err()); // Over capacity
    
    // Edge: Duplicate IDs
    queue.push(job_with_id("test"));
    assert!(queue.push(job_with_id("test")).is_err());
}
```

**Target**: +150 edge case tests

---

### **Concurrency Testing** (Increase from 20% to 85%)

**Patterns to Test**:
1. **Multiple readers**: 10+ concurrent reads
2. **Multiple writers**: Synchronized writes
3. **Reader-writer**: Mixed access
4. **Resource contention**: Shared resources
5. **Deadlock prevention**: Lock ordering
6. **Data races**: Arc/Mutex usage

**Example**:
```rust
#[tokio::test]
async fn test_concurrent_job_execution() {
    let executor = Arc::new(Executor::new());
    let mut handles = vec![];
    
    // Launch 100 concurrent jobs
    for i in 0..100 {
        let exec = Arc::clone(&executor);
        handles.push(tokio::spawn(async move {
            let job = create_job(i);
            exec.execute(job).await
        }));
    }
    
    // Wait for all to complete
    let results = futures::future::join_all(handles).await;
    
    // Verify all succeeded
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 100);
    
    // Verify no data corruption
    assert_eq!(executor.stats().completed, 100);
}
```

**Target**: +100 concurrency tests

---

### **Integration Testing** (Increase from 28% to 90%)

**Full System Flows**:
1. **Complete workflows**: End-to-end user journeys
2. **Cross-crate interactions**: Multi-module tests
3. **External dependencies**: Mock services
4. **Data persistence**: Database round-trips
5. **Network communication**: Real network I/O

**Example**:
```rust
#[tokio::test]
async fn test_full_job_lifecycle() {
    // Setup: Start all services
    let server = start_test_server().await;
    let client = Client::connect(server.addr()).await.unwrap();
    
    // Step 1: Submit job
    let job_id = client.submit_job(JobSpec {
        runtime: "wasm",
        code: include_bytes!("test.wasm"),
        args: vec!["arg1"],
    }).await.unwrap();
    
    // Step 2: Poll until complete
    let mut status = JobStatus::Pending;
    for _ in 0..30 {
        status = client.get_status(&job_id).await.unwrap();
        if status == JobStatus::Completed {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(status, JobStatus::Completed);
    
    // Step 3: Retrieve results
    let result = client.get_result(&job_id).await.unwrap();
    assert_eq!(result.exit_code, 0);
    assert!(!result.stdout.is_empty());
    
    // Step 4: Cleanup
    client.delete_job(&job_id).await.unwrap();
    server.shutdown().await;
}
```

**Target**: +80 integration tests

---

## 📅 EXECUTION TIMELINE

### **Week 1-2: Foundation (50% Coverage)**
- **Focus**: Priority 1 (API handlers)
- **Tests**: ~40 new
- **Coverage**: 42.84% → 50%

### **Week 3-4: Core Logic (55% Coverage)**
- **Focus**: Priority 2 (Job scheduling)
- **Tests**: ~40 new
- **Coverage**: 50% → 55%

### **Week 5-6: Configuration (60% Coverage)**
- **Focus**: Priority 3 (Config loading)
- **Tests**: ~45 new
- **Coverage**: 55% → 60%

### **Week 7-8: Client & Integration (65% Coverage)**
- **Focus**: Priority 4-5
- **Tests**: ~80 new
- **Coverage**: 60% → 65%

### **Week 9-10: Error Paths (75% Coverage)**
- **Focus**: Error handling everywhere
- **Tests**: ~100 new
- **Coverage**: 65% → 75%

### **Week 11-12: Edge Cases & Concurrency (85% Coverage)**
- **Focus**: Edge cases, concurrency
- **Tests**: ~150 new
- **Coverage**: 75% → 85%

### **Week 13-14: Final Push (90% Coverage)**
- **Focus**: Remaining gaps
- **Tests**: ~100 new
- **Coverage**: 85% → 90%

**Total**: 14 weeks, 555-650 new tests

---

## 🔧 TESTING INFRASTRUCTURE

### **Tools & Frameworks**:
```toml
[dev-dependencies]
# Already have
tokio-test = "0.4"
tempfile = "3.8"
mockito = "1.2"

# Should add
proptest = "1.4"           # Property-based testing
quickcheck = "1.0"         # Randomized testing
wiremock = "0.6"           # HTTP mocking
testcontainers = "0.15"    # Docker containers for tests
criterion = "0.5"          # Benchmarking (for coverage verification)
```

### **Test Helpers to Create**:
1. **Builders**: Fluent test data builders
2. **Fixtures**: Reusable test data
3. **Mocks**: Common service mocks
4. **Assertions**: Custom assertion helpers
5. **Utilities**: Test setup/teardown

**Example**:
```rust
// crates/testing/src/builders/job_builder.rs
pub struct JobBuilder {
    runtime: RuntimeType,
    code: Vec<u8>,
    args: Vec<String>,
    env: HashMap<String, String>,
    timeout: Duration,
}

impl JobBuilder {
    pub fn new() -> Self { /* ... */ }
    pub fn runtime(mut self, rt: RuntimeType) -> Self { /* ... */ }
    pub fn code(mut self, code: &[u8]) -> Self { /* ... */ }
    pub fn build(self) -> JobSpec { /* ... */ }
}

// Usage in tests
#[test]
fn test_wasm_job() {
    let job = JobBuilder::new()
        .runtime(RuntimeType::Wasm)
        .code(include_bytes!("test.wasm"))
        .timeout(Duration::from_secs(5))
        .build();
    
    // Test with job...
}
```

---

## ✅ COVERAGE VALIDATION

### **Continuous Monitoring**:
```bash
# Generate coverage report
cargo llvm-cov --workspace --html

# Check coverage threshold
cargo llvm-cov --workspace --fail-under-lines 90

# Show uncovered lines
cargo llvm-cov --workspace --show-missing

# Per-crate coverage
cargo llvm-cov --package toadstool-core --summary-only
```

### **CI Integration**:
```yaml
# .github/workflows/coverage.yml
name: Coverage

on: [push, pull_request]

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-llvm-cov
      - run: cargo llvm-cov --workspace --lcov --output-path coverage.lcov
      - uses: codecov/codecov-action@v3
        with:
          files: coverage.lcov
          fail_ci_if_error: true
```

---

## 📊 PROGRESS TRACKING

### **Weekly Checkpoints**:
- **Monday**: Set week's test targets
- **Wednesday**: Mid-week review (on track?)
- **Friday**: Generate coverage report, update STATUS.md

### **Metrics to Track**:
1. **Overall coverage %**
2. **Tests added this week**
3. **Coverage gained this week**
4. **Tests passing/failing**
5. **Coverage by priority area**

### **Report Template**:
```markdown
## Week X Coverage Report

**Overall**: 45.2% → 52.3% (+7.1%)
**Tests Added**: 42 new tests
**Status**: 🟢 On Track

### Progress by Priority:
- API Handlers: 38% → 65% ✅
- Job Scheduling: 55% → 58% 🟡
- Configuration: 40% → 42% 🟡

### Next Week:
- Complete Priority 2 (Job scheduling)
- Start Priority 3 (Configuration)
```

---

## 🎯 SUCCESS CRITERIA

### **90% Coverage Achieved When**:
1. ✅ Overall coverage ≥ 90%
2. ✅ Each crate ≥ 85%
3. ✅ All critical paths 100%
4. ✅ Error paths ≥ 80%
5. ✅ Edge cases ≥ 70%
6. ✅ Concurrency ≥ 85%
7. ✅ Integration tests ≥ 90%
8. ✅ All tests passing
9. ✅ No flaky tests
10. ✅ CI coverage checks enabled

---

## 🚀 QUICK START

### **To Begin Test Expansion**:

```bash
# 1. Check current coverage
cargo llvm-cov --workspace --html
open target/llvm-cov/html/index.html

# 2. Identify lowest-coverage files
cargo llvm-cov --workspace --summary-only | sort -k3 -n

# 3. Pick a Priority 1 file (e.g., handlers.rs)
# Identify uncovered lines (red in HTML report)

# 4. Write tests for uncovered code
touch crates/api/tests/handlers_error_paths.rs

# 5. Run tests and check coverage
cargo test --package api
cargo llvm-cov --package api --html

# 6. Repeat until file reaches 70%+

# 7. Move to next priority file
```

---

## 📞 NEXT STEPS

1. **Review this plan** with team
2. **Set up llvm-cov** in CI
3. **Create test helpers** (builders, mocks)
4. **Start Week 1** (Priority 1 tests)
5. **Track weekly progress**
6. **Celebrate milestones** (50%, 60%, 75%, 90%)

---

**Status**: ✅ PLAN COMPLETE - READY TO EXECUTE  
**Current**: 42.84% coverage (413 tests)  
**Target**: 90% coverage (~1000 tests)  
**Timeline**: 14 weeks  
**Effort**: ~200-250 hours  
**Impact**: CRITICAL (production readiness)

---

*End of Test Coverage Expansion Plan*

