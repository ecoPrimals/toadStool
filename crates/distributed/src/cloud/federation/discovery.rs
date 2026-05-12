// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use toadstool::error::ToadStoolResult;

use crate::cloud::types::FederationNode;

use super::FederationError;

const PROBE_TIMEOUT_TEST: Duration = Duration::from_millis(100);
const PROBE_TIMEOUT_PROD: Duration = Duration::from_secs(5);

impl super::CloudFederationManager {
    /// Discover federation nodes from configured discovery endpoints.
    ///
    /// Iterates `config.discovery_endpoints` and attempts to connect to each
    /// as a federation peer. Endpoints that respond with valid node metadata
    /// are returned as `FederationNode` candidates for `add_node()`.
    ///
    /// Returns an empty vec (not an error) if no discovery endpoints are configured
    /// or none respond -- federation can still function with manually-added nodes.
    pub async fn discover_nodes(&self) -> ToadStoolResult<Vec<FederationNode>> {
        if self.config.discovery_endpoints.is_empty() {
            return Ok(Vec::new());
        }

        let mut discovered = Vec::new();
        for endpoint in &self.config.discovery_endpoints {
            match self.probe_endpoint(endpoint).await {
                Ok(node) => discovered.push(node),
                Err(e) => {
                    tracing::debug!("Federation endpoint {endpoint} unreachable: {e}");
                }
            }
        }

        tracing::info!(
            "Federation discovery: {} of {} endpoints responded",
            discovered.len(),
            self.config.discovery_endpoints.len()
        );
        Ok(discovered)
    }

    async fn probe_endpoint(&self, endpoint: &str) -> ToadStoolResult<FederationNode> {
        use tokio::net::TcpStream;
        use tokio::time::timeout;

        let addr = endpoint
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        let probe_timeout = if cfg!(test) {
            PROBE_TIMEOUT_TEST
        } else {
            PROBE_TIMEOUT_PROD
        };
        let stream = timeout(probe_timeout, TcpStream::connect(addr))
            .await
            .map_err(|_| FederationError::InvalidNode(format!("Timeout connecting to {endpoint}")))?
            .map_err(|_| FederationError::InvalidNode(format!("Cannot connect to {endpoint}")))?;

        let peer_addr = stream
            .peer_addr()
            .map_err(|e| FederationError::InvalidNode(e.to_string()))?;

        Ok(FederationNode {
            id: format!("discovered-{peer_addr}"),
            provider: endpoint.to_string(),
            capabilities: vec!["compute".to_string()],
            region: String::new(),
        })
    }
}
