// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::sync::Arc;

use super::types::{GateGpuInfo, RoutingDecision, RoutingReason};

/// Cross-gate job router
///
/// Selects the best gate for a compute job based on GPU capabilities,
/// model locality, and queue depth.
#[derive(Debug, Clone)]
pub struct JobRouter {
    /// Known gates and their capabilities
    gates: HashMap<Arc<str>, GateGpuInfo>,
    /// Local gate ID (`Arc<str>` avoids allocation on route decisions)
    local_gate_id: Arc<str>,
}

impl JobRouter {
    /// Create a new job router for the given local gate
    #[must_use]
    pub fn new(local_gate_id: impl AsRef<str>) -> Self {
        Self {
            gates: HashMap::new(),
            local_gate_id: Arc::from(local_gate_id.as_ref()),
        }
    }

    /// Update gate capabilities (called when Plasmodium reports new state)
    pub fn update_gate(&mut self, info: GateGpuInfo) {
        self.gates.insert(Arc::clone(&info.gate_id), info);
    }

    /// Remove a gate (went offline)
    pub fn remove_gate(&mut self, gate_id: &str) {
        self.gates.remove(gate_id);
    }

    /// Route a job to the best available gate
    ///
    /// Selection priority:
    /// 1. Gate with model already loaded (avoids VRAM load time)
    /// 2. Gate with most available VRAM (can load model fastest)
    /// 3. Gate with shortest queue (lowest wait time)
    /// 4. Local gate (no mesh hop latency)
    #[must_use]
    pub fn route(&self, model: &str, vram_required_mb: u64) -> RoutingDecision {
        let reachable: Vec<&GateGpuInfo> = self.gates.values().filter(|g| g.reachable).collect();

        if reachable.is_empty() {
            return RoutingDecision {
                gate_id: Arc::clone(&self.local_gate_id),
                reason: RoutingReason::OnlyOption,
                estimated_wait_ms: 0,
            };
        }

        // 1. Check if model is already loaded somewhere
        if let Some(gate) = reachable
            .iter()
            .find(|g| g.loaded_models.iter().any(|m| m == model))
        {
            return RoutingDecision {
                gate_id: Arc::clone(&gate.gate_id),
                reason: RoutingReason::ModelLoaded,
                #[allow(clippy::cast_possible_truncation)]
                estimated_wait_ms: gate.queue_depth as u64 * 100, // rough estimate
            };
        }

        // 2. Gate with enough VRAM and most available
        let mut candidates: Vec<&&GateGpuInfo> = reachable
            .iter()
            .filter(|g| g.vram_available_mb >= vram_required_mb)
            .collect();

        if !candidates.is_empty() {
            candidates.sort_by(|a, b| b.vram_available_mb.cmp(&a.vram_available_mb));
            let best = candidates[0];
            return RoutingDecision {
                gate_id: Arc::clone(&best.gate_id),
                reason: RoutingReason::MostVramAvailable,
                #[allow(clippy::cast_possible_truncation)]
                estimated_wait_ms: best.queue_depth as u64 * 100,
            };
        }

        // 3. Shortest queue regardless of VRAM
        if let Some(gate) = reachable.iter().min_by_key(|g| g.queue_depth) {
            return RoutingDecision {
                gate_id: Arc::clone(&gate.gate_id),
                reason: RoutingReason::ShortestQueue,
                #[allow(clippy::cast_possible_truncation)]
                estimated_wait_ms: gate.queue_depth as u64 * 100,
            };
        }

        // 4. Fallback to local
        RoutingDecision {
            gate_id: Arc::clone(&self.local_gate_id),
            reason: RoutingReason::Local,
            estimated_wait_ms: 0,
        }
    }

    /// Get all known gates
    #[must_use]
    pub fn gates(&self) -> &HashMap<Arc<str>, GateGpuInfo> {
        &self.gates
    }

    /// Whether the given gate_id is a remote gate (not the local one).
    #[must_use]
    pub fn is_remote_gate(&self, gate_id: &str) -> bool {
        gate_id != self.local_gate_id.as_ref()
    }

    /// Get the endpoint for a remote gate.
    #[must_use]
    pub fn gate_endpoint(&self, gate_id: &str) -> Option<String> {
        self.gates.get(gate_id).and_then(|g| g.endpoint.clone())
    }
}
