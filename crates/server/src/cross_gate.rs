//! Cross-Gate Compute Delegation
//!
//! Routes compute jobs to the best available GPU across the mesh.
//!
//! ## Architecture
//!
//! - Plasmodium knows all gates and their GPU capabilities
//! - Job router selects gate by: VRAM available, model already loaded, queue depth
//! - Jobs forwarded via Songbird mesh TCP relay
//! - Results returned through the mesh
//!
//! ## Status: PENDING
//!
//! Requires Songbird mesh relay to be active. This module defines the types
//! and routing interface. Implementation will be activated when mesh relay
//! is available.
//!
//! ## Example
//!
//! Gate2 (RTX 3090, 24GB) is better for large models. Tower (RTX 4070)
//! is better for quick inference. The router picks the right gate automatically.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// GPU capabilities for a single gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateGpuInfo {
    /// Gate identifier (e.g., "tower", "gate2")
    pub gate_id: String,
    /// GPU model name (e.g., "RTX 4070", "RTX 3090")
    pub gpu_model: String,
    /// Total VRAM in MB
    pub vram_total_mb: u64,
    /// Available VRAM in MB
    pub vram_available_mb: u64,
    /// Models currently loaded in VRAM
    pub loaded_models: Vec<String>,
    /// Current queue depth (pending jobs)
    pub queue_depth: usize,
    /// Whether this gate is reachable via mesh
    pub reachable: bool,
}

/// Routing decision for a compute job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Selected gate ID
    pub gate_id: String,
    /// Reason for selection
    pub reason: RoutingReason,
    /// Estimated wait time in milliseconds
    pub estimated_wait_ms: u64,
}

/// Why a particular gate was selected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingReason {
    /// Model already loaded in VRAM (fastest)
    ModelLoaded,
    /// Most VRAM available for new model loading
    MostVramAvailable,
    /// Shortest queue (lowest wait time)
    ShortestQueue,
    /// Only gate available
    OnlyOption,
    /// Local execution (no mesh hop needed)
    Local,
}

/// Cross-gate job router
///
/// Selects the best gate for a compute job based on GPU capabilities,
/// model locality, and queue depth.
#[derive(Debug, Clone)]
pub struct JobRouter {
    /// Known gates and their capabilities
    gates: HashMap<String, GateGpuInfo>,
    /// Local gate ID
    local_gate_id: String,
}

impl JobRouter {
    /// Create a new job router for the given local gate
    #[must_use]
    pub fn new(local_gate_id: impl Into<String>) -> Self {
        Self {
            gates: HashMap::new(),
            local_gate_id: local_gate_id.into(),
        }
    }

    /// Update gate capabilities (called when Plasmodium reports new state)
    pub fn update_gate(&mut self, info: GateGpuInfo) {
        self.gates.insert(info.gate_id.clone(), info);
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
                gate_id: self.local_gate_id.clone(),
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
                gate_id: gate.gate_id.clone(),
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
                gate_id: best.gate_id.clone(),
                reason: RoutingReason::MostVramAvailable,
                estimated_wait_ms: best.queue_depth as u64 * 100,
            };
        }

        // 3. Shortest queue regardless of VRAM
        if let Some(gate) = reachable.iter().min_by_key(|g| g.queue_depth) {
            return RoutingDecision {
                gate_id: gate.gate_id.clone(),
                reason: RoutingReason::ShortestQueue,
                estimated_wait_ms: gate.queue_depth as u64 * 100,
            };
        }

        // 4. Fallback to local
        RoutingDecision {
            gate_id: self.local_gate_id.clone(),
            reason: RoutingReason::Local,
            estimated_wait_ms: 0,
        }
    }

    /// Get all known gates
    #[must_use]
    pub fn gates(&self) -> &HashMap<String, GateGpuInfo> {
        &self.gates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tower_gpu() -> GateGpuInfo {
        GateGpuInfo {
            gate_id: "tower".to_string(),
            gpu_model: "RTX 4070".to_string(),
            vram_total_mb: 12288,
            vram_available_mb: 8000,
            loaded_models: vec!["tinyllama:latest".to_string()],
            queue_depth: 2,
            reachable: true,
        }
    }

    fn gate2_gpu() -> GateGpuInfo {
        GateGpuInfo {
            gate_id: "gate2".to_string(),
            gpu_model: "RTX 3090".to_string(),
            vram_total_mb: 24576,
            vram_available_mb: 20000,
            loaded_models: vec!["llama3:70b".to_string()],
            queue_depth: 0,
            reachable: true,
        }
    }

    #[test]
    fn test_route_model_already_loaded() {
        let mut router = JobRouter::new("tower".to_string());
        router.update_gate(tower_gpu());
        router.update_gate(gate2_gpu());

        // tinyllama is loaded on tower
        let decision = router.route("tinyllama:latest", 2000);
        assert_eq!(decision.gate_id, "tower");
        assert!(matches!(decision.reason, RoutingReason::ModelLoaded));
    }

    #[test]
    fn test_route_large_model_to_big_gpu() {
        let mut router = JobRouter::new("tower".to_string());
        router.update_gate(tower_gpu());
        router.update_gate(gate2_gpu());

        // New model needing 16GB VRAM -> gate2 has 20GB available
        let decision = router.route("mixtral:8x7b", 16000);
        assert_eq!(decision.gate_id, "gate2");
        assert!(matches!(decision.reason, RoutingReason::MostVramAvailable));
    }

    #[test]
    fn test_route_no_gates_falls_back_local() {
        let router = JobRouter::new("tower".to_string());
        let decision = router.route("any_model", 4000);
        assert_eq!(decision.gate_id, "tower");
        assert!(matches!(decision.reason, RoutingReason::OnlyOption));
    }

    #[test]
    fn test_route_shortest_queue() {
        let mut router = JobRouter::new("tower".to_string());

        let mut tower = tower_gpu();
        tower.loaded_models.clear();
        tower.vram_available_mb = 1000; // Not enough
        tower.queue_depth = 5;

        let mut gate2 = gate2_gpu();
        gate2.loaded_models.clear();
        gate2.vram_available_mb = 1000; // Not enough
        gate2.queue_depth = 1;

        router.update_gate(tower);
        router.update_gate(gate2);

        // Neither has enough VRAM, pick shortest queue
        let decision = router.route("huge_model", 30000);
        assert_eq!(decision.gate_id, "gate2");
        assert!(matches!(decision.reason, RoutingReason::ShortestQueue));
    }

    #[test]
    fn test_update_and_remove_gate() {
        let mut router = JobRouter::new("tower".to_string());
        router.update_gate(tower_gpu());
        assert_eq!(router.gates().len(), 1);

        router.update_gate(gate2_gpu());
        assert_eq!(router.gates().len(), 2);

        router.remove_gate("gate2");
        assert_eq!(router.gates().len(), 1);
    }

    #[test]
    fn test_unreachable_gate_skipped() {
        let mut router = JobRouter::new("tower".to_string());

        let mut gate2 = gate2_gpu();
        gate2.reachable = false;
        router.update_gate(gate2);

        let tower = tower_gpu();
        router.update_gate(tower);

        // gate2 has more VRAM but is unreachable
        let decision = router.route("new_model", 4000);
        assert_eq!(decision.gate_id, "tower");
    }

    #[test]
    fn test_all_gates_unreachable_falls_back_to_local() {
        let mut router = JobRouter::new("local".to_string());

        let mut gate = tower_gpu();
        gate.reachable = false;
        router.update_gate(gate);

        let decision = router.route("any_model", 1000);
        assert_eq!(decision.gate_id, "local");
        assert!(matches!(decision.reason, RoutingReason::OnlyOption));
    }

    #[test]
    fn test_route_estimated_wait_ms_model_loaded() {
        let mut router = JobRouter::new("tower".to_string());
        let mut tower = tower_gpu();
        tower.queue_depth = 3;
        router.update_gate(tower);

        let decision = router.route("tinyllama:latest", 100);
        // Wait should be queue_depth * 100
        assert_eq!(decision.estimated_wait_ms, 300);
    }

    #[test]
    fn test_route_estimated_wait_ms_most_vram() {
        let mut router = JobRouter::new("tower".to_string());
        let mut gate2 = gate2_gpu();
        gate2.queue_depth = 4;
        gate2.loaded_models.clear();
        router.update_gate(gate2);

        let decision = router.route("new_model", 1000);
        assert!(matches!(decision.reason, RoutingReason::MostVramAvailable));
        assert_eq!(decision.estimated_wait_ms, 400);
    }

    #[test]
    fn test_update_gate_replaces_existing() {
        let mut router = JobRouter::new("tower".to_string());
        router.update_gate(tower_gpu());

        let mut updated = tower_gpu();
        updated.vram_available_mb = 1234;
        router.update_gate(updated);

        // Should still be one gate (updated in-place)
        assert_eq!(router.gates().len(), 1);
        assert_eq!(router.gates()["tower"].vram_available_mb, 1234);
    }

    #[test]
    fn test_remove_nonexistent_gate_is_noop() {
        let mut router = JobRouter::new("tower".to_string());
        router.remove_gate("nonexistent");
        assert_eq!(router.gates().len(), 0);
    }

    #[test]
    fn test_route_prefers_model_loaded_over_more_vram() {
        let mut router = JobRouter::new("tower".to_string());

        // tower has less VRAM available but model is loaded
        router.update_gate(tower_gpu()); // 8000 MB, tinyllama loaded
        router.update_gate(gate2_gpu()); // 20000 MB, no tinyllama

        let decision = router.route("tinyllama:latest", 2000);
        assert_eq!(decision.gate_id, "tower");
        assert!(matches!(decision.reason, RoutingReason::ModelLoaded));
    }
}
