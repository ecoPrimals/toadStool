// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compliance module tests.

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::*;
    use crate::cloud::types::{
        CloudCapabilities, ComplianceCertification, ComplianceConfig, ComputeType,
        DataSovereigntyRequirement, NetworkingFeature, Region, SecurityFeature, StorageType,
    };
    use toadstool::error::ToadStoolError;

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
            created_at: std::time::SystemTime::now(),
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

    #[tokio::test]
    async fn test_compliance_validation_data_sovereignty_empty_required() {
        let cfg = make_config(vec![], vec![], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        let caps = caps_full_security(vec![Region {
            name: "us-east-1".to_string(),
            location: "N. Virginia".to_string(),
            availability_zones: vec!["us-east-1a".to_string()],
        }]);
        enforcer.add_provider_compliance("p", &caps).await.unwrap();
        let report = enforcer.report_for_provider("p").unwrap();
        let sovereignty = report
            .checks
            .iter()
            .find(|c| c.check_name == "data_sovereignty")
            .expect("data_sovereignty check");
        assert!(matches!(sovereignty.result, CheckResult::Pass));
    }

    #[tokio::test]
    async fn test_tier_evaluation_compliant_regions_intersection() {
        let cfg = make_config(
            vec![],
            vec!["us-east-1".to_string(), "eu-west-1".to_string()],
            vec![],
        );
        let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
        let caps = caps_full_security(vec![
            Region {
                name: "us-east-1".to_string(),
                location: "N. Virginia".to_string(),
                availability_zones: vec![],
            },
            Region {
                name: "ap-south-1".to_string(),
                location: "Mumbai".to_string(),
                availability_zones: vec![],
            },
        ]);
        enforcer.add_provider_compliance("p", &caps).await.unwrap();
        let report = enforcer.report_for_provider("p").unwrap();
        assert_eq!(report.compliant_regions.len(), 1);
        assert!(report.compliant_regions.contains(&"us-east-1".to_string()));
    }

    #[tokio::test]
    async fn test_security_tier_basic_skips_resource_isolation() {
        let cfg = make_config(vec![], vec![], vec![]);
        let mut enforcer = CloudComplianceEnforcer::new(cfg)
            .await
            .unwrap()
            .with_security_tier(SecurityTier::Basic);
        let caps = CloudCapabilities {
            compute_types: vec![ComputeType::VM],
            storage_types: vec![StorageType::BlockStorage],
            networking_features: vec![NetworkingFeature::VPC],
            security_features: vec![SecurityFeature::Encryption],
            compliance_certifications: vec![],
            regions: vec![],
            max_cpu_cores: None,
            max_memory_gb: None,
            gpu_support: false,
            kubernetes_support: false,
            serverless_support: false,
        };
        enforcer
            .add_provider_compliance("basic", &caps)
            .await
            .unwrap();
        let report = enforcer.report_for_provider("basic").unwrap();
        let isolation = report
            .checks
            .iter()
            .find(|c| c.check_name == "resource_isolation")
            .expect("resource_isolation check");
        assert!(matches!(isolation.result, CheckResult::Pass));
    }
}
