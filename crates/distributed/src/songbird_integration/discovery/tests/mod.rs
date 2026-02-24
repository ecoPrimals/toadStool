//! Discovery module tests

mod client_tests;
mod discovery_tests;
mod registry_tests;

use std::sync::Arc;
use std::time::Duration;

use crate::songbird_integration::types::{
    NodeCapabilities, NodeMetadata, NodeRegistration, NodeType, ProtocolConfig, SongbirdConnection,
    SongbirdDiscoveryConfig, SongbirdNetworkDiscovery,
};

use crate::songbird_integration::types::ConnectionHealth;

fn make_protocol_config() -> ProtocolConfig {
    use crate::songbird_integration::types::{
        GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig, SongbirdProtocol,
    };
    use std::collections::HashMap;

    ProtocolConfig {
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
            queue_name: "test".to_string(),
            exchange: "test".to_string(),
            routing_key: "test".to_string(),
        },
    }
}

fn make_songbird_connection() -> SongbirdConnection {
    SongbirdConnection {
        endpoints: vec!["unix:///tmp/test-songbird.sock".to_string()],
        active_endpoint: "unix:///tmp/test-songbird.sock".to_string(),
        auth_token: None,
        health_status: ConnectionHealth::Healthy,
        protocol_config: make_protocol_config(),
        #[cfg(feature = "channels")]
        reply_channel: None,
    }
}

pub(super) fn make_node_registration(
    node_id: &str,
    node_type: NodeType,
    cpu: f64,
    memory_gb: f64,
    storage_gb: f64,
) -> NodeRegistration {
    let caps = NodeCapabilities {
        cpu_cores: cpu,
        memory_gb,
        storage_gb,
        gpu_count: 0,
        specialized_hardware: vec![],
        software_capabilities: vec![],
    };
    NodeRegistration {
        node_id: node_id.to_string(),
        node_type,
        capabilities: caps.clone(),
        endpoints: vec!["http://127.0.0.1:8080".to_string()],
        protocols: vec!["http".to_string()],
        metadata: NodeMetadata {
            version: "1.0".to_string(),
            build_info: "test".to_string(),
            capabilities: caps,
        },
    }
}

pub(super) fn make_discovery() -> (SongbirdNetworkDiscovery, std::path::PathBuf) {
    let config = SongbirdDiscoveryConfig {
        discovery_interval: Duration::from_secs(60),
        node_timeout: Duration::from_secs(30),
    };
    let conn = Arc::new(make_songbird_connection());
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("songbird.sock");
    let discovery = SongbirdNetworkDiscovery::for_test(config, conn, socket_path.clone());
    (discovery, socket_path)
}
