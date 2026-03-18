// SPDX-License-Identifier: AGPL-3.0-or-later
//! Rate limiting for security hardening
//!
//! Extracted from security_hardening.rs for modularity (Feb 14, 2026).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::ToadStoolResult;

use super::config::RateLimitingConfig;

/// Rate limiter
pub struct RateLimiter {
    /// Configuration
    config: RateLimitingConfig,
    /// Client request counts
    client_requests: Arc<RwLock<HashMap<String, ClientRateData>>>,
}

/// Client rate limiting data
#[derive(Debug, Clone)]
struct ClientRateData {
    /// Request timestamps
    request_times: Vec<Instant>,
    /// Total requests today
    daily_requests: u32,
    /// Last reset time
    last_reset: Instant,
    /// Is currently banned
    is_banned: bool,
    /// Ban expiry time
    ban_expiry: Option<Instant>,
}

impl RateLimiter {
    /// Create new rate limiter
    #[must_use]
    pub fn new(config: RateLimitingConfig) -> Self {
        Self {
            config,
            client_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if client is allowed to make request
    #[allow(clippy::significant_drop_tightening)] // client_data borrows from lock for many operations
    pub async fn check_rate_limit(&self, client_id: &str) -> ToadStoolResult<bool> {
        let mut clients = self.client_requests.write().await;
        let now = Instant::now();

        let client_data = clients
            .entry(client_id.to_string())
            .or_insert_with(|| ClientRateData {
                request_times: Vec::new(),
                daily_requests: 0,
                last_reset: now,
                is_banned: false,
                ban_expiry: None,
            });

        // Check if ban has expired
        if let Some(ban_expiry) = client_data.ban_expiry {
            if now > ban_expiry {
                client_data.is_banned = false;
                client_data.ban_expiry = None;
                info!("Rate limit ban expired for client: {}", client_id);
            }
        }

        // Check if currently banned
        if client_data.is_banned {
            return Ok(false);
        }

        // Clean old requests outside sliding window
        client_data
            .request_times
            .retain(|&time| now.duration_since(time) < self.config.sliding_window);

        // Check daily reset
        if now.duration_since(client_data.last_reset) > Duration::from_secs(86400) {
            client_data.daily_requests = 0;
            client_data.last_reset = now;
        }

        // Check rate limits
        let requests_in_window = u32::try_from(client_data.request_times.len()).unwrap_or(u32::MAX);

        if requests_in_window >= self.config.max_requests_per_minute {
            warn!(
                "Rate limit exceeded for client {}: {} requests/minute",
                client_id, requests_in_window
            );
            return Ok(false);
        }

        if client_data.daily_requests >= self.config.max_requests_per_day {
            warn!(
                "Daily rate limit exceeded for client {}: {} requests/day",
                client_id, client_data.daily_requests
            );
            return Ok(false);
        }

        // Record request
        client_data.request_times.push(now);
        client_data.daily_requests += 1;

        Ok(true)
    }

    /// Ban client for specified duration
    #[allow(clippy::significant_drop_tightening)] // client_data borrows from guard for two field updates
    pub async fn ban_client(&self, client_id: &str, duration: Duration) {
        let now = Instant::now();

        let mut clients = self.client_requests.write().await;
        let client_data = clients
            .entry(client_id.to_string())
            .or_insert_with(|| ClientRateData {
                request_times: Vec::new(),
                daily_requests: 0,
                last_reset: now,
                is_banned: false,
                ban_expiry: None,
            });

        client_data.is_banned = true;
        client_data.ban_expiry = Some(now + duration);

        warn!("Client {} banned for {:?}", client_id, duration);
    }
}
