// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Service Communication
//!
//! Manages communication channels and messaging with ecosystem services.
//!
//! ## WateringHole Sovereignty: Discover by Capability, Address by Name
//!
//! This module receives **already-discovered** services. The caller must discover
//! by capability (e.g., `discover_capability("crypto.encrypt")`), not by hardcoded
//! primal name. Once discovered, we use `service.name` for:
//! - **IPC addressing** (socket paths, endpoint resolution) — correct use of name
//! - **Logging and error messages** — informational only
//!
//! **Evolution path**: Callers should use `RuntimeDiscovery::discover_capability()`
//! to obtain services; never pass services selected by `if name == "beardog"`.

mod tests;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::constants::timeouts;
use toadstool_common::constants::PRIMAL_NAME;
#[cfg(feature = "networking")]
use toadstool_common::interned_strings::protocols;
use toadstool_common::service_discovery::DiscoveredService;

use super::types::{EcosystemMessage, ServiceChannel, ServiceClient, ServiceStatus};

/// Communication manager for service channels and messaging
pub struct CommunicationManager {
    channels: Arc<RwLock<HashMap<String, ServiceChannel>>>,
    _default_timeout: Duration,
}

impl CommunicationManager {
    pub fn new() -> Self {
        Self::with_timeout(timeouts::DEFAULT_REQUEST_TIMEOUT)
    }

    #[must_use]
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            _default_timeout: timeout,
        }
    }

    pub async fn create_channel(
        &self,
        service: &DiscoveredService,
    ) -> ToadStoolResult<ServiceChannel> {
        info!("📡 Creating channel for service: {}", service.name);

        let endpoint = service
            .primary_endpoint()
            .map(|e| e.url())
            .ok_or_else(|| {
                ToadStoolError::integration(format!(
                    "No endpoint discovered for service: {}. Deep Debt: Services must be discovered at runtime, not hardcoded.",
                    service.name
                ))
            })?;

        let client = self.create_client_for_service(service)?;

        let channel = ServiceChannel {
            service_id: service.id.clone(),
            service_name: service.name.clone(),
            endpoint,
            client,
            last_heartbeat: std::time::SystemTime::now(),
            status: ServiceStatus::Discovered,
        };

        let mut channels = self.channels.write().await;
        channels
            .entry(service.id.clone())
            .or_insert_with(|| channel.clone());

        info!("✅ Channel created for service: {}", service.name);
        Ok(channel)
    }

    #[allow(clippy::unused_async)] // Conditional async: has await when networking enabled
    pub async fn send_message(
        &self,
        channel: &ServiceChannel,
        message: EcosystemMessage,
    ) -> ToadStoolResult<EcosystemMessage> {
        debug!("📤 Sending message to service: {}", channel.service_name);

        match &channel.client {
            #[cfg(feature = "networking")]
            ServiceClient::Tarpc(wrapper_mutex) => {
                let guard = wrapper_mutex.lock().await;
                let wrapper = guard.as_ref().ok_or_else(|| {
                    ToadStoolError::runtime(format!(
                        "tarpc wrapper not initialized for {}",
                        channel.service_name
                    ))
                })?;
                debug!(
                    "tarpc endpoint for {} — using JSON-RPC fallback transport",
                    channel.service_name
                );
                self.send_via_unix_socket(wrapper.fallback_client(), message)
                    .await
            }

            #[cfg(feature = "networking")]
            ServiceClient::UnixSocket(rpc_client) => {
                self.send_via_unix_socket(rpc_client, message).await
            }

            #[cfg(not(feature = "networking"))]
            ServiceClient::Disabled => Ok(self.fallback_response(message)),
        }
    }

    #[allow(clippy::unused_async)] // Conditional async: has await when networking enabled
    pub async fn check_health(&self, channel: &ServiceChannel) -> ToadStoolResult<()> {
        debug!("🔍 Checking health of service: {}", channel.service_name);

        match &channel.client {
            #[cfg(feature = "networking")]
            ServiceClient::UnixSocket(rpc_client) => {
                let _: serde_json::Value = rpc_client
                    .call("health", serde_json::json!({}))
                    .await
                    .map_err(|e| {
                    ToadStoolError::network(format!("Health check failed: {e}"))
                })?;
                Ok(())
            }

            #[cfg(feature = "networking")]
            ServiceClient::Tarpc(wrapper_mutex) => {
                let guard = wrapper_mutex.lock().await;
                let wrapper = guard.as_ref().ok_or_else(|| {
                    ToadStoolError::runtime(format!(
                        "tarpc wrapper not initialized for {}",
                        channel.service_name
                    ))
                })?;
                let _: serde_json::Value = wrapper
                    .fallback_client()
                    .call("health", serde_json::json!({}))
                    .await
                    .map_err(|e| ToadStoolError::network(format!("Health check failed: {e}")))?;
                Ok(())
            }

            #[cfg(not(feature = "networking"))]
            ServiceClient::Disabled => Ok(()),
        }
    }

    pub async fn send_heartbeat(&self, service_id: &str) -> ToadStoolResult<()> {
        let channels = self.channels.read().await;
        let channel = channels
            .get(service_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Channel not found: {service_id}")))?;

        let heartbeat_msg =
            EcosystemMessage::heartbeat(PRIMAL_NAME.to_string(), channel.service_name.clone());

        self.send_message(channel, heartbeat_msg).await?;

        drop(channels);
        let mut channels = self.channels.write().await;
        if let Some(channel) = channels.get_mut(service_id) {
            channel.last_heartbeat = std::time::SystemTime::now();
        }

        Ok(())
    }

    pub async fn get_channel(&self, service_id: &str) -> Option<ServiceChannel> {
        let channels = self.channels.read().await;
        channels.get(service_id).cloned()
    }

    pub async fn get_all_channels(&self) -> Vec<ServiceChannel> {
        let channels = self.channels.read().await;
        channels.values().cloned().collect()
    }

    pub async fn remove_channel(&self, service_id: &str) {
        let mut channels = self.channels.write().await;
        if let Some(channel) = channels.remove(service_id) {
            info!("🗑️  Removed channel for service: {}", channel.service_name);
        }
    }

    pub async fn update_channel_status(&self, service_id: &str, status: ServiceStatus) {
        let mut channels = self.channels.write().await;
        if let Some(channel) = channels.get_mut(service_id) {
            channel.status = status;
        }
    }

    #[cfg(feature = "networking")]
    fn create_client_for_service(
        &self,
        service: &DiscoveredService,
    ) -> ToadStoolResult<ServiceClient> {
        for endpoint in &service.endpoints {
            let is_jsonrpc =
                endpoint.protocol == protocols::JSONRPC || endpoint.protocol == "json-rpc"; // backward-compat alias
            let is_unix =
                endpoint.protocol == protocols::UNIX || endpoint.protocol == "unix-socket"; // backward-compat alias
            if is_jsonrpc || is_unix {
                let socket_path = Self::extract_socket_path(endpoint, &service.name);
                return Ok(ServiceClient::UnixSocket(
                    toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
                ));
            }
            if endpoint.protocol == protocols::TARPC {
                let socket_path = Self::extract_socket_path(endpoint, &service.name);
                let fallback =
                    toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);
                // Evolution path: direct tarpc binary transport should replace this fallback in Phase 3.
                info!(
                    service_name = %service.name,
                    "tarpc endpoint using JSON-RPC fallback transport — operators: tarpc is speaking JSON-RPC underneath until Phase 3 binary transport"
                );
                return Ok(ServiceClient::Tarpc(Arc::new(tokio::sync::Mutex::new(
                    Some(super::types::TarpcClientWrapper::with_fallback(fallback)),
                ))));
            }
        }

        let socket_path = toadstool_common::primal_sockets::resolve_socket_path_for_service(
            &service.name,
            &toadstool_common::primal_sockets::SocketPathEnv::from_env(),
            None,
        );
        Ok(ServiceClient::UnixSocket(
            toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        ))
    }

    #[cfg(not(feature = "networking"))]
    fn create_client_for_service(
        &self,
        _service: &DiscoveredService,
    ) -> ToadStoolResult<ServiceClient> {
        Ok(ServiceClient::Disabled)
    }

    #[cfg(feature = "networking")]
    fn extract_socket_path(
        endpoint: &toadstool_common::primal_identity::ServiceEndpoint,
        service_name: &str,
    ) -> std::path::PathBuf {
        if endpoint.address.starts_with('/') {
            return std::path::PathBuf::from(&endpoint.address);
        }
        if let Some(path) = endpoint.metadata.get("socket_path") {
            return std::path::PathBuf::from(path);
        }
        if let Some(path) = endpoint.metadata.get("path") {
            return std::path::PathBuf::from(path);
        }
        toadstool_common::primal_sockets::resolve_socket_path_for_service(
            service_name,
            &toadstool_common::primal_sockets::SocketPathEnv::from_env(),
            None,
        )
    }

    #[cfg(feature = "networking")]
    async fn send_via_unix_socket(
        &self,
        rpc_client: &toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
        message: EcosystemMessage,
    ) -> ToadStoolResult<EcosystemMessage> {
        let params = serde_json::to_value(&message)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize message: {e}")))?;

        let response_message: EcosystemMessage = rpc_client
            .call_typed("ecosystem.send_message", params)
            .await
            .map_err(|e| ToadStoolError::network(format!("Unix socket RPC failed: {e}")))?;

        Ok(response_message)
    }

    #[cfg(not(feature = "networking"))]
    fn fallback_response(&self, original: EcosystemMessage) -> EcosystemMessage {
        EcosystemMessage {
            id: uuid::Uuid::new_v4(),
            from: format!("{}_local", PRIMAL_NAME),
            to: original.from,
            message_type: super::types::EcosystemMessageType::StatusUpdate,
            payload: serde_json::json!({
                "status": "networking_disabled",
                "reason": "Networking feature not compiled",
                "mode": "degraded",
                "original_message_id": original.id.to_string()
            }),
            timestamp: std::time::SystemTime::now(),
        }
    }
}

impl Default for CommunicationManager {
    fn default() -> Self {
        Self::new()
    }
}
