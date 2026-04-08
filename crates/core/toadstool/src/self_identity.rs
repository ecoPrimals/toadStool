// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Self-Identity Module
//!
//! "Know Yourself, Discover Others at Runtime"
//!
//! This module implements ToadStool's self-awareness - knowing only what it IS,
//! not what others ARE. Other primals are discovered at runtime via capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// ToadStool's self-identity - what we know about OURSELVES
///
/// This struct contains ONLY self-knowledge. No peer information, no hardcoded
/// endpoints, no assumptions about other primals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfIdentity {
    /// Our unique instance ID (generated at startup)
    pub instance_id: Uuid,

    /// Our primal type (always "toadstool")
    pub primal_type: &'static str,

    /// Our version
    pub version: String,

    /// Our capabilities (what we CAN do)
    pub capabilities: Vec<Capability>,

    /// Our requirements (what we NEED from others)
    pub requirements: Vec<CapabilityRequirement>,

    /// Our network identity (if applicable)
    pub network: Option<NetworkIdentity>,

    /// Our resource limits
    pub resources: ResourceProfile,
}

/// A capability that this instance provides
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capability {
    /// Capability name (e.g., "compute", "orchestration")
    pub name: String,

    /// Capability version
    pub version: String,

    /// Sub-capabilities or features
    pub features: Vec<String>,

    /// Performance characteristics
    pub characteristics: HashMap<String, String>,
}

/// A capability requirement - what we need from the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequirement {
    /// Required capability name
    pub capability: String,

    /// Minimum version (semver)
    pub min_version: Option<String>,

    /// Is this required or optional?
    pub required: bool,

    /// Specific features needed
    pub features: Vec<String>,

    /// Why do we need this?
    pub purpose: String,
}

/// Network identity for this instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIdentity {
    /// Hostname we're running on
    pub hostname: String,

    /// Port we're listening on (if applicable)
    pub port: Option<u16>,

    /// Our advertised endpoint
    pub endpoint: String,

    /// Protocols we support
    pub protocols: Vec<String>,
}

/// Resource profile for this instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceProfile {
    /// Available CPU cores
    pub cpu_cores: usize,

    /// Available memory (bytes)
    pub memory_bytes: u64,

    /// GPU availability
    pub gpu_available: bool,

    /// Storage available (bytes)
    pub storage_bytes: Option<u64>,
}

impl SelfIdentity {
    /// Create our self-identity
    ///
    /// This is the ONLY information we know about ourselves at startup.
    /// Everything else is discovered at runtime.
    pub fn new() -> Self {
        Self {
            instance_id: Uuid::new_v4(),
            primal_type: "toadstool",
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: Self::our_capabilities(),
            requirements: Self::our_requirements(),
            network: None, // Set by caller if networking is enabled
            resources: Self::detect_resources(),
        }
    }

    /// Define what we CAN do (our capabilities)
    fn our_capabilities() -> Vec<Capability> {
        vec![
            Capability {
                name: "compute".to_string(),
                version: "1.0".to_string(),
                features: vec!["cpu".to_string(), "parallel".to_string()],
                characteristics: {
                    let mut map = HashMap::new();
                    map.insert("type".to_string(), "universal".to_string());
                    map.insert("scheduling".to_string(), "intelligent".to_string());
                    map
                },
            },
            Capability {
                name: "orchestration".to_string(),
                version: "1.0".to_string(),
                features: vec![
                    "workload-management".to_string(),
                    "resource-allocation".to_string(),
                    "auto-scheduling".to_string(),
                ],
                characteristics: HashMap::new(),
            },
            Capability {
                name: "byob".to_string(), // Bring Your Own Binary
                version: "1.0".to_string(),
                features: vec!["deployment".to_string(), "lifecycle".to_string()],
                characteristics: HashMap::new(),
            },
        ]
    }

    /// Define what we NEED from others (our requirements)
    fn our_requirements() -> Vec<CapabilityRequirement> {
        vec![
            CapabilityRequirement {
                capability: "coordination".to_string(),
                min_version: Some("1.0".to_string()),
                required: false, // Optional - we can work standalone
                features: vec!["routing".to_string(), "discovery".to_string()],
                purpose: "Network coordination and service discovery".to_string(),
            },
            CapabilityRequirement {
                capability: "storage".to_string(),
                min_version: Some("1.0".to_string()),
                required: false, // Optional - we can use local storage
                features: vec!["object-store".to_string(), "metadata".to_string()],
                purpose: "Persistent storage for workloads and artifacts".to_string(),
            },
            CapabilityRequirement {
                capability: "security".to_string(),
                min_version: Some("1.0".to_string()),
                required: false, // Optional - we have basic security
                features: vec!["authentication".to_string(), "policy".to_string()],
                purpose: "Enhanced security and policy enforcement".to_string(),
            },
            CapabilityRequirement {
                capability: "ai".to_string(),
                min_version: Some("1.0".to_string()),
                required: false, // Optional - for enhanced features
                features: vec!["orchestration".to_string(), "optimization".to_string()],
                purpose: "AI-powered workload optimization and orchestration".to_string(),
            },
        ]
    }

    /// Detect our resource profile
    fn detect_resources() -> ResourceProfile {
        ResourceProfile {
            cpu_cores: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or_else(|_| {
                    tracing::warn!("Failed to detect CPU parallelism, defaulting to 1 core");
                    1
                }),
            memory_bytes: Self::detect_memory(),
            gpu_available: Self::detect_gpu(),
            storage_bytes: None, // Could be detected if needed
        }
    }

    fn detect_memory() -> u64 {
        toadstool_sysmon::memory_info()
            .map(|m| m.total)
            .unwrap_or(0)
    }

    /// Detect GPU availability using wgpu runtime detection.
    ///
    /// Result is cached via `OnceLock` since GPU availability does not change
    /// during process lifetime. This also prevents SIGSEGV from concurrent
    /// wgpu instance creation in parallel test harnesses.
    fn detect_gpu() -> bool {
        use std::sync::OnceLock;
        static GPU_AVAILABLE: OnceLock<bool> = OnceLock::new();

        *GPU_AVAILABLE.get_or_init(|| {
            #[cfg(feature = "wgpu")]
            {
                std::thread::scope(|s| {
                    s.spawn(|| {
                        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                            backends: wgpu::Backends::all(),
                            ..Default::default()
                        });

                        // wgpu 22+ exposes synchronous `enumerate_adapters`; no async runtime bridge.
                        let has_gpu = instance
                            .enumerate_adapters(wgpu::Backends::all())
                            .into_iter()
                            .any(|adapter| {
                                let info = adapter.get_info();
                                matches!(
                                    info.device_type,
                                    wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
                                )
                            });

                        tracing::debug!(gpu_detected = has_gpu, "GPU hardware probe complete");
                        has_gpu
                    })
                    .join()
                    .unwrap_or(false)
                })
            }
            #[cfg(not(feature = "wgpu"))]
            {
                false
            }
        })
    }

    /// Set our network identity
    pub fn with_network(
        mut self,
        hostname: String,
        port: Option<u16>,
        protocols: Vec<String>,
    ) -> Self {
        self.network = Some(NetworkIdentity {
            endpoint: port.map_or_else(|| hostname.clone(), |p| format!("{hostname}:{p}")),
            hostname,
            port,
            protocols,
        });
        self
    }

    /// Advertise ourselves for discovery
    ///
    /// This creates the message we broadcast for others to discover us.
    pub fn to_advertisement(&self) -> ServiceAdvertisement {
        ServiceAdvertisement {
            instance_id: self.instance_id,
            primal_type: self.primal_type.to_string(),
            version: self.version.clone(),
            capabilities: self.capabilities.clone(),
            endpoint: self.network.as_ref().map(|n| n.endpoint.clone()),
            protocols: self
                .network
                .as_ref()
                .map(|n| n.protocols.clone())
                .unwrap_or_default(),
        }
    }

    /// Check if a discovered service matches our requirements
    pub fn matches_requirement(
        &self,
        requirement: &CapabilityRequirement,
        service: &DiscoveredService,
    ) -> bool {
        // Check if service has the capability we need
        service.capabilities.iter().any(|cap| {
            cap.name == requirement.capability
                && requirement
                    .features
                    .iter()
                    .all(|feat| cap.features.contains(feat))
        })
    }
}

impl Default for SelfIdentity {
    fn default() -> Self {
        Self::new()
    }
}

/// Advertisement message for service discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAdvertisement {
    /// Unique instance identifier.
    pub instance_id: Uuid,
    /// Primal type (e.g. coordination, storage).
    pub primal_type: String,
    /// Service version.
    pub version: String,
    /// Advertised capabilities.
    pub capabilities: Vec<Capability>,
    /// Endpoint URL if known.
    pub endpoint: Option<String>,
    /// Supported protocols.
    pub protocols: Vec<String>,
}

/// A service discovered at runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    /// Unique instance identifier.
    pub instance_id: Uuid,
    /// Primal type (e.g. coordination, storage).
    pub primal_type: String,
    /// Service version.
    pub version: String,
    /// Discovered capabilities.
    pub capabilities: Vec<Capability>,
    /// Resolved endpoint URL.
    pub endpoint: String,
    /// Supported protocols.
    pub protocols: Vec<String>,
    /// When first discovered.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub discovered_at: SystemTime,
    /// Last seen timestamp.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub last_seen: SystemTime,
}

impl From<ServiceAdvertisement> for DiscoveredService {
    fn from(ad: ServiceAdvertisement) -> Self {
        let now = SystemTime::now();
        Self {
            instance_id: ad.instance_id,
            primal_type: ad.primal_type,
            version: ad.version,
            capabilities: ad.capabilities,
            endpoint: ad.endpoint.unwrap_or_else(|| "unknown".to_string()),
            protocols: ad.protocols,
            discovered_at: now,
            last_seen: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_identity_creation() {
        let identity = SelfIdentity::new();

        assert_eq!(identity.primal_type, "toadstool");
        assert!(!identity.capabilities.is_empty());
        assert!(!identity.requirements.is_empty());
        assert_eq!(identity.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_capabilities_are_self_knowledge_only() {
        let identity = SelfIdentity::new();

        // We should have compute capability
        assert!(identity.capabilities.iter().any(|c| c.name == "compute"));

        // We should have orchestration capability
        assert!(
            identity
                .capabilities
                .iter()
                .any(|c| c.name == "orchestration")
        );
    }

    #[test]
    fn test_requirements_are_optional() {
        let identity = SelfIdentity::new();

        // All requirements should be optional (we can work standalone)
        assert!(identity.requirements.iter().all(|r| !r.required));
    }

    #[test]
    fn test_network_identity_optional() {
        let identity = SelfIdentity::new();

        // Network identity is optional (we might be standalone)
        assert!(identity.network.is_none());
    }

    #[test]
    fn test_with_network() {
        let identity = SelfIdentity::new().with_network(
            "localhost".to_string(),
            Some(8084),
            vec!["http".to_string()],
        );

        assert!(identity.network.is_some());
        let network = identity
            .network
            .as_ref()
            .expect("Network info should be present in test");
        assert_eq!(network.hostname, "localhost");
        assert_eq!(network.port, Some(8084));
        assert_eq!(network.endpoint, "localhost:8084");
    }

    #[test]
    fn test_advertisement() {
        let identity = SelfIdentity::new().with_network(
            "localhost".to_string(),
            Some(8084),
            vec!["http".to_string()],
        );

        let ad = identity.to_advertisement();

        assert_eq!(ad.primal_type, "toadstool");
        assert!(ad.endpoint.is_some());
        assert!(!ad.capabilities.is_empty());
    }

    #[test]
    fn test_matches_requirement() {
        let identity = SelfIdentity::new();

        let requirement = CapabilityRequirement {
            capability: "storage".to_string(),
            min_version: Some("1.0".to_string()),
            required: false,
            features: vec!["object-store".to_string()],
            purpose: "Test".to_string(),
        };

        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: "storage".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability {
                name: "storage".to_string(),
                version: "1.0".to_string(),
                features: vec!["object-store".to_string(), "metadata".to_string()],
                characteristics: HashMap::new(),
            }],
            endpoint: "localhost:8082".to_string(),
            protocols: vec!["http".to_string()],
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
        };

        assert!(identity.matches_requirement(&requirement, &service));
    }
}
