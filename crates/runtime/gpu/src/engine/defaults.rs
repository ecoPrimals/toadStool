// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synchronous default for types that only support async initialization.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tokio::sync::RwLock as AsyncRwLock;

use crate::compiler::KernelStringOptimizer;
use crate::config::{CompilationConfig, ResourceConfig, UniversalGpuConfig};
use crate::coordinator::ComputeResourceCoordinator;
use crate::strategy::{BackendSelectionStrategy, EvolutionMetrics};

use super::UniversalGpuEngine;

impl Default for UniversalGpuEngine {
    fn default() -> Self {
        // Note: Default construction creates an uninitialized engine
        // Use UniversalGpuEngine::new() for proper async initialization
        Self {
            frameworks: Arc::new(AsyncRwLock::new(HashMap::new())),
            devices: Arc::new(AsyncRwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            _kernel_compiler: Arc::new(KernelStringOptimizer::new(CompilationConfig::default())),
            resource_coordinator: Arc::new(ComputeResourceCoordinator::new(
                ResourceConfig::default(),
            )),
            config: UniversalGpuConfig::default(),
            resource_monitor: None,
            selection_strategy: BackendSelectionStrategy::default(),
            evolution_metrics: Arc::new(RwLock::new(EvolutionMetrics::default())),
        }
    }
}
