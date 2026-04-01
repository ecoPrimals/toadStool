// SPDX-License-Identifier: AGPL-3.0-only
//! Compliance checking logic: certifications, data sovereignty, security tier, resource isolation.
//!
//! Produces structured compliance reports with pass/fail per check.

use std::collections::HashSet;
use thiserror::Error;
use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::UniversalJob;

use super::security_tier::SecurityTier;
use crate::cloud::types::{
    CloudCapabilities, ComplianceConfig, ComplianceConstraints, ComplianceRequirements,
    NetworkingFeature, SecurityFeature,
};

// ─── Report Types ───────────────────────────────────────────────────────────

/// Result of a single compliance check.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CheckResult {
    /// Check succeeded.
    Pass,
    /// Check failed with an explanation.
    Fail {
        /// Why the check did not pass.
        reason: String,
    },
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

// ─── Compliance Errors ───────────────────────────────────────────────────────

/// Compliance-related errors.
#[derive(Debug, Error)]
pub enum ComplianceError {
    /// A compliance rule returned a failure result.
    #[error("Compliance check failed: {0}")]
    CheckFailed(String),

    /// Provider lacks region metadata needed for sovereignty validation.
    #[error("Provider '{0}' has no region information for sovereignty check")]
    NoRegionInfo(String),

    /// Security tier string or value could not be parsed or applied.
    #[error("Invalid security tier: {0}")]
    InvalidSecurityTier(String),
}

impl From<ComplianceError> for ToadStoolError {
    fn from(e: ComplianceError) -> Self {
        Self::security(e.to_string())
    }
}

// ─── CloudComplianceEnforcer ─────────────────────────────────────────────────

/// Cloud compliance enforcer with data sovereignty, security tier, and isolation checks.
pub struct CloudComplianceEnforcer {
    /// Certifications, regions, and data-sovereignty rules to enforce.
    pub(crate) requirements: ComplianceRequirements,
    /// Registered provider id → advertised capabilities.
    pub(crate) provider_compliance: std::collections::HashMap<String, CloudCapabilities>,
    /// Required security features implied by this tier.
    pub(crate) security_tier: SecurityTier,
}

impl CloudComplianceEnforcer {
    /// Creates an enforcer from compliance configuration (default security tier: standard).
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
    #[allow(clippy::missing_const_for_fn)] // Mutates self; CloudComplianceEnforcer contains HashMap
    pub fn with_security_tier(mut self, tier: SecurityTier) -> Self {
        self.security_tier = tier;
        self
    }

    /// Registers or updates a provider's capabilities for reports and job constraints.
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

    /// Builds job-level compliance constraints from currently compliant providers and policy.
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

    /// Provider names whose [`ComplianceReport::overall_pass`] is true.
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
            let all_required: HashSet<_> = self
                .requirements
                .data_sovereignty
                .iter()
                .flat_map(|ds| ds.allowed_regions.clone())
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
                NetworkingFeature::PrivateNetworking | NetworkingFeature::VPN
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
