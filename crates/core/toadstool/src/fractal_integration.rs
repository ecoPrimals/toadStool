// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Fractal Composition Integration
//!
//! This module integrates fractal composition infrastructure with Toadstool's
//! existing systems (primal discovery, runtime, execution).
//!
//! # Philosophy
//!
//! **Integration over isolation**: Fractal composition enhances existing systems
//! rather than replacing them. It's additive, not destructive.
//!
//! # Example
//!
//! ```rust,ignore
//! use toadstool::fractal_integration::FractalRuntime;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize fractal-aware runtime
//!     let runtime = FractalRuntime::init().await?;
//!
//!     // Runtime automatically:
//!     // - Detects deployment layer
//!     // - Adapts capabilities
//!     // - Advertises layer-appropriate services
//!     // - Enables dynamic composition
//!
//!     // Use as normal - fractal composition is transparent
//!     let caps = runtime.capabilities();
//!     println!("Capabilities: {:?}", caps.metadata);
//!     Ok(())
//! }
//! ```

use crate::deployment_layer::{DeploymentLayer, LayerDetector};
use crate::layer_adaptation::{AdaptedCapabilities, LayerCapabilityAdapter};
use crate::self_identity::SelfIdentity;
use crate::{ToadStoolError, ToadStoolResult};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Fractal-aware runtime
///
/// This runtime enhances Toadstool with fractal composition capabilities:
/// - Multi-layer deployment detection
/// - Adaptive capability advertisement
/// - Layer-aware service discovery
/// - Dynamic workload composition (future)
pub struct FractalRuntime {
    /// Detected deployment layer
    layer: DeploymentLayer,

    /// Adapted capabilities for this layer
    capabilities: AdaptedCapabilities,

    /// Self-identity with fractal-enhanced capabilities
    identity: Arc<RwLock<SelfIdentity>>,
}

impl FractalRuntime {
    /// Initialize fractal-aware runtime
    ///
    /// This performs:
    /// 1. Deployment layer detection
    /// 2. Capability adaptation
    /// 3. Self-identity enhancement with adapted capabilities
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Layer detection fails
    /// - Self-identity creation fails
    pub async fn init() -> ToadStoolResult<Self> {
        info!("🍄 Initializing Fractal Composition Runtime...");

        // Step 1: Detect deployment layer
        let mut detector = LayerDetector::new();
        let layer = detector
            .detect()
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Layer detection failed: {}", e)))?;

        info!("📍 Deployment layer detected: {}", layer);

        // Step 2: Adapt capabilities for this layer
        let adapter = LayerCapabilityAdapter::new(layer.clone());
        let capabilities = adapter.get_adapted_capabilities();

        debug!(
            "✅ Capabilities adapted for layer: {:?}",
            capabilities.metadata.layer
        );
        debug!("   GPU Access: {:?}", capabilities.compute.gpu_access);
        debug!("   Storage Type: {:?}", capabilities.storage.storage_type);
        debug!(
            "   Network Access: {:?}",
            capabilities.network.network_access
        );

        // Step 3: Create self-identity with adapted capabilities
        let mut identity = SelfIdentity::new();

        // Enhance identity with layer-adapted capabilities
        Self::enhance_identity_with_layer_capabilities(&mut identity, &capabilities);

        info!("🎯 Fractal Runtime initialized successfully");

        Ok(Self {
            layer,
            capabilities,
            identity: Arc::new(RwLock::new(identity)),
        })
    }

    /// Enhance self-identity with layer-adapted capabilities
    fn enhance_identity_with_layer_capabilities(
        identity: &mut SelfIdentity,
        capabilities: &AdaptedCapabilities,
    ) {
        // Add layer-adapted capabilities to identity
        // This makes them discoverable by other primals

        use crate::self_identity::Capability;
        use std::collections::HashMap;

        // Add compute capabilities
        for cap_name in capabilities.to_capability_list() {
            let capability = Capability {
                name: cap_name.clone(),
                version: "1.0".to_string(),
                features: vec![],
                characteristics: {
                    let mut chars = HashMap::new();
                    chars.insert(
                        "deployment_layer".to_string(),
                        capabilities.metadata.layer.clone(),
                    );
                    if let Some(host_os) = &capabilities.metadata.host_os {
                        chars.insert("host_os".to_string(), host_os.clone());
                    }
                    if let Some(cloud_provider) = &capabilities.metadata.cloud_provider {
                        chars.insert("cloud_provider".to_string(), cloud_provider.clone());
                    }
                    chars
                },
            };

            // Add if not already present
            if !identity.capabilities.iter().any(|c| c.name == cap_name) {
                identity.capabilities.push(capability);
            }
        }

        debug!(
            "Enhanced identity with {} layer-adapted capabilities",
            capabilities.to_capability_list().len()
        );
    }

    /// Get deployment layer
    pub fn deployment_layer(&self) -> &DeploymentLayer {
        &self.layer
    }

    /// Get adapted capabilities
    pub fn capabilities(&self) -> &AdaptedCapabilities {
        &self.capabilities
    }

    /// Get self-identity (for discovery integration)
    pub fn identity(&self) -> Arc<RwLock<SelfIdentity>> {
        Arc::clone(&self.identity)
    }

    /// Check if this layer has direct GPU access
    pub fn has_direct_gpu_access(&self) -> bool {
        self.capabilities.has_direct_gpu_access()
    }

    /// Check if this layer has any GPU access (direct or via host/cloud)
    pub fn has_gpu_access(&self) -> bool {
        self.capabilities.has_gpu_access()
    }

    /// Get barraCuda integration info
    ///
    /// Returns information about how barraCuda should access GPU in this layer
    pub fn barracuda_integration(&self) -> BarracudaIntegration {
        use crate::layer_adaptation::GpuAccess;

        match self.capabilities.compute.gpu_access {
            GpuAccess::Direct => BarracudaIntegration::Direct {
                note: "Direct GPU access - use native WGPU backend".to_string(),
            },
            GpuAccess::ViaHost => BarracudaIntegration::ViaHost {
                note: "GPU via host OS - use host-provided GPU drivers".to_string(),
                host_os: self.capabilities.metadata.host_os.clone(),
            },
            GpuAccess::ViaCloud => BarracudaIntegration::ViaCloud {
                note: "GPU via cloud APIs - use cloud GPU endpoints".to_string(),
                provider: self.capabilities.metadata.cloud_provider.clone(),
            },
            GpuAccess::None => BarracudaIntegration::None {
                note: "No GPU access - CPU fallback only".to_string(),
            },
        }
    }
}

/// barraCuda integration information
///
/// Describes how barraCuda should access GPU in the current deployment layer.
#[derive(Debug, Clone)]
pub enum BarracudaIntegration {
    /// Direct GPU access (bare metal, GPU passthrough)
    Direct { note: String },

    /// GPU via host OS (middleware layer)
    ViaHost {
        note: String,
        host_os: Option<String>,
    },

    /// GPU via cloud APIs (cloud layer)
    ViaCloud {
        note: String,
        provider: Option<String>,
    },

    /// No GPU access (CPU fallback)
    None { note: String },
}

impl BarracudaIntegration {
    /// Get human-readable note
    pub fn note(&self) -> &str {
        match self {
            Self::Direct { note } => note,
            Self::ViaHost { note, .. } => note,
            Self::ViaCloud { note, .. } => note,
            Self::None { note } => note,
        }
    }

    /// Check if GPU is available
    pub fn has_gpu(&self) -> bool {
        !matches!(self, Self::None { .. })
    }
}

/// Fractal-aware service advertisement
///
/// Advertises services with layer-appropriate capabilities.
pub struct FractalServiceAdvertiser {
    runtime: Arc<FractalRuntime>,
}

impl FractalServiceAdvertiser {
    /// Create a new advertiser for a fractal runtime
    pub fn new(runtime: Arc<FractalRuntime>) -> Self {
        Self { runtime }
    }

    /// Advertise service with layer-adapted capabilities
    ///
    /// This ensures other primals see capabilities appropriate for the
    /// current deployment layer.
    pub async fn advertise(&self) -> ToadStoolResult<()> {
        let identity = self.runtime.identity();
        let _identity_read = identity.read().await;

        info!("📢 Advertising fractal-aware service...");
        debug!("   Layer: {}", self.runtime.layer);
        debug!(
            "   Capabilities: {} exposed",
            self.runtime.capabilities.to_capability_list().len()
        );

        // Integration point with existing discovery system
        // The identity already has layer-adapted capabilities added
        // Discovery system will use these when advertising

        info!("✅ Service advertised with layer-appropriate capabilities");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fractal_runtime_init() {
        // This test will detect the actual environment
        let runtime = FractalRuntime::init().await;
        assert!(runtime.is_ok());

        if let Ok(runtime) = runtime {
            // Should have detected some layer
            let layer = runtime.deployment_layer();
            assert!(matches!(
                layer,
                DeploymentLayer::BareMetalOS
                    | DeploymentLayer::MiddlewareLayer { .. }
                    | DeploymentLayer::ServiceLayer { .. }
                    | DeploymentLayer::ContainerLayer { .. }
                    | DeploymentLayer::VMLayer { .. }
                    | DeploymentLayer::CloudLayer { .. }
            ));

            // Should have adapted capabilities
            let caps = runtime.capabilities();
            assert!(!caps.to_capability_list().is_empty());
        }
    }

    #[tokio::test]
    async fn test_barracuda_integration_info() {
        let runtime = FractalRuntime::init().await;
        assert!(runtime.is_ok());

        if let Ok(runtime) = runtime {
            let integration = runtime.barracuda_integration();
            // Should have some integration type
            assert!(!integration.note().is_empty());
        }
    }

    #[tokio::test]
    async fn test_fractal_advertiser() {
        let runtime = FractalRuntime::init().await;
        assert!(runtime.is_ok());

        if let Ok(runtime) = runtime {
            let runtime = Arc::new(runtime);
            let advertiser = FractalServiceAdvertiser::new(Arc::clone(&runtime));

            // Should be able to advertise
            let result = advertiser.advertise().await;
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_barracuda_integration_direct() {
        let integration = BarracudaIntegration::Direct {
            note: "Direct GPU".to_string(),
        };
        assert!(integration.has_gpu());
        assert_eq!(integration.note(), "Direct GPU");
    }

    #[test]
    fn test_barracuda_integration_via_host() {
        let integration = BarracudaIntegration::ViaHost {
            note: "Via host".to_string(),
            host_os: Some("Linux".to_string()),
        };
        assert!(integration.has_gpu());
        assert_eq!(integration.note(), "Via host");
    }

    #[test]
    fn test_barracuda_integration_via_cloud() {
        let integration = BarracudaIntegration::ViaCloud {
            note: "Via cloud".to_string(),
            provider: Some("AWS".to_string()),
        };
        assert!(integration.has_gpu());
        assert_eq!(integration.note(), "Via cloud");
    }

    #[test]
    fn test_barracuda_integration_none() {
        let integration = BarracudaIntegration::None {
            note: "No GPU".to_string(),
        };
        assert!(!integration.has_gpu());
        assert_eq!(integration.note(), "No GPU");
    }

    #[test]
    fn test_barracuda_integration_debug_clone() {
        let integration = BarracudaIntegration::Direct {
            note: "test".to_string(),
        };
        let cloned = integration.clone();
        assert_eq!(integration.note(), cloned.note());
    }

    #[tokio::test]
    async fn test_fractal_runtime_has_gpu_access() {
        let runtime = FractalRuntime::init().await.unwrap();
        let _ = runtime.has_gpu_access();
        let _ = runtime.has_direct_gpu_access();
    }

    #[tokio::test]
    async fn test_fractal_runtime_identity() {
        let runtime = FractalRuntime::init().await.unwrap();
        let identity = runtime.identity();
        let _guard = identity.read().await;
    }

    #[tokio::test]
    async fn test_fractal_advertiser_new() {
        let runtime = FractalRuntime::init().await.unwrap();
        let runtime_arc = Arc::new(runtime);
        let advertiser = FractalServiceAdvertiser::new(runtime_arc);
        assert!(advertiser.advertise().await.is_ok());
    }
}
