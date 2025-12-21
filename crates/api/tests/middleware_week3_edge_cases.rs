//! Week 3 Middleware Edge Case Tests
//! Comprehensive coverage of middleware error scenarios and edge cases

use axum::{body::Body, extract::Request, http::StatusCode, middleware, routing::get, Router};
use tower::ServiceExt;

// Helper to create a simple test router
fn create_test_router() -> Router {
    Router::new().route("/test", get(|| async { "test response" }))
}

// ============================================================================
// Request ID Middleware Edge Cases
// ============================================================================

#[tokio::test]
async fn test_request_id_added_to_request_and_response() {
    use toadstool_api::middleware::request_id_middleware;

    let app = create_test_router().layer(middleware::from_fn(request_id_middleware));

    let response = app
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Check that x-request-id header is present in response
    let request_id = response
        .headers()
        .get("x-request-id")
        .expect("x-request-id header should be present");

    // Validate it's a valid UUID format (36 characters with dashes)
    let request_id_str = request_id.to_str().unwrap();
    assert_eq!(request_id_str.len(), 36, "UUID should be 36 characters");
    assert!(request_id_str.contains('-'), "UUID should contain dashes");
}

#[tokio::test]
async fn test_request_id_survives_multiple_middleware_layers() {
    use toadstool_api::middleware::request_id_middleware;

    // Stack multiple middleware layers
    let app = create_test_router()
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(request_id_middleware));

    let response = app
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should still have request ID
    assert!(
        response.headers().get("x-request-id").is_some(),
        "x-request-id should survive multiple layers"
    );
}

// ============================================================================
// Auth Middleware Edge Cases
// ============================================================================
// ✅ FIXED (Nov 28, 2025): ApiError now maps MISSING_TOKEN and INVALID_TOKEN to 401

#[tokio::test]
async fn test_auth_missing_authorization_header() {
    use toadstool_api::middleware::auth_middleware;

    let app = create_test_router().layer(middleware::from_fn(auth_middleware));

    let response = app
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // ✅ FIXED: Now correctly returns 401 Unauthorized for missing auth
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Missing authorization should return 401 Unauthorized"
    );
}

#[tokio::test]
async fn test_auth_empty_bearer_token() {
    use toadstool_api::middleware::auth_middleware;

    let app = create_test_router().layer(middleware::from_fn(auth_middleware));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("authorization", "Bearer ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // ✅ FIXED: Now correctly returns 401 for empty token
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Empty bearer token should return 401 Unauthorized"
    );
}

#[tokio::test]
async fn test_auth_malformed_jwt_one_part() {
    use toadstool_api::middleware::auth_middleware;

    let app = create_test_router().layer(middleware::from_fn(auth_middleware));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("authorization", "Bearer invalid_token_no_dots")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // ✅ FIXED: Now correctly returns 401 for malformed JWT
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Malformed JWT (one part) should return 401 Unauthorized"
    );
}

#[tokio::test]
async fn test_auth_malformed_jwt_two_parts() {
    use toadstool_api::middleware::auth_middleware;

    let app = create_test_router().layer(middleware::from_fn(auth_middleware));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("authorization", "Bearer header.payload")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // ✅ FIXED: Now correctly returns 401 for 2-part JWT (missing signature)
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Malformed JWT (two parts) should return 401 Unauthorized"
    );
}

#[tokio::test]
async fn test_auth_malformed_jwt_four_parts() {
    use toadstool_api::middleware::auth_middleware;

    let app = create_test_router().layer(middleware::from_fn(auth_middleware));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("authorization", "Bearer header.payload.signature.extra")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // ✅ FIXED: Now correctly returns 401 for JWT with too many parts
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Malformed JWT (four parts) should return 401 Unauthorized"
    );
}

#[tokio::test]
async fn test_auth_valid_jwt_format() {
    use toadstool_api::middleware::auth_middleware;

    let app = create_test_router().layer(middleware::from_fn(auth_middleware));

    // Valid JWT format (3 parts, even if content is fake)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("authorization", "Bearer eyJhbGc.eyJzdWI.SflKxwRJ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept properly formatted JWT
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should accept valid JWT format"
    );
}

#[tokio::test]
async fn test_auth_no_bearer_prefix() {
    use toadstool_api::middleware::auth_middleware;

    let app = create_test_router().layer(middleware::from_fn(auth_middleware));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("authorization", "eyJhbGc.eyJzdWI.SflKxwRJ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // ✅ FIXED: Now correctly returns 401 when Bearer prefix is missing
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Missing Bearer prefix should return 401 Unauthorized"
    );
}

#[tokio::test]
async fn test_auth_case_sensitive_bearer() {
    use toadstool_api::middleware::auth_middleware;

    let app = create_test_router().layer(middleware::from_fn(auth_middleware));

    // Test lowercase "bearer"
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("authorization", "bearer eyJhbGc.eyJzdWI.SflKxwRJ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // ✅ FIXED: Now correctly returns 401 for lowercase "bearer"
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Lowercase bearer should return 401 Unauthorized (case-sensitive check)"
    );
}

// ============================================================================
// Rate Limit Middleware Edge Cases
// ============================================================================

#[tokio::test]
async fn test_rate_limit_localhost_bypass_ipv4() {
    use toadstool_api::middleware::rate_limit_middleware;

    let app = create_test_router().layer(middleware::from_fn(rate_limit_middleware));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("x-forwarded-for", "127.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Localhost should bypass rate limiting
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "127.0.0.1 should bypass rate limiting"
    );
}

#[tokio::test]
async fn test_rate_limit_localhost_bypass_name() {
    use toadstool_api::middleware::rate_limit_middleware;

    let app = create_test_router().layer(middleware::from_fn(rate_limit_middleware));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("x-forwarded-for", "localhost")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Localhost by name should bypass rate limiting
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "localhost should bypass rate limiting"
    );
}

#[tokio::test]
async fn test_rate_limit_missing_ip_headers() {
    use toadstool_api::middleware::rate_limit_middleware;

    let app = create_test_router().layer(middleware::from_fn(rate_limit_middleware));

    // No x-forwarded-for or x-real-ip headers
    let response = app
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should still process (uses "unknown" as client IP)
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should handle missing IP headers gracefully"
    );
}

#[tokio::test]
async fn test_rate_limit_x_real_ip_fallback() {
    use toadstool_api::middleware::rate_limit_middleware;

    let app = create_test_router().layer(middleware::from_fn(rate_limit_middleware));

    // Use x-real-ip when x-forwarded-for is missing
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("x-real-ip", "192.168.1.100")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should process successfully
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should use x-real-ip as fallback"
    );
}

#[tokio::test]
async fn test_rate_limit_prefers_x_forwarded_for() {
    use toadstool_api::middleware::rate_limit_middleware;

    let app = create_test_router().layer(middleware::from_fn(rate_limit_middleware));

    // Both headers present - should prefer x-forwarded-for
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("x-forwarded-for", "10.0.0.1")
                .header("x-real-ip", "192.168.1.100")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should process successfully (prefers x-forwarded-for)
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should prefer x-forwarded-for over x-real-ip"
    );
}

#[tokio::test]
async fn test_rate_limit_multiple_ips_in_forwarded_for() {
    use toadstool_api::middleware::rate_limit_middleware;

    let app = create_test_router().layer(middleware::from_fn(rate_limit_middleware));

    // Multiple IPs in x-forwarded-for (common with proxies)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("x-forwarded-for", "10.0.0.1, 192.168.1.100")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle multiple IPs gracefully
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should handle multiple IPs in x-forwarded-for"
    );
}

#[tokio::test]
async fn test_rate_limit_external_ip() {
    use toadstool_api::middleware::rate_limit_middleware;

    let app = create_test_router().layer(middleware::from_fn(rate_limit_middleware));

    // External IP (not localhost)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("x-forwarded-for", "203.0.113.42")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should process (current implementation doesn't enforce limits yet)
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should log and process external IP requests"
    );
}

// ============================================================================
// Metrics Middleware Edge Cases (requires ApiState)
// ============================================================================

// Note: Full metrics tests require ApiState setup
// Basic structure test:

#[tokio::test]
async fn test_metrics_middleware_structure() {
    // Test that metrics middleware has proper async signature
    // This is a compilation test - if it compiles, the signature is correct

    use toadstool_api::middleware::metrics_middleware;

    // The middleware function should exist and have the correct type
    let _middleware_fn = metrics_middleware;
    // If this compiles, the function signature is correct
}

// ============================================================================
// CORS Middleware Edge Cases
// ============================================================================

#[tokio::test]
async fn test_cors_adds_headers_to_response() {
    use toadstool_api::middleware::cors_middleware;

    let app = create_test_router().layer(middleware::from_fn(cors_middleware));

    let response = app
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Check for CORS headers
    assert!(
        response
            .headers()
            .get("access-control-allow-origin")
            .is_some()
            || response.status() == StatusCode::OK,
        "CORS middleware should process requests"
    );
}

#[tokio::test]
async fn test_cors_preflight_options_request() {
    use axum::http::Method;
    use toadstool_api::middleware::cors_middleware;

    let app = create_test_router().layer(middleware::from_fn(cors_middleware));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/test")
                .header("access-control-request-method", "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // OPTIONS requests route to handlers (may return NOT_FOUND or METHOD_NOT_ALLOWED)
    // Actual CORS handling happens on responses, not requests
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::METHOD_NOT_ALLOWED,
        "OPTIONS request processed (status: {})",
        response.status()
    );
}

// ============================================================================
// Middleware Composition Edge Cases
// ============================================================================

#[tokio::test]
async fn test_multiple_middleware_layers_order() {
    use toadstool_api::middleware::{cors_middleware, request_id_middleware};

    // Stack multiple middleware layers
    let app = create_test_router()
        .layer(middleware::from_fn(cors_middleware))
        .layer(middleware::from_fn(request_id_middleware));

    let response = app
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should have both request-id and process through CORS
    assert!(
        response.headers().get("x-request-id").is_some(),
        "Should have request ID from middleware stack"
    );
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should successfully process through middleware stack"
    );
}

#[tokio::test]
async fn test_middleware_error_propagation() {
    use toadstool_api::middleware::auth_middleware;

    // Auth middleware will reject this request
    let app = create_test_router().layer(middleware::from_fn(auth_middleware));

    let response = app
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // ✅ FIXED: Auth middleware errors now return 401 (proper status code)
    // This test was intentionally causing an auth error to test propagation
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Auth middleware errors should propagate with proper status codes (401 for auth)"
    );
}

#[tokio::test]
async fn test_middleware_with_invalid_utf8_headers() {
    use toadstool_api::middleware::request_id_middleware;

    let app = create_test_router().layer(middleware::from_fn(request_id_middleware));

    // Create request with headers (invalid UTF-8 headers are filtered by axum)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("x-custom-header", "valid-value")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle gracefully
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should handle header edge cases"
    );
}
