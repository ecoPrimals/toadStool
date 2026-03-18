// SPDX-License-Identifier: AGPL-3.0-or-later
//! Broadcasting and messaging system

use std::collections::HashMap;
use std::sync::Arc;

use toadstool::error::ToadStoolResult;
use tracing::{debug, info};

use super::types::{
    BroadcastChannel, BroadcastConfig, MessageTypeRegistry, SongbirdBroadcastMessage,
    SongbirdBroadcaster, SongbirdConnection, SubscriptionManager,
};

impl SongbirdBroadcaster {
    pub async fn new(
        config: BroadcastConfig,
        _connection: Arc<SongbirdConnection>,
    ) -> ToadStoolResult<Self> {
        debug!("Initializing broadcaster");
        let registry = MessageTypeRegistry::new();
        let mut channels = HashMap::new();
        // Pre-create channels listed in config so subscribers can attach immediately
        for channel_name in &config.channels {
            registry.register(channel_name.clone());
            channels.insert(channel_name.clone(), BroadcastChannel::new(channel_name));
        }
        Ok(Self {
            channels,
            message_types: registry,
            subscription_manager: SubscriptionManager::new(),
        })
    }

    /// Publish `message` to all subscribers of `message.channel`.
    ///
    /// The message is delivered to:
    /// 1. Any in-process `BroadcastChannel` with the matching name.
    /// 2. The `SubscriptionManager` for late-arriving subscribers.
    pub async fn broadcast(&self, message: &SongbirdBroadcastMessage) -> ToadStoolResult<()> {
        let channel_name = message.channel_name();
        debug!(channel = %channel_name, "Broadcasting message");
        // Log a warning if the message type is not registered — helps catch typos.
        if !self.message_types.is_known(channel_name) {
            tracing::debug!(
                channel = %channel_name,
                "Broadcasting on unregistered channel — register via MessageTypeRegistry"
            );
        }

        // Deliver to pre-configured BroadcastChannel if present
        let channel_subscribers = self
            .channels
            .get(channel_name)
            .map_or(0, |ch| ch.publish(message.clone()).unwrap_or(0));

        // Also deliver through SubscriptionManager (dynamic subscriptions)
        let dynamic_subscribers = self
            .subscription_manager
            .publish(channel_name, message.clone());

        info!(
            channel = %channel_name,
            receivers = channel_subscribers + dynamic_subscribers,
            "Broadcast delivered"
        );
        Ok(())
    }

    /// Subscribe to a named channel and return the receive handle.
    ///
    /// Uses the `SubscriptionManager` for dynamic channel subscriptions.
    pub async fn subscribe_to_channel(
        &self,
        channel_name: &str,
    ) -> ToadStoolResult<tokio::sync::broadcast::Receiver<SongbirdBroadcastMessage>> {
        debug!(channel = %channel_name, "Subscribing to channel");
        Ok(self.subscription_manager.subscribe(channel_name))
    }

    pub async fn unsubscribe_from_channel(&self, channel_name: &str) -> ToadStoolResult<()> {
        debug!(channel = %channel_name, "Closing channel subscription");
        self.subscription_manager.close_channel(channel_name);
        Ok(())
    }

    pub async fn send_targeted_message(
        &self,
        target_nodes: &[String],
        message: &SongbirdBroadcastMessage,
    ) -> ToadStoolResult<()> {
        // For now, targeted messages are delivered via the channel with the same
        // semantics as broadcast. When the Songbird Unix-socket RPC is wired, this
        // will route through `_connection` to deliver only to `target_nodes`.
        debug!(
            channel = %message.channel_name(),
            targets = target_nodes.len(),
            "Sending targeted message"
        );
        self.broadcast(message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::songbird_integration::types::{
        ConnectionHealth, GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig,
        NodeCapabilities, ProtocolConfig, SongbirdBroadcastMessage, SongbirdConnection,
        SongbirdProtocol,
    };
    use std::collections::HashMap;
    use std::time::Duration;
    use std::time::SystemTime;

    fn make_mock_connection() -> SongbirdConnection {
        SongbirdConnection {
            endpoints: vec!["http://localhost:8080".to_string()],
            active_endpoint: "http://localhost:8080".to_string(),
            auth_token: None,
            health_status: ConnectionHealth::Healthy,
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
            #[cfg(feature = "channels")]
            reply_channel: None,
        }
    }

    #[tokio::test]
    async fn test_broadcaster_new() {
        let config = BroadcastConfig {
            channels: vec!["events".to_string(), "alerts".to_string()],
            message_retention: Duration::from_secs(60),
        };
        let connection = Arc::new(make_mock_connection());

        let broadcaster = SongbirdBroadcaster::new(config, connection).await.unwrap();
        let _rx = broadcaster.subscribe_to_channel("events").await.unwrap();
    }

    #[tokio::test]
    async fn test_broadcaster_new_empty_channels() {
        let config = BroadcastConfig {
            channels: vec![],
            message_retention: Duration::from_secs(60),
        };
        let connection = Arc::new(make_mock_connection());

        let broadcaster = SongbirdBroadcaster::new(config, connection).await.unwrap();
        let _rx = broadcaster.subscribe_to_channel("dynamic").await.unwrap();
    }

    #[tokio::test]
    async fn test_broadcast_capability_update() {
        let config = BroadcastConfig {
            channels: vec!["capability-updates".to_string()],
            message_retention: Duration::from_secs(60),
        };
        let connection = Arc::new(make_mock_connection());
        let broadcaster = SongbirdBroadcaster::new(config, connection).await.unwrap();

        let msg = SongbirdBroadcastMessage::CapabilityUpdate {
            node_id: "n1".to_string(),
            capabilities: NodeCapabilities {
                cpu_cores: 4.0,
                memory_gb: 8.0,
                storage_gb: 100.0,
                gpu_count: 0,
                specialized_hardware: vec![],
                software_capabilities: vec![],
            },
            timestamp: SystemTime::now(),
        };

        let result = broadcaster.broadcast(&msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_broadcast_health_update() {
        let config = BroadcastConfig {
            channels: vec!["health-updates".to_string()],
            message_retention: Duration::from_secs(60),
        };
        let connection = Arc::new(make_mock_connection());
        let broadcaster = SongbirdBroadcaster::new(config, connection).await.unwrap();

        let msg = SongbirdBroadcastMessage::HealthUpdate {
            node_id: "n1".to_string(),
            health_status: "healthy".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = broadcaster.broadcast(&msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_broadcast_custom_message() {
        let config = BroadcastConfig {
            channels: vec!["job-complete".to_string()],
            message_retention: Duration::from_secs(60),
        };
        let connection = Arc::new(make_mock_connection());
        let broadcaster = SongbirdBroadcaster::new(config, connection).await.unwrap();

        let msg = SongbirdBroadcastMessage::CustomMessage {
            message_type: "job-complete".to_string(),
            payload: serde_json::json!({"job_id": "abc"}),
            timestamp: SystemTime::now(),
        };

        let result = broadcaster.broadcast(&msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscribe_to_channel() {
        let config = BroadcastConfig {
            channels: vec!["events".to_string()],
            message_retention: Duration::from_secs(60),
        };
        let connection = Arc::new(make_mock_connection());
        let broadcaster = SongbirdBroadcaster::new(config, connection).await.unwrap();

        let _rx = broadcaster.subscribe_to_channel("events").await.unwrap();
    }

    #[tokio::test]
    async fn test_subscribe_to_dynamic_channel() {
        let config = BroadcastConfig {
            channels: vec![],
            message_retention: Duration::from_secs(60),
        };
        let connection = Arc::new(make_mock_connection());
        let broadcaster = SongbirdBroadcaster::new(config, connection).await.unwrap();

        let _rx = broadcaster
            .subscribe_to_channel("dynamic-channel")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_unsubscribe_from_channel() {
        let config = BroadcastConfig {
            channels: vec!["ch1".to_string()],
            message_retention: Duration::from_secs(60),
        };
        let connection = Arc::new(make_mock_connection());
        let broadcaster = SongbirdBroadcaster::new(config, connection).await.unwrap();

        let _ = broadcaster.subscribe_to_channel("ch1").await.unwrap();
        let result = broadcaster.unsubscribe_from_channel("ch1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_targeted_message() {
        let config = BroadcastConfig {
            channels: vec!["events".to_string()],
            message_retention: Duration::from_secs(60),
        };
        let connection = Arc::new(make_mock_connection());
        let broadcaster = SongbirdBroadcaster::new(config, connection).await.unwrap();

        let msg = SongbirdBroadcastMessage::HealthUpdate {
            node_id: "n1".to_string(),
            health_status: "ok".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = broadcaster
            .send_targeted_message(&["node1".to_string(), "node2".to_string()], &msg)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_broadcast_unregistered_channel() {
        let config = BroadcastConfig {
            channels: vec!["registered".to_string()],
            message_retention: Duration::from_secs(60),
        };
        let connection = Arc::new(make_mock_connection());
        let broadcaster = SongbirdBroadcaster::new(config, connection).await.unwrap();

        let msg = SongbirdBroadcastMessage::CustomMessage {
            message_type: "unregistered-type".to_string(),
            payload: serde_json::json!({}),
            timestamp: SystemTime::now(),
        };

        let result = broadcaster.broadcast(&msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_broadcast_with_subscriber() {
        let config = BroadcastConfig {
            channels: vec!["events".to_string()],
            message_retention: Duration::from_secs(60),
        };
        let connection = Arc::new(make_mock_connection());
        let broadcaster = SongbirdBroadcaster::new(config, connection).await.unwrap();

        let mut rx = broadcaster
            .subscribe_to_channel("health-updates")
            .await
            .unwrap();
        let msg = SongbirdBroadcastMessage::HealthUpdate {
            node_id: "n1".to_string(),
            health_status: "ok".to_string(),
            timestamp: SystemTime::now(),
        };

        broadcaster.broadcast(&msg).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.channel_name(), "health-updates");
    }
}
