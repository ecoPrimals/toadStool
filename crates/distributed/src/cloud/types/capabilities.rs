// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cloud capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudCapabilities {
    pub compute_types: Vec<ComputeType>,
    pub storage_types: Vec<StorageType>,
    pub networking_features: Vec<NetworkingFeature>,
    pub security_features: Vec<SecurityFeature>,
    pub compliance_certifications: Vec<ComplianceCertification>,
    pub regions: Vec<Region>,
    pub max_cpu_cores: Option<u32>,
    pub max_memory_gb: Option<u32>,
    pub gpu_support: bool,
    pub kubernetes_support: bool,
    pub serverless_support: bool,
}

/// Cloud provider metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviderMetadata {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub supported_protocols: Vec<String>,
    pub documentation_url: String,
    pub support_contact: String,
}

/// Resource specifications
#[derive(Debug, Clone)]
pub struct ResourceSpec {
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub storage_gb: f64,
    pub gpu_count: Option<u32>,
    pub network_bandwidth_mbps: Option<u64>,
}

/// Pricing information
#[derive(Debug, Clone)]
pub struct PricingInfo {
    pub cpu_cost_per_hour: f64,
    pub memory_cost_per_gb_hour: f64,
    pub storage_cost_per_gb_month: f64,
    pub network_cost_per_gb: f64,
    pub total_estimated_cost: f64,
}

/// Availability information
#[derive(Debug, Clone)]
pub struct AvailabilityInfo {
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub storage_gb: f64,
    pub gpu_count: u32,
    pub regions: Vec<String>,
    pub availability_zones: Vec<String>,
}

/// Multi-cloud availability tracking
#[derive(Debug, Clone)]
pub struct MultiCloudAvailability {
    providers: HashMap<String, AvailabilityInfo>,
    unavailable_providers: Vec<String>,
}

impl Default for MultiCloudAvailability {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiCloudAvailability {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            unavailable_providers: Vec::new(),
        }
    }

    pub fn add_provider(&mut self, name: impl Into<String>, availability: AvailabilityInfo) {
        self.providers.insert(name.into(), availability);
    }

    pub fn mark_provider_unavailable(&mut self, name: impl Into<String>) {
        self.unavailable_providers.push(name.into());
    }
}

/// Region information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub name: String,
    pub location: String,
    pub availability_zones: Vec<String>,
}

/// Compute type options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputeType {
    VM,
    Container,
    Serverless,
    BareMetalC,
    GPU,
    FPGA,
}

/// Storage type options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    BlockStorage,
    ObjectStorage,
    FileStorage,
    DatabaseStorage,
}

/// Networking feature options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkingFeature {
    VPC,
    LoadBalancer,
    CDN,
    PrivateNetworking,
    VPN,
}

/// Security feature options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SecurityFeature {
    Encryption,
    IdentityManagement,
    NetworkSecurity,
    Compliance,
}

/// Compliance certifications
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceCertification {
    SOC2,
    ISO27001,
    HIPAA,
    PciDss,
    GDPR,
    FedRAMP,
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_cloud_availability_new() {
        let avail = MultiCloudAvailability::new();
        assert!(avail.providers.is_empty());
    }

    #[test]
    fn test_multi_cloud_availability_add_provider() {
        let mut avail = MultiCloudAvailability::new();
        avail.add_provider(
            "aws",
            AvailabilityInfo {
                cpu_cores: 64.0,
                memory_gb: 256.0,
                storage_gb: 1000.0,
                gpu_count: 4,
                regions: vec!["us-east-1".to_string()],
                availability_zones: vec!["us-east-1a".to_string()],
            },
        );
        let _ = &avail;
    }

    #[test]
    fn test_multi_cloud_availability_mark_unavailable() {
        let mut avail = MultiCloudAvailability::new();
        avail.mark_provider_unavailable("gcp");
        let _ = &avail;
    }

    #[test]
    fn test_region_construction() {
        let region = Region {
            name: "us-east-1".to_string(),
            location: "N. Virginia".to_string(),
            availability_zones: vec!["us-east-1a".to_string(), "us-east-1b".to_string()],
        };
        assert_eq!(region.name, "us-east-1");
        assert_eq!(region.availability_zones.len(), 2);
    }

    #[test]
    fn test_compliance_certification_serialization_roundtrip() {
        for cert in [
            ComplianceCertification::SOC2,
            ComplianceCertification::ISO27001,
            ComplianceCertification::HIPAA,
            ComplianceCertification::GDPR,
            ComplianceCertification::Custom("custom-cert".to_string()),
        ] {
            let json = serde_json::to_string(&cert).expect("serialize");
            let parsed: ComplianceCertification = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(format!("{cert:?}"), format!("{parsed:?}"));
        }
    }

    #[test]
    fn test_compute_type_serialization_roundtrip() {
        for ct in [
            ComputeType::VM,
            ComputeType::Container,
            ComputeType::Serverless,
            ComputeType::GPU,
        ] {
            let json = serde_json::to_string(&ct).expect("serialize");
            let parsed: ComputeType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(format!("{ct:?}"), format!("{parsed:?}"));
        }
    }

    #[test]
    fn test_multi_cloud_availability_default() {
        let _avail = MultiCloudAvailability::default();
        // default() delegates to new() which initializes empty providers
    }
}
