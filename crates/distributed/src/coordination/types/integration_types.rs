// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core integration struct types

use std::collections::HashMap;
use std::sync::Arc;

use std::sync::RwLock;

use super::CapabilityTracker;
use super::NetworkHealthMonitor;
use super::broadcast_types::{BroadcastChannel, MessageTypeRegistry, SubscriptionManager};
use super::capacity_types::LocalCapacityManager;
use super::discovery::{DiscoveryClient, NodeRegistry};
use super::distribution_types::{JobCoordinator, JobSplittingStrategy, LoadEstimator};
use super::load_balancing_types::{
    CoordinationFeedbackSender, NodeCapacityTracker, PerformanceMetrics,
};
use crate::UniversalJobType;
use crate::common::distribution::DistributionAlgorithm;

// ============================================================================
// Core Integration Types
// ============================================================================

/// Main handle tying local scheduling/capacity to a Coordination connection.
pub struct ToadStoolCoordinationIntegration {
    pub(crate) instance_id: String,
    pub(crate) connection: super::connection_types::CoordinationConnection,
    pub(crate) local_capacity: Arc<LocalCapacityManager>,
    pub(crate) workload_scheduler: Arc<crate::universal::UniversalScheduler>,
}

/// Splits and coordinates large jobs across the mesh using configured strategies.
pub struct MassiveJobDistributor {
    pub(crate) splitting_strategies: HashMap<UniversalJobType, JobSplittingStrategy>,
    pub(crate) distribution_algorithms: Vec<DistributionAlgorithm>,
    pub(crate) load_estimator: LoadEstimator,
    pub(crate) job_coordinator: JobCoordinator,
}

/// Discovers peers and tracks capabilities and health for the mesh.
pub struct CoordinationNetworkDiscovery {
    pub(crate) discovery_client: DiscoveryClient,
    pub(crate) node_registry: RwLock<NodeRegistry>,
    pub(crate) capability_tracker: CapabilityTracker,
    pub(crate) health_monitor: NetworkHealthMonitor,
}

/// Selects nodes using strategies, metrics, and feedback to Coordination.
pub struct CoordinationLoadBalancer {
    pub(crate) strategies: HashMap<String, super::load_balancing_types::LoadBalancingStrategy>,
    pub(crate) capacity_tracker: NodeCapacityTracker,
    pub(crate) performance_metrics: PerformanceMetrics,
    pub(crate) feedback_sender: CoordinationFeedbackSender,
}

/// Named pub/sub channels and subscription bookkeeping for Coordination.
pub struct CoordinationBroadcaster {
    pub(crate) channels: HashMap<String, BroadcastChannel>,
    pub(crate) message_types: MessageTypeRegistry,
    pub(crate) subscription_manager: SubscriptionManager,
}
