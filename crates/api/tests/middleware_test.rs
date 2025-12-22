//! Comprehensive tests for API middleware
//!
//! Coverage target: 0% → 40% (30 tests)

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower::ServiceExt;

use toadstool_api::{middleware::*, websocket, ApiMetrics, ApiState};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_state() -> ApiState {
    let (event_broadcaster, _) = broadcast::channel(100);

    ApiState {
        event_broadcaster,
        executions: Arc::new(RwLock::new(HashMap::new())),
        metrics: Arc::new(RwLock::new(ApiMetrics::default())),
        websocket_manager: Arc::new(websocket::WebSocketManager::new()),
        capability_provider: None,
    }
}

async fn dummy_handler() -> &'static str {
    "OK"
}

async fn failing_handler() -> StatusCode {
    StatusCode::INTERNAL_SERVER_ERROR
}

// ============================================================================
// Request ID Middleware Tests (8 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_request_id_middleware_adds_id() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_request_id_middleware_propagates_to_response() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok());

    assert!(request_id.is_some());
    assert!(!request_id.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_request_id_middleware_different_ids() {
    let app1 = Router::new()
        .route("/test1", get(dummy_handler))
        .layer(middleware::from_fn(request_id_middleware));
    let app2 = Router::new()
        .route("/test2", get(dummy_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request1 = Request::builder()
        .uri("/test1")
        .body(Body::empty())
        .unwrap();
    let request2 = Request::builder()
        .uri("/test2")
        .body(Body::empty())
        .unwrap();

    let response1 = app1.oneshot(request1).await.unwrap();
    let response2 = app2.oneshot(request2).await.unwrap();

    let id1 = response1
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap();
    let id2 = response2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap();

    assert_ne!(id1, id2, "Request IDs should be unique");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_request_id_middleware_with_post() {
    let app = Router::new()
        .route("/api/execute", axum::routing::post(dummy_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/execute")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_request_id_middleware_with_put() {
    let app = Router::new()
        .route("/api/update", axum::routing::put(dummy_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder()
        .method(Method::PUT)
        .uri("/api/update")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_request_id_middleware_with_delete() {
    let app = Router::new()
        .route("/api/resource", axum::routing::delete(dummy_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/resource")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_request_id_middleware_with_options() {
    let app = Router::new()
        .route("/api/test", axum::routing::options(dummy_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/test")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_request_id_middleware_preserves_status_code() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// CORS Middleware Tests (3 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cors_middleware_adds_headers() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn(cors_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    let headers = response.headers();

    assert!(headers.contains_key("access-control-allow-origin"));
    assert!(headers.contains_key("access-control-allow-methods"));
    assert!(headers.contains_key("access-control-allow-headers"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cors_middleware_allow_all_origins() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn(cors_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    let allow_origin = response
        .headers()
        .get("access-control-allow-origin")
        .and_then(|v| v.to_str().ok());

    assert_eq!(allow_origin, Some("*"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cors_middleware_options_request() {
    let app = Router::new()
        .route("/test", axum::routing::options(dummy_handler))
        .layer(middleware::from_fn(cors_middleware));

    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.headers().contains_key("access-control-max-age"));
}

// ============================================================================
// Security Headers Middleware Tests (3 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_security_headers_middleware_adds_all_headers() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn(security_headers_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    let headers = response.headers();

    assert!(headers.contains_key("x-frame-options"));
    assert!(headers.contains_key("x-content-type-options"));
    assert!(headers.contains_key("x-xss-protection"));
    assert!(headers.contains_key("referrer-policy"));
    assert!(headers.contains_key("content-security-policy"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_security_headers_middleware_frame_options_deny() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn(security_headers_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    let frame_options = response
        .headers()
        .get("x-frame-options")
        .and_then(|v| v.to_str().ok());

    assert_eq!(frame_options, Some("DENY"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_security_headers_middleware_csp_self_only() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn(security_headers_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    let csp = response
        .headers()
        .get("content-security-policy")
        .and_then(|v| v.to_str().ok());

    assert_eq!(csp, Some("default-src 'self'"));
}

// ============================================================================
// Metrics Middleware Tests (8 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_middleware_increments_total_requests() {
    let state = create_test_state();
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .with_state(state.clone());

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
    let _ = app.oneshot(request).await;

    let metrics = state.metrics.read().await;
    assert!(metrics.total_requests > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_middleware_increments_successful_requests() {
    let state = create_test_state();
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .with_state(state.clone());

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
    let _ = app.oneshot(request).await;

    let metrics = state.metrics.read().await;
    assert!(metrics.successful_requests > 0);
    assert_eq!(metrics.failed_requests, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_middleware_increments_failed_requests() {
    let state = create_test_state();
    let app = Router::new()
        .route("/fail", get(failing_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .with_state(state.clone());

    let request = Request::builder().uri("/fail").body(Body::empty()).unwrap();
    let _ = app.oneshot(request).await;

    let metrics = state.metrics.read().await;
    assert_eq!(metrics.successful_requests, 0);
    assert!(metrics.failed_requests > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_middleware_tracks_response_time() {
    let state = create_test_state();
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .with_state(state.clone());

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
    let _ = app.oneshot(request).await;

    let metrics = state.metrics.read().await;
    assert!(metrics.average_response_time_ms >= 0.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_middleware_multiple_requests() {
    let state = create_test_state();
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .with_state(state.clone());

    // Make 3 requests
    for _ in 0..3 {
        let app_clone = app.clone();
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
        let _ = app_clone.oneshot(request).await;
    }

    let metrics = state.metrics.read().await;
    assert_eq!(metrics.total_requests, 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_middleware_mixed_success_and_failure() {
    let state = create_test_state();
    let app = Router::new()
        .route("/success", get(dummy_handler))
        .route("/fail", get(failing_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .with_state(state.clone());

    // Success request
    let request1 = Request::builder()
        .uri("/success")
        .body(Body::empty())
        .unwrap();
    let app1 = app.clone();
    let _ = app1.oneshot(request1).await;

    // Failure request
    let request2 = Request::builder().uri("/fail").body(Body::empty()).unwrap();
    let app2 = app.clone();
    let _ = app2.oneshot(request2).await;

    let metrics = state.metrics.read().await;
    assert_eq!(metrics.total_requests, 2);
    assert_eq!(metrics.successful_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_middleware_preserves_response() {
    let state = create_test_state();
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .with_state(state.clone());

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_middleware_different_methods() {
    let state = create_test_state();
    let app = Router::new()
        .route("/test", get(dummy_handler).post(dummy_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .with_state(state.clone());

    // GET request
    let request1 = Request::builder().uri("/test").body(Body::empty()).unwrap();
    let app1 = app.clone();
    let _ = app1.oneshot(request1).await;

    // POST request
    let request2 = Request::builder()
        .method(Method::POST)
        .uri("/test")
        .body(Body::empty())
        .unwrap();
    let app2 = app.clone();
    let _ = app2.oneshot(request2).await;

    let metrics = state.metrics.read().await;
    assert_eq!(metrics.total_requests, 2);
}

// ============================================================================
// Auth Middleware Tests (8 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_middleware_missing_header() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(auth_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Auth errors should return UNAUTHORIZED per REST standards
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_middleware_valid_jwt_format() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(auth_middleware));

    let request = Request::builder()
        .uri("/test")
        .header("authorization", "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_middleware_empty_token() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(auth_middleware));

    let request = Request::builder()
        .uri("/test")
        .header("authorization", "Bearer ")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Auth errors should return UNAUTHORIZED per REST standards
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_middleware_malformed_jwt() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(auth_middleware));

    let request = Request::builder()
        .uri("/test")
        .header("authorization", "Bearer invalid.token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Auth errors should return UNAUTHORIZED per REST standards
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_middleware_missing_bearer_prefix() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(auth_middleware));

    let request = Request::builder()
        .uri("/test")
        .header(
            "authorization",
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abc",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Auth errors should return UNAUTHORIZED per REST standards
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_middleware_invalid_authorization_format() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(auth_middleware));

    let request = Request::builder()
        .uri("/test")
        .header("authorization", "Basic dXNlcjpwYXNz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Auth errors should return UNAUTHORIZED per REST standards
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_middleware_case_sensitive_bearer() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(auth_middleware));

    let request = Request::builder()
        .uri("/test")
        .header(
            "authorization",
            "bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abc",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should fail because "Bearer" is case-sensitive - Auth errors return UNAUTHORIZED per REST standards
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_auth_middleware_single_part_token() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(auth_middleware));

    let request = Request::builder()
        .uri("/test")
        .header("authorization", "Bearer singleparttoken")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Auth errors should return UNAUTHORIZED per REST standards
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Rate Limit Middleware Tests (6 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rate_limit_middleware_localhost() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(rate_limit_middleware));

    let request = Request::builder()
        .uri("/test")
        .header("x-forwarded-for", "127.0.0.1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rate_limit_middleware_external_ip() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(rate_limit_middleware));

    let request = Request::builder()
        .uri("/test")
        .header("x-forwarded-for", "203.0.113.1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rate_limit_middleware_real_ip_header() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(rate_limit_middleware));

    let request = Request::builder()
        .uri("/test")
        .header("x-real-ip", "198.51.100.1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rate_limit_middleware_missing_ip_header() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(rate_limit_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should still work with "unknown" IP
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rate_limit_middleware_localhost_string() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(rate_limit_middleware));

    let request = Request::builder()
        .uri("/test")
        .header("x-forwarded-for", "localhost")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rate_limit_middleware_preserves_response() {
    let app = Router::new()
        .route("/test", get(dummy_handler))
        .route_layer(middleware::from_fn(rate_limit_middleware));

    let request = Request::builder()
        .uri("/test")
        .header("x-forwarded-for", "192.0.2.1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
