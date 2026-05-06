// SPDX-License-Identifier: AGPL-3.0-or-later
//! Intrusion detection system
//!
//! Extracted from `security_hardening.rs` for modularity (Feb 14, 2026).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;

use tokio::sync::RwLock;
use tracing::error;

use super::config::IntrusionDetectionConfig;

/// Intrusion detection system
pub struct IntrusionDetectionSystem {
    /// Configuration
    config: IntrusionDetectionConfig,
    /// Client activity tracking
    client_activity: Arc<RwLock<HashMap<String, ClientActivity>>>,
    /// Banned clients
    banned_clients: Arc<RwLock<HashMap<String, BanInfo>>>,
}

/// Client activity tracking
#[derive(Debug, Clone)]
struct ClientActivity {
    /// Request count
    request_count: u32,
    /// Failed attempts
    failed_attempts: u32,
    /// Suspicious patterns
    suspicious_patterns: u32,
    /// Activity window start
    window_start: Instant,
    /// Risk score
    risk_score: f64,
}

/// Ban information
#[derive(Debug, Clone)]
struct BanInfo {
    /// Ban start time
    ban_start: Instant,
    /// Ban duration
    duration: Duration,
    /// Reason
    _reason: String,
}

impl IntrusionDetectionSystem {
    /// Create new intrusion detection system
    #[must_use]
    pub fn new(config: IntrusionDetectionConfig) -> Self {
        Self {
            config,
            client_activity: Arc::new(RwLock::new(HashMap::new())),
            banned_clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record client activity
    pub async fn record_activity(&self, client_id: &str, activity_type: ActivityType) {
        let mut activities = self.client_activity.write().await;
        let now = Instant::now();

        let activity = activities
            .entry(client_id.to_string())
            .or_insert_with(|| ClientActivity {
                request_count: 0,
                failed_attempts: 0,
                suspicious_patterns: 0,
                window_start: now,
                risk_score: 0.0,
            });

        // Reset window if needed
        if now.duration_since(activity.window_start) > self.config.activity_window {
            activity.request_count = 0;
            activity.failed_attempts = 0;
            activity.suspicious_patterns = 0;
            activity.window_start = now;
            activity.risk_score = 0.0;
        }

        // Update activity
        match activity_type {
            ActivityType::Request => {
                activity.request_count += 1;
            }
            ActivityType::FailedAttempt => {
                activity.failed_attempts += 1;
                activity.risk_score += 0.1;
            }
            ActivityType::SuspiciousPattern => {
                activity.suspicious_patterns += 1;
                activity.risk_score += 0.2;
            }
        }

        // Check if should ban
        if activity.risk_score >= self.config.anomaly_threshold
            || activity.failed_attempts >= self.config.auto_ban_threshold
        {
            drop(activities); // Release lock before calling ban_client
            self.ban_client(
                client_id,
                self.config.ban_duration,
                "Suspicious activity detected",
            )
            .await;
        }
    }

    /// Check if client is banned
    #[expect(
        clippy::option_if_let_else,
        reason = "mutable borrow needed for remove(); map_or closure cannot mutate"
    )]
    pub async fn is_banned(&self, client_id: &str) -> bool {
        let mut banned = self.banned_clients.write().await;
        let now = Instant::now();

        if let Some(ban_info) = banned.get(client_id) {
            if now.duration_since(ban_info.ban_start) > ban_info.duration {
                // Ban expired
                banned.remove(client_id);
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Ban client
    pub async fn ban_client(&self, client_id: &str, duration: Duration, reason: &str) {
        let now = Instant::now();

        self.banned_clients.write().await.insert(
            client_id.to_string(),
            BanInfo {
                ban_start: now,
                duration,
                _reason: reason.to_string(),
            },
        );

        error!("Client {} banned for {:?}: {}", client_id, duration, reason);
    }
}

/// Activity types for intrusion detection
#[derive(Debug, Clone)]
pub enum ActivityType {
    /// Normal request.
    Request,
    /// Failed authentication or authorization attempt.
    FailedAttempt,
    /// Suspicious pattern (rate, sequence, etc.).
    SuspiciousPattern,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strict_config() -> IntrusionDetectionConfig {
        IntrusionDetectionConfig {
            anomaly_threshold: 0.5,
            activity_window: Duration::from_secs(300),
            auto_ban_threshold: 3,
            ban_duration: Duration::from_secs(60),
            allowed_ips: vec!["127.0.0.1".into()],
        }
    }

    #[tokio::test]
    async fn new_client_is_not_banned() {
        let ids = IntrusionDetectionSystem::new(strict_config());
        assert!(!ids.is_banned("fresh").await);
    }

    #[tokio::test]
    async fn normal_requests_do_not_trigger_ban() {
        let ids = IntrusionDetectionSystem::new(strict_config());
        ids.record_activity("c", ActivityType::Request).await;
        ids.record_activity("c", ActivityType::Request).await;
        assert!(!ids.is_banned("c").await);
    }

    #[tokio::test]
    async fn failed_attempts_trigger_auto_ban_at_threshold() {
        let ids = IntrusionDetectionSystem::new(strict_config());
        ids.record_activity("attacker", ActivityType::FailedAttempt)
            .await;
        ids.record_activity("attacker", ActivityType::FailedAttempt)
            .await;
        assert!(!ids.is_banned("attacker").await);
        ids.record_activity("attacker", ActivityType::FailedAttempt)
            .await;
        assert!(ids.is_banned("attacker").await);
    }

    #[tokio::test]
    async fn suspicious_patterns_accumulate_risk_score() {
        let ids = IntrusionDetectionSystem::new(strict_config());
        // 3 suspicious patterns = 0.6 risk score, exceeds 0.5 threshold
        ids.record_activity("probe", ActivityType::SuspiciousPattern)
            .await;
        ids.record_activity("probe", ActivityType::SuspiciousPattern)
            .await;
        assert!(!ids.is_banned("probe").await);
        ids.record_activity("probe", ActivityType::SuspiciousPattern)
            .await;
        assert!(ids.is_banned("probe").await);
    }

    #[tokio::test]
    async fn manual_ban_works() {
        let ids = IntrusionDetectionSystem::new(strict_config());
        ids.ban_client("target", Duration::from_secs(60), "test")
            .await;
        assert!(ids.is_banned("target").await);
    }

    #[tokio::test]
    async fn ban_does_not_affect_other_clients() {
        let ids = IntrusionDetectionSystem::new(strict_config());
        ids.ban_client("bad", Duration::from_secs(60), "test").await;
        assert!(!ids.is_banned("good").await);
    }

    #[tokio::test(start_paused = true)]
    async fn expired_ban_is_cleared() {
        let ids = IntrusionDetectionSystem::new(strict_config());
        ids.ban_client("temp", Duration::from_millis(1), "test")
            .await;
        tokio::time::advance(Duration::from_millis(5)).await;
        assert!(!ids.is_banned("temp").await);
    }
}
