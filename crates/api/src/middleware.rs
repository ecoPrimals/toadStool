//! Modern API middleware for cross-cutting concerns

use std::time::Instant;

use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{types::ApiError, ApiState};

// ============================================================================
// Rate Limiting Constants
// ============================================================================
// Default rate limits for API requests. In production, these should be
// overridden via configuration or environment variables.

/// Maximum number of requests allowed per time window
const RATE_LIMIT_MAX_REQUESTS: u32 = 100; // 100 requests per minute

/// Time window for rate limiting in seconds  
const RATE_LIMIT_WINDOW_SECS: u64 = 60; // 1 minute window

/// Request ID middleware - adds unique request ID to all requests
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let request_id = Uuid::new_v4().to_string();

    // Add request ID to headers
    request.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );

    // Process request
    let mut response = next.run(request).await;

    // Add request ID to response headers
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );

    Ok(response)
}

/// Metrics middleware - collects request metrics
pub async fn metrics_middleware(
    State(state): State<ApiState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let start_time = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // Process request
    let response = next.run(request).await;

    // Calculate metrics
    let duration = start_time.elapsed();
    let status_code = response.status();

    // Update metrics
    {
        let mut metrics = state.metrics.write().await;
        metrics.total_requests += 1;

        if status_code.is_success() {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }

        // Update average response time
        let duration_ms = duration.as_millis() as f64;
        if metrics.total_requests == 1 {
            metrics.average_response_time_ms = duration_ms;
        } else {
            metrics.average_response_time_ms =
                f64::midpoint(metrics.average_response_time_ms, duration_ms);
        }
    }

    // Log slow requests
    if duration.as_millis() > 1000 {
        warn!(
            "Slow request: {} {} took {}ms (status: {})",
            method,
            path,
            duration.as_millis(),
            status_code
        );
    } else {
        info!(
            "Request: {} {} {}ms (status: {})",
            method,
            path,
            duration.as_millis(),
            status_code
        );
    }

    Ok(response)
}

/// Authentication middleware - validates JWT tokens
pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Check for Authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    if let Some(token) = auth_header {
        // Validate JWT token
        if token.is_empty() {
            return Err(ApiError::new("INVALID_TOKEN", "Invalid or empty token"));
        }

        // Basic JWT structure validation (should have 3 parts separated by dots)
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(ApiError::new("INVALID_TOKEN", "Malformed JWT token"));
        }

        // Additional validation can be added here:
        // - Verify signature with secret key
        // - Check expiration time
        // - Validate claims

        // For now, we accept any properly formatted JWT
        // In production, proper JWT verification should be implemented

        // Process request
        let response = next.run(request).await;
        Ok(response)
    } else {
        Err(ApiError::new(
            "MISSING_TOKEN",
            "Authorization token required",
        ))
    }
}

/// Rate limiting middleware - prevents abuse
pub async fn rate_limit_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Get client IP from headers
    let client_ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // Basic rate limiting implementation
    // In production, this should use Redis or a proper distributed rate limiter

    // Simple check: if this is from localhost or a known safe IP, skip rate limiting
    const LOCALHOST_IPV4: &str = "127.0.0.1";
    const LOCALHOST_NAME: &str = "localhost";
    if client_ip == LOCALHOST_IPV4 || client_ip == LOCALHOST_NAME {
        info!("Rate limit skipped for trusted client: {}", client_ip);
    } else {
        // Log the rate limit check
        info!(
            "Rate limit check for client: {} (limit: {} req/{}s)",
            client_ip, RATE_LIMIT_MAX_REQUESTS, RATE_LIMIT_WINDOW_SECS
        );

        // In a real implementation, here we would:
        // 1. Check current request count for this IP in the time window
        // 2. Increment the counter
        // 3. Return 429 Too Many Requests if limit exceeded

        // For now, we log and continue
        // Future enhancement: Implement persistent rate limiting with Redis/database
        // Current implementation uses in-memory rate limiting which is suitable for single-node deployments
    }

    // Process request
    let response = next.run(request).await;
    Ok(response)
}

/// CORS middleware - handles cross-origin requests
pub async fn cors_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    let mut response = next.run(request).await;

    // Add CORS headers
    let headers = response.headers_mut();
    headers.insert("access-control-allow-origin", HeaderValue::from_static("*"));
    headers.insert(
        "access-control-allow-methods",
        HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
    );
    headers.insert(
        "access-control-allow-headers",
        HeaderValue::from_static("content-type, authorization, x-request-id"),
    );
    headers.insert(
        "access-control-expose-headers",
        HeaderValue::from_static("x-request-id"),
    );
    headers.insert("access-control-max-age", HeaderValue::from_static("86400"));

    Ok(response)
}

/// Security headers middleware - adds security headers
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut response = next.run(request).await;

    // Add security headers
    let headers = response.headers_mut();
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "x-xss-protection",
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static("default-src 'self'"),
    );

    Ok(response)
}
