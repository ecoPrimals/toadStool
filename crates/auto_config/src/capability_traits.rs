// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Capability Detection Traits
//!
//! Trait-based interfaces for hardware and ecosystem detection.
//! Enables dependency injection and testing with mock implementations.

use async_trait::async_trait;

use crate::ecosystem::DiscoveredServices;
use crate::hardware::SystemCapabilities;
use crate::ToadStoolResult;

/// Trait for hardware capability detection
///
/// This trait enables dependency injection and testing by allowing
/// both real and mock implementations of hardware detection.
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait HardwareCapabilityDetector: Send + Sync {
    /// Scan system hardware capabilities
    async fn scan_system(&mut self) -> ToadStoolResult<SystemCapabilities>;
}

/// Trait for ecosystem service discovery
///
/// This trait enables dependency injection and testing by allowing
/// both real and mock implementations of service discovery.
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait EcosystemServiceDiscoverer: Send + Sync {
    /// Discover available ecosystem services
    async fn discover_services(&mut self) -> ToadStoolResult<DiscoveredServices>;
}

// ============================================================================
// Adapters for Real Implementations
// ============================================================================

/// Adapter to make `HardwareDetector` implement the trait
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl HardwareCapabilityDetector for crate::hardware::HardwareDetector {
    async fn scan_system(&mut self) -> ToadStoolResult<SystemCapabilities> {
        crate::hardware::HardwareDetector::scan_system(self).await
    }
}

/// Adapter to make `EcosystemDiscoverer` implement the trait
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl EcosystemServiceDiscoverer for crate::ecosystem::EcosystemDiscoverer {
    async fn discover_services(&mut self) -> ToadStoolResult<DiscoveredServices> {
        crate::ecosystem::EcosystemDiscoverer::discover_services(self).await
    }
}

// ============================================================================
// Mock Implementations for Testing
// ============================================================================
//
// Evolution path: These mocks are behind #[cfg(any(test, feature = "test-mocks"))].
// Production always uses real HardwareDetector and EcosystemDiscoverer (see
// IntelligentAutoConfig::new(), ConfigBuilder::build(), get_system_summary()).
// If test-mocks feature is enabled, callers can inject mocks for integration tests.

/// Mock hardware detector for testing
///
/// Returns pre-configured capabilities instantly without any I/O.
/// Perfect for fast unit and integration tests.
#[cfg(any(test, feature = "test-mocks"))]
pub struct MockHardwareDetector {
    /// Pre-configured capabilities to return
    pub capabilities: SystemCapabilities,
}

#[cfg(any(test, feature = "test-mocks"))]
impl MockHardwareDetector {
    /// Create a mock detector with default test capabilities
    #[must_use]
    pub fn new() -> Self {
        Self {
            capabilities: SystemCapabilities {
                cpu_info: Default::default(),
                cpu_cores: 8.0,
                memory_info: Default::default(),
                memory_gb: 16.0,
                gpu_info: Vec::new(),
                gpu_count: 0,
                gpu_memory_gb: None,
                storage_info: Default::default(),
                storage_gb: 512.0,
                network_info: Default::default(),
                performance_class: crate::hardware::PerformanceClass::Mainstream,
            },
        }
    }

    /// Create a mock detector with custom capabilities
    #[must_use]
    pub fn with_capabilities(capabilities: SystemCapabilities) -> Self {
        Self { capabilities }
    }
}

#[cfg(any(test, feature = "test-mocks"))]
impl Default for MockHardwareDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "test-mocks"))]
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl HardwareCapabilityDetector for MockHardwareDetector {
    async fn scan_system(&mut self) -> ToadStoolResult<SystemCapabilities> {
        // Instant return - no I/O
        Ok(self.capabilities.clone())
    }
}

/// Mock ecosystem discoverer for testing
///
/// Returns empty service list instantly without any network I/O.
/// Perfect for fast unit and integration tests.
#[cfg(any(test, feature = "test-mocks"))]
pub struct MockEcosystemDiscoverer {
    /// Pre-configured services to return
    pub services: DiscoveredServices,
}

#[cfg(any(test, feature = "test-mocks"))]
impl MockEcosystemDiscoverer {
    /// Create a mock discoverer with no services
    #[must_use]
    pub fn new() -> Self {
        Self {
            services: DiscoveredServices {
                discovered_services: std::collections::HashMap::new(),
                discovery_summary: crate::ecosystem::DiscoverySummary {
                    total_services_found: 0,
                    discovery_methods_used: vec!["mock".to_string()],
                    services_by_type: std::collections::HashMap::new(),
                    discovery_errors: Vec::new(),
                },
                discovery_timestamp: std::time::SystemTime::now(),
            },
        }
    }

    /// Create a mock discoverer with custom services
    #[must_use]
    pub fn with_services(services: DiscoveredServices) -> Self {
        Self { services }
    }
}

#[cfg(any(test, feature = "test-mocks"))]
impl Default for MockEcosystemDiscoverer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "test-mocks"))]
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl EcosystemServiceDiscoverer for MockEcosystemDiscoverer {
    async fn discover_services(&mut self) -> ToadStoolResult<DiscoveredServices> {
        // Instant return - no network I/O
        Ok(self.services.clone())
    }
}

/// Implement the traits for real detectors
impl crate::hardware::HardwareDetector {
    /// Convert to trait object
    pub fn into_trait(self) -> Box<dyn HardwareCapabilityDetector> {
        Box::new(RealHardwareDetector { inner: self })
    }
}

impl crate::ecosystem::EcosystemDiscoverer {
    /// Convert to trait object
    pub fn into_trait(self) -> Box<dyn EcosystemServiceDiscoverer> {
        Box::new(RealEcosystemDiscoverer { inner: self })
    }
}

/// Wrapper for real hardware detector
struct RealHardwareDetector {
    inner: crate::hardware::HardwareDetector,
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl HardwareCapabilityDetector for RealHardwareDetector {
    async fn scan_system(&mut self) -> ToadStoolResult<SystemCapabilities> {
        self.inner.scan_system().await
    }
}

/// Wrapper for real ecosystem discoverer
struct RealEcosystemDiscoverer {
    inner: crate::ecosystem::EcosystemDiscoverer,
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl EcosystemServiceDiscoverer for RealEcosystemDiscoverer {
    async fn discover_services(&mut self) -> ToadStoolResult<DiscoveredServices> {
        self.inner.discover_services().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_hardware_detector() {
        let mut detector = MockHardwareDetector::new();
        let result = detector.scan_system().await;

        assert!(result.is_ok());
        let capabilities = result.unwrap();
        assert_eq!(capabilities.cpu_cores, 8.0);
        assert_eq!(capabilities.memory_gb, 16.0);
    }

    #[tokio::test]
    async fn test_mock_ecosystem_discoverer() {
        let mut discoverer = MockEcosystemDiscoverer::new();
        let result = discoverer.discover_services().await;

        assert!(result.is_ok());
        let services = result.unwrap();
        assert_eq!(services.discovered_services.len(), 0);
    }

    #[tokio::test]
    async fn test_mock_is_fast() {
        use std::time::Instant;

        let mut detector = MockHardwareDetector::new();
        let start = Instant::now();
        let _ = detector.scan_system().await;
        let duration = start.elapsed();

        // Should complete quickly (no I/O) - allowing 10ms for system variance
        assert!(
            duration.as_millis() < 10,
            "Mock should be fast but took {}ms",
            duration.as_millis()
        );
    }

    #[tokio::test]
    async fn test_mock_hardware_detector_with_custom_capabilities() {
        use crate::hardware::{PerformanceClass, SystemCapabilities};

        let caps = SystemCapabilities {
            cpu_cores: 16.0,
            memory_gb: 32.0,
            performance_class: PerformanceClass::HighEnd,
            ..Default::default()
        };

        let mut detector = MockHardwareDetector::with_capabilities(caps);
        let result = detector.scan_system().await;

        assert!(result.is_ok());
        let capabilities = result.unwrap();
        assert_eq!(capabilities.cpu_cores, 16.0);
        assert_eq!(capabilities.memory_gb, 32.0);
        assert!(matches!(
            capabilities.performance_class,
            PerformanceClass::HighEnd
        ));
    }

    #[tokio::test]
    async fn test_mock_ecosystem_discoverer_with_services() {
        use crate::ecosystem::{DiscoveredServices, DiscoverySummary};

        let mut services = std::collections::HashMap::new();
        services.insert(
            "test-svc".to_string(),
            crate::ecosystem::ServiceInfo {
                name: "test-svc".to_string(),
                endpoint: "http://localhost:8080".to_string(),
                service_type: "Test".to_string(),
                version: "1.0".to_string(),
                capabilities: vec!["test".to_string()],
                status: crate::ecosystem::ServiceStatus::Healthy,
                discovered_via: "unit_test".to_string(),
                response_time_ms: 0,
            },
        );

        let discovered = DiscoveredServices {
            discovered_services: services,
            discovery_summary: DiscoverySummary::default(),
            discovery_timestamp: std::time::SystemTime::now(),
        };

        let mut discoverer = MockEcosystemDiscoverer::with_services(discovered);
        let result = discoverer.discover_services().await;

        assert!(result.is_ok());
        let services = result.unwrap();
        assert_eq!(services.discovered_services.len(), 1);
        assert!(services.discovered_services.contains_key("test-svc"));
    }
}
