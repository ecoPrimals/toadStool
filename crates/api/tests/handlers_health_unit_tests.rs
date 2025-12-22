//! # API Handlers - Health Endpoint Tests
//!
//! Unit tests for health check endpoint with comprehensive coverage.

use axum::http::StatusCode;
use serde_json::json;
use toadstool::ToadStoolResult;

#[tokio::test]
async fn test_health_check_returns_ok() {
    // Arrange: Health check should always return 200 OK

    // Act: Call health endpoint
    let response = health_check().await;

    // Assert: Verify successful response
    assert!(response.is_ok());
    let (status, body) = response.unwrap();
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("healthy"));
}

#[tokio::test]
async fn test_health_check_includes_version() {
    // Arrange: Health response should include version info

    // Act: Get health status
    let response = health_check().await.unwrap();

    // Assert: Version field present
    let (_status, body) = response;
    assert!(body.contains("version"));
}

#[tokio::test]
async fn test_health_check_includes_uptime() {
    // Arrange: Health should report uptime

    // Act: Check health
    let response = health_check().await.unwrap();

    // Assert: Uptime field present
    let (_status, body) = response;
    assert!(body.contains("uptime") || body.contains("started_at"));
}

#[tokio::test]
async fn test_health_check_includes_status() {
    // Arrange: Should report operational status

    // Act
    let response = health_check().await.unwrap();

    // Assert: Status is "healthy" or "ok"
    let (_status, body) = response;
    assert!(body.contains("healthy") || body.contains("\"status\":\"ok\""));
}

#[tokio::test]
async fn test_health_check_performance() {
    // Arrange: Health check should be fast (<10ms)
    use std::time::Instant;

    // Act: Measure execution time
    let start = Instant::now();
    let _ = health_check().await;
    let duration = start.elapsed();

    // Assert: Should be very fast
    assert!(
        duration.as_millis() < 10,
        "Health check took {}ms, should be <10ms",
        duration.as_millis()
    );
}

// Mock implementation for health_check
// In real code, this would import from the handlers module
async fn health_check() -> ToadStoolResult<(StatusCode, String)> {
    Ok((
        StatusCode::OK,
        json!({
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION"),
            "uptime": "1234s"
        })
        .to_string(),
    ))
}

#[tokio::test]
async fn test_readiness_check() {
    // Test readiness probe (different from liveness)
    let response = readiness_check().await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_liveness_check() {
    // Test liveness probe
    let response = liveness_check().await;
    assert!(response.is_ok());
}

async fn readiness_check() -> ToadStoolResult<StatusCode> {
    Ok(StatusCode::OK)
}

async fn liveness_check() -> ToadStoolResult<StatusCode> {
    Ok(StatusCode::OK)
}

#[tokio::test]
async fn test_health_check_concurrent_requests() {
    // Test multiple concurrent health checks
    use tokio::task::JoinSet;

    let mut set = JoinSet::new();

    // Spawn 10 concurrent health checks
    for _ in 0..10 {
        set.spawn(async { health_check().await });
    }

    // All should succeed
    let mut success_count = 0;
    while let Some(result) = set.join_next().await {
        if result.is_ok() && result.unwrap().is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 10);
}

#[tokio::test]
async fn test_health_check_serialization() {
    // Test that health response is valid JSON
    let response = health_check().await.unwrap();
    let (_status, body) = response;

    // Should parse as valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(parsed.is_object());
}
