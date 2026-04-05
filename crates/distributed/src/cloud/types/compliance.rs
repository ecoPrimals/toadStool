// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use toadstool_common::constants::timeouts;

use super::capabilities::ComplianceCertification;
use super::config::DataSovereigntyRequirement;

/// Cloud health checker
#[derive(Debug, Clone)]
pub struct CloudHealthChecker {
    /// Health endpoint URL or host to probe.
    pub endpoint: String,
    /// Interval between health checks.
    pub check_interval: Duration,
    /// Per-request timeout.
    pub timeout: Duration,
}

impl CloudHealthChecker {
    /// Creates a checker with empty endpoint; actual endpoint from config/discovery (no AWS default).
    pub fn new(_provider: String) -> Self {
        Self {
            endpoint: String::new(),
            check_interval: timeouts::HEALTH_CHECK_INTERVAL,
            timeout: timeouts::TCP_CONNECT_TIMEOUT,
        }
    }
}

/// Compliance requirements
#[derive(Debug, Clone)]
pub struct ComplianceRequirements {
    /// Required compliance certifications (SOC2, ISO, etc.).
    pub certifications: Vec<ComplianceCertification>,
    /// Allowed or required region identifiers.
    pub regions: Vec<String>,
    /// Per-data-type sovereignty and region rules.
    pub data_sovereignty: Vec<DataSovereigntyRequirement>,
}

/// Compliance constraints for a job
#[derive(Debug, Clone)]
pub struct ComplianceConstraints {
    /// Provider names permitted for this job.
    pub allowed_providers: Vec<String>,
    /// Regions the job must run in.
    pub required_regions: Vec<String>,
    /// Whether encryption at rest/in transit is mandatory.
    pub encryption_required: bool,
}

/// Trust level for cloud providers
#[derive(Debug, Clone, Default)]
pub enum TrustLevel {
    /// Provider is treated as fully trusted by default.
    #[default]
    Trusted,
    /// Provider is not trusted without extra validation.
    Untrusted,
    /// Trust depends on policy or runtime checks.
    Conditional,
}

/// Trust configuration
#[derive(Debug, Clone, Default)]
pub struct TrustConfig {
    /// Whether explicit validation must run before use.
    pub validation_required: bool,
    /// Minimum score or threshold to treat a provider as trusted.
    pub trust_threshold: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_health_checker_new() {
        let checker = CloudHealthChecker::new("ec2".to_string());
        assert!(
            checker.endpoint.is_empty(),
            "endpoint discovered via config at runtime"
        );
    }

    #[test]
    fn test_trust_level_default() {
        let level = TrustLevel::default();
        assert!(matches!(level, TrustLevel::Trusted));
    }

    #[test]
    fn test_trust_config_default() {
        let config = TrustConfig::default();
        assert!(!config.validation_required);
        assert!((config.trust_threshold - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compliance_requirements_construction() {
        let reqs = ComplianceRequirements {
            certifications: vec![],
            regions: vec!["us-east-1".to_string()],
            data_sovereignty: vec![],
        };
        assert_eq!(reqs.regions.len(), 1);
    }

    #[test]
    fn test_compliance_constraints_construction() {
        let constraints = ComplianceConstraints {
            allowed_providers: vec!["aws".to_string()],
            required_regions: vec![],
            encryption_required: true,
        };
        assert!(constraints.encryption_required);
    }
}
