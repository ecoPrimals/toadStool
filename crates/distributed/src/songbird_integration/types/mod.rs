// SPDX-License-Identifier: AGPL-3.0-only
//! Type definitions for Songbird Integration

mod broadcast_types;
mod capabilities;
mod capacity_types;
mod config_types;
mod connection_types;
mod discovery;
mod distribution_types;
mod integration_types;
mod job_types;
mod load_balancing_types;
mod message_types;
mod node;
mod protocols;
mod receiver_types;

#[cfg(test)]
mod channel_name_tests;

#[cfg(test)]
mod tests;

// Re-exports: capabilities
pub use capabilities::{CapabilitySnapshot, CapabilityTracker};

// Re-exports: config
pub use config_types::{
    AuthConfig, AuthenticationConfig, BroadcastConfig, CapacityConfig, DistributionConfig,
    LoadBalancerConfig, ReceiverConfig, SongbirdConnectionConfig, SongbirdDiscoveryConfig,
    SongbirdIntegrationConfig,
};

// Re-exports: connection
pub use connection_types::{ConnectionHealth, SongbirdConnection};

// Re-exports: discovery
pub use discovery::{
    AvailableCapacity, DiscoveryClient, LoadBalancingAdvice, NetworkCapacity, NetworkHealthMonitor,
    NetworkRequirements, NetworkStatus, NodeMetadata, NodeRegistration, NodeRegistry, NodeType,
    RegistrationResponse, ResourceReservation,
};

// Re-exports: distribution
pub use distribution_types::{
    DistributionAlgorithm, JobCoordinator, JobSplittingStrategy, LoadEstimator, LoadMetric,
    SplittingStrategyType,
};

// Re-exports: job
pub use job_types::{
    CompletionStrategy, ComplexityLevel, CoordinationJob, CoordinationStrategy, DistributionPlan,
    ExecutionMetrics, IntensityLevel, JobAnalysis, JobComplexity, JobDistributionStrategy,
    JobResult, MassiveJobResult, SongbirdJobRequest, SongbirdJobResponse, SubTask, SubTaskHandle,
    SubTaskPlan, SubTaskStatus,
};

// Re-exports: load balancing
pub use load_balancing_types::{
    LoadBalancingStrategy, NodeCapacityTracker, PerformanceMetrics, SongbirdFeedback,
    SongbirdFeedbackReceiver, SongbirdFeedbackSender,
};

// Re-exports: broadcast
pub use broadcast_types::{BroadcastChannel, MessageTypeRegistry, SubscriptionManager};

// Re-exports: message
pub use message_types::{SongbirdBroadcastMessage, SongbirdJobMessage};

// Re-exports: node
pub use node::{NodeCapabilities, NodeId};

// Re-exports: protocols
pub use protocols::{
    GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig, ProtocolConfig,
    SongbirdProtocol,
};

// Re-exports: receiver
pub use receiver_types::{JobReceiver, UniversalJobProcessor};

// Re-exports: integration structs
pub use integration_types::{
    MassiveJobDistributor, SongbirdBroadcaster, SongbirdLoadBalancer, SongbirdNetworkDiscovery,
    ToadStoolSongbirdIntegration,
};

// Re-exports: capacity
pub use capacity_types::{CapacityInfo, LocalCapacityManager};
