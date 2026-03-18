// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::SystemTime;
use uuid::Uuid;

use crate::config::AnalyticsConfig;
use crate::types::{Alert, AlertCondition, AlertSeverity, AlertStatus, AnalyticsDataPoint};

pub fn compute_triggered_alerts(
    points: &[&AnalyticsDataPoint],
    config: &AnalyticsConfig,
) -> Vec<Alert> {
    let mut triggered_alerts = Vec::new();

    for dp in points {
        if dp.metric_name.contains("cpu") && dp.value > config.alert_thresholds.cpu_threshold {
            triggered_alerts.push(Alert {
                id: Uuid::new_v4(),
                name: format!("High CPU Usage: {}", dp.metric_name),
                metric_name: dp.metric_name.clone(),
                condition: AlertCondition::Threshold {
                    operator: ">".to_string(),
                    value: config.alert_thresholds.cpu_threshold,
                },
                severity: AlertSeverity::Warning,
                created_at: SystemTime::now(),
                last_triggered: Some(SystemTime::now()),
                status: AlertStatus::Active,
                recipients: vec!["admin@example.com".to_string()],
            });
        }

        if dp.metric_name.contains("memory") && dp.value > config.alert_thresholds.memory_threshold
        {
            triggered_alerts.push(Alert {
                id: Uuid::new_v4(),
                name: format!("High Memory Usage: {}", dp.metric_name),
                metric_name: dp.metric_name.clone(),
                condition: AlertCondition::Threshold {
                    operator: ">".to_string(),
                    value: config.alert_thresholds.memory_threshold,
                },
                severity: AlertSeverity::Critical,
                created_at: SystemTime::now(),
                last_triggered: Some(SystemTime::now()),
                status: AlertStatus::Active,
                recipients: vec!["admin@example.com".to_string()],
            });
        }
    }

    triggered_alerts
}
