// SPDX-License-Identifier: AGPL-3.0-or-later
//! Engine construction, framework discovery, and device enumeration.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, warn};

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::compiler::UniversalKernelCompiler;
use crate::config::UniversalGpuConfig;
use crate::coordinator::ComputeResourceCoordinator;
use crate::parallel_framework_dispatch::ParallelComputeFrameworkDispatch;
use crate::strategy::{BackendSelectionStrategy, EvolutionMetrics};
use crate::traits::ParallelComputeFramework;
use crate::types::GpuFramework;

use super::UniversalGpuEngine;

impl UniversalGpuEngine {
    /// Create new GPU engine with default configuration
    ///
    /// # Errors
    ///
    /// Returns when engine initialization or framework discovery fails.
    pub async fn new() -> ToadStoolResult<Self> {
        Self::with_config(UniversalGpuConfig::default()).await
    }

    /// Create new GPU engine with custom configuration
    ///
    /// # Errors
    ///
    /// Returns when engine initialization or framework discovery fails.
    pub async fn with_config(config: UniversalGpuConfig) -> ToadStoolResult<Self> {
        Self::with_config_and_strategy(config, BackendSelectionStrategy::default()).await
    }

    /// Create new GPU engine with custom configuration and selection strategy
    ///
    /// # Errors
    ///
    /// Returns when engine initialization or framework discovery fails.
    pub async fn with_config_and_strategy(
        config: UniversalGpuConfig,
        selection_strategy: BackendSelectionStrategy,
    ) -> ToadStoolResult<Self> {
        let frameworks = Arc::new(RwLock::new(HashMap::new()));
        let devices = Arc::new(RwLock::new(HashMap::new()));
        let active_sessions = Arc::new(RwLock::new(HashMap::new()));
        let kernel_compiler = Arc::new(UniversalKernelCompiler::new(config.compilation.clone()));
        let resource_coordinator =
            Arc::new(ComputeResourceCoordinator::new(config.resources.clone()));
        let evolution_metrics = Arc::new(RwLock::new(EvolutionMetrics::default()));

        let engine = Self {
            frameworks,
            devices,
            active_sessions,
            _kernel_compiler: kernel_compiler,
            resource_coordinator,
            config,
            resource_monitor: None,
            selection_strategy,
            evolution_metrics,
        };

        // Log evolution status on startup
        engine.log_evolution_status().await;

        // Initialize frameworks and discover devices
        engine.discover_frameworks().await?;
        engine.discover_devices().await?;

        Ok(engine)
    }

    /// Discover and initialize available compute frameworks
    async fn discover_frameworks(&self) -> ToadStoolResult<()> {
        let mut frameworks = self.frameworks.write().await;

        for framework_type in &self.config.discovery.enabled_frameworks {
            match self.create_framework_instance(framework_type.clone()).await {
                Ok(framework) => {
                    frameworks.insert(framework_type.clone(), framework);
                    info!("Initialized framework: {}", framework_type.name());
                }
                Err(e) => {
                    if self.config.discovery.auto_fallback {
                        warn!(
                            "Failed to initialize framework {}: {}",
                            framework_type.name(),
                            e
                        );
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        if frameworks.is_empty() {
            return Err(ToadStoolError::runtime(
                "No compute frameworks could be initialized",
            ));
        }

        drop(frameworks);
        Ok(())
    }

    /// Create instance of specific framework
    async fn create_framework_instance(
        &self,
        framework_type: GpuFramework,
    ) -> ToadStoolResult<Arc<ParallelComputeFrameworkDispatch>> {
        match framework_type {
            GpuFramework::WebGpu => {
                let framework = crate::frameworks::WebGpuFramework::new()?;
                Ok(Arc::new(ParallelComputeFrameworkDispatch::WebGpu(
                    framework,
                )))
            }
            GpuFramework::Vulkan => {
                // Vulkan support requires additional platform-specific dependencies
                // Users should use WebGPU for cross-platform GPU compute
                Err(ToadStoolError::configuration(
                    "Vulkan framework requires manual enablement via 'vulkan' feature flag. \
                     Consider using WebGPU for cross-platform compatibility.",
                ))
            }
            GpuFramework::OpenCl => Err(ToadStoolError::configuration(
                "OpenCL framework removed from this crate (S198). Use gpu.dispatch.opencl \
                 capability provider via IPC, or WebGPU/Vulkan for in-tree GPU compute.",
            )),
            _ => {
                // For other frameworks, use fallback implementation
                let framework = crate::frameworks::FallbackFramework::new(framework_type);
                Ok(Arc::new(ParallelComputeFrameworkDispatch::Fallback(
                    framework,
                )))
            }
        }
    }

    /// Discover available compute devices
    async fn discover_devices(&self) -> ToadStoolResult<()> {
        let frameworks = self.frameworks.read().await;
        let framework_values: Vec<_> = frameworks.values().cloned().collect();
        drop(frameworks);

        let mut devices = self.devices.write().await;

        for framework in framework_values {
            match framework.discover_devices().await {
                Ok(framework_devices) => {
                    for device in framework_devices {
                        devices.insert(device.id.clone(), device);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to discover devices for framework {}: {}",
                        framework.framework_type().name(),
                        e
                    );
                }
            }
        }

        info!("Discovered {} compute devices", devices.len());
        drop(devices);
        Ok(())
    }
}
