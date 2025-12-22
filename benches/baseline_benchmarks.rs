//! Performance Benchmarking Suite
//!
//! Establishes baseline metrics for critical paths in ToadStool

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;
use tokio::runtime::Runtime;

// Import ToadStool types
use toadstool::execution::{ExecutionRequest, RuntimeOrchestrator, RuntimeSelectionStrategy};
use toadstool::runtime::{RuntimeConfig, RuntimeType};
use toadstool::{WorkloadSpec, SecurityContext, ExecutionInput};
use toadstool::resources::ResourceRequirements;
use uuid::Uuid;
use std::collections::HashMap;

/// Benchmark runtime orchestrator initialization
fn bench_orchestrator_init(c: &mut Criterion) {
    c.bench_function("orchestrator_init", |b| {
        b.iter(|| {
            let orch = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
            black_box(orch);
        });
    });
}

/// Benchmark capability-based discovery
fn bench_discovery(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("capability_discovery", |b| {
        b.to_async(&rt).iter(|| async {
            // Benchmark discovery engine lookup
            let result = toadstool::discovery::discover_orchestration().await;
            black_box(result);
        });
    });
}

/// Benchmark workload request creation
fn bench_workload_creation(c: &mut Criterion) {
    c.bench_function("workload_request_creation", |b| {
        b.iter(|| {
            let request = ExecutionRequest {
                execution_id: Uuid::new_v4(),
                workload: WorkloadSpec::Container {
                    image: "alpine:latest".to_string(),
                    command: Some(vec!["echo".to_string()]),
                    args: Some(vec!["test".to_string()]),
                    env_vars: HashMap::new(),
                    working_dir: None,
                    volumes: vec![],
                    ports: vec![],
                    registry_auth: None,
                },
                runtime_hint: None,
                resources: ResourceRequirements::default(),
                security_context: SecurityContext::default(),
                timeout: Some(Duration::from_secs(30)),
                environment: HashMap::new(),
                input_data: ExecutionInput::default(),
                callback_config: None,
            };
            black_box(request);
        });
    });
}

/// Benchmark concurrent orchestrator access
fn bench_concurrent_access(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_access");
    
    for concurrency in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async move {
                    let orchestrator = std::sync::Arc::new(
                        RuntimeOrchestrator::new(RuntimeSelectionStrategy::LoadBalanced)
                    );
                    
                    let mut handles = Vec::new();
                    for _ in 0..concurrency {
                        let orch = orchestrator.clone();
                        handles.push(tokio::spawn(async move {
                            // Simulate lightweight operation
                            let _ = orch.clone();
                        }));
                    }
                    
                    for handle in handles {
                        let _ = handle.await;
                    }
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark configuration parsing
fn bench_config_parsing(c: &mut Criterion) {
    use toadstool_config::Config;
    
    c.bench_function("config_parse", |b| {
        b.iter(|| {
            let config = Config::from_env();
            black_box(config);
        });
    });
}

/// Benchmark port resolution with environment overrides
fn bench_port_resolution(c: &mut Criterion) {
    use toadstool_config::ports;
    
    c.bench_function("port_resolution", |b| {
        b.iter(|| {
            let port = ports::server_port();
            black_box(port);
        });
    });
}

/// Benchmark resource requirement validation
fn bench_resource_validation(c: &mut Criterion) {
    c.bench_function("resource_validation", |b| {
        b.iter(|| {
            let requirements = ResourceRequirements::default();
            // Validation happens on construction
            black_box(requirements);
        });
    });
}

/// Benchmark UUID generation (used for execution IDs)
fn bench_uuid_generation(c: &mut Criterion) {
    c.bench_function("uuid_generation", |b| {
        b.iter(|| {
            let id = Uuid::new_v4();
            black_box(id);
        });
    });
}

criterion_group!(
    benches,
    bench_orchestrator_init,
    bench_discovery,
    bench_workload_creation,
    bench_concurrent_access,
    bench_config_parsing,
    bench_port_resolution,
    bench_resource_validation,
    bench_uuid_generation,
);

criterion_main!(benches);

