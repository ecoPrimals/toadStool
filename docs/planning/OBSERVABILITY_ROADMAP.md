# Observability & Monitoring Roadmap

**Status**: Planning  
**Priority**: P2 (Operations & Production Readiness)  
**Estimated Effort**: 8-10 hours  
**Target**: Comprehensive visibility into system behavior

---

## 🎯 **Objectives**

### Primary Goals
1. **Distributed Tracing**: Follow request flows across components
2. **Structured Logging**: Consistent, queryable logs with context
3. **Rich Metrics**: Performance, health, and business metrics
4. **Health Checks**: Liveness, readiness, and dependency health
5. **Debug Tooling**: Runtime inspection and troubleshooting

### Success Metrics
- <1ms tracing overhead per span
- 100% request traceability
- Structured logs with full context propagation
- Health checks expose all dependencies
- Debug endpoints for runtime inspection
- OpenTelemetry-compatible exports

---

## 📋 **Current State Analysis**

### What We Have
✅ **Basic Metrics**: Execution counts, durations, success rates  
✅ **Simple Logging**: Info/debug logs in key paths  
✅ **Performance Tracking**: Execution time measurements  
✅ **Error Handling**: Structured errors with context  

### What's Missing
❌ **Distributed Tracing**: No trace ID propagation across boundaries  
❌ **Span Hierarchy**: No parent-child span relationships  
❌ **Structured Logging**: Logs lack consistent structure/levels  
❌ **Log Correlation**: Logs not correlated with traces  
❌ **Health Endpoints**: No /health or /ready endpoints  
❌ **Debug Endpoints**: No runtime inspection APIs  
❌ **OpenTelemetry**: No OTLP export support  
❌ **Metric Cardinality**: Limited metric dimensions  

---

## 🚀 **Implementation Plan**

### Phase 1: Distributed Tracing (3-4 hours)

#### Components
1. **Tracing Infrastructure**
   ```rust
   use tracing::{info_span, Instrument};
   use tracing_opentelemetry::OpenTelemetryLayer;
   
   pub struct TracingConfig {
       pub service_name: String,
       pub otlp_endpoint: Option<String>,
       pub sample_rate: f64,
       pub export_batch_size: usize,
   }
   ```

2. **Span Hierarchy**
   ```rust
   // Executor-level span
   async fn execute(&self, request: Request) -> Result<Response> {
       let span = info_span!("executor.execute",
           executor_id = %self.id,
           workload_type = ?request.workload.workload_type,
           request_id = %request.id
       );
       
       async move {
           // Runtime-level span (child)
           let runtime_span = info_span!("runtime.execute",
               runtime = %self.runtime_name()
           );
           
           runtime_span.in_scope(|| {
               // Execution logic
           })
       }.instrument(span).await
   }
   ```

3. **Context Propagation**
   ```rust
   pub struct ExecutionContext {
       pub trace_id: TraceId,
       pub span_id: SpanId,
       pub parent_span_id: Option<SpanId>,
       pub baggage: HashMap<String, String>,
   }
   ```

#### Instrumentation Points
- **Executor**: `execute()`, `initialize()`, `shutdown()`
- **Runtime Engines**: `execute()`, `load_module()`, `cache_miss()`
- **Discovery**: `register_service()`, `find_by_capability()`
- **biomeOS Integration**: `request_token()`, `provision_volume()`, `deploy_agent()`
- **BYOB**: `apply_config()`, `start_deployment()`, `health_check()`

#### Testing
- Span creation and propagation
- Parent-child relationships
- Trace ID consistency
- Sampling behavior

---

### Phase 2: Structured Logging (2-3 hours)

#### Components
1. **Log Configuration**
   ```rust
   use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
   
   pub struct LogConfig {
       pub level: LogLevel,
       pub format: LogFormat, // Json | Pretty | Compact
       pub include_timestamps: bool,
       pub include_thread_ids: bool,
       pub include_file_locations: bool,
   }
   ```

2. **Structured Fields**
   ```rust
   use tracing::{info, warn, error};
   
   // Rich structured logging
   info!(
       executor_id = %self.id,
       workload_type = ?request.workload.workload_type,
       duration_ms = execution_time.as_millis(),
       status = ?result.status,
       "Execution completed"
   );
   ```

3. **Log Levels**
   - **ERROR**: Failures requiring attention
   - **WARN**: Degraded performance, recoverable issues
   - **INFO**: Key lifecycle events (start, stop, execution)
   - **DEBUG**: Detailed operational information
   - **TRACE**: Very verbose for deep debugging

#### Log Correlation
```rust
// Logs automatically include trace context
info!(
    trace_id = %current_span.trace_id(),
    span_id = %current_span.span_id(),
    "Processing request"
);
```

#### Testing
- Log output format validation
- Field presence and types
- Log level filtering
- Trace correlation

---

### Phase 3: Enhanced Metrics (2-3 hours)

#### Components
1. **Metric Types**
   ```rust
   use prometheus::{Counter, Histogram, Gauge, IntGauge};
   
   pub struct ToadStoolMetrics {
       // Counters
       pub executions_total: Counter,
       pub errors_total: Counter,
       pub cache_hits: Counter,
       pub cache_misses: Counter,
       
       // Histograms
       pub execution_duration: Histogram,
       pub queue_wait_time: Histogram,
       pub module_load_time: Histogram,
       
       // Gauges
       pub active_executions: IntGauge,
       pub queue_depth: IntGauge,
       pub cache_size_bytes: IntGauge,
   }
   ```

2. **Metric Dimensions (Labels)**
   ```rust
   // Multi-dimensional metrics
   executions_total
       .with_label_values(&[
           &executor_id,
           workload_type.as_str(),
           runtime_name,
           status.as_str(),
       ])
       .inc();
   ```

3. **Custom Metrics**
   - **Discovery**: Service registration/removal rates
   - **WASM**: Module cache hit ratio, compilation time
   - **GPU**: Device utilization, memory usage
   - **BYOB**: Deployment success rate, rollback frequency
   - **biomeOS**: API latency, token refresh rate

#### Metric Endpoints
```rust
// Prometheus-compatible endpoint
GET /metrics -> text/plain (Prometheus format)
GET /metrics/json -> application/json (structured)
```

#### Testing
- Metric registration
- Label cardinality limits
- Histogram bucket configuration
- Metric export formats

---

### Phase 4: Health Checks (1-2 hours)

#### Components
1. **Health Check Types**
   ```rust
   pub enum HealthStatus {
       Healthy,
       Degraded { reason: String },
       Unhealthy { reason: String },
   }
   
   pub struct HealthCheck {
       pub name: String,
       pub status: HealthStatus,
       pub last_checked: Instant,
       pub duration: Duration,
   }
   ```

2. **Liveness vs. Readiness**
   ```rust
   // Liveness: Can the service recover?
   GET /health/live -> 200 (process alive) or 503 (deadlock)
   
   // Readiness: Can the service handle requests?
   GET /health/ready -> 200 (ready) or 503 (not ready)
   ```

3. **Dependency Checks**
   ```rust
   pub struct DependencyHealthChecker {
       checks: Vec<Box<dyn HealthChecker>>,
   }
   
   // Check each dependency
   - Runtime engines initialized
   - Cache accessible
   - Discovery service responsive
   - biomeOS API reachable
   - Disk space available
   - Memory within limits
   ```

#### Health Response Format
```json
{
  "status": "healthy",
  "timestamp": "2025-12-16T10:30:00Z",
  "checks": [
    {
      "name": "wasm_runtime",
      "status": "healthy",
      "duration_ms": 2
    },
    {
      "name": "biomeos_api",
      "status": "degraded",
      "reason": "high latency (500ms)",
      "duration_ms": 502
    }
  ],
  "uptime_seconds": 86400
}
```

#### Testing
- Individual health checks
- Aggregated health status
- Timeout handling
- Failure detection

---

### Phase 5: Debug Endpoints (1-2 hours)

#### Components
1. **Runtime Inspection**
   ```rust
   GET /debug/runtime/{runtime_name}/state
   GET /debug/runtime/{runtime_name}/metrics
   GET /debug/runtime/{runtime_name}/cache
   ```

2. **Execution Tracing**
   ```rust
   GET /debug/executions/active
   GET /debug/executions/{execution_id}/trace
   GET /debug/executions/history?limit=100
   ```

3. **Configuration Dump**
   ```rust
   GET /debug/config -> Full configuration
   GET /debug/config/runtime/{runtime_name}
   GET /debug/config/discovery
   GET /debug/config/byob
   ```

4. **Performance Profiling**
   ```rust
   GET /debug/pprof/heap -> Memory profile
   GET /debug/pprof/profile?seconds=30 -> CPU profile
   GET /debug/pprof/goroutine -> Async task states
   ```

#### Security
- Debug endpoints only enabled with `--enable-debug`
- Optional authentication/authorization
- Rate-limited to prevent abuse
- No sensitive data in responses

#### Testing
- Endpoint availability
- Response format validation
- Authentication (if enabled)
- Performance impact

---

## 📊 **Configuration Examples**

### Development
```toml
[observability]
tracing_enabled = true
tracing_sample_rate = 1.0
log_level = "debug"
log_format = "pretty"
metrics_enabled = true
health_checks_enabled = true
debug_endpoints_enabled = true

[tracing]
otlp_endpoint = "http://localhost:4317"
service_name = "toadstool-dev"
export_batch_size = 512
```

### Production
```toml
[observability]
tracing_enabled = true
tracing_sample_rate = 0.1  # Sample 10% of requests
log_level = "info"
log_format = "json"
metrics_enabled = true
health_checks_enabled = true
debug_endpoints_enabled = false  # Disable for security

[tracing]
otlp_endpoint = "http://otel-collector:4317"
service_name = "toadstool-prod"
export_batch_size = 2048

[health]
check_interval_secs = 30
dependency_timeout_secs = 5
```

---

## 🧪 **Testing Strategy**

### Unit Tests (25 tests)
- ✅ Span creation and attributes
- ✅ Log formatting and fields
- ✅ Metric registration and updates
- ✅ Health check execution
- ✅ Debug endpoint responses

### Integration Tests (15 tests)
- ✅ End-to-end trace propagation
- ✅ Log-trace correlation
- ✅ Metric export to Prometheus
- ✅ Health check aggregation
- ✅ OTLP export to collector

### Property Tests (5 tests)
- ✅ Trace IDs are unique
- ✅ Span hierarchies are valid
- ✅ Metrics never decrease (counters)
- ✅ Health checks timeout properly
- ✅ Log levels filter correctly

**Expected Coverage Impact**: +4-5% (45 new tests)

---

## 🔧 **Implementation Checklist**

### Phase 1: Distributed Tracing (3-4 hours)
- [ ] Add `tracing` and `tracing-opentelemetry` dependencies
- [ ] Create `TracingConfig` with OTLP export
- [ ] Instrument `ExecutorEngine::execute()`
- [ ] Instrument all `RuntimeEngine` implementations
- [ ] Add trace context to `ExecutionContext`
- [ ] Propagate context across async boundaries
- [ ] Write 10 unit tests for span creation
- [ ] Write 5 integration tests for E2E tracing
- [ ] Document tracing configuration

### Phase 2: Structured Logging (2-3 hours)
- [ ] Configure `tracing-subscriber` with layers
- [ ] Add structured fields to all log statements
- [ ] Correlate logs with traces (trace_id in logs)
- [ ] Implement JSON and pretty formatters
- [ ] Add log sampling for high-volume paths
- [ ] Write 8 unit tests for log formatting
- [ ] Write 3 integration tests for log correlation
- [ ] Document logging best practices

### Phase 3: Enhanced Metrics (2-3 hours)
- [ ] Add `prometheus` crate with custom registry
- [ ] Define core metric types (counters, histograms, gauges)
- [ ] Instrument all execution paths with metrics
- [ ] Add metric labels for dimensions (executor, runtime, status)
- [ ] Implement `/metrics` endpoint (Prometheus format)
- [ ] Write 10 unit tests for metric behavior
- [ ] Write 4 integration tests for metric export
- [ ] Document metric naming conventions

### Phase 4: Health Checks (1-2 hours)
- [ ] Create `HealthChecker` trait
- [ ] Implement checks for each runtime engine
- [ ] Implement checks for biomeOS dependencies
- [ ] Aggregate health status across checks
- [ ] Add `/health/live` and `/health/ready` endpoints
- [ ] Write 8 unit tests for health checks
- [ ] Write 4 integration tests for aggregation
- [ ] Document health check API

### Phase 5: Debug Endpoints (1-2 hours)
- [ ] Implement runtime inspection endpoints
- [ ] Implement execution tracing endpoints
- [ ] Implement configuration dump endpoints
- [ ] Add authentication for debug endpoints
- [ ] Rate-limit debug endpoints
- [ ] Write 9 unit tests for debug endpoints
- [ ] Write 4 integration tests for security
- [ ] Document debug API and security considerations

---

## 📈 **Expected Benefits**

### Operations
- **Root Cause Analysis**: Trace requests end-to-end
- **Performance Tuning**: Identify bottlenecks with spans
- **Alerting**: Rich metrics for alert rules
- **Troubleshooting**: Debug endpoints for live inspection

### Development
- **Local Debugging**: Pretty-printed logs with full context
- **Performance Profiling**: CPU and memory profiles
- **Integration Testing**: Validate trace propagation

### Production
- **SLO Monitoring**: Track latency, error rates, availability
- **Capacity Planning**: Resource utilization metrics
- **Incident Response**: Rapid diagnosis with full observability

---

## 🎓 **Design Principles**

### 1. Low Overhead
- <1ms tracing overhead per span
- Async export to avoid blocking
- Sampling for high-volume paths
- Efficient metric aggregation

### 2. Standard Protocols
- OpenTelemetry for tracing
- Prometheus for metrics
- JSON for structured logs
- HTTP for health checks

### 3. Contextual & Correlated
- Trace IDs in every log
- Span IDs for correlation
- Baggage for cross-cutting concerns
- Unified context propagation

### 4. Production-Ready
- Configurable sampling rates
- Resource limits (batch sizes)
- Security controls (debug endpoints)
- Graceful degradation if export fails

---

## 🔗 **Related Work**

### Dependencies
- `tracing` - Structured, composable tracing
- `tracing-opentelemetry` - OTLP export
- `tracing-subscriber` - Log formatting and filtering
- `prometheus` - Metrics collection and export
- `opentelemetry` - Distributed tracing standards

### Integration Points
- **Jaeger/Tempo**: Trace visualization
- **Prometheus/Grafana**: Metrics and dashboards
- **Loki**: Log aggregation
- **OpenTelemetry Collector**: Unified telemetry pipeline

### Future Enhancements (Phase 6+)
- Exemplars (link traces to metrics)
- Service mesh integration (sidecar tracing)
- Continuous profiling (pprof)
- Distributed context propagation (W3C TraceContext)
- Custom exporters (CloudWatch, DataDog, etc.)

---

## ✅ **Success Criteria**

### Functional
- ✅ Every request has a trace ID
- ✅ Spans propagate across all boundaries
- ✅ Logs include trace context
- ✅ Metrics cover all key operations
- ✅ Health checks expose all dependencies
- ✅ Debug endpoints provide runtime inspection

### Performance
- ✅ <1ms tracing overhead per span
- ✅ <5ms health check latency
- ✅ Metrics export <100ms latency

### Quality
- ✅ 45+ comprehensive tests
- ✅ Full documentation with examples
- ✅ Production-ready configuration
- ✅ Integration with standard tooling (Jaeger, Prometheus)

---

**Status**: Ready for implementation  
**Next Steps**: Begin Phase 1 (Distributed Tracing)  
**Timeline**: 8-10 hours total, can be done incrementally

---

*This roadmap transforms ToadStool from "basic monitoring" to "comprehensive observability," enabling confident operations at scale with full visibility into system behavior.*

