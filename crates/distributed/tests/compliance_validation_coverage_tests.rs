// SPDX-License-Identifier: AGPL-3.0-only
//! Integration coverage for [`toadstool_distributed::cloud::compliance::validation`]: compliance
//! reports, certification and sovereignty rules, security tiers, resource isolation, and
//! constraint derivation for jobs.

use std::time::SystemTime;

use toadstool::ExecutionRequest;
use toadstool::error::ToadStoolError;
use toadstool_distributed::cloud::{
    CheckResult, CloudCapabilities, CloudComplianceEnforcer, ComplianceCertification,
    ComplianceCheck, ComplianceConfig, ComplianceConstraints, ComplianceError, ComplianceReport,
    ComputeType, DataSovereigntyRequirement, NetworkingFeature, Region, SecurityFeature,
    SecurityTier, StorageType,
};
use toadstool_distributed::{
    DistributedRetryConfig, ExecutionTarget, JobPriority, ResourceRequirements, UniversalJob,
};
use uuid::Uuid;

fn compliance_config(
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

fn region(name: &str) -> Region {
    Region {
        name: name.to_string(),
        location: String::new(),
        availability_zones: vec![],
    }
}

fn base_capabilities() -> CloudCapabilities {
    CloudCapabilities {
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
    }
}

fn high_tier_isolation_ready_capabilities(regions: Vec<Region>) -> CloudCapabilities {
    CloudCapabilities {
        networking_features: vec![NetworkingFeature::VPC, NetworkingFeature::PrivateNetworking],
        security_features: vec![
            SecurityFeature::Encryption,
            SecurityFeature::Compliance,
            SecurityFeature::NetworkSecurity,
        ],
        regions,
        ..base_capabilities()
    }
}

fn universal_job_stub() -> UniversalJob {
    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: None,
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    }
}

#[tokio::test]
async fn check_result_pass_and_fail_serde_roundtrip() {
    let pass = CheckResult::Pass;
    let json = serde_json::to_string(&pass).unwrap();
    let back: CheckResult = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, CheckResult::Pass));

    let fail = CheckResult::Fail {
        reason: "r".to_string(),
    };
    let json = serde_json::to_string(&fail).unwrap();
    let back: CheckResult = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        back,
        CheckResult::Fail { reason } if reason == "r"
    ));
}

#[tokio::test]
async fn check_result_fail_clone_copies_reason() {
    let a = CheckResult::Fail {
        reason: "x".to_string(),
    };
    let b = a;
    assert!(matches!(b, CheckResult::Fail { ref reason } if reason == "x"));
}

#[tokio::test]
async fn compliance_check_clone_and_serde_roundtrip() {
    let c = ComplianceCheck {
        check_name: "n".to_string(),
        result: CheckResult::Pass,
    };
    let d = c;
    assert_eq!(d.check_name, "n");

    let json = serde_json::to_string(&d).unwrap();
    let back: ComplianceCheck = serde_json::from_str(&json).unwrap();
    assert_eq!(back.check_name, "n");
    assert!(matches!(back.result, CheckResult::Pass));
}

#[tokio::test]
async fn compliance_report_clone_and_serde_roundtrip() {
    let r = ComplianceReport {
        provider_name: "p".to_string(),
        checks: vec![],
        overall_pass: true,
        compliant_regions: vec!["a".to_string()],
    };
    let s = r;
    assert_eq!(s.compliant_regions, vec!["a".to_string()]);

    let json = serde_json::to_string(&s).unwrap();
    let back: ComplianceReport = serde_json::from_str(&json).unwrap();
    assert_eq!(back.provider_name, "p");
    assert!(back.overall_pass);
    assert_eq!(back.compliant_regions, vec!["a".to_string()]);
}

#[test]
fn compliance_error_display_and_into_toadstool_error() {
    let e = ComplianceError::CheckFailed("cf".to_string());
    assert!(e.to_string().contains("cf"));

    let e = ComplianceError::NoRegionInfo("prov".to_string());
    let s = e.to_string();
    assert!(s.contains("prov"));
    assert!(s.contains("region"));

    let e = ComplianceError::InvalidSecurityTier("bad".to_string());
    assert!(e.to_string().contains("bad"));

    let ts: ToadStoolError = ComplianceError::CheckFailed("x".into()).into();
    assert!(!ts.to_string().is_empty());
}

#[tokio::test]
async fn new_enforcer_applies_config_regions_to_job_constraints() {
    let cfg = compliance_config(
        vec![ComplianceCertification::SOC2],
        vec!["eu-central-1".to_string()],
        vec![],
    );
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.compliance_certifications = vec![ComplianceCertification::SOC2];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let constraints = enforcer
        .get_constraints_for_job(&universal_job_stub())
        .await
        .unwrap();
    assert_eq!(
        constraints.required_regions,
        vec!["eu-central-1".to_string()]
    );
}

#[tokio::test]
async fn report_for_provider_unregistered_returns_check_failed() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let err = enforcer.report_for_provider("missing").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("missing"));
    assert!(msg.contains("not registered"));
}

#[tokio::test]
async fn certification_rule_passes_when_required_empty() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.compliance_certifications = vec![];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let c = report
        .checks
        .iter()
        .find(|x| x.check_name == "certifications")
        .unwrap();
    assert!(matches!(c.result, CheckResult::Pass));
}

#[tokio::test]
async fn certification_rule_fails_when_required_missing() {
    let cfg = compliance_config(
        vec![ComplianceCertification::SOC2, ComplianceCertification::GDPR],
        vec![],
        vec![],
    );
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.compliance_certifications = vec![ComplianceCertification::SOC2];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let c = report
        .checks
        .iter()
        .find(|x| x.check_name == "certifications")
        .unwrap();
    match &c.result {
        CheckResult::Fail { reason } => {
            assert!(reason.contains("Missing"));
            assert!(reason.contains("GDPR"));
        }
        CheckResult::Pass => panic!("expected certification failure"),
    }
    assert!(!report.overall_pass);
}

#[tokio::test]
async fn certification_rule_accepts_custom_certification_match() {
    let cfg = compliance_config(
        vec![ComplianceCertification::Custom("acme".to_string())],
        vec![],
        vec![],
    );
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.compliance_certifications = vec![ComplianceCertification::Custom("acme".to_string())];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let c = report
        .checks
        .iter()
        .find(|x| x.check_name == "certifications")
        .unwrap();
    assert!(matches!(c.result, CheckResult::Pass));
}

#[tokio::test]
async fn data_sovereignty_passes_when_no_region_requirements_configured() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.regions = vec![region("us-east-1")];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let c = report
        .checks
        .iter()
        .find(|x| x.check_name == "data_sovereignty")
        .unwrap();
    assert!(matches!(c.result, CheckResult::Pass));
}

#[tokio::test]
async fn data_sovereignty_fails_when_allowed_region_not_in_provider_regions() {
    let cfg = compliance_config(vec![], vec!["eu-south-1".to_string()], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.regions = vec![region("us-east-1")];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let c = report
        .checks
        .iter()
        .find(|x| x.check_name == "data_sovereignty")
        .unwrap();
    assert!(matches!(c.result, CheckResult::Fail { .. }));
    assert!(!report.overall_pass);
}

#[tokio::test]
async fn data_sovereignty_passes_when_provider_has_one_of_required_regions() {
    let cfg = compliance_config(
        vec![],
        vec!["us-east-1".to_string(), "eu-west-1".to_string()],
        vec![],
    );
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.regions = vec![region("us-east-1")];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let c = report
        .checks
        .iter()
        .find(|x| x.check_name == "data_sovereignty")
        .unwrap();
    assert!(matches!(c.result, CheckResult::Pass));
}

#[tokio::test]
async fn data_sovereignty_per_data_type_requires_region_in_allowed_set() {
    let cfg = compliance_config(
        vec![],
        vec!["us-east-1".to_string()],
        vec![DataSovereigntyRequirement {
            data_type: "health".to_string(),
            allowed_regions: vec!["eu-west-1".to_string()],
            encryption_required: true,
        }],
    );
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.regions = vec![region("us-east-1")];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let c = report
        .checks
        .iter()
        .find(|x| x.check_name == "data_sovereignty")
        .unwrap();
    match &c.result {
        CheckResult::Fail { reason } => {
            assert!(reason.contains("health"));
            assert!(reason.contains("eu-west-1"));
        }
        CheckResult::Pass => panic!("expected sovereignty failure"),
    }
}

#[tokio::test]
async fn security_tier_standard_requires_encryption_and_compliance_features() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.security_features = vec![SecurityFeature::Encryption];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let names: Vec<_> = report
        .checks
        .iter()
        .filter(|c| c.check_name.starts_with("security_"))
        .map(|c| c.check_name.as_str())
        .collect();
    assert!(names.contains(&"security_encryption"));
    assert!(names.contains(&"security_compliance"));
    let compliance_check = report
        .checks
        .iter()
        .find(|c| c.check_name == "security_compliance")
        .unwrap();
    assert!(matches!(compliance_check.result, CheckResult::Fail { .. }));
    assert!(!report.overall_pass);
}

#[tokio::test]
async fn security_tier_basic_requires_only_encryption() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg)
        .await
        .unwrap()
        .with_security_tier(SecurityTier::Basic);
    let mut caps = base_capabilities();
    caps.security_features = vec![SecurityFeature::Encryption];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    assert!(report.overall_pass);
    assert_eq!(
        report
            .checks
            .iter()
            .filter(|c| c.check_name.starts_with("security_"))
            .count(),
        1
    );
}

#[test]
fn security_tier_required_features_cover_basic_standard_and_high() {
    assert_eq!(SecurityTier::Basic.required_features().len(), 1);
    assert_eq!(SecurityTier::Standard.required_features().len(), 2);
    assert_eq!(SecurityTier::High.required_features().len(), 3);
    let t = SecurityTier::High;
    assert_eq!(t, t);
}

#[tokio::test]
async fn high_security_tier_emits_network_security_check() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg)
        .await
        .unwrap()
        .with_security_tier(SecurityTier::High);
    let caps = high_tier_isolation_ready_capabilities(vec![]);
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    assert!(
        report
            .checks
            .iter()
            .any(|c| c.check_name == "security_networksecurity")
    );
}

#[tokio::test]
async fn resource_isolation_high_tier_passes_with_network_security_and_private_networking() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg)
        .await
        .unwrap()
        .with_security_tier(SecurityTier::High);
    let caps = high_tier_isolation_ready_capabilities(vec![region("us-east-1")]);
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let iso = report
        .checks
        .iter()
        .find(|c| c.check_name == "resource_isolation")
        .unwrap();
    assert!(matches!(iso.result, CheckResult::Pass));
    assert!(report.overall_pass);
}

#[tokio::test]
async fn resource_isolation_high_tier_passes_with_vpn_instead_of_private_networking() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg)
        .await
        .unwrap()
        .with_security_tier(SecurityTier::High);
    let mut caps = high_tier_isolation_ready_capabilities(vec![]);
    caps.networking_features = vec![NetworkingFeature::VPC, NetworkingFeature::VPN];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let iso = report
        .checks
        .iter()
        .find(|c| c.check_name == "resource_isolation")
        .unwrap();
    assert!(matches!(iso.result, CheckResult::Pass));
}

#[tokio::test]
async fn resource_isolation_high_tier_fails_without_segmentation_features() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg)
        .await
        .unwrap()
        .with_security_tier(SecurityTier::High);
    let mut caps = base_capabilities();
    caps.security_features = vec![
        SecurityFeature::Encryption,
        SecurityFeature::Compliance,
        SecurityFeature::NetworkSecurity,
    ];
    caps.networking_features = vec![NetworkingFeature::VPC];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let iso = report
        .checks
        .iter()
        .find(|c| c.check_name == "resource_isolation")
        .unwrap();
    match &iso.result {
        CheckResult::Fail { reason } => {
            assert!(reason.contains("High security tier"));
        }
        CheckResult::Pass => panic!("expected resource isolation failure"),
    }
    assert!(!report.overall_pass);
}

#[tokio::test]
async fn resource_isolation_non_high_tier_always_passes() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg)
        .await
        .unwrap()
        .with_security_tier(SecurityTier::Standard);
    let mut caps = base_capabilities();
    caps.networking_features = vec![NetworkingFeature::VPC];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let iso = report
        .checks
        .iter()
        .find(|c| c.check_name == "resource_isolation")
        .unwrap();
    assert!(matches!(iso.result, CheckResult::Pass));
}

#[tokio::test]
async fn compliant_regions_lists_all_provider_regions_when_no_requirement_filter() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.regions = vec![region("a"), region("b")];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    let mut rs = report.compliant_regions;
    rs.sort();
    assert_eq!(rs, vec!["a".to_string(), "b".to_string()]);
}

#[tokio::test]
async fn compliant_regions_intersects_with_allowed_regions_configuration() {
    let cfg = compliance_config(
        vec![],
        vec!["x".to_string(), "y".to_string(), "z".to_string()],
        vec![],
    );
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.regions = vec![region("x"), region("w")];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    assert_eq!(report.compliant_regions, vec!["x".to_string()]);
}

#[tokio::test]
async fn get_constraints_for_job_lists_only_compliant_registered_providers() {
    let cfg = compliance_config(vec![ComplianceCertification::SOC2], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();

    let mut good = base_capabilities();
    good.compliance_certifications = vec![ComplianceCertification::SOC2];
    enforcer.add_provider_compliance("ok", &good).await.unwrap();

    let mut bad = base_capabilities();
    bad.compliance_certifications = vec![];
    enforcer.add_provider_compliance("no", &bad).await.unwrap();

    let constraints = enforcer
        .get_constraints_for_job(&universal_job_stub())
        .await
        .unwrap();
    assert!(constraints.allowed_providers.contains(&"ok".to_string()));
    assert!(!constraints.allowed_providers.contains(&"no".to_string()));
}

#[tokio::test]
async fn get_constraints_for_job_reflects_requirements_and_compliant_providers() {
    let cfg = compliance_config(
        vec![ComplianceCertification::SOC2],
        vec!["r1".to_string(), "r2".to_string()],
        vec![],
    );
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.compliance_certifications = vec![ComplianceCertification::SOC2];
    caps.regions = vec![region("r1")];
    enforcer
        .add_provider_compliance("prov", &caps)
        .await
        .unwrap();

    let constraints: ComplianceConstraints = enforcer
        .get_constraints_for_job(&universal_job_stub())
        .await
        .unwrap();
    assert!(constraints.encryption_required);
    assert_eq!(
        constraints.required_regions,
        vec!["r1".to_string(), "r2".to_string()]
    );
    assert!(constraints.allowed_providers.contains(&"prov".to_string()));
}

#[tokio::test]
async fn get_constraints_for_job_empty_allowed_providers_when_none_compliant() {
    let cfg = compliance_config(vec![ComplianceCertification::SOC2], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.compliance_certifications = vec![];
    enforcer
        .add_provider_compliance("prov", &caps)
        .await
        .unwrap();

    let constraints = enforcer
        .get_constraints_for_job(&universal_job_stub())
        .await
        .unwrap();
    assert!(constraints.allowed_providers.is_empty());
}

#[tokio::test]
async fn overall_pass_true_only_when_every_check_passes() {
    let cfg = compliance_config(vec![], vec![], vec![]);
    let mut enforcer = CloudComplianceEnforcer::new(cfg).await.unwrap();
    let mut caps = base_capabilities();
    caps.security_features = vec![SecurityFeature::Encryption, SecurityFeature::Compliance];
    enforcer.add_provider_compliance("p", &caps).await.unwrap();
    let report = enforcer.report_for_provider("p").unwrap();
    assert!(report.overall_pass);
    assert!(
        report
            .checks
            .iter()
            .all(|c| matches!(c.result, CheckResult::Pass))
    );
}
