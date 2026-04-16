// SPDX-License-Identifier: AGPL-3.0-or-later

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
                estimated_wait_ms: best.queue_depth as u64 * 100,
            };
        }

        // 3. Shortest queue regardless of VRAM
        if let Some(gate) = reachable.iter().min_by_key(|g| g.queue_depth) {
            return RoutingDecision {
                gate_id: Arc::clone(&gate.gate_id),
                reason: RoutingReason::ShortestQueue,
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

#[cfg(test)]
mod tests {
    use super::JobRouter;
    use crate::cross_gate::{GateGpuInfo, RoutingReason};
    use std::sync::Arc;

    fn gate(
        id: &str,
        vram_available_mb: u64,
        loaded_models: &[&str],
        queue_depth: usize,
        reachable: bool,
        endpoint: Option<&str>,
    ) -> GateGpuInfo {
        GateGpuInfo {
            gate_id: Arc::from(id),
            gpu_model: "Test GPU".into(),
            vram_total_mb: 8192,
            vram_available_mb,
            loaded_models: loaded_models.iter().map(|s| (*s).to_string()).collect(),
            queue_depth,
            reachable,
            endpoint: endpoint.map(String::from),
        }
    }

    #[test]
    fn route_with_no_reachable_gates_falls_back_to_local_only_option() {
        let router = JobRouter::new("local");
        let d = router.route("llama-7b", 4096);
        assert_eq!(d.gate_id.as_ref(), "local");
        assert!(matches!(d.reason, RoutingReason::OnlyOption));
        assert_eq!(d.estimated_wait_ms, 0);
    }

    #[test]
    fn route_prefers_gate_with_model_already_loaded() {
        let mut router = JobRouter::new("local");
        router.update_gate(gate(
            "remote-a",
            2048,
            &["llama-7b"],
            2,
            true,
            Some("/tmp/a.sock"),
        ));
        router.update_gate(gate("remote-b", 8192, &[], 0, true, Some("/tmp/b.sock")));

        let d = router.route("llama-7b", 4096);
        assert_eq!(d.gate_id.as_ref(), "remote-a");
        assert!(matches!(d.reason, RoutingReason::ModelLoaded));
        assert_eq!(d.estimated_wait_ms, 200);
    }

    #[test]
    fn route_picks_highest_vram_when_model_not_loaded_and_requirement_met() {
        let mut router = JobRouter::new("local");
        router.update_gate(gate("g-low", 4096, &[], 1, true, None));
        router.update_gate(gate("g-high", 16384, &[], 3, true, None));

        let d = router.route("new-model", 2048);
        assert_eq!(d.gate_id.as_ref(), "g-high");
        assert!(matches!(d.reason, RoutingReason::MostVramAvailable));
        assert_eq!(d.estimated_wait_ms, 300);
    }

    #[test]
    fn route_shortest_queue_when_no_gate_has_sufficient_vram() {
        let mut router = JobRouter::new("local");
        router.update_gate(gate("busy", 512, &[], 10, true, None));
        router.update_gate(gate("idle", 256, &[], 1, true, None));

        let d = router.route("huge-model", 100_000);
        assert_eq!(d.gate_id.as_ref(), "idle");
        assert!(matches!(d.reason, RoutingReason::ShortestQueue));
        assert_eq!(d.estimated_wait_ms, 100);
    }

    #[test]
    fn unreachable_gates_are_ignored_for_routing() {
        let mut router = JobRouter::new("local");
        router.update_gate(gate("away", 32768, &["m"], 0, false, None));

        let d = router.route("m", 100);
        assert!(matches!(d.reason, RoutingReason::OnlyOption));
        assert_eq!(d.gate_id.as_ref(), "local");
    }

    #[test]
    fn remove_gate_drops_route_target() {
        let mut router = JobRouter::new("local");
        router.update_gate(gate("gone", 8192, &[], 0, true, None));
        router.remove_gate("gone");

        assert!(router.gates().is_empty());
        let d = router.route("x", 1);
        assert!(matches!(d.reason, RoutingReason::OnlyOption));
    }

    #[test]
    fn is_remote_gate_compares_to_local_id() {
        let router = JobRouter::new("tower");
        assert!(!router.is_remote_gate("tower"));
        assert!(router.is_remote_gate("edge-2"));
    }

    #[test]
    fn gate_endpoint_unknown_gate_returns_none() {
        let router = JobRouter::new("local");
        assert!(router.gate_endpoint("nope").is_none());
    }

    #[test]
    fn gate_endpoint_returns_cloned_endpoint_for_known_gate() {
        let mut router = JobRouter::new("local");
        router.update_gate(gate("edge", 1024, &[], 0, true, Some("/run/edge.sock")));

        assert_eq!(
            router.gate_endpoint("edge").as_deref(),
            Some("/run/edge.sock")
        );
    }
}
