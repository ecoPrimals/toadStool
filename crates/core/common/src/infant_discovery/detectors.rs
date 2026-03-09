// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

//! Substrate detectors - detect the runtime environment without hardcoding
//!
//! ToadStool only cares about **hardware capabilities** (CPU, GPU, NPU, memory).
//! Vendor-specific orchestration (K8s, Docker, Consul, AWS/GCP/Azure) is not
//! ToadStool's concern - service discovery is delegated to Songbird (comms primal).
//!
//! ## Philosophy
//!
//! - **Hardware-first**: We detect compute substrates, not vendor platforms
//! - **Vendor-agnostic**: No K8s, Docker, cloud provider detection
//! - **Songbird delegation**: mDNS/service discovery handled by comms primal
//! - **Self-knowledge**: ToadStool knows its own hardware, Songbird knows the network
//!
//! ## Evolution (Feb 15, 2026)
//!
//! Removed vendor-specific detectors:
//! - `KubernetesDetector` (vendor lock-in)
//! - `DockerDetector` (vendor lock-in)
//! - `ConsulDetector` (vendor lock-in)
//! - `CloudDetector` (AWS/GCP/Azure - vendor lock-in)
//!
//! Kept: `BareMetalDetector` (hardware capabilities)

use std::future::Future;
use std::pin::Pin;

use super::capabilities::{
    DetectedSubstrate, DiscoveryError, SubstrateCapability, SubstrateDetector, SubstrateType,
};

/// Environment snapshot for hardware detection.
///
/// Production uses `HardwareEnvironment::from_env()`.
/// Tests use explicit values - zero env var mutation.
///
/// ## Evolution (Feb 15, 2026)
///
/// Removed vendor-specific fields (AWS, GCP, Azure, K8s, Consul).
/// ToadStool only needs hostname for self-identification.
#[derive(Debug, Clone, Default)]
pub struct HardwareEnvironment {
    /// Machine hostname (for logging/identification)
    pub hostname: Option<String>,
}

impl HardwareEnvironment {
    /// Capture current environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            hostname: std::env::var("HOSTNAME").ok(),
        }
    }
}

/// Bare metal detector - detects hardware capabilities
///
/// This is the only substrate detector ToadStool needs.
/// It identifies the compute substrate (bare metal, VM, container)
/// based on hardware inspection, not vendor platforms.
pub struct BareMetalDetector;

impl BareMetalDetector {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Detect if running in a container (any container runtime)
    fn is_containerized() -> bool {
        // Check for cgroup-based containerization (generic, not Docker-specific)
        std::path::Path::new("/.dockerenv").exists()
            || std::fs::read_to_string("/proc/self/cgroup").is_ok_and(|content| {
                content.contains("docker")
                    || content.contains("containerd")
                    || content.contains("lxc")
            })
    }
}

impl Default for BareMetalDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateDetector for BareMetalDetector {
    fn detect(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DetectedSubstrate>, DiscoveryError>> + Send + '_>>
    {
        Box::pin(async move {
            let mut metadata = std::collections::HashMap::new();

            // Detect deployment type based on hardware inspection
            let deployment_type = if Self::is_containerized() {
                "container"
            } else {
                "bare_metal"
            };
            metadata.insert("deployment".to_string(), deployment_type.to_string());

            // Add hostname for identification
            if let Ok(hostname) = std::env::var("HOSTNAME") {
                metadata.insert("hostname".to_string(), hostname);
            }

            // Add OS information
            if let Ok(os) = std::env::var("OS") {
                metadata.insert("os".to_string(), os);
            }

            // Detect CPU count (hardware capability)
            if let Ok(parallelism) = std::thread::available_parallelism() {
                metadata.insert("cpu_threads".to_string(), parallelism.get().to_string());
            }

            Ok(Some(DetectedSubstrate {
                substrate_type: SubstrateType::Bare,
                capabilities: vec![SubstrateCapability::BareMetal],
                metadata,
            }))
        })
    }

    fn name(&self) -> &'static str {
        "bare_metal"
    }
}

/// Create standard detector chain
///
/// ## Evolution (Feb 15, 2026)
///
/// Only `BareMetalDetector` remains. Vendor-specific detectors removed:
/// - No `KubernetesDetector` (Songbird handles service discovery)
/// - No `DockerDetector` (container detection is generic now)
/// - No `ConsulDetector` (Songbird handles service mesh)
/// - No `CloudDetector` (vendor lock-in, not our concern)
#[must_use]
pub fn standard_detectors() -> Vec<Box<dyn SubstrateDetector>> {
    vec![Box::new(BareMetalDetector::new())]
}

// ============================================================================
// Legacy type aliases for backward compatibility
// ============================================================================

/// `CloudEnvironment` is deprecated - use `HardwareEnvironment`
#[deprecated(
    since = "0.16.0",
    note = "Use HardwareEnvironment instead - vendor detection removed"
)]
pub type CloudEnvironment = HardwareEnvironment;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_bare_metal_detector_always_succeeds() {
        let detector = BareMetalDetector::new();
        let result = detector.detect().await.unwrap();
        assert!(result.is_some());
        let substrate = result.unwrap();
        assert_eq!(substrate.substrate_type, SubstrateType::Bare);
    }

    #[test]
    fn test_detector_chain() {
        let detectors = standard_detectors();
        assert_eq!(detectors.len(), 1);
        assert_eq!(detectors[0].name(), "bare_metal");
    }

    #[test]
    fn test_bare_metal_detector_new() {
        let detector = BareMetalDetector::new();
        assert_eq!(detector.name(), "bare_metal");
    }

    #[test]
    fn test_bare_metal_detector_default() {
        let detector = BareMetalDetector;
        assert_eq!(detector.name(), "bare_metal");
    }

    #[tokio::test]
    async fn test_bare_metal_detector_capabilities() {
        let detector = BareMetalDetector::new();
        let result = detector.detect().await.unwrap();
        assert!(result.is_some());
        let substrate = result.unwrap();
        assert_eq!(substrate.substrate_type, SubstrateType::Bare);
        assert!(!substrate.capabilities.is_empty());
    }

    #[tokio::test]
    async fn test_bare_metal_detector_metadata() {
        let detector = BareMetalDetector::new();
        let result = detector.detect().await.unwrap();
        assert!(result.is_some());
        let substrate = result.unwrap();
        assert!(!substrate.metadata.is_empty());
        assert!(substrate.metadata.contains_key("deployment"));
    }

    #[tokio::test]
    async fn test_bare_metal_detector_cpu_threads() {
        let detector = BareMetalDetector::new();
        let result = detector.detect().await.unwrap();
        assert!(result.is_some());
        let substrate = result.unwrap();
        // Should have cpu_threads metadata
        assert!(substrate.metadata.contains_key("cpu_threads"));
        let threads: usize = substrate
            .metadata
            .get("cpu_threads")
            .unwrap()
            .parse()
            .unwrap();
        assert!(threads >= 1);
    }

    #[test]
    fn test_standard_detectors_count() {
        let detectors = standard_detectors();
        assert_eq!(detectors.len(), 1);
    }

    #[test]
    fn test_hardware_environment_from_env() {
        let env = HardwareEnvironment::from_env();
        // May or may not have hostname depending on test environment
        let _ = env.hostname;
    }

    #[test]
    fn test_hardware_environment_default() {
        let env = HardwareEnvironment::default();
        assert!(env.hostname.is_none());
    }

    #[tokio::test]
    async fn test_all_detectors_dont_panic() {
        let detectors = standard_detectors();
        for detector in detectors {
            let result = detector.detect().await;
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_is_containerized_doesnt_panic() {
        // Just verify it doesn't panic - result depends on environment
        let _ = BareMetalDetector::is_containerized();
    }
}
