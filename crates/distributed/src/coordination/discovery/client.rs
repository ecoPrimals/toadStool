// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discovery client - RPC-based node discovery via Coordination

#![expect(
    deprecated,
    reason = "IPC addressing requires well-known names during migration"
)]

use std::sync::Arc;
use tracing::debug;

#[cfg(test)]
use toadstool::error::ToadStoolError;
use toadstool::error::ToadStoolResult;

use crate::coordination::types::{CoordinationConnection, DiscoveryClient, NodeRegistration};
#[cfg(test)]
use crate::coordination::types::{NodeCapabilities, NodeMetadata, NodeType};

impl Clone for DiscoveryClient {
    fn clone(&self) -> Self {
        let biomeos = toadstool_common::primal_sockets::get_biomeos_dir();
        let socket_path = biomeos.join("coordination.sock");

        Self {
            connection: Arc::clone(&self.connection),
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        }
    }
}

impl DiscoveryClient {
    /// Create a client bound to the coordination Unix JSON-RPC socket.
    pub async fn new(connection: Arc<CoordinationConnection>) -> ToadStoolResult<Self> {
        let socket_path = toadstool_common::primal_sockets::discover_coordination_socket()
            .await
            .unwrap_or_else(|_| {
                toadstool_common::primal_sockets::get_biomeos_dir().join("coordination.sock")
            });

        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        Ok(Self {
            connection,
            rpc_client,
        })
    }

    /// Test-only constructor that bypasses async discovery (avoids block_on inside tokio runtime).
    #[cfg(test)]
    pub fn for_test(
        connection: Arc<CoordinationConnection>,
        socket_path: std::path::PathBuf,
    ) -> Self {
        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);
        Self {
            connection,
            rpc_client,
        }
    }

    /// Call `coordination.discover_nodes` and return registrations (empty on RPC failure).
    pub async fn discover_nodes(&self) -> ToadStoolResult<Vec<NodeRegistration>> {
        let mut params = serde_json::json!({});

        if let Some(ref token) = self.connection.auth_token {
            params["auth_token"] = serde_json::json!(token);
        }

        let nodes: Vec<NodeRegistration> = self
            .rpc_client
            .call_typed("coordination.discover_nodes", params)
            .await
            .unwrap_or_else(|e| {
                debug!("Discovery failed: {e}, returning empty list");
                Vec::new()
            });

        Ok(nodes)
    }

    #[cfg(test)]
    pub(crate) fn parse_node_data(
        &self,
        node_data: &serde_json::Value,
    ) -> ToadStoolResult<NodeRegistration> {
        use toadstool_common::constants::ecosystem::node_type;
        use toadstool_common::interned_strings::capabilities;

        let node_id = node_data["node_id"]
            .as_str()
            .ok_or_else(|| ToadStoolError::runtime("Missing node_id in discovery data"))?
            .to_string();

        // `type` may be a legacy product label (node_type::*) or a capability id (see `capabilities::*`).
        let type_str = node_data["type"].as_str().unwrap_or(node_type::TOADSTOOL);
        let parsed_node_type = match type_str {
            s if s == node_type::TOADSTOOL => NodeType::ToadStool,
            s if s == node_type::NESTGATE || s == capabilities::STORAGE => NodeType::Storage,
            s if s == node_type::BEARDOG || s == capabilities::CRYPTO || s == "security" => {
                NodeType::Security
            }
            s if s == node_type::SONGBIRD || s == capabilities::COORDINATION => {
                NodeType::Coordination
            }
            custom => NodeType::Custom(custom.to_string()),
        };

        let capabilities = NodeCapabilities {
            cpu_cores: node_data["capabilities"]["cpu_cores"]
                .as_f64()
                .unwrap_or(0.0),
            memory_gb: node_data["capabilities"]["memory_gb"]
                .as_f64()
                .unwrap_or(0.0),
            storage_gb: node_data["capabilities"]["storage_gb"]
                .as_f64()
                .unwrap_or(0.0),
            gpu_count: node_data["capabilities"]["gpu_count"].as_u64().unwrap_or(0) as u32,
            specialized_hardware: node_data["capabilities"]["specialized_hardware"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            software_capabilities: node_data["capabilities"]["software_capabilities"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        };

        let endpoints = node_data["endpoints"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| vec!["unknown".to_string()]);

        let protocols = node_data["protocols"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| vec!["http".to_string()]);

        Ok(NodeRegistration {
            node_id,
            node_type: parsed_node_type,
            capabilities: capabilities.clone(),
            endpoints,
            protocols,
            metadata: NodeMetadata {
                version: node_data["version"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                build_info: node_data["build_info"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                capabilities,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use toadstool_common::constants::ecosystem::node_type;

    use crate::coordination::types::{
        ConnectionHealth, CoordinationConnection, CoordinationTransport, GrpcProtocolConfig,
        HttpProtocolConfig, MessageQueueProtocolConfig, NodeType, ProtocolConfig,
    };

    fn test_client() -> super::DiscoveryClient {
        let protocol = ProtocolConfig {
            protocol: CoordinationTransport::HTTP,
            http: HttpProtocolConfig {
                timeout_ms: 5000,
                max_retries: 3,
                headers: Default::default(),
            },
            grpc: GrpcProtocolConfig {
                timeout_ms: 5000,
                max_message_size: 1024 * 1024,
                compression: false,
            },
            message_queue: MessageQueueProtocolConfig {
                queue_name: "q".to_string(),
                exchange: "ex".to_string(),
                routing_key: "rk".to_string(),
            },
        };
        let conn = Arc::new(CoordinationConnection {
            endpoints: vec![],
            active_endpoint: "unix:///tmp/x.sock".to_string(),
            auth_token: None,
            health_status: ConnectionHealth::Healthy,
            protocol_config: protocol,
            #[cfg(feature = "channels")]
            reply_channel: None,
        });
        super::DiscoveryClient::for_test(
            conn,
            std::path::PathBuf::from("/tmp/parse-node-test.sock"),
        )
    }

    #[test]
    fn parse_node_data_errors_when_node_id_missing() {
        let client = test_client();
        let err = client
            .parse_node_data(&json!({ "type": node_type::TOADSTOOL }))
            .unwrap_err();
        assert!(err.to_string().contains("node_id"), "{err}");
    }

    #[test]
    fn parse_node_data_maps_legacy_toadstool_label() {
        let client = test_client();
        let reg = client
            .parse_node_data(&json!({
                "node_id": "n1",
                "type": node_type::TOADSTOOL,
                "capabilities": { "cpu_cores": 2.5, "memory_gb": 8.0, "storage_gb": 100.0 },
            }))
            .unwrap();
        assert_eq!(reg.node_id, "n1");
        assert!(matches!(reg.node_type, NodeType::ToadStool));
        assert!((reg.capabilities.cpu_cores - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_node_data_maps_nestgate_to_storage() {
        let client = test_client();
        let reg = client
            .parse_node_data(&json!({
                "node_id": "s1",
                "type": node_type::NESTGATE,
            }))
            .unwrap();
        assert!(matches!(reg.node_type, NodeType::Storage));
    }

    #[test]
    fn parse_node_data_maps_security_alias() {
        let client = test_client();
        let reg = client
            .parse_node_data(&json!({
                "node_id": "sec",
                "type": "security",
            }))
            .unwrap();
        assert!(matches!(reg.node_type, NodeType::Security));
    }

    #[test]
    fn parse_node_data_defaults_type_to_toadstool_when_absent() {
        let client = test_client();
        let reg = client.parse_node_data(&json!({ "node_id": "n2" })).unwrap();
        assert!(matches!(reg.node_type, NodeType::ToadStool));
    }

    #[test]
    fn parse_node_data_unknown_type_becomes_custom() {
        let client = test_client();
        let reg = client
            .parse_node_data(&json!({
                "node_id": "c1",
                "type": "MyCustomWorker",
            }))
            .unwrap();
        assert!(matches!(
            reg.node_type,
            NodeType::Custom(ref s) if s == "MyCustomWorker"
        ));
    }

    #[test]
    fn parse_node_data_defaults_endpoints_and_protocols_when_missing() {
        let client = test_client();
        let reg = client.parse_node_data(&json!({ "node_id": "e1" })).unwrap();
        assert_eq!(reg.endpoints, vec!["unknown".to_string()]);
        assert_eq!(reg.protocols, vec!["http".to_string()]);
    }

    #[test]
    fn parse_node_data_collects_specialized_hardware_array() {
        let client = test_client();
        let reg = client
            .parse_node_data(&json!({
                "node_id": "hw",
                "capabilities": {
                    "specialized_hardware": ["fpga-x", "nvlink"],
                },
            }))
            .unwrap();
        assert_eq!(
            reg.capabilities.specialized_hardware,
            vec!["fpga-x".to_string(), "nvlink".to_string()]
        );
    }
}
