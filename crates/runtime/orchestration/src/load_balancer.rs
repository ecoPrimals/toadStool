//! Load balancer — dynamic workload distribution across compute substrates.
//!
//! Implements three strategies:
//! - **Equal**: round-robin across all registered substrates
//! - **Weighted**: proportional to declared capacity
//! - **Dynamic**: least-loaded substrate based on sampled utilisation

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A compute substrate that can receive workloads.
#[derive(Debug, Clone)]
pub struct Substrate {
    /// Unique substrate identifier.
    pub id: String,
    /// Declared capacity weight (relative, arbitrary units).
    pub capacity_weight: u32,
    /// Current utilisation [0.0, 1.0]. Updated externally by the orchestrator.
    pub utilisation: f32,
}

impl Substrate {
    /// Create a new substrate with default weight and zero utilisation.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            capacity_weight: 1,
            utilisation: 0.0,
        }
    }

    /// Create a substrate with a specific capacity weight.
    #[must_use]
    pub fn with_weight(id: impl Into<String>, weight: u32) -> Self {
        Self {
            capacity_weight: weight,
            ..Self::new(id)
        }
    }
}

/// Load balancer for distributing work across substrates.
#[derive(Debug)]
pub struct LoadBalancer {
    strategy: BalancingStrategy,
    substrates: Vec<Substrate>,
    /// Round-robin cursor (only used for `Equal` strategy).
    rr_cursor: Arc<AtomicUsize>,
}

impl LoadBalancer {
    /// Create a new load balancer with the default `Weighted` strategy.
    #[must_use]
    pub fn new() -> Self {
        Self::with_strategy(BalancingStrategy::default())
    }

    /// Create a new load balancer with an explicit strategy.
    #[must_use]
    pub fn with_strategy(strategy: BalancingStrategy) -> Self {
        Self {
            strategy,
            substrates: Vec::new(),
            rr_cursor: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Register a substrate that can receive workloads.
    pub fn register(&mut self, substrate: Substrate) {
        self.substrates.push(substrate);
    }

    /// Deregister a substrate by ID.
    pub fn deregister(&mut self, id: &str) {
        self.substrates.retain(|s| s.id != id);
    }

    /// Update the utilisation of a registered substrate.
    ///
    /// Utilisation should be in `[0.0, 1.0]`.
    pub fn update_utilisation(&mut self, id: &str, utilisation: f32) {
        if let Some(s) = self.substrates.iter_mut().find(|s| s.id == id) {
            s.utilisation = utilisation.clamp(0.0, 1.0);
        }
    }

    /// Select the best substrate for the next workload.
    ///
    /// Returns `None` if no substrates are registered.
    #[must_use]
    pub fn select(&self) -> Option<&Substrate> {
        if self.substrates.is_empty() {
            return None;
        }
        match self.strategy {
            BalancingStrategy::Equal => self.select_round_robin(),
            BalancingStrategy::Weighted => self.select_weighted(),
            BalancingStrategy::Dynamic => self.select_least_loaded(),
        }
    }

    fn select_round_robin(&self) -> Option<&Substrate> {
        let n = self.substrates.len();
        let idx = self.rr_cursor.fetch_add(1, Ordering::Relaxed) % n;
        self.substrates.get(idx)
    }

    fn select_weighted(&self) -> Option<&Substrate> {
        // Weighted round-robin using capacity_weight.
        // Pick the substrate with the highest (weight / utilisation_adjusted) score.
        self.substrates.iter().max_by(|a, b| {
            let score_a = a.capacity_weight as f32 * (1.0 - a.utilisation);
            let score_b = b.capacity_weight as f32 * (1.0 - b.utilisation);
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    fn select_least_loaded(&self) -> Option<&Substrate> {
        self.substrates.iter().min_by(|a, b| {
            a.utilisation
                .partial_cmp(&b.utilisation)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Return the current strategy.
    #[must_use]
    pub fn strategy(&self) -> BalancingStrategy {
        self.strategy
    }

    /// Return the number of registered substrates.
    #[must_use]
    pub fn substrate_count(&self) -> usize {
        self.substrates.len()
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new()
    }
}

/// Load balancing strategies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BalancingStrategy {
    /// Round-robin across all substrates (equal share).
    Equal,

    /// Weighted by substrate capacity, adjusted for current utilisation.
    #[default]
    Weighted,

    /// Always pick the substrate with the lowest current utilisation.
    Dynamic,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_balancer_creation() {
        let balancer = LoadBalancer::new();
        assert_eq!(balancer.strategy(), BalancingStrategy::Weighted);
        assert_eq!(balancer.substrate_count(), 0);
    }

    #[test]
    fn test_select_empty_returns_none() {
        let lb = LoadBalancer::new();
        assert!(lb.select().is_none());
    }

    #[test]
    fn test_round_robin_selection() {
        let mut lb = LoadBalancer::with_strategy(BalancingStrategy::Equal);
        lb.register(Substrate::new("a"));
        lb.register(Substrate::new("b"));
        lb.register(Substrate::new("c"));
        let ids: Vec<&str> = (0..6).map(|_| lb.select().unwrap().id.as_str()).collect();
        // Should cycle through a, b, c
        assert_eq!(&ids[0..3], &["a", "b", "c"]);
        assert_eq!(&ids[3..6], &["a", "b", "c"]);
    }

    #[test]
    fn test_dynamic_selects_least_loaded() {
        let mut lb = LoadBalancer::with_strategy(BalancingStrategy::Dynamic);
        lb.register(Substrate::with_weight("high-load", 1));
        lb.register(Substrate::with_weight("low-load", 1));
        lb.update_utilisation("high-load", 0.9);
        lb.update_utilisation("low-load", 0.1);
        assert_eq!(lb.select().unwrap().id, "low-load");
    }

    #[test]
    fn test_weighted_prefers_high_capacity() {
        let mut lb = LoadBalancer::with_strategy(BalancingStrategy::Weighted);
        lb.register(Substrate::with_weight("heavy", 10));
        lb.register(Substrate::with_weight("light", 1));
        assert_eq!(lb.select().unwrap().id, "heavy");
    }

    #[test]
    fn test_deregister() {
        let mut lb = LoadBalancer::new();
        lb.register(Substrate::new("a"));
        lb.register(Substrate::new("b"));
        lb.deregister("a");
        assert_eq!(lb.substrate_count(), 1);
        assert_eq!(lb.select().unwrap().id, "b");
    }
}
