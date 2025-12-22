# Rate Limiting & Backpressure Roadmap

**Status**: Planning  
**Priority**: P2 (Performance & Reliability)  
**Estimated Effort**: 6-8 hours  
**Target**: Protect system resources and ensure fair allocation

---

## 🎯 **Objectives**

### Primary Goals
1. **Request Rate Limiting**: Prevent resource exhaustion from excessive requests
2. **Adaptive Backpressure**: Dynamically adjust based on system load
3. **Fair Resource Allocation**: Ensure no single client monopolizes resources
4. **Graceful Degradation**: Maintain service under overload conditions

### Success Metrics
- 99.9% uptime under high load
- <100ms rate limit check latency
- Configurable per-client and global limits
- Automatic recovery from overload
- Zero resource starvation

---

## 📋 **Current State Analysis**

### What We Have
✅ **Basic Request Handling**: Execute requests without limits  
✅ **Timeout Mechanisms**: Per-request timeouts configured  
✅ **Concurrent Execution**: Multiple requests in parallel  
✅ **Metrics Collection**: Track request counts and durations  

### What's Missing
❌ **Rate Limiting**: No request rate caps per client/global  
❌ **Backpressure**: No adaptive throttling under load  
❌ **Token Buckets**: No sophisticated rate limit algorithms  
❌ **Queue Management**: No bounded queues with backpressure  
❌ **Circuit Breakers**: No automatic overload protection  
❌ **Priority Queues**: No prioritization of critical requests  

---

## 🚀 **Implementation Plan**

### Phase 1: Token Bucket Rate Limiting (2-3 hours)

#### Components
1. **TokenBucket Struct**
   ```rust
   pub struct TokenBucket {
       capacity: usize,
       tokens: AtomicUsize,
       refill_rate: Duration,
       last_refill: RwLock<Instant>,
   }
   ```

2. **Rate Limiter Manager**
   ```rust
   pub struct RateLimiterManager {
       global_limiter: TokenBucket,
       client_limiters: DashMap<ClientId, TokenBucket>,
       config: RateLimitConfig,
   }
   ```

3. **Configuration**
   ```rust
   pub struct RateLimitConfig {
       /// Global requests per second
       pub global_rps: usize,
       /// Per-client requests per second
       pub per_client_rps: usize,
       /// Burst capacity (multiple of rps)
       pub burst_multiplier: usize,
       /// Enable adaptive limiting
       pub adaptive: bool,
   }
   ```

#### Integration Points
- `ExecutorEngine::execute()` - Check limits before execution
- `RuntimeEngine::execute()` - Apply runtime-specific limits
- Metrics - Track rate limit hits/misses

#### Testing
- Token bucket refill mechanics
- Burst handling
- Multi-client fairness
- Global limit enforcement

---

### Phase 2: Adaptive Backpressure (2-3 hours)

#### Components
1. **Load Monitor**
   ```rust
   pub struct LoadMonitor {
       cpu_threshold: f64,
       memory_threshold: f64,
       queue_depth_threshold: usize,
       current_load: AtomicU8, // 0-100%
   }
   ```

2. **Backpressure Controller**
   ```rust
   pub struct BackpressureController {
       load_monitor: Arc<LoadMonitor>,
       rate_adjuster: RateAdjuster,
       shedding_strategy: LoadSheddingStrategy,
   }
   ```

3. **Load Shedding Strategies**
   - **Probabilistic**: Drop requests with probability proportional to load
   - **Priority-based**: Drop low-priority requests first
   - **Age-based**: Drop oldest queued requests

#### Adaptive Algorithm
```rust
fn adjust_rate_limits(&self) -> Result<()> {
    let load = self.load_monitor.current_load();
    
    let adjustment = match load {
        0..=70 => 1.0,        // Normal operation
        71..=85 => 0.8,       // Light backpressure
        86..=95 => 0.5,       // Heavy backpressure
        96..=100 => 0.2,      // Emergency mode
    };
    
    self.rate_adjuster.scale_limits(adjustment)
}
```

#### Testing
- Load spike simulation
- Gradual load increase/decrease
- Recovery from overload
- Priority preservation

---

### Phase 3: Circuit Breakers (2 hours)

#### Components
1. **Circuit Breaker**
   ```rust
   pub struct CircuitBreaker {
       state: AtomicCircuitState,
       failure_threshold: usize,
       success_threshold: usize,
       timeout: Duration,
       half_open_max_calls: usize,
   }
   
   pub enum CircuitState {
       Closed,   // Normal operation
       Open,     // Failing - reject requests
       HalfOpen, // Testing recovery
   }
   ```

2. **Integration with Backends**
   - Wrap biomeOS API calls
   - Wrap runtime engine calls
   - Wrap storage/agent operations

#### State Transitions
```
Closed --[failures >= threshold]--> Open
Open --[timeout elapsed]--> HalfOpen
HalfOpen --[success >= threshold]--> Closed
HalfOpen --[any failure]--> Open
```

#### Testing
- Failure threshold triggering
- Automatic recovery
- Half-open state behavior
- Multiple concurrent circuit breakers

---

## 📊 **Configuration Examples**

### Conservative (Production Default)
```toml
[rate_limiting]
global_rps = 1000
per_client_rps = 100
burst_multiplier = 2
adaptive = true

[backpressure]
cpu_threshold = 80.0
memory_threshold = 85.0
queue_depth_threshold = 1000

[circuit_breaker]
failure_threshold = 10
success_threshold = 5
timeout_secs = 30
half_open_max_calls = 3
```

### Aggressive (High-Throughput)
```toml
[rate_limiting]
global_rps = 10000
per_client_rps = 1000
burst_multiplier = 5
adaptive = true

[backpressure]
cpu_threshold = 90.0
memory_threshold = 90.0
queue_depth_threshold = 5000

[circuit_breaker]
failure_threshold = 20
success_threshold = 10
timeout_secs = 10
half_open_max_calls = 10
```

---

## 🧪 **Testing Strategy**

### Unit Tests (20 tests)
- ✅ Token bucket refill logic
- ✅ Burst capacity handling
- ✅ Multi-client isolation
- ✅ Global limit enforcement
- ✅ Adaptive rate adjustment
- ✅ Load calculation accuracy
- ✅ Circuit breaker state transitions
- ✅ Recovery timing

### Integration Tests (10 tests)
- ✅ End-to-end rate limiting
- ✅ Backpressure under load
- ✅ Circuit breaker with real backends
- ✅ Multi-runtime coordination

### Load Tests (5 tests)
- ✅ Sustained high load
- ✅ Spike handling
- ✅ Gradual ramp-up
- ✅ Mixed priority workloads
- ✅ Recovery from overload

### Property Tests (5 tests)
- ✅ Rate limits never exceeded
- ✅ Fairness across clients
- ✅ No resource starvation
- ✅ Monotonic token refill
- ✅ Circuit breaker eventually recovers

**Expected Coverage Impact**: +3-4% (40 new tests)

---

## 🔧 **Implementation Checklist**

### Phase 1: Token Bucket (2-3 hours)
- [ ] Create `TokenBucket` struct with atomic refill
- [ ] Implement `RateLimiterManager` with per-client tracking
- [ ] Add `RateLimitConfig` with builder pattern
- [ ] Integrate into `ExecutorEngine::execute()`
- [ ] Add rate limit metrics (hits, misses, rejections)
- [ ] Write 15 unit tests
- [ ] Write 3 integration tests
- [ ] Document configuration options

### Phase 2: Adaptive Backpressure (2-3 hours)
- [ ] Create `LoadMonitor` with CPU/memory tracking
- [ ] Implement `BackpressureController` with adaptive algorithm
- [ ] Add load shedding strategies (probabilistic, priority, age)
- [ ] Integrate with rate limiter for dynamic adjustment
- [ ] Add backpressure metrics (load, shedding rate, adjustments)
- [ ] Write 15 unit tests
- [ ] Write 4 integration tests
- [ ] Write 5 load tests
- [ ] Document adaptive behavior

### Phase 3: Circuit Breakers (2 hours)
- [ ] Create `CircuitBreaker` with state machine
- [ ] Integrate with biomeOS backends
- [ ] Integrate with runtime engines
- [ ] Add circuit breaker metrics (state, trips, recoveries)
- [ ] Write 10 unit tests
- [ ] Write 3 integration tests
- [ ] Write 5 property tests
- [ ] Document circuit breaker patterns

---

## 📈 **Expected Benefits**

### Reliability
- **99.9% uptime** under high load conditions
- **Automatic recovery** from overload (30-60s)
- **Fair resource allocation** across clients

### Performance
- **<100ms overhead** for rate limit checks
- **Predictable latency** under load
- **Graceful degradation** vs. hard failures

### Operational
- **Configurable policies** per environment
- **Real-time metrics** for monitoring
- **Adaptive behavior** reduces manual tuning

---

## 🎓 **Design Principles**

### 1. Defense in Depth
- **Token buckets** for request-level limits
- **Backpressure** for system-level protection
- **Circuit breakers** for failure isolation

### 2. Adaptive Over Static
- Dynamic rate adjustment based on load
- Gradual recovery from overload
- Learning from failure patterns

### 3. Fairness & Prioritization
- Per-client quotas prevent monopolization
- Priority queues for critical requests
- Fair sharing of limited resources

### 4. Observable & Tunable
- Rich metrics for all limiting decisions
- Configurable thresholds per environment
- Clear documentation of trade-offs

---

## 🔗 **Related Work**

### Dependencies
- `governor` crate for token bucket algorithms
- `tokio` rate limiting utilities
- `dashmap` for concurrent client tracking

### Integration Points
- **Executor**: Apply limits before execution
- **Metrics**: Track rate limit events
- **Config**: Load limits from environment/file
- **Logging**: Debug rate limit decisions

### Future Enhancements (Phase 4+)
- Distributed rate limiting (Redis-backed)
- Machine learning for adaptive thresholds
- Per-workload-type rate limits
- Client reputation scoring
- Request prioritization based on SLA

---

## 📚 **References**

### Academic & Industry
- "Overload Control for Scaling WeChat Microservices" (USENIX 2018)
- "Adaptive Concurrency Control" (Google SRE Book)
- "The Netflix API Rate Limiting Strategy"

### Implementation Patterns
- Token Bucket Algorithm (industry standard)
- Leaky Bucket (alternative approach)
- Sliding Window Counters (Redis pattern)
- AIMD (Additive Increase, Multiplicative Decrease)

---

## ✅ **Success Criteria**

### Functional
- ✅ Global and per-client rate limits enforced
- ✅ Adaptive backpressure under load
- ✅ Circuit breakers prevent cascade failures
- ✅ Configurable policies per environment

### Performance
- ✅ <100ms rate limit check latency
- ✅ 99.9% uptime under 10x normal load
- ✅ Fair allocation within 5% variance

### Quality
- ✅ 40+ comprehensive tests
- ✅ Full documentation with examples
- ✅ Metrics for all limiting decisions
- ✅ Production-ready configuration

---

**Status**: Ready for implementation  
**Next Steps**: Begin Phase 1 (Token Bucket Rate Limiting)  
**Timeline**: 6-8 hours total, can be done incrementally

---

*This roadmap provides a comprehensive path from basic rate limiting to sophisticated adaptive backpressure, ensuring ToadStool remains reliable and fair under all load conditions.*

