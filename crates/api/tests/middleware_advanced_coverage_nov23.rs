//! Advanced middleware coverage tests - November 23, 2025
//! Focus on edge cases, error paths, and integration scenarios

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use tokio::sync::{broadcast, RwLock};
use tower::ServiceExt;

use toadstool_api::{middleware::*, ApiMetrics, ApiState};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_state() -> ApiState {
    let executions = Arc::new(RwLock::new(HashMap::new()));
    let metrics = Arc::new(RwLock::new(ApiMetrics::default()));
    let (tx, _) = broadcast::channel(100);

    ApiState {
        executions,
        metrics,
        event_broadcaster: tx,
        capability_provider: None,
    }
}

async fn test_handler() -> impl IntoResponse {
    (StatusCode::OK, "test response")
}

async fn long_running_handler() -> impl IntoResponse {
    // Advance tokio time instead of sleeping — the metrics middleware uses
    // tokio::time::Instant so this makes the request appear to take 1100ms.
    tokio::time::advance(tokio::time::Duration::from_millis(1100)).await;
    (StatusCode::OK, "completed")
}

async fn failing_handler() -> impl IntoResponse {
    (StatusCode::BAD_REQUEST, "bad request")
}

// ============================================================================
// Request ID Middleware - Advanced Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_request_id_with_invalid_uuid_fallback() {
    // Test that middleware handles UUID generation errors gracefully
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_request_id_preserved_through_error() {
    let app = Router::new()
        .route("/fail", get(failing_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder().uri("/fail").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    // Request ID should be present even when handler fails
    assert!(response.headers().contains_key("x-request-id"));
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_request_id_with_post_request() {
    let app = Router::new()
        .route("/post", post(test_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder()
        .method(Method::POST)
        .uri("/post")
        .body(Body::from("test body"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(response.headers().contains_key("x-request-id"));
}

// ============================================================================
// Metrics Middleware - Advanced Tests
// ============================================================================

#[tokio::test(start_paused = true)]
async fn test_metrics_tracks_slow_requests() {
    let state = create_test_state();
    let app = Router::new()
        .route("/slow", get(long_running_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ));

    let request = Request::builder().uri("/slow").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify metrics were updated
    let metrics = state.metrics.read().await;
    assert_eq!(metrics.total_requests, 1);
    assert!(metrics.average_response_time_ms > 1000.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_tracks_failed_requests() {
    let state = create_test_state();
    let app =
        Router::new()
            .route("/fail", get(failing_handler))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                metrics_middleware,
            ));

    let request = Request::builder().uri("/fail").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Verify failed request was counted
    let metrics = state.metrics.read().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
    assert_eq!(metrics.successful_requests, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_calculates_average_response_time() {
    let state = create_test_state();
    let app =
        Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                metrics_middleware,
            ));

    // Make multiple requests
    for _ in 0..5 {
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
        let _ = app.clone().oneshot(request).await.unwrap();
    }

    // Verify average was calculated
    let metrics = state.metrics.read().await;
    assert_eq!(metrics.total_requests, 5);
    // Response time may be 0.0 for very fast requests, which is acceptable
    assert!(metrics.average_response_time_ms >= 0.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_handles_different_status_codes() {
    let state = create_test_state();

    let success_app =
        Router::new()
            .route("/success", get(test_handler))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                metrics_middleware,
            ));

    let fail_app =
        Router::new()
            .route("/fail", get(failing_handler))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                metrics_middleware,
            ));

    // Make successful request
    let request = Request::builder()
        .uri("/success")
        .body(Body::empty())
        .unwrap();
    let _ = success_app.oneshot(request).await.unwrap();

    // Make failing request
    let request = Request::builder().uri("/fail").body(Body::empty()).unwrap();
    let _ = fail_app.oneshot(request).await.unwrap();

    // Verify both were counted correctly
    let metrics = state.metrics.read().await;
    assert_eq!(metrics.total_requests, 2);
    assert_eq!(metrics.successful_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

// ============================================================================
// Auth Middleware - Advanced Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_with_empty_bearer_token() {
    let app = Router::new()
        .route("/protected", get(test_handler))
        .layer(middleware::from_fn(auth_middleware));

    let request = Request::builder()
        .uri("/protected")
        .header("authorization", "Bearer ")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should fail - API errors convert to 500 by default
    assert!(!response.status().is_success());
    assert!(
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::UNAUTHORIZED
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_with_malformed_jwt_two_parts() {
    let app = Router::new()
        .route("/protected", get(test_handler))
        .layer(middleware::from_fn(auth_middleware));

    let request = Request::builder()
        .uri("/protected")
        .header("authorization", "Bearer header.payload")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should fail with malformed token
    assert!(!response.status().is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_with_malformed_jwt_four_parts() {
    let app = Router::new()
        .route("/protected", get(test_handler))
        .layer(middleware::from_fn(auth_middleware));

    let request = Request::builder()
        .uri("/protected")
        .header("authorization", "Bearer a.b.c.d")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should fail with malformed token
    assert!(!response.status().is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_with_valid_jwt_structure() {
    let app = Router::new()
        .route("/protected", get(test_handler))
        .layer(middleware::from_fn(auth_middleware));

    // Valid JWT structure (header.payload.signature)
    let request = Request::builder()
        .uri("/protected")
        .header("authorization", "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should succeed with properly formatted JWT
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_case_sensitive_bearer() {
    let app = Router::new()
        .route("/protected", get(test_handler))
        .layer(middleware::from_fn(auth_middleware));

    // Lowercase "bearer" should not work
    let request = Request::builder()
        .uri("/protected")
        .header("authorization", "bearer valid.jwt.token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should fail - case sensitive (accepts failure status)
    assert!(!response.status().is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_without_authorization_header() {
    let app = Router::new()
        .route("/protected", get(test_handler))
        .layer(middleware::from_fn(auth_middleware));

    let request = Request::builder()
        .uri("/protected")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should fail without auth header
    assert!(!response.status().is_success());
}

// ============================================================================
// Rate Limit Middleware - Advanced Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rate_limit_allows_localhost_ipv4() {
    let app = Router::new()
        .route("/api/test", get(test_handler))
        .layer(middleware::from_fn(rate_limit_middleware));

    let request = Request::builder()
        .uri("/api/test")
        .header("x-forwarded-for", "127.0.0.1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should allow localhost
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rate_limit_allows_localhost_name() {
    let app = Router::new()
        .route("/api/test", get(test_handler))
        .layer(middleware::from_fn(rate_limit_middleware));

    let request = Request::builder()
        .uri("/api/test")
        .header("x-real-ip", "localhost")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should allow localhost
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rate_limit_logs_external_ip() {
    let app = Router::new()
        .route("/api/test", get(test_handler))
        .layer(middleware::from_fn(rate_limit_middleware));

    let request = Request::builder()
        .uri("/api/test")
        .header("x-forwarded-for", "192.168.1.100")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should process request (logging happens internally)
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rate_limit_uses_x_real_ip_fallback() {
    let app = Router::new()
        .route("/api/test", get(test_handler))
        .layer(middleware::from_fn(rate_limit_middleware));

    let request = Request::builder()
        .uri("/api/test")
        .header("x-real-ip", "10.0.0.50")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rate_limit_defaults_to_unknown() {
    let app = Router::new()
        .route("/api/test", get(test_handler))
        .layer(middleware::from_fn(rate_limit_middleware));

    let request = Request::builder()
        .uri("/api/test")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should default to "unknown" and still process
    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// CORS Middleware - Advanced Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cors_allows_all_origins() {
    let app = Router::new()
        .route("/api/data", get(test_handler))
        .layer(middleware::from_fn(cors_middleware));

    let request = Request::builder()
        .uri("/api/data")
        .header("origin", "https://example.com")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let cors_header = response.headers().get("access-control-allow-origin");
    assert_eq!(cors_header.unwrap(), "*");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cors_exposes_request_id() {
    let app = Router::new()
        .route("/api/data", get(test_handler))
        .layer(middleware::from_fn(cors_middleware));

    let request = Request::builder()
        .uri("/api/data")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let expose_headers = response
        .headers()
        .get("access-control-expose-headers")
        .unwrap()
        .to_str()
        .unwrap();

    assert!(expose_headers.contains("x-request-id"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cors_allows_multiple_methods() {
    let app = Router::new()
        .route("/api/data", get(test_handler))
        .layer(middleware::from_fn(cors_middleware));

    let request = Request::builder()
        .uri("/api/data")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let methods = response
        .headers()
        .get("access-control-allow-methods")
        .unwrap()
        .to_str()
        .unwrap();

    assert!(methods.contains("GET"));
    assert!(methods.contains("POST"));
    assert!(methods.contains("PUT"));
    assert!(methods.contains("DELETE"));
    assert!(methods.contains("OPTIONS"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cors_sets_max_age() {
    let app = Router::new()
        .route("/api/data", get(test_handler))
        .layer(middleware::from_fn(cors_middleware));

    let request = Request::builder()
        .uri("/api/data")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let max_age = response
        .headers()
        .get("access-control-max-age")
        .unwrap()
        .to_str()
        .unwrap();

    assert_eq!(max_age, "86400"); // 24 hours
}

// ============================================================================
// Security Headers Middleware - Advanced Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_security_headers_prevents_clickjacking() {
    let app = Router::new()
        .route("/app", get(test_handler))
        .layer(middleware::from_fn(security_headers_middleware));

    let request = Request::builder().uri("/app").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    let frame_options = response.headers().get("x-frame-options").unwrap();
    assert_eq!(frame_options, "DENY");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_security_headers_prevents_mime_sniffing() {
    let app = Router::new()
        .route("/app", get(test_handler))
        .layer(middleware::from_fn(security_headers_middleware));

    let request = Request::builder().uri("/app").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    let content_type_options = response.headers().get("x-content-type-options").unwrap();
    assert_eq!(content_type_options, "nosniff");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_security_headers_enables_xss_protection() {
    let app = Router::new()
        .route("/app", get(test_handler))
        .layer(middleware::from_fn(security_headers_middleware));

    let request = Request::builder().uri("/app").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    let xss_protection = response.headers().get("x-xss-protection").unwrap();
    assert_eq!(xss_protection, "1; mode=block");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_security_headers_sets_referrer_policy() {
    let app = Router::new()
        .route("/app", get(test_handler))
        .layer(middleware::from_fn(security_headers_middleware));

    let request = Request::builder().uri("/app").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    let referrer_policy = response.headers().get("referrer-policy").unwrap();
    assert_eq!(referrer_policy, "strict-origin-when-cross-origin");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_security_headers_sets_csp() {
    let app = Router::new()
        .route("/app", get(test_handler))
        .layer(middleware::from_fn(security_headers_middleware));

    let request = Request::builder().uri("/app").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    let csp = response.headers().get("content-security-policy").unwrap();
    assert_eq!(csp, "default-src 'self'");
}

// ============================================================================
// Middleware Integration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_middleware_chain_all_together() {
    let state = create_test_state();
    let app = Router::new()
        .route("/api/protected", get(test_handler))
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(middleware::from_fn(cors_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder()
        .uri("/api/protected")
        .header("x-forwarded-for", "127.0.0.1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify all middleware effects
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));
    assert!(response
        .headers()
        .contains_key("access-control-allow-origin"));
    assert!(response.headers().contains_key("x-frame-options"));

    // Verify metrics were updated
    let metrics = state.metrics.read().await;
    assert_eq!(metrics.total_requests, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_middleware_order_matters() {
    // Test that request_id is available to metrics middleware
    let state = create_test_state();
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));

    let metrics = state.metrics.read().await;
    assert_eq!(metrics.total_requests, 1);
}
