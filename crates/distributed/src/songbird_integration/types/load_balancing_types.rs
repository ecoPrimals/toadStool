// SPDX-License-Identifier: AGPL-3.0-or-later
//! Load balancing types

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use tokio::sync::mpsc;

use super::node::NodeId;

// ============================================================================
// Load Balancing Types
// ============================================================================

/// Named strategy handle — the string identifies which algorithm to apply
/// (e.g. "round-robin", "least-loaded", "capability-aware").
pub type LoadBalancingStrategy = String;

/// Per-node load tracking.
///
/// Records the most-recently observed load fraction (0.0–1.0) and when it
/// was updated. Used by `SongbirdLoadBalancer::request_advice` to select
/// the least-loaded eligible node.
pub struct NodeCapacityTracker {
    /// `node_id → (load_fraction, updated_at)`
    inner: Mutex<HashMap<NodeId, (f64, Instant)>>,
}

impl NodeCapacityTracker {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Record a load observation for the given node (0.0 = idle, 1.0 = saturated).
    pub fn update(&self, node_id: &NodeId, load: f64) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(node_id.clone(), (load.clamp(0.0, 1.0), Instant::now()));
        }
    }

    /// Return the node with the lowest tracked load, or `None` if no data.
    pub fn least_loaded(&self) -> Option<NodeId> {
        self.inner.lock().ok().and_then(|guard| {
            guard
                .iter()
                .min_by(|a, b| {
                    a.1 .0
                        .partial_cmp(&b.1 .0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(id, _)| id.clone())
        })
    }

    /// Current load snapshot: `node_id → load_fraction`.
    pub fn snapshot(&self) -> HashMap<NodeId, f64> {
        self.inner
            .lock()
            .ok()
            .map(|g| g.iter().map(|(k, (v, _))| (k.clone(), *v)).collect())
            .unwrap_or_default()
    }
}

impl Default for NodeCapacityTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Lightweight in-process performance counters for a Songbird connection.
///
/// Tracks request count, error count, and cumulative latency so callers
/// can compute p50/p95 approximations or derive error rates.
pub struct PerformanceMetrics {
    inner: Mutex<PerformanceCounters>,
}

#[derive(Default)]
struct PerformanceCounters {
    requests: u64,
    errors: u64,
    total_latency_ms: u64,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PerformanceCounters::default()),
        }
    }

    /// Record a completed request with its latency and whether it failed.
    pub fn record(&self, latency_ms: u64, is_error: bool) {
        if let Ok(mut g) = self.inner.lock() {
            g.requests += 1;
            g.total_latency_ms += latency_ms;
            if is_error {
                g.errors += 1;
            }
        }
    }

    /// Error rate in [0.0, 1.0]; 0.0 when no requests recorded.
    pub fn error_rate(&self) -> f64 {
        self.inner
            .lock()
            .ok()
            .map(|g| {
                if g.requests == 0 {
                    0.0
                } else {
                    g.errors as f64 / g.requests as f64
                }
            })
            .unwrap_or(0.0)
    }

    /// Mean latency in milliseconds; 0.0 when no requests recorded.
    pub fn mean_latency_ms(&self) -> f64 {
        self.inner
            .lock()
            .ok()
            .map(|g| {
                if g.requests == 0 {
                    0.0
                } else {
                    g.total_latency_ms as f64 / g.requests as f64
                }
            })
            .unwrap_or(0.0)
    }

    pub fn request_count(&self) -> u64 {
        self.inner.lock().ok().map(|g| g.requests).unwrap_or(0)
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Feedback messages sent back to Songbird about local node state.
#[derive(Debug, Clone)]
pub enum SongbirdFeedback {
    LoadUpdate { node_id: NodeId, load: f64 },
    ErrorReport { node_id: NodeId, error: String },
    CapacityAvailable { node_id: NodeId },
}

/// Sends feedback events to Songbird's coordination loop.
///
/// Backed by an unbounded mpsc channel. Callers `send()` feedback; a
/// background task (or the Songbird client) drains `SongbirdFeedbackReceiver`.
pub struct SongbirdFeedbackSender {
    tx: mpsc::UnboundedSender<SongbirdFeedback>,
}

pub struct SongbirdFeedbackReceiver {
    pub rx: mpsc::UnboundedReceiver<SongbirdFeedback>,
}

impl SongbirdFeedbackSender {
    pub fn new() -> (Self, SongbirdFeedbackReceiver) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, SongbirdFeedbackReceiver { rx })
    }

    /// Emit a feedback event. Returns `false` if the receiver has been dropped.
    pub fn send(&self, feedback: SongbirdFeedback) -> bool {
        self.tx.send(feedback).is_ok()
    }
}

impl Default for SongbirdFeedbackSender {
    fn default() -> Self {
        Self::new().0
    }
}
