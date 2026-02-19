//! Compliance and security enforcement
//!
//! This module contains the compliance enforcer and related compliance functionality.

use std::collections::HashMap;
use toadstool::error::ToadStoolResult;

use crate::UniversalJob;

use super::types::{
    CloudCapabilities, ComplianceConfig, ComplianceConstraints, ComplianceRequirements,
};

/// Cloud compliance enforcer
pub struct CloudComplianceEnforcer {
    pub(crate) requirements: ComplianceRequirements,
    pub(crate) provider_compliance: HashMap<String, CloudCapabilities>,
}

impl CloudComplianceEnforcer {
    pub async fn new(config: ComplianceConfig) -> ToadStoolResult<Self> {
        Ok(Self {
            requirements: ComplianceRequirements {
                certifications: config.required_certifications,
                regions: config.allowed_regions,
                data_sovereignty: config.data_sovereignty_requirements,
            },
            provider_compliance: HashMap::new(),
        })
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

    pub async fn get_constraints_for_job(
        &self,
        _job: &UniversalJob,
    ) -> ToadStoolResult<ComplianceConstraints> {
        // Analyze job to determine compliance constraints
        Ok(ComplianceConstraints {
            allowed_providers: self.get_compliant_providers(),
            required_regions: self.requirements.regions.clone(),
            encryption_required: true,
        })
    }

    pub(crate) fn get_compliant_providers(&self) -> Vec<String> {
        self.provider_compliance
            .iter()
            .filter(|(_, capabilities)| self.is_provider_compliant(capabilities))
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn is_provider_compliant(&self, capabilities: &CloudCapabilities) -> bool {
        // Check if provider meets all compliance requirements
        self.requirements
            .certifications
            .iter()
            .all(|req_cert| capabilities.compliance_certifications.contains(req_cert))
    }
}

/// Cloud trust manager
#[allow(dead_code)]
pub(crate) struct CloudTrustManager {
    trust_config: super::types::TrustConfig,
    trust_relationships: HashMap<String, super::types::TrustLevel>,
}

impl CloudTrustManager {
    #[allow(dead_code)]
    pub fn new(trust_config: super::types::TrustConfig) -> Self {
        Self {
            trust_config,
            trust_relationships: HashMap::new(),
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::types::{
        CloudCapabilities, ComplianceCertification, ComputeType, NetworkingFeature,
        SecurityFeature, StorageType,
    };

    fn caps_with_certs(certs: Vec<ComplianceCertification>) -> CloudCapabilities {
        CloudCapabilities {
            compute_types: vec![ComputeType::VM],
            storage_types: vec![StorageType::BlockStorage],
            networking_features: vec![NetworkingFeature::VPC],
            security_features: vec![SecurityFeature::Encryption],
            compliance_certifications: certs,
            regions: vec![],
            max_cpu_cores: None,
            max_memory_gb: None,
            gpu_support: false,
            kubernetes_support: false,
            serverless_support: false,
        }
    }

    fn make_config(certs: Vec<ComplianceCertification>) -> ComplianceConfig {
        ComplianceConfig {
            required_certifications: certs,
            allowed_regions: vec!["us-east-1".to_string()],
            data_sovereignty_requirements: vec![],
        }
    }

    #[tokio::test]
    async fn test_new_enforcer_empty() {
        let cfg = make_config(vec![ComplianceCertification::SOC2]);
        let enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        assert!(enforcer.provider_compliance.is_empty());
        assert_eq!(enforcer.requirements.certifications.len(), 1);
    }

    #[tokio::test]
    async fn test_add_provider_compliance_registers() {
        let cfg = make_config(vec![]);
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
        let cfg = make_config(vec![ComplianceCertification::SOC2]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();

        // Provider with SOC2 — passes compliance check
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
        let cfg = make_config(vec![ComplianceCertification::SOC2]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();

        // Provider without SOC2 — fails compliance check
        let caps_bad = caps_with_certs(vec![]);
        enforcer
            .add_provider_compliance("bad", &caps_bad)
            .await
            .unwrap();

        let compliant = enforcer.get_compliant_providers();
        assert!(!compliant.contains(&"bad".to_string()));
    }

    #[tokio::test]
    async fn test_mixed_compliance_filtering() {
        let cfg = make_config(vec![ComplianceCertification::HIPAA]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();

        enforcer
            .add_provider_compliance(
                "hipaa-compliant",
                &caps_with_certs(vec![ComplianceCertification::HIPAA]),
            )
            .await
            .unwrap();
        enforcer
            .add_provider_compliance(
                "soc2-only",
                &caps_with_certs(vec![ComplianceCertification::SOC2]),
            )
            .await
            .unwrap();

        let compliant = enforcer.get_compliant_providers();
        assert!(compliant.contains(&"hipaa-compliant".to_string()));
        assert!(!compliant.contains(&"soc2-only".to_string()));
    }

    #[tokio::test]
    async fn test_no_requirements_all_providers_compliant() {
        let cfg = make_config(vec![]); // no required certs
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();

        enforcer
            .add_provider_compliance("any-provider", &caps_with_certs(vec![]))
            .await
            .unwrap();

        let compliant = enforcer.get_compliant_providers();
        assert!(compliant.contains(&"any-provider".to_string()));
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
}
