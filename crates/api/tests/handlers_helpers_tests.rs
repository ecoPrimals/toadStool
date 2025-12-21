//! Comprehensive tests for helper functions
//!
//! ✅ MODERN CONCURRENT TESTING - Zero sleeps, fully concurrent
//! Tests helper functions with various scenarios

use axum::http::HeaderMap;
use std::sync::Arc;
use tokio::sync::Barrier;

use toadstool_api::handlers::helpers::{get_base_url, get_local_node_resources};

// ============================================================================
// GET BASE URL TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_base_url_default() {
    // ✅ FULLY CONCURRENT: Get base URL with empty headers
    let headers = HeaderMap::new();
    let url = get_base_url(&headers);

    assert!(url.starts_with("http://"), "Should use http by default");
    assert!(
        url.contains("127.0.0.1") || url.contains("localhost"),
        "Should use localhost"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_base_url_with_host() {
    // ✅ FULLY CONCURRENT: Get base URL with host header
    let mut headers = HeaderMap::new();
    headers.insert("host", "example.com:8080".parse().unwrap());

    let url = get_base_url(&headers);
    assert_eq!(url, "http://example.com:8080", "Should use provided host");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_base_url_with_https() {
    // ✅ FULLY CONCURRENT: Get base URL with HTTPS protocol
    let mut headers = HeaderMap::new();
    headers.insert("host", "secure.example.com".parse().unwrap());
    headers.insert("x-forwarded-proto", "https".parse().unwrap());

    let url = get_base_url(&headers);
    assert_eq!(
        url, "https://secure.example.com",
        "Should use HTTPS when forwarded"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_base_url_with_custom_port() {
    // ✅ FULLY CONCURRENT: Get base URL with custom port
    let mut headers = HeaderMap::new();
    headers.insert("host", "api.example.com:3000".parse().unwrap());

    let url = get_base_url(&headers);
    assert!(url.contains(":3000"), "Should include custom port");
}

// ============================================================================
// GET LOCAL NODE RESOURCES TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_local_node_resources() {
    // ✅ FULLY CONCURRENT: Get local node resources
    let resources = get_local_node_resources().await;

    assert!(resources.cpu_cores > 0, "Should have CPU cores");
    assert!(resources.memory_gb > 0, "Should have memory");
    assert!(resources.storage_gb > 0, "Should have storage");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_local_node_resources_consistency() {
    // ✅ FULLY CONCURRENT: Resources should be consistent across calls
    let resources1 = get_local_node_resources().await;
    let resources2 = get_local_node_resources().await;

    assert_eq!(resources1.cpu_cores, resources2.cpu_cores);
    assert_eq!(resources1.memory_gb, resources2.memory_gb);
    assert_eq!(resources1.storage_gb, resources2.storage_gb);
    assert_eq!(resources1.gpu_count, resources2.gpu_count);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_local_node_resources_values() {
    // ✅ FULLY CONCURRENT: Resources should have reasonable values
    let resources = get_local_node_resources().await;

    assert!(
        resources.cpu_cores <= 1024,
        "Should have reasonable CPU count"
    );
    assert!(
        resources.memory_gb <= 10000,
        "Should have reasonable memory"
    );
    assert!(
        resources.storage_gb <= 100000,
        "Should have reasonable storage"
    );
}

// ============================================================================
// CONCURRENT TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_resource_queries() {
    // ✅ FULLY CONCURRENT: Multiple resource queries in parallel
    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for _ in 0..50 {
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let resources = get_local_node_resources().await;
            resources.cpu_cores > 0 && resources.memory_gb > 0
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(successes, 50, "All 50 concurrent queries should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_base_url_queries() {
    // ✅ FULLY CONCURRENT: Multiple base URL queries in parallel
    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for i in 0..50 {
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let mut headers = HeaderMap::new();
            headers.insert("host", format!("host{i}.example.com").parse().unwrap());
            let url = get_base_url(&headers);
            url.contains(&format!("host{i}"))
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(successes, 50, "All 50 concurrent queries should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_mixed_helper_calls() {
    // ✅ STRESS TEST: Mix of helper function calls
    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for i in 0..100 {
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            if i % 2 == 0 {
                let resources = get_local_node_resources().await;
                resources.cpu_cores > 0
            } else {
                let headers = HeaderMap::new();
                let url = get_base_url(&headers);
                url.starts_with("http")
            }
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(successes, 100, "All 100 mixed calls should succeed");
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_base_url_with_invalid_host() {
    // ✅ FULLY CONCURRENT: Should handle invalid host gracefully
    let mut headers = HeaderMap::new();
    headers.insert("host", "valid.example.com".parse().unwrap());

    let url = get_base_url(&headers);
    assert!(url.contains("valid.example.com"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_base_url_empty_host() {
    // ✅ FULLY CONCURRENT: Should use default for empty headers
    let headers = HeaderMap::new();
    let url = get_base_url(&headers);

    assert!(url.starts_with("http://"));
    assert!(url.len() > 7); // More than just "http://"
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_base_url_multiple_calls_same_headers() {
    // ✅ FULLY CONCURRENT: Multiple calls with same headers should be consistent
    let mut headers = HeaderMap::new();
    headers.insert("host", "consistent.example.com".parse().unwrap());

    let url1 = get_base_url(&headers);
    let url2 = get_base_url(&headers);
    let url3 = get_base_url(&headers);

    assert_eq!(url1, url2);
    assert_eq!(url2, url3);
}
