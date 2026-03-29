// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cloud provider capabilities for placement decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudCapabilities {
    /// Supported compute types.
    pub compute_types: Vec<ComputeType>,
    /// Supported storage types.
    pub storage_types: Vec<StorageType>,
    /// Networking features.
    pub networking_features: Vec<NetworkingFeature>,
    /// Security features.
    pub security_features: Vec<SecurityFeature>,
    /// Compliance certifications.
    pub compliance_certifications: Vec<ComplianceCertification>,
    /// Available regions.
    pub regions: Vec<Region>,
    /// Max CPU cores (if limited).
    pub max_cpu_cores: Option<u32>,
    /// Max memory in GB (if limited).
    pub max_memory_gb: Option<u32>,
    /// GPU support available.
    pub gpu_support: bool,
    /// Kubernetes support.
    pub kubernetes_support: bool,
    /// Serverless support.
    pub serverless_support: bool,
}

/// Cloud provider metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviderMetadata {
    /// Provider name.
    pub name: String,
    /// Provider version.
    pub version: String,
    /// API version.
    pub api_version: String,
    /// Supported protocols.
    pub supported_protocols: Vec<String>,
    /// Documentation URL.
    pub documentation_url: String,
    /// Support contact.
    pub support_contact: String,
}

/// Resource specifications for cloud placement.
#[derive(Debug, Clone)]
pub struct ResourceSpec {
    /// CPU cores.
    pub cpu_cores: f64,
    /// Memory in GB.
    pub memory_gb: f64,
    /// Storage in GB.
    pub storage_gb: f64,
    /// GPU count (optional).
    pub gpu_count: Option<u32>,
    /// Network bandwidth in Mbps (optional).
    pub network_bandwidth_mbps: Option<u64>,
}

/// Pricing information for cost estimation.
#[derive(Debug, Clone)]
pub struct PricingInfo {
    /// CPU cost per hour.
    pub cpu_cost_per_hour: f64,
    /// Memory cost per GB-hour.
    pub memory_cost_per_gb_hour: f64,
    /// Storage cost per GB-month.
    pub storage_cost_per_gb_month: f64,
    /// Network cost per GB.
    pub network_cost_per_gb: f64,
    /// Total estimated cost.
    pub total_estimated_cost: f64,
}

/// Availability information for a provider/region.
#[derive(Debug, Clone)]
pub struct AvailabilityInfo {
    /// Available CPU cores.
    pub cpu_cores: f64,
    /// Available memory in GB.
    pub memory_gb: f64,
    /// Available storage in GB.
    pub storage_gb: f64,
    /// Available GPU count.
    pub gpu_count: u32,
    /// Available regions.
    pub regions: Vec<String>,
    /// Availability zones.
    pub availability_zones: Vec<String>,
}

/// Multi-cloud availability tracking.
#[derive(Debug, Clone)]
pub struct MultiCloudAvailability {
    /// Per-provider availability.
    providers: HashMap<String, AvailabilityInfo>,
    /// Unavailable providers.
    unavailable_providers: Vec<String>,
}

impl Default for MultiCloudAvailability {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiCloudAvailability {
    /// Creates an empty multi-cloud availability tracker.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            unavailable_providers: Vec::new(),
        }
    }

    /// Adds a provider with availability info.
    pub fn add_provider(&mut self, name: impl Into<String>, availability: AvailabilityInfo) {
        self.providers.insert(name.into(), availability);
    }

    /// Marks a provider as unavailable.
    pub fn mark_provider_unavailable(&mut self, name: impl Into<String>) {
        self.unavailable_providers.push(name.into());
    }

    /// Returns the names of providers that reported availability successfully.
    pub fn available_provider_names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// Returns the names of providers that failed availability checks.
    pub fn unavailable_provider_names(&self) -> &[String] {
        &self.unavailable_providers
    }
}

/// Cloud region information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    /// Region name.
    pub name: String,
    /// Geographic location.
    pub location: String,
    /// Availability zones in region.
    pub availability_zones: Vec<String>,
}

/// Compute type for cloud placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputeType {
    /// Virtual machine.
    VM,
    /// Container.
    Container,
    /// Serverless (lambda, etc.).
    Serverless,
    /// Bare metal.
    BareMetalC,
    /// GPU instance.
    GPU,
    /// FPGA instance.
    FPGA,
}

/// Storage type for cloud placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    /// Block storage (EBS, etc.).
    BlockStorage,
    /// Object storage (S3, etc.).
    ObjectStorage,
    /// File storage (EFS, etc.).
    FileStorage,
    /// Database storage.
    DatabaseStorage,
}

/// Networking feature for cloud placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkingFeature {
    /// VPC support.
    VPC,
    /// Load balancer.
    LoadBalancer,
    /// CDN.
    CDN,
    /// Private networking.
    PrivateNetworking,
    /// VPN.
    VPN,
}

/// Security feature for compliance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SecurityFeature {
    /// Encryption at rest/transit.
    Encryption,
    /// Identity and access management.
    IdentityManagement,
    /// Network security (firewall, etc.).
    NetworkSecurity,
    /// Compliance tooling.
    Compliance,
}

/// Compliance certification for cloud provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceCertification {
    /// SOC 2.
    SOC2,
    /// ISO 27001.
    ISO27001,
    /// HIPAA.
    HIPAA,
    /// PCI DSS.
    PciDss,
    /// GDPR.
    GDPR,
    /// FedRAMP.
    FedRAMP,
    /// Custom certification.
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
