// SPDX-License-Identifier: AGPL-3.0-only
//! Analytics configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Advanced analytics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// Enable real-time analytics
    pub enable_realtime: bool,
    /// Historical data retention period
    pub retention_days: u32,
    /// Prediction window for forecasting
    pub prediction_window_hours: u32,
    /// Metrics collection interval
    pub collection_interval_secs: u64,
    /// Alert threshold configurations
    pub alert_thresholds: AlertThresholds,
    /// External integrations
    pub external_integrations: ExternalIntegrations,
}

/// Alert threshold configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// CPU usage alert threshold (percentage)
    pub cpu_threshold: f64,
    /// Memory usage alert threshold (percentage)
    pub memory_threshold: f64,
    /// Error rate threshold (percentage)
    pub error_rate_threshold: f64,
    /// Response time threshold (milliseconds)
    pub response_time_threshold: u64,
}

/// External integration configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalIntegrations {
    /// Custom webhook endpoints
    pub webhooks: Vec<WebhookConfig>,
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub name: String,
    pub url: String,
    pub event_types: Vec<String>,
    pub headers: HashMap<String, String>,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enable_realtime: true,
            retention_days: 90,
            prediction_window_hours: 24,
            collection_interval_secs: 60,
            alert_thresholds: AlertThresholds {
                cpu_threshold: 80.0,
                memory_threshold: 85.0,
                error_rate_threshold: 5.0,
                response_time_threshold: 1000,
            },
            external_integrations: ExternalIntegrations { webhooks: vec![] },
        }
    }
}
