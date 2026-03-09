// SPDX-License-Identifier: AGPL-3.0-only
//! Core integration struct types

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::broadcast_types::{BroadcastChannel, MessageTypeRegistry, SubscriptionManager};
use super::capacity_types::LocalCapacityManager;
use super::discovery::{DiscoveryClient, NodeRegistry};
use super::distribution_types::{JobCoordinator, JobSplittingStrategy, LoadEstimator};
use super::load_balancing_types::{
    NodeCapacityTracker, PerformanceMetrics, SongbirdFeedbackSender,
};
use super::CapabilityTracker;
use super::NetworkHealthMonitor;
use crate::common::distribution::DistributionAlgorithm;
use crate::UniversalJobType;

// ============================================================================
// Core Integration Types
// ============================================================================

pub struct ToadStoolSongbirdIntegration {
    pub(crate) instance_id: String,
    pub(crate) connection: super::connection_types::SongbirdConnection,
    pub(crate) local_capacity: Arc<LocalCapacityManager>,
    pub(crate) workload_scheduler: Arc<crate::universal::UniversalScheduler>,
}

pub struct MassiveJobDistributor {
    pub(crate) splitting_strategies: HashMap<UniversalJobType, JobSplittingStrategy>,
    pub(crate) distribution_algorithms: Vec<DistributionAlgorithm>,
    pub(crate) load_estimator: LoadEstimator,
    pub(crate) job_coordinator: JobCoordinator,
}

pub struct SongbirdNetworkDiscovery {
    pub(crate) discovery_client: DiscoveryClient,
    pub(crate) node_registry: RwLock<NodeRegistry>,
    pub(crate) capability_tracker: CapabilityTracker,
    pub(crate) health_monitor: NetworkHealthMonitor,
}

pub struct SongbirdLoadBalancer {
    pub(crate) strategies: HashMap<String, super::load_balancing_types::LoadBalancingStrategy>,
    pub(crate) capacity_tracker: NodeCapacityTracker,
    pub(crate) performance_metrics: PerformanceMetrics,
    pub(crate) feedback_sender: SongbirdFeedbackSender,
}

pub struct SongbirdBroadcaster {
    pub(crate) channels: HashMap<String, BroadcastChannel>,
    pub(crate) message_types: MessageTypeRegistry,
    pub(crate) subscription_manager: SubscriptionManager,
}
