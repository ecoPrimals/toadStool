use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Health status for a primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimalHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub primal_id: String,
    pub status: PrimalHealthStatus,
    pub last_check: DateTime<Utc>,
    pub details: Option<String>,
    pub metrics: std::collections::HashMap<String, f64>,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            primal_id: String::new(),
            status: PrimalHealthStatus::Unknown,
            last_check: Utc::now(),
            details: None,
            metrics: std::collections::HashMap::new(),
        }
    }
}

/// Health monitor for tracking primal health
pub struct HealthMonitor {
    checks: std::collections::HashMap<String, HealthCheck>,
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            checks: std::collections::HashMap::new(),
        }
    }

    pub fn update_health(&mut self, primal_id: String, status: PrimalHealthStatus) {
        let check = HealthCheck {
            primal_id: primal_id.clone(),
            status,
            last_check: Utc::now(),
            details: None,
            metrics: std::collections::HashMap::new(),
        };
        self.checks.insert(primal_id, check);
    }

    pub fn get_health(&self, primal_id: &str) -> Option<&HealthCheck> {
        self.checks.get(primal_id)
    }
}
