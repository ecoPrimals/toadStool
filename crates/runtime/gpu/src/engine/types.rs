// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core GPU engine type and shared state.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use toadstool::resources::ResourceMonitor;

use crate::compiler::UniversalKernelCompiler;
use crate::config::UniversalGpuConfig;
use crate::coordinator::ComputeResourceCoordinator;
use crate::strategy::{BackendSelectionStrategy, EvolutionMetrics};
use crate::traits::ParallelComputeFramework;
use crate::types::{ComputeSession, DeviceId, GpuFramework, UniversalComputeDevice};

/// Universal GPU Compute Engine - the heart of parallel compute orchestration
pub struct UniversalGpuEngine {
    /// Discovered compute frameworks and their capabilities
    pub(super) frameworks: Arc<RwLock<HashMap<GpuFramework, Arc<dyn ParallelComputeFramework>>>>,
    /// Available compute devices across all frameworks
    pub(super) devices: Arc<RwLock<HashMap<DeviceId, UniversalComputeDevice>>>,
    /// Active compute sessions (supports recursive execution)
    pub(super) active_sessions: Arc<RwLock<HashMap<Uuid, ComputeSession>>>,
    /// Universal kernel compiler and optimizer
    pub(super) _kernel_compiler: Arc<UniversalKernelCompiler>,
    /// Device resource coordinator
    pub(super) resource_coordinator: Arc<ComputeResourceCoordinator>,
    /// Configuration
    pub(super) config: UniversalGpuConfig,
    /// Resource monitor
    pub(super) resource_monitor: Option<Arc<dyn ResourceMonitor>>,
    /// Backend selection strategy (sovereign vs pragmatic)
    pub(super) selection_strategy: BackendSelectionStrategy,
    /// Evolution metrics (ecosystem maturity tracking)
    pub(super) evolution_metrics: Arc<RwLock<EvolutionMetrics>>,
}
