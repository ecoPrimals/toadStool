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
        let channel_subscribers = if let Some(ch) = self.channels.get(channel_name) {
            ch.publish(message.clone()).unwrap_or(0)
        } else {
            0
        };

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
