// SPDX-License-Identifier: AGPL-3.0-only

use std::time::Duration;

/// Cost model for a provider
#[derive(Debug, Clone)]
pub struct CostModel {
    pub cpu_cost_per_core_hour: f64,
    pub memory_cost_per_gb_hour: f64,
    pub storage_cost_per_gb_month: f64,
    pub network_cost_per_gb: f64,
}

/// Spend tracker
#[derive(Debug, Clone)]
pub struct SpendTracker {
    pub current_spend: f64,
    pub monthly_spend: f64,
    pub projected_spend: f64,
}

/// Budget manager
#[derive(Debug, Clone)]
pub struct BudgetManager {
    pub monthly_budget: Option<f64>,
    pub alert_thresholds: Vec<f64>,
}

/// Spot instance manager
#[derive(Debug, Clone)]
pub struct SpotInstanceManager {
    pub spot_preference: f64,
    pub max_interruption_tolerance: Duration,
}

/// Performance metric
#[derive(Debug, Clone)]
pub struct PerformanceMetric {
    pub name: String,
    pub value: f64,
    pub timestamp: std::time::SystemTime,
}

impl Default for PerformanceMetric {
    fn default() -> Self {
        Self {
            name: String::new(),
            value: 0.0,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Cost alert
#[derive(Debug, Clone, Default)]
pub struct CostAlert {
    pub threshold: f64,
    pub message: String,
    pub severity: AlertSeverity,
}

/// Alert severity levels
#[derive(Debug, Clone, Default)]
pub enum AlertSeverity {
    #[default]
    Info,
    Warning,
    Critical,
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
mod tests {
    use super::*;

    #[test]
    fn test_alert_severity_default() {
        let severity = AlertSeverity::default();
        assert!(matches!(severity, AlertSeverity::Info));
    }

    #[test]
    fn test_performance_metric_default() {
        let metric = PerformanceMetric::default();
        assert!(metric.name.is_empty());
        assert_eq!(metric.value, 0.0);
    }

    #[test]
    fn test_cost_alert_default() {
        let alert = CostAlert::default();
        assert_eq!(alert.threshold, 0.0);
        assert!(alert.message.is_empty());
        assert!(matches!(alert.severity, AlertSeverity::Info));
    }

    #[test]
    fn test_alert_severity_all_variants() {
        assert!(matches!(AlertSeverity::Info, AlertSeverity::Info));
        assert!(matches!(AlertSeverity::Warning, AlertSeverity::Warning));
        assert!(matches!(AlertSeverity::Critical, AlertSeverity::Critical));
    }

    #[test]
    fn test_cost_model_construction() {
        let model = CostModel {
            cpu_cost_per_core_hour: 0.05,
            memory_cost_per_gb_hour: 0.01,
            storage_cost_per_gb_month: 0.10,
            network_cost_per_gb: 0.02,
        };
        assert!((model.cpu_cost_per_core_hour - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_budget_manager_construction() {
        let manager = BudgetManager {
            monthly_budget: Some(1000.0),
            alert_thresholds: vec![0.8, 0.9, 1.0],
        };
        assert_eq!(manager.alert_thresholds.len(), 3);
    }

    #[test]
    fn test_spend_tracker_construction() {
        let tracker = SpendTracker {
            current_spend: 100.0,
            monthly_spend: 500.0,
            projected_spend: 600.0,
        };
        assert!((tracker.current_spend - 100.0).abs() < f64::EPSILON);
    }
}
