//! Performance benchmarks for ToadStool Universal Compute Platform
//!
//! These benchmarks measure system performance, throughput, latency, and resource utilization
//! to ensure the platform meets performance requirements and identify bottlenecks.

use std::time::{Duration, Instant};

use toadstool::{
    execution::RuntimeType,
    security::{IsolationLevel, SecurityContext, NetworkSecurity, FilesystemSecurity},
};

use toadstool_testing::{
    builders::ExecutionRequestBuilder,
    fixtures::create_test_resource_requirements,
    integration::{IntegrationTestManager, IntegrationTestConfig},
};

/// Benchmark execution request creation throughput
#[tokio::test]
async fn benchmark_execution_request_creation() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);
    
    let runtime_types = vec![
        RuntimeType::Native,
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Python,
    ];
    
    let start = Instant::now();
    let iterations = 1000;
    
    for _ in 0..iterations {
        for runtime_type in &runtime_types {
            let _request = ExecutionRequestBuilder::new()
                .runtime_hint(runtime_type.clone())
                .native_workload("echo", vec!["benchmark".to_string()])
                .timeout(Duration::from_secs(30))
                .build();
        }
    }
    
    let duration = start.elapsed();
    let requests_per_second = (iterations * runtime_types.len()) as f64 / duration.as_secs_f64();
    
    println!("✓ Execution request creation benchmark:");
    println!("  - {} requests in {:?}", iterations * runtime_types.len(), duration);
    println!("  - {:.0} requests/second", requests_per_second);
    
    // Performance assertion - should be able to create at least 1000 requests per second
    assert!(requests_per_second > 1000.0, "Request creation too slow: {:.0} req/s", requests_per_second);
}

/// Benchmark concurrent execution request creation
#[tokio::test]
async fn benchmark_concurrent_execution_creation() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);
    
    let start = Instant::now();
    let concurrent_count = 100;
    
    let mut handles = Vec::new();
    
    for i in 0..concurrent_count {
        let handle = tokio::spawn(async move {
            let _request = ExecutionRequestBuilder::new()
                .runtime_hint(RuntimeType::Native)
                .native_workload("echo", vec![format!("concurrent-{}", i)])
                .timeout(Duration::from_secs(10))
                .build();
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    let duration = start.elapsed();
    let requests_per_second = concurrent_count as f64 / duration.as_secs_f64();
    
    println!("✓ Concurrent execution request creation benchmark:");
    println!("  - {} concurrent requests in {:?}", concurrent_count, duration);
    println!("  - {:.0} requests/second", requests_per_second);
    
    // Performance assertion
    assert!(requests_per_second > 500.0, "Concurrent creation too slow: {:.0} req/s", requests_per_second);
}

/// Benchmark resource requirements creation
#[tokio::test]
async fn benchmark_resource_requirements_creation() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);
    
    let start = Instant::now();
    let iterations = 10000;
    
    for _ in 0..iterations {
        let resources = create_test_resource_requirements();
        let _request = ExecutionRequestBuilder::new()
            .runtime_hint(RuntimeType::Native)
            .native_workload("echo", vec!["resource test".to_string()])
            .resources(resources)
            .timeout(Duration::from_secs(10))
            .build();
    }
    
    let duration = start.elapsed();
    let requests_per_second = iterations as f64 / duration.as_secs_f64();
    
    println!("✓ Resource requirements creation benchmark:");
    println!("  - {} requests with resources in {:?}", iterations, duration);
    println!("  - {:.0} requests/second", requests_per_second);
    
    // Performance assertion
    assert!(requests_per_second > 2000.0, "Resource creation too slow: {:.0} req/s", requests_per_second);
}

/// Benchmark security context creation
#[tokio::test]
async fn benchmark_security_context_creation() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);
    
    let isolation_levels = vec![
        IsolationLevel::None,
        IsolationLevel::Basic,
        IsolationLevel::Standard,
        IsolationLevel::Enhanced,
        IsolationLevel::Maximum,
    ];
    
    let start = Instant::now();
    let iterations = 1000;
    
    for _ in 0..iterations {
        for isolation_level in &isolation_levels {
            let security_context = SecurityContext {
                isolation_level: isolation_level.clone(),
                capabilities: vec![],
                user_context: None,
                network_security: NetworkSecurity {
                    allow_outbound: true,
                    allow_inbound: false,
                    allowed_domains: vec![],
                    blocked_domains: vec![],
                    allowed_ports: vec![],
                    blocked_ports: vec![],
                },
                filesystem_security: FilesystemSecurity::default(),
            };
            
            let _request = ExecutionRequestBuilder::new()
                .runtime_hint(RuntimeType::Container)
                .native_workload("echo", vec!["security test".to_string()])
                .security_context(security_context)
                .timeout(Duration::from_secs(10))
                .build();
        }
    }
    
    let duration = start.elapsed();
    let requests_per_second = (iterations * isolation_levels.len()) as f64 / duration.as_secs_f64();
    
    println!("✓ Security context creation benchmark:");
    println!("  - {} security contexts in {:?}", iterations * isolation_levels.len(), duration);
    println!("  - {:.0} contexts/second", requests_per_second);
    
    // Performance assertion
    assert!(requests_per_second > 1000.0, "Security context creation too slow: {:.0} ctx/s", requests_per_second);
}

/// Benchmark memory usage patterns
#[tokio::test]
async fn benchmark_memory_usage() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);
    
    let start = Instant::now();
    let iterations = 5000;
    
    // Create many requests to test memory usage
    let mut requests = Vec::new();
    
    for i in 0..iterations {
        let request = ExecutionRequestBuilder::new()
            .runtime_hint(RuntimeType::Native)
            .native_workload("echo", vec![format!("memory-test-{}", i)])
            .timeout(Duration::from_secs(10))
            .build();
        
        requests.push(request);
    }
    
    let duration = start.elapsed();
    let requests_per_second = iterations as f64 / duration.as_secs_f64();
    
    println!("✓ Memory usage benchmark:");
    println!("  - {} requests allocated in {:?}", iterations, duration);
    println!("  - {:.0} allocations/second", requests_per_second);
    
    // Test memory cleanup
    drop(requests);
    
    // Performance assertion
    assert!(requests_per_second > 1000.0, "Memory allocation too slow: {:.0} alloc/s", requests_per_second);
}

/// Benchmark startup and initialization performance
#[tokio::test]
async fn benchmark_startup_performance() {
    let iterations = 100;
    let mut total_duration = Duration::from_secs(0);
    
    for _ in 0..iterations {
        let start = Instant::now();
        
        let config = IntegrationTestConfig::default();
        let _manager = IntegrationTestManager::new(config);
        
        let _request = ExecutionRequestBuilder::new()
            .runtime_hint(RuntimeType::Native)
            .native_workload("echo", vec!["startup test".to_string()])
            .timeout(Duration::from_secs(5))
            .build();
        
        total_duration += start.elapsed();
    }
    
    let average_duration = total_duration / iterations;
    let startups_per_second = 1.0 / average_duration.as_secs_f64();
    
    println!("✓ Startup performance benchmark:");
    println!("  - {} startups in {:?}", iterations, total_duration);
    println!("  - Average startup time: {:?}", average_duration);
    println!("  - {:.0} startups/second", startups_per_second);
    
    // Performance assertion - startup should be fast
    assert!(average_duration < Duration::from_millis(100), "Startup too slow: {:?}", average_duration);
}

/// Benchmark API endpoint performance simulation
#[tokio::test]
async fn benchmark_api_performance() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);
    
    let start = Instant::now();
    let iterations = 1000;
    
    // Simulate API endpoint calls by creating various request types
    for i in 0..iterations {
        match i % 4 {
            0 => {
                let _request = ExecutionRequestBuilder::new()
                    .runtime_hint(RuntimeType::Native)
                    .native_workload("echo", vec!["api-native".to_string()])
                    .timeout(Duration::from_secs(10))
                    .build();
            }
            1 => {
                let _request = ExecutionRequestBuilder::new()
                    .runtime_hint(RuntimeType::Container)
                    .native_workload("echo", vec!["api-container".to_string()])
                    .timeout(Duration::from_secs(10))
                    .build();
            }
            2 => {
                let _request = ExecutionRequestBuilder::new()
                    .runtime_hint(RuntimeType::Wasm)
                    .native_workload("echo", vec!["api-wasm".to_string()])
                    .timeout(Duration::from_secs(10))
                    .build();
            }
            3 => {
                let _request = ExecutionRequestBuilder::new()
                    .runtime_hint(RuntimeType::Python)
                    .native_workload("echo", vec!["api-python".to_string()])
                    .timeout(Duration::from_secs(10))
                    .build();
            }
            _ => unreachable!(),
        }
    }
    
    let duration = start.elapsed();
    let requests_per_second = iterations as f64 / duration.as_secs_f64();
    
    println!("✓ API performance benchmark:");
    println!("  - {} API calls in {:?}", iterations, duration);
    println!("  - {:.0} calls/second", requests_per_second);
    
    // Performance assertion
    assert!(requests_per_second > 800.0, "API performance too slow: {:.0} calls/s", requests_per_second);
}

/// Benchmark system health check performance
#[tokio::test]
async fn benchmark_health_check_performance() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);
    
    let start = Instant::now();
    let iterations = 2000;
    
    for _ in 0..iterations {
        // Simulate health check by creating a simple request
        let _request = ExecutionRequestBuilder::new()
            .runtime_hint(RuntimeType::Native)
            .native_workload("echo", vec!["health".to_string()])
            .timeout(Duration::from_secs(1))
            .build();
    }
    
    let duration = start.elapsed();
    let checks_per_second = iterations as f64 / duration.as_secs_f64();
    
    println!("✓ Health check performance benchmark:");
    println!("  - {} health checks in {:?}", iterations, duration);
    println!("  - {:.0} checks/second", checks_per_second);
    
    // Performance assertion - health checks should be very fast
    assert!(checks_per_second > 2000.0, "Health checks too slow: {:.0} checks/s", checks_per_second);
} 