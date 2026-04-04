// SPDX-License-Identifier: AGPL-3.0-only
//! # Songbird Integration - Universal Signal Coordination
//!
//! **DEPRECATED**: Use `coordination_integration` for vendor-agnostic coordination services.
//!
//! ToadStool's integration with Songbird, the universal signal coordinator.
//! Songbird handles orchestration, load balancing, discovery, and broadcasting.
//! ToadStool handles compute execution.
//!
//! When ToadStool needs to talk outside local (or even sometimes local), it uses Songbird.
//! When massive jobs drop, ToadStool breaks them up and sends them via Songbird to hundreds of nodes.

#![allow(deprecated)]

pub mod broadcasting;
pub mod capability_client; // ✅ Production-ready capability-based discovery client
pub(crate) mod capacity;
pub mod connection;
pub mod discovery;
pub mod distribution;
pub mod integration;
pub mod load_balancing;
pub(crate) mod messaging;
pub(crate) mod transport;
pub mod types;

// Re-export main types
pub use capability_client::{CapabilityClient, ClientStats};
pub use toadstool_common::auth::AuthType; // Re-export from canonical source
pub use types::ToadStoolSongbirdIntegration;
pub use types::{
    AuthConfig, AuthenticationConfig, AvailableCapacity, BroadcastChannel, BroadcastConfig,
    CapabilitySnapshot, CapabilityTracker, CapacityConfig, CapacityInfo, CompletionStrategy,
    ComplexityLevel, ConnectionHealth, CoordinationJob, CoordinationStrategy, DiscoveryClient,
    DistributionAlgorithm, DistributionConfig, DistributionPlan, ExecutionMetrics,
    GrpcProtocolConfig, HttpProtocolConfig, IntensityLevel, JobAnalysis, JobComplexity,
    JobCoordinator, JobDistributionStrategy, JobReceiver, JobResult, JobSplittingStrategy,
    LoadBalancerConfig, LoadBalancingAdvice, LoadBalancingStrategy, LoadEstimator, LoadMetric,
    LocalCapacityManager, MassiveJobDistributor, MassiveJobResult, MessageQueueProtocolConfig,
    MessageTypeRegistry, NetworkCapacity, NetworkHealthMonitor, NetworkRequirements, NetworkStatus,
    NodeCapabilities, NodeCapacityTracker, NodeId, NodeMetadata, NodeRegistration, NodeRegistry,
    NodeType, PerformanceMetrics, ProtocolConfig, ReceiverConfig, RegistrationResponse,
    ResourceReservation, SongbirdBroadcastMessage, SongbirdBroadcaster, SongbirdConnection,
    SongbirdConnectionConfig, SongbirdDiscoveryConfig, SongbirdFeedbackSender,
    SongbirdIntegrationConfig, SongbirdJobMessage, SongbirdJobRequest, SongbirdJobResponse,
    SongbirdLoadBalancer, SongbirdNetworkDiscovery, SongbirdProtocol, SplittingStrategyType,
    SubTask, SubTaskHandle, SubTaskPlan, SubTaskStatus, SubscriptionManager, UniversalJobProcessor,
};
