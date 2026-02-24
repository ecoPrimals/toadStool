//! Broadcasting types

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use tokio::sync::broadcast;

use super::message_types::SongbirdBroadcastMessage;

// ============================================================================
// Broadcasting Types
// ============================================================================

/// A named pub/sub broadcast channel backed by `tokio::sync::broadcast`.
///
/// `BroadcastChannel::subscribe()` returns a `broadcast::Receiver` that
/// receives all future messages sent on this channel.
pub struct BroadcastChannel {
    name: String,
    tx: broadcast::Sender<SongbirdBroadcastMessage>,
}

impl BroadcastChannel {
    pub fn new(name: impl Into<String>) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            name: name.into(),
            tx,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Publish a message to all current subscribers.
    ///
    /// Returns `Ok(n)` where `n` is the number of active receivers.
    /// Returns `Err(broadcast::error::SendError)` only when there are no
    /// subscribers (harmless — callers can ignore or log).
    pub fn publish(
        &self,
        msg: SongbirdBroadcastMessage,
    ) -> Result<usize, broadcast::error::SendError<SongbirdBroadcastMessage>> {
        self.tx.send(msg)
    }

    /// Subscribe to this channel.
    pub fn subscribe(&self) -> broadcast::Receiver<SongbirdBroadcastMessage> {
        self.tx.subscribe()
    }
}

/// Registry of known message type names for routing / validation.
///
/// Prevents typos in channel names and provides a single source of truth
/// for which message types are in use.
pub struct MessageTypeRegistry {
    types: Mutex<HashSet<String>>,
}

impl MessageTypeRegistry {
    pub fn new() -> Self {
        Self {
            types: Mutex::new(HashSet::new()),
        }
    }

    /// Register a message type; idempotent.
    pub fn register(&self, type_name: impl Into<String>) {
        if let Ok(mut g) = self.types.lock() {
            g.insert(type_name.into());
        }
    }

    /// `true` if the type was previously registered.
    pub fn is_known(&self, type_name: &str) -> bool {
        self.types
            .lock()
            .ok()
            .is_some_and(|g| g.contains(type_name))
    }

    pub fn known_types(&self) -> Vec<String> {
        self.types
            .lock()
            .ok()
            .map(|g| g.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for MessageTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Maps channel names to their broadcast channels.
///
/// Callers call `get_or_create(name)` to obtain a subscriber handle.
/// The channel is created on first access; subsequent calls return a
/// new `broadcast::Receiver` from the same sender.
pub struct SubscriptionManager {
    channels: Mutex<HashMap<String, broadcast::Sender<SongbirdBroadcastMessage>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
        }
    }

    /// Subscribe to a named channel, creating it if it does not yet exist.
    pub fn subscribe(&self, channel: &str) -> broadcast::Receiver<SongbirdBroadcastMessage> {
        let mut guard = self
            .channels
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard
            .entry(channel.to_string())
            .or_insert_with(|| broadcast::channel(64).0)
            .subscribe()
    }

    /// Publish a message to a named channel.
    ///
    /// Returns `0` if the channel does not exist or has no subscribers.
    pub fn publish(&self, channel: &str, msg: SongbirdBroadcastMessage) -> usize {
        self.channels
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(channel)
            .and_then(|tx| tx.send(msg).ok())
            .unwrap_or(0)
    }

    /// Unsubscribe by dropping all senders for a channel (channel closes).
    pub fn close_channel(&self, channel: &str) {
        self.channels
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(channel);
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}
