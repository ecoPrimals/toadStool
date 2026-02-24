//! Compliance and security enforcement
//!
//! This module validates data sovereignty, security tier requirements (encryption, audit logging),
//! and resource isolation. Returns structured compliance reports with pass/fail per check.

use std::collections::HashSet;
use thiserror::Error;
use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::UniversalJob;

use super::types::{
    CloudCapabilities, ComplianceConfig, ComplianceConstraints, ComplianceRequirements,
    SecurityFeature,
};

// ─── Security Tier Requirements ───────────────────────────────────────────────

/// Security tier levels that map to required features.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityTier {
    /// Basic: encryption at rest/transit only.
    Basic,
    /// Standard: encryption + audit logging.
    Standard,
    /// High: encryption + audit + resource isolation (dedicated, network segmentation).
    High,
}

impl SecurityTier {
    /// Required security features for this tier.
    pub fn required_features(self) -> &'static [SecurityFeature] {
        match self {
            SecurityTier::Basic => &[SecurityFeature::Encryption],
            SecurityTier::Standard => &[SecurityFeature::Encryption, SecurityFeature::Compliance],
            SecurityTier::High => &[
                SecurityFeature::Encryption,
                SecurityFeature::Compliance,
                SecurityFeature::NetworkSecurity,
            ],
        }
    }
}

// ─── Structured Compliance Report ─────────────────────────────────────────────

/// Result of a single compliance check.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CheckResult {
    Pass,
    Fail { reason: String },
}

/// A single compliance check with pass/fail and optional details.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComplianceCheck {
    /// Check category (e.g., "data_sovereignty", "encryption", "audit_logging").
    pub check_name: String,
    /// Pass or fail with reason.
    pub result: CheckResult,
}

/// Full compliance report for a provider/job combination.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComplianceReport {
    /// Provider name evaluated.
    pub provider_name: String,
    /// Individual check results.
    pub checks: Vec<ComplianceCheck>,
    /// Overall pass: true only if all checks pass.
    pub overall_pass: bool,
    /// Regions that satisfy data sovereignty (if applicable).
    pub compliant_regions: Vec<String>,
}

// ─── Compliance Errors ─────────────────────────────────────────────────────────

/// Compliance-related errors.
#[derive(Debug, Error)]
pub enum ComplianceError {
    #[error("Compliance check failed: {0}")]
    CheckFailed(String),

    #[error("Provider '{0}' has no region information for sovereignty check")]
    NoRegionInfo(String),

    #[error("Invalid security tier: {0}")]
    InvalidSecurityTier(String),
}

impl From<ComplianceError> for ToadStoolError {
    fn from(e: ComplianceError) -> Self {
        ToadStoolError::security(e.to_string())
    }
}

// ─── CloudComplianceEnforcer ─────────────────────────────────────────────────

/// Cloud compliance enforcer with data sovereignty, security tier, and isolation checks.
pub struct CloudComplianceEnforcer {
    pub(crate) requirements: ComplianceRequirements,
    pub(crate) provider_compliance: std::collections::HashMap<String, CloudCapabilities>,
    pub(crate) security_tier: SecurityTier,
}

impl CloudComplianceEnforcer {
    pub async fn new(config: ComplianceConfig) -> ToadStoolResult<Self> {
        let security_tier = SecurityTier::Standard; // Default
        Ok(Self {
            requirements: ComplianceRequirements {
                certifications: config.required_certifications,
                regions: config.allowed_regions,
                data_sovereignty: config.data_sovereignty_requirements,
            },
            provider_compliance: std::collections::HashMap::new(),
            security_tier,
        })
    }

    /// Create enforcer with explicit security tier.
    pub fn with_security_tier(mut self, tier: SecurityTier) -> Self {
        self.security_tier = tier;
        self
    }

    pub async fn add_provider_compliance(
        &mut self,
        name: &str,
        capabilities: &CloudCapabilities,
    ) -> ToadStoolResult<()> {
        self.provider_compliance
            .insert(name.to_string(), capabilities.clone());
        Ok(())
    }

    /// Produce a full compliance report for a provider.
    pub fn report_for_provider(&self, provider_name: &str) -> ToadStoolResult<ComplianceReport> {
        let capabilities = self.provider_compliance.get(provider_name).ok_or_else(|| {
            ComplianceError::CheckFailed(format!("Provider '{provider_name}' not registered"))
        })?;

        let mut checks = Vec::new();

        // 1. Certification check
        let cert_check = self.check_certifications(provider_name, capabilities);
        checks.push(cert_check);

        // 2. Data sovereignty check
        let sovereignty_check = self.check_data_sovereignty(provider_name, capabilities)?;
        checks.push(sovereignty_check);

        // 3. Security tier (encryption, audit, isolation)
        let security_checks = self.check_security_tier(provider_name, capabilities);
        checks.extend(security_checks);

        // 4. Resource isolation check
        let isolation_check = self.check_resource_isolation(provider_name, capabilities);
        checks.push(isolation_check);

        let overall_pass = checks.iter().all(|c| matches!(c.result, CheckResult::Pass));

        let compliant_regions = self.compute_compliant_regions(capabilities);

        Ok(ComplianceReport {
            provider_name: provider_name.to_string(),
            checks,
            overall_pass,
            compliant_regions,
        })
    }

    pub async fn get_constraints_for_job(
        &self,
        _job: &UniversalJob,
    ) -> ToadStoolResult<ComplianceConstraints> {
        let compliant_providers = self.get_compliant_providers();
        let required_regions = self.requirements.regions.clone();
        let encryption_required = self.security_tier.required_features().iter().any(|f| {
            std::mem::discriminant(f) == std::mem::discriminant(&SecurityFeature::Encryption)
        });

        Ok(ComplianceConstraints {
            allowed_providers: compliant_providers,
            required_regions,
            encryption_required,
        })
    }

    pub(crate) fn get_compliant_providers(&self) -> Vec<String> {
        self.provider_compliance
            .keys()
            .filter_map(|name| {
                self.report_for_provider(name)
                    .ok()
                    .filter(|r| r.overall_pass)
                    .map(|_| name.clone())
            })
            .collect()
    }

    fn check_certifications(
        &self,
        _provider: &str,
        capabilities: &CloudCapabilities,
    ) -> ComplianceCheck {
        let certs: HashSet<_> = capabilities.compliance_certifications.iter().collect();
        let required: HashSet<_> = self.requirements.certifications.iter().collect();
        let missing: Vec<_> = required.difference(&certs).collect();

        if missing.is_empty() {
            ComplianceCheck {
                check_name: "certifications".to_string(),
                result: CheckResult::Pass,
            }
        } else {
            let names: Vec<String> = missing.iter().map(|c| format!("{c:?}")).collect();
            ComplianceCheck {
                check_name: "certifications".to_string(),
                result: CheckResult::Fail {
                    reason: format!("Missing: {}", names.join(", ")),
                },
            }
        }
    }

    fn check_data_sovereignty(
        &self,
        _provider: &str,
        capabilities: &CloudCapabilities,
    ) -> ToadStoolResult<ComplianceCheck> {
        let provider_region_names: HashSet<String> = capabilities
            .regions
            .iter()
            .map(|r| r.name.clone())
            .collect();

        let required_regions: HashSet<String> = self.requirements.regions.iter().cloned().collect();

        if required_regions.is_empty() {
            return Ok(ComplianceCheck {
                check_name: "data_sovereignty".to_string(),
                result: CheckResult::Pass,
            });
        }

        let has_required = required_regions
            .iter()
            .any(|r| provider_region_names.contains(r));

        if !has_required {
            let sovereignty_reqs: Vec<String> = self
                .requirements
                .data_sovereignty
                .iter()
                .flat_map(|ds| ds.allowed_regions.clone())
                .collect();
            let all_required: HashSet<_> = sovereignty_reqs
                .into_iter()
                .chain(required_regions)
                .collect();
            let missing: Vec<_> = all_required
                .difference(&provider_region_names)
                .cloned()
                .collect();
            return Ok(ComplianceCheck {
                check_name: "data_sovereignty".to_string(),
                result: CheckResult::Fail {
                    reason: format!(
                        "Provider regions {provider_region_names:?} do not include required regions: {missing:?}",
                    ),
                },
            });
        }

        for ds in &self.requirements.data_sovereignty {
            let allowed: HashSet<_> = ds.allowed_regions.iter().collect();
            let provider_has = capabilities
                .regions
                .iter()
                .any(|r| allowed.contains(&r.name));
            if !provider_has {
                return Ok(ComplianceCheck {
                    check_name: "data_sovereignty".to_string(),
                    result: CheckResult::Fail {
                        reason: format!(
                            "Data type '{0}' requires regions {1:?}; provider lacks them",
                            ds.data_type, ds.allowed_regions
                        ),
                    },
                });
            }
        }

        Ok(ComplianceCheck {
            check_name: "data_sovereignty".to_string(),
            result: CheckResult::Pass,
        })
    }

    fn check_security_tier(
        &self,
        _provider: &str,
        capabilities: &CloudCapabilities,
    ) -> Vec<ComplianceCheck> {
        let provider_features: HashSet<_> = capabilities.security_features.iter().collect();
        let mut checks = Vec::new();

        for req in self.security_tier.required_features() {
            let has = provider_features.contains(req);
            let name = format!("security_{req:?}").to_lowercase().replace(' ', "_");
            checks.push(ComplianceCheck {
                check_name: name,
                result: if has {
                    CheckResult::Pass
                } else {
                    CheckResult::Fail {
                        reason: format!("Required feature {req:?} not in provider capabilities"),
                    }
                },
            });
        }

        checks
    }

    fn check_resource_isolation(
        &self,
        _provider: &str,
        capabilities: &CloudCapabilities,
    ) -> ComplianceCheck {
        if self.security_tier != SecurityTier::High {
            return ComplianceCheck {
                check_name: "resource_isolation".to_string(),
                result: CheckResult::Pass,
            };
        }

        let has_network_security = capabilities
            .security_features
            .contains(&SecurityFeature::NetworkSecurity);
        let has_private_networking = capabilities.networking_features.iter().any(|f| {
            matches!(
                f,
                super::types::NetworkingFeature::PrivateNetworking
                    | super::types::NetworkingFeature::VPN
            )
        });

        if has_network_security && has_private_networking {
            ComplianceCheck {
                check_name: "resource_isolation".to_string(),
                result: CheckResult::Pass,
            }
        } else {
            ComplianceCheck {
                check_name: "resource_isolation".to_string(),
                result: CheckResult::Fail {
                    reason: "High security tier requires NetworkSecurity and PrivateNetworking/VPN"
                        .to_string(),
                },
            }
        }
    }

    fn compute_compliant_regions(&self, capabilities: &CloudCapabilities) -> Vec<String> {
        let provider_names: HashSet<_> = capabilities
            .regions
            .iter()
            .map(|r| r.name.clone())
            .collect();
        let required: HashSet<_> = self.requirements.regions.iter().cloned().collect();

        if required.is_empty() {
            return capabilities
                .regions
                .iter()
                .map(|r| r.name.clone())
                .collect();
        }

        required.intersection(&provider_names).cloned().collect()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::types::{
        ComplianceCertification, ComputeType, DataSovereigntyRequirement, NetworkingFeature,
        Region, SecurityFeature, StorageType,
    };

    fn caps_with_certs(certs: Vec<ComplianceCertification>) -> CloudCapabilities {
        CloudCapabilities {
            compute_types: vec![ComputeType::VM],
            storage_types: vec![StorageType::BlockStorage],
            networking_features: vec![NetworkingFeature::VPC],
            security_features: vec![SecurityFeature::Encryption, SecurityFeature::Compliance],
            compliance_certifications: certs,
            regions: vec![],
            max_cpu_cores: None,
            max_memory_gb: None,
            gpu_support: false,
            kubernetes_support: false,
            serverless_support: false,
        }
    }

    fn caps_full_security(regions: Vec<Region>) -> CloudCapabilities {
        CloudCapabilities {
            compute_types: vec![ComputeType::VM],
            storage_types: vec![StorageType::BlockStorage],
            networking_features: vec![
                NetworkingFeature::VPC,
                NetworkingFeature::PrivateNetworking,
                NetworkingFeature::VPN,
            ],
            security_features: vec![
                SecurityFeature::Encryption,
                SecurityFeature::Compliance,
                SecurityFeature::NetworkSecurity,
            ],
            compliance_certifications: vec![ComplianceCertification::SOC2],
            regions,
            max_cpu_cores: None,
            max_memory_gb: None,
            gpu_support: false,
            kubernetes_support: false,
            serverless_support: false,
        }
    }

    fn make_config(
        certs: Vec<ComplianceCertification>,
        allowed_regions: Vec<String>,
        data_sovereignty: Vec<DataSovereigntyRequirement>,
    ) -> ComplianceConfig {
        ComplianceConfig {
            required_certifications: certs,
            allowed_regions,
            data_sovereignty_requirements: data_sovereignty,
        }
    }

    #[tokio::test]
    async fn test_new_enforcer_empty() {
        let cfg = make_config(
            vec![ComplianceCertification::SOC2],
            vec!["us-east-1".to_string()],
            vec![],
        );
        let enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        assert!(enforcer.provider_compliance.is_empty());
        assert_eq!(enforcer.requirements.certifications.len(), 1);
    }

    #[tokio::test]
    async fn test_add_provider_compliance_registers() {
        let cfg = make_config(vec![], vec![], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        let caps = caps_with_certs(vec![ComplianceCertification::SOC2]);
        enforcer
            .add_provider_compliance("aws", &caps)
            .await
            .unwrap();
        assert!(enforcer.provider_compliance.contains_key("aws"));
    }

    #[tokio::test]
    async fn test_compliant_provider_included() {
        let cfg = make_config(vec![ComplianceCertification::SOC2], vec![], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();

        let caps_ok = caps_with_certs(vec![ComplianceCertification::SOC2]);
        enforcer
            .add_provider_compliance("good", &caps_ok)
            .await
            .unwrap();

        let compliant = enforcer.get_compliant_providers();
        assert!(compliant.contains(&"good".to_string()));
    }

    #[tokio::test]
    async fn test_non_compliant_provider_excluded() {
        let cfg = make_config(vec![ComplianceCertification::SOC2], vec![], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();

        let caps_bad = caps_with_certs(vec![]);
        enforcer
            .add_provider_compliance("bad", &caps_bad)
            .await
            .unwrap();

        let compliant = enforcer.get_compliant_providers();
        assert!(!compliant.contains(&"bad".to_string()));
    }

    #[tokio::test]
    async fn test_data_sovereignty_pass() {
        let cfg = make_config(vec![], vec!["us-east-1".to_string()], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        let caps = caps_full_security(vec![
            Region {
                name: "us-east-1".to_string(),
                location: "N. Virginia".to_string(),
                availability_zones: vec!["us-east-1a".to_string()],
            },
            Region {
                name: "eu-west-1".to_string(),
                location: "Ireland".to_string(),
                availability_zones: vec!["eu-west-1a".to_string()],
            },
        ]);
        enforcer
            .add_provider_compliance("compliant", &caps)
            .await
            .unwrap();

        let report = enforcer.report_for_provider("compliant").unwrap();
        assert!(report.overall_pass);
        assert!(!report.compliant_regions.is_empty());
    }

    #[tokio::test]
    async fn test_data_sovereignty_fail() {
        let cfg = make_config(vec![], vec!["eu-south-1".to_string()], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        let caps = caps_full_security(vec![Region {
            name: "us-east-1".to_string(),
            location: "N. Virginia".to_string(),
            availability_zones: vec!["us-east-1a".to_string()],
        }]);
        enforcer
            .add_provider_compliance("non_compliant", &caps)
            .await
            .unwrap();

        let report = enforcer.report_for_provider("non_compliant").unwrap();
        assert!(!report.overall_pass);
        let sovereignty_check = report
            .checks
            .iter()
            .find(|c| c.check_name == "data_sovereignty")
            .expect("data_sovereignty check present");
        assert!(matches!(sovereignty_check.result, CheckResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_security_tier_checks() {
        let cfg = make_config(vec![], vec![], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        let caps = caps_with_certs(vec![]); // encryption only
        enforcer
            .add_provider_compliance("basic", &caps)
            .await
            .unwrap();

        let report = enforcer.report_for_provider("basic").unwrap();
        let security_checks: Vec<_> = report
            .checks
            .iter()
            .filter(|c| c.check_name.starts_with("security_"))
            .collect();
        assert!(!security_checks.is_empty());
    }

    #[tokio::test]
    async fn test_report_structure() {
        let cfg = make_config(vec![ComplianceCertification::SOC2], vec![], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        let caps = caps_with_certs(vec![ComplianceCertification::SOC2]);
        enforcer.add_provider_compliance("p", &caps).await.unwrap();

        let report = enforcer.report_for_provider("p").unwrap();
        assert_eq!(report.provider_name, "p");
        assert!(!report.checks.is_empty());
        assert!(report.overall_pass);
    }

    #[tokio::test]
    async fn test_allowed_regions_in_requirements() {
        let cfg = ComplianceConfig {
            required_certifications: vec![],
            allowed_regions: vec!["eu-west-1".to_string(), "eu-central-1".to_string()],
            data_sovereignty_requirements: vec![],
        };
        let enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        assert!(enforcer
            .requirements
            .regions
            .contains(&"eu-west-1".to_string()));
        assert!(enforcer
            .requirements
            .regions
            .contains(&"eu-central-1".to_string()));
    }

    #[tokio::test]
    async fn test_with_security_tier_changes_tier() {
        let cfg = make_config(vec![], vec![], vec![]);
        let enforcer = CloudComplianceEnforcer::new(cfg)
            .await
            .unwrap()
            .with_security_tier(SecurityTier::High);
        assert_eq!(enforcer.security_tier, SecurityTier::High);
    }

    #[tokio::test]
    async fn test_get_constraints_for_job_returns_allowed_providers() {
        let cfg = make_config(vec![ComplianceCertification::SOC2], vec![], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        let caps = caps_with_certs(vec![ComplianceCertification::SOC2]);
        enforcer
            .add_provider_compliance("compliant-prov", &caps)
            .await
            .unwrap();

        let job = crate::UniversalJob {
            job_id: uuid::Uuid::new_v4(),
            job_type: None,
            execution_request: toadstool::ExecutionRequest::default(),
            target: crate::ExecutionTarget::Local,
            priority: crate::JobPriority::Normal,
            dependencies: vec![],
            resource_requirements: crate::types::ResourceRequirements::default(),
            retry_config: crate::types::DistributedRetryConfig::default(),
            created_at: chrono::Utc::now(),
        };

        let constraints = enforcer.get_constraints_for_job(&job).await.unwrap();
        assert!(constraints
            .allowed_providers
            .contains(&"compliant-prov".to_string()));
        assert!(constraints.encryption_required);
    }

    #[tokio::test]
    async fn test_security_tier_basic_required_features() {
        let features = SecurityTier::Basic.required_features();
        assert_eq!(features.len(), 1);
        assert!(features.contains(&SecurityFeature::Encryption));
    }

    #[tokio::test]
    async fn test_security_tier_standard_required_features() {
        let features = SecurityTier::Standard.required_features();
        assert_eq!(features.len(), 2);
        assert!(features.contains(&SecurityFeature::Encryption));
        assert!(features.contains(&SecurityFeature::Compliance));
    }

    #[tokio::test]
    async fn test_security_tier_high_required_features() {
        let features = SecurityTier::High.required_features();
        assert_eq!(features.len(), 3);
        assert!(features.contains(&SecurityFeature::Encryption));
        assert!(features.contains(&SecurityFeature::Compliance));
        assert!(features.contains(&SecurityFeature::NetworkSecurity));
    }

    #[tokio::test]
    async fn test_report_for_provider_not_registered_fails() {
        let cfg = make_config(vec![], vec![], vec![]);
        let enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        let res = enforcer.report_for_provider("nonexistent");
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_high_security_tier_resource_isolation_pass() {
        let cfg = make_config(vec![], vec![], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg)
            .await
            .unwrap()
            .with_security_tier(SecurityTier::High);
        let caps = caps_full_security(vec![Region {
            name: "us-east-1".to_string(),
            location: "N. Virginia".to_string(),
            availability_zones: vec!["us-east-1a".to_string()],
        }]);
        enforcer
            .add_provider_compliance("high-sec", &caps)
            .await
            .unwrap();
        let report = enforcer.report_for_provider("high-sec").unwrap();
        assert!(report.overall_pass);
    }

    #[tokio::test]
    async fn test_high_security_tier_resource_isolation_fail() {
        let cfg = make_config(vec![], vec![], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg)
            .await
            .unwrap()
            .with_security_tier(SecurityTier::High);
        // Caps without NetworkSecurity or PrivateNetworking
        let caps = CloudCapabilities {
            compute_types: vec![ComputeType::VM],
            storage_types: vec![StorageType::BlockStorage],
            networking_features: vec![NetworkingFeature::VPC],
            security_features: vec![SecurityFeature::Encryption, SecurityFeature::Compliance],
            compliance_certifications: vec![],
            regions: vec![],
            max_cpu_cores: None,
            max_memory_gb: None,
            gpu_support: false,
            kubernetes_support: false,
            serverless_support: false,
        };
        enforcer
            .add_provider_compliance("low-sec", &caps)
            .await
            .unwrap();
        let report = enforcer.report_for_provider("low-sec").unwrap();
        assert!(!report.overall_pass);
    }

    #[tokio::test]
    async fn test_check_result_serialization() {
        let pass = CheckResult::Pass;
        let json_pass = serde_json::to_string(&pass).unwrap();
        let parsed_pass: CheckResult = serde_json::from_str(&json_pass).unwrap();
        assert!(matches!(parsed_pass, CheckResult::Pass));

        let fail = CheckResult::Fail {
            reason: "Missing cert".to_string(),
        };
        let json_fail = serde_json::to_string(&fail).unwrap();
        let parsed_fail: CheckResult = serde_json::from_str(&json_fail).unwrap();
        match parsed_fail {
            CheckResult::Fail { reason } => assert_eq!(reason, "Missing cert"),
            _ => panic!("Expected Fail variant"),
        }
    }

    #[tokio::test]
    async fn test_compliance_error_display() {
        let err = ComplianceError::CheckFailed("provider x failed".to_string());
        assert!(format!("{err}").contains("provider x failed"));

        let err = ComplianceError::NoRegionInfo("aws".to_string());
        assert!(format!("{err}").contains("aws"));
        assert!(format!("{err}").contains("region"));

        let err = ComplianceError::InvalidSecurityTier("unknown".to_string());
        assert!(format!("{err}").contains("unknown"));
    }

    #[tokio::test]
    async fn test_data_sovereignty_data_type_requirement_fail() {
        let cfg = make_config(
            vec![],
            vec!["us-east-1".to_string()],
            vec![DataSovereigntyRequirement {
                data_type: "pii".to_string(),
                allowed_regions: vec!["eu-west-1".to_string()],
                encryption_required: false,
            }],
        );
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        let caps = caps_full_security(vec![Region {
            name: "us-east-1".to_string(),
            location: "N. Virginia".to_string(),
            availability_zones: vec!["us-east-1a".to_string()],
        }]);
        enforcer
            .add_provider_compliance("us_only", &caps)
            .await
            .unwrap();

        let report = enforcer.report_for_provider("us_only").unwrap();
        assert!(!report.overall_pass);
        let sovereignty_check = report
            .checks
            .iter()
            .find(|c| c.check_name == "data_sovereignty")
            .expect("data_sovereignty check present");
        assert!(matches!(sovereignty_check.result, CheckResult::Fail { .. }));
        if let CheckResult::Fail { reason } = &sovereignty_check.result {
            assert!(reason.contains("pii"));
            assert!(reason.contains("eu-west-1"));
        }
    }

    #[tokio::test]
    async fn test_compliance_report_serialization() {
        let report = ComplianceReport {
            provider_name: "test-provider".to_string(),
            checks: vec![ComplianceCheck {
                check_name: "certifications".to_string(),
                result: CheckResult::Pass,
            }],
            overall_pass: true,
            compliant_regions: vec!["us-east-1".to_string()],
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: ComplianceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.provider_name, "test-provider");
        assert!(parsed.overall_pass);
    }

    #[tokio::test]
    async fn test_compliance_error_from_toadstool_error() {
        let compliance_err = ComplianceError::CheckFailed("test".to_string());
        let _toadstool_err: ToadStoolError = compliance_err.into();
    }

    #[tokio::test]
    async fn test_compute_compliant_regions_empty_required_returns_all() {
        let cfg = make_config(vec![], vec![], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        let caps = caps_full_security(vec![
            Region {
                name: "us-east-1".to_string(),
                location: "N. Virginia".to_string(),
                availability_zones: vec![],
            },
            Region {
                name: "eu-west-1".to_string(),
                location: "Ireland".to_string(),
                availability_zones: vec![],
            },
        ]);
        enforcer.add_provider_compliance("p", &caps).await.unwrap();

        let report = enforcer.report_for_provider("p").unwrap();
        assert_eq!(report.compliant_regions.len(), 2);
    }
}
