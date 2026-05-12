// SPDX-License-Identifier: AGPL-3.0-or-later
//! Alert rules and evaluation logic

use tokio::time::Duration;
use uuid::Uuid;

use crate::monitoring::types::{
    ActiveAlert, AlertCondition, AlertRule, AlertSeverity, ComparisonOperator, HealthStatus,
    SystemHealth,
};

const ALERT_THRESHOLD_DURATION_SECS: u64 = 300;
const ALERT_COOLDOWN_SECS: u64 = 600;
const CRITICAL_THRESHOLD_DURATION_SECS: u64 = 60;
const CRITICAL_COOLDOWN_SECS: u64 = 1800;

/// Returns the default set of alert rules for the monitoring system
pub fn load_default_alert_rules() -> Vec<AlertRule> {
    vec![
        AlertRule {
            id: "high_cpu".to_string(),
            name: "High CPU Usage".to_string(),
            condition: AlertCondition::Threshold {
                metric: "cpu_usage_percent".to_string(),
                operator: ComparisonOperator::GreaterThan,
                value: 90.0,
                duration: Duration::from_secs(ALERT_THRESHOLD_DURATION_SECS),
            },
            severity: AlertSeverity::Warning,
            enabled: true,
            cooldown: Duration::from_secs(ALERT_COOLDOWN_SECS),
            last_triggered: None,
        },
        AlertRule {
            id: "high_memory".to_string(),
            name: "High Memory Usage".to_string(),
            condition: AlertCondition::Threshold {
                metric: "memory_usage_percent".to_string(),
                operator: ComparisonOperator::GreaterThan,
                value: 85.0,
                duration: Duration::from_secs(ALERT_THRESHOLD_DURATION_SECS),
            },
            severity: AlertSeverity::Warning,
            enabled: true,
            cooldown: Duration::from_secs(ALERT_COOLDOWN_SECS),
            last_triggered: None,
        },
        AlertRule {
            id: "low_storage".to_string(),
            name: "Low Storage Space".to_string(),
            condition: AlertCondition::Threshold {
                metric: "storage_usage_percent".to_string(),
                operator: ComparisonOperator::GreaterThan,
                value: 95.0,
                duration: Duration::from_secs(CRITICAL_THRESHOLD_DURATION_SECS),
            },
            severity: AlertSeverity::Critical,
            enabled: true,
            cooldown: Duration::from_secs(CRITICAL_COOLDOWN_SECS),
            last_triggered: None,
        },
    ]
}

/// Evaluates health status and returns active alerts
pub fn evaluate_health_alerts(health: &SystemHealth) -> Vec<ActiveAlert> {
    let mut alerts = Vec::new();
    let now = std::time::SystemTime::now();

    if matches!(
        health.cpu_health,
        HealthStatus::Critical | HealthStatus::Warning
    ) {
        alerts.push(ActiveAlert {
            id: Uuid::new_v4().to_string(),
            rule_name: "cpu_high".to_string(),
            severity: if matches!(health.cpu_health, HealthStatus::Critical) {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            },
            message: "CPU usage elevated".to_string(),
            triggered_at: now,
            target: "system".to_string(),
        });
    }

    if matches!(
        health.memory_health,
        HealthStatus::Critical | HealthStatus::Warning
    ) {
        alerts.push(ActiveAlert {
            id: Uuid::new_v4().to_string(),
            rule_name: "memory_high".to_string(),
            severity: if matches!(health.memory_health, HealthStatus::Critical) {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            },
            message: "Memory usage elevated".to_string(),
            triggered_at: now,
            target: "system".to_string(),
        });
    }

    alerts
}
