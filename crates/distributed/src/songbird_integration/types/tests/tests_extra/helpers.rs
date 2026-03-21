// SPDX-License-Identifier: AGPL-3.0-only

use crate::songbird_integration::types::protocols::{
    GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig, ProtocolConfig,
    SongbirdProtocol,
};
use crate::songbird_integration::types::{
    BroadcastConfig, CapacityConfig, DistributionConfig, LoadBalancerConfig, ReceiverConfig,
    SongbirdConnectionConfig, SongbirdDiscoveryConfig, SongbirdIntegrationConfig,
};
use std::collections::HashMap;
use std::time::Duration;
use toadstool_common::auth::ServiceAuthConfig;
use toadstool_common::config_bases::ConnectionPoolConfig;

pub(super) fn sample_songbird_connection_config_with_endpoints(
    endpoints: Vec<String>,
) -> SongbirdConnectionConfig {
    SongbirdConnectionConfig {
        endpoints,
        protocol_config: ProtocolConfig {
            protocol: SongbirdProtocol::HTTP,
            http: HttpProtocolConfig {
                timeout_ms: 5000,
                max_retries: 3,
                headers: HashMap::new(),
            },
            grpc: GrpcProtocolConfig {
                timeout_ms: 5000,
                max_message_size: 1024 * 1024,
                compression: false,
            },
            message_queue: MessageQueueProtocolConfig {
                queue_name: "default".to_string(),
                exchange: "default".to_string(),
                routing_key: "default".to_string(),
            },
        },
        auth_config: ServiceAuthConfig::default(),
        pool: ConnectionPoolConfig::default(),
    }
}

pub(super) fn sample_songbird_connection_config() -> SongbirdConnectionConfig {
    sample_songbird_connection_config_with_endpoints(vec!["http://localhost:8080".to_string()])
}

pub(super) fn sample_songbird_integration_config() -> SongbirdIntegrationConfig {
    SongbirdIntegrationConfig {
        connection_config: sample_songbird_connection_config(),
        distribution_config: DistributionConfig {
            max_subtasks: 8,
            splitting_strategies: HashMap::new(),
        },
        discovery_config: SongbirdDiscoveryConfig {
            discovery_interval: Duration::from_secs(60),
            node_timeout: Duration::from_secs(30),
        },
        load_balancer_config: LoadBalancerConfig {
            strategy: "round-robin".to_string(),
            feedback_interval: Duration::from_secs(5),
        },
        broadcast_config: BroadcastConfig {
            channels: vec![],
            message_retention: Duration::from_secs(60),
        },
        capacity_config: CapacityConfig {
            monitoring_interval: Duration::from_secs(10),
            resource_buffer: 0.0,
        },
        receiver_config: ReceiverConfig {
            max_concurrent_jobs: 4,
            job_timeout: Duration::from_secs(120),
        },
    }
}
