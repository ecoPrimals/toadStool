// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::coordination::types::protocols::{
    CoordinationTransport, HttpProtocolConfig, MessageQueueProtocolConfig, ProtocolConfig,
};
use crate::coordination::types::{
    BroadcastConfig, CapacityConfig, CoordinationConnectionConfig, CoordinationDiscoveryConfig,
    CoordinationIntegrationConfig, DistributionConfig, LoadBalancerConfig, ReceiverConfig,
};
use std::collections::HashMap;
use std::time::Duration;
use toadstool_common::auth::ServiceAuthConfig;
use toadstool_common::config_bases::ConnectionPoolConfig;

pub(super) fn sample_coordination_connection_config_with_endpoints(
    endpoints: Vec<String>,
) -> CoordinationConnectionConfig {
    CoordinationConnectionConfig {
        endpoints,
        protocol_config: ProtocolConfig {
            protocol: CoordinationTransport::HTTP,
            http: HttpProtocolConfig {
                timeout_ms: 5000,
                max_retries: 3,
                headers: HashMap::new(),
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

pub(super) fn sample_coordination_connection_config() -> CoordinationConnectionConfig {
    sample_coordination_connection_config_with_endpoints(vec!["http://localhost:8080".to_string()])
}

pub(super) fn sample_coordination_config() -> CoordinationIntegrationConfig {
    CoordinationIntegrationConfig {
        connection_config: sample_coordination_connection_config(),
        distribution_config: DistributionConfig {
            max_subtasks: 8,
            splitting_strategies: HashMap::new(),
        },
        discovery_config: CoordinationDiscoveryConfig {
            discovery_interval: Duration::from_mins(1),
            node_timeout: Duration::from_secs(30),
        },
        load_balancer_config: LoadBalancerConfig {
            strategy: "round-robin".to_string(),
            feedback_interval: Duration::from_secs(5),
        },
        broadcast_config: BroadcastConfig {
            channels: vec![],
            message_retention: Duration::from_mins(1),
        },
        capacity_config: CapacityConfig {
            monitoring_interval: Duration::from_secs(10),
            resource_buffer: 0.0,
        },
        receiver_config: ReceiverConfig {
            max_concurrent_jobs: 4,
            job_timeout: Duration::from_mins(2),
        },
    }
}
