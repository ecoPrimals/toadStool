// SPDX-License-Identifier: AGPL-3.0-only
//! Discovery client - RPC-based node discovery via Songbird

#![allow(deprecated)] // Intentional: IPC addressing requires well-known names

use std::sync::Arc;
use tracing::debug;

#[cfg(test)]
use toadstool::error::ToadStoolError;
use toadstool::error::ToadStoolResult;

use crate::songbird_integration::types::{DiscoveryClient, NodeRegistration, SongbirdConnection};
#[cfg(test)]
use crate::songbird_integration::types::{NodeCapabilities, NodeMetadata, NodeType};

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
    pub async fn new(connection: Arc<SongbirdConnection>) -> ToadStoolResult<Self> {
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
    pub fn for_test(connection: Arc<SongbirdConnection>, socket_path: std::path::PathBuf) -> Self {
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

        let node_id = node_data["node_id"]
            .as_str()
            .ok_or_else(|| ToadStoolError::runtime("Missing node_id in discovery data"))?
            .to_string();

        let type_str = node_data["type"].as_str().unwrap_or(node_type::TOADSTOOL);
        let parsed_node_type = match type_str {
            s if s == node_type::TOADSTOOL => NodeType::ToadStool,
            s if s == node_type::NESTGATE => NodeType::NestGate,
            s if s == node_type::BEARDOG => NodeType::BearDog,
            s if s == node_type::SONGBIRD => NodeType::Songbird,
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
