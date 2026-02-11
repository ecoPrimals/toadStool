//! Evaluation recording for optimization and surrogate training
//!
//! Captures every function evaluation during optimization, not just the best.
//! This is critical for surrogate learning: RBF models are trained on ALL
//! accumulated evaluations, providing both good regions (from optimizer
//! convergence) and exploratory regions (from initial sampling).
//!
//! # Key Insight
//!
//! The SparsitySampler algorithm from Diaw et al. (2024) trains surrogates
//! on ALL evaluations, not just the best. This provides the surrogate with
//! both exploitation data (near optima) and exploration data (away from
//! optima), leading to dramatically better surrogate accuracy.
//!
//! # Cross-Domain Applications
//!
//! - **Surrogate learning**: Train RBF on all evaluations
//! - **Bayesian optimization**: Build GP model from evaluation history
//! - **Sensitivity analysis**: Analyze function behavior across parameter space
//! - **Debugging**: Inspect optimization trajectories

/// A single function evaluation record: input point and output value.
#[derive(Debug, Clone)]
pub struct EvaluationRecord {
    /// Input point (parameter vector)
    pub x: Vec<f64>,
    /// Function value at this point
    pub f: f64,
}

/// Accumulates all function evaluations during optimization.
///
/// Wraps an objective function to transparently capture every evaluation.
/// The cache provides training data for surrogate models.
///
/// # Examples
///
/// ```
/// use barracuda::optimize::EvaluationCache;
///
/// let mut cache = EvaluationCache::new();
///
/// // Evaluate and record
/// let f = |x: &[f64]| x[0].powi(2) + x[1].powi(2);
/// let x = vec![1.0, 2.0];
/// let val = f(&x);
/// cache.record(x, val);
///
/// assert_eq!(cache.len(), 1);
/// assert_eq!(cache.best_f(), Some(5.0));
/// ```
#[derive(Debug, Clone)]
pub struct EvaluationCache {
    /// All recorded evaluations in chronological order
    records: Vec<EvaluationRecord>,
    /// Best function value seen so far
    best_idx: Option<usize>,
}

impl EvaluationCache {
    /// Create a new empty evaluation cache.
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            best_idx: None,
        }
    }

    /// Create a cache with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            records: Vec::with_capacity(capacity),
            best_idx: None,
        }
    }

    /// Record a function evaluation.
    pub fn record(&mut self, x: Vec<f64>, f: f64) {
        let idx = self.records.len();
        self.records.push(EvaluationRecord { x, f });

        // Update best
        match self.best_idx {
            None => self.best_idx = Some(idx),
            Some(best) => {
                if f < self.records[best].f {
                    self.best_idx = Some(idx);
                }
            }
        }
    }

    /// Number of recorded evaluations.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Best function value seen, if any.
    pub fn best_f(&self) -> Option<f64> {
        self.best_idx.map(|idx| self.records[idx].f)
    }

    /// Best point found so far, if any.
    pub fn best_x(&self) -> Option<&[f64]> {
        self.best_idx.map(|idx| self.records[idx].x.as_slice())
    }

    /// Best evaluation record, if any.
    pub fn best(&self) -> Option<&EvaluationRecord> {
        self.best_idx.map(|idx| &self.records[idx])
    }

    /// Get all recorded evaluations as a slice.
    pub fn records(&self) -> &[EvaluationRecord] {
        &self.records
    }

    /// Extract training data: (X, y) where X is matrix of inputs, y is vector of outputs.
    ///
    /// Returns `(x_data, y_data)` suitable for feeding into `RBFSurrogate::train()`.
    pub fn training_data(&self) -> (Vec<Vec<f64>>, Vec<f64>) {
        let x_data: Vec<Vec<f64>> = self.records.iter().map(|r| r.x.clone()).collect();
        let y_data: Vec<f64> = self.records.iter().map(|r| r.f).collect();
        (x_data, y_data)
    }

    /// Merge another cache into this one.
    ///
    /// Useful for combining results from multiple parallel solvers.
    pub fn merge(&mut self, other: &EvaluationCache) {
        for record in &other.records {
            self.record(record.x.clone(), record.f);
        }
    }
}

impl Default for EvaluationCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let mut cache = EvaluationCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(cache.best_f().is_none());

        cache.record(vec![1.0, 2.0], 5.0);
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());
        assert_eq!(cache.best_f(), Some(5.0));
    }

    #[test]
    fn test_cache_tracks_best() {
        let mut cache = EvaluationCache::new();

        cache.record(vec![0.0], 10.0);
        cache.record(vec![1.0], 3.0); // best
        cache.record(vec![2.0], 7.0);
        cache.record(vec![3.0], 15.0);

        assert_eq!(cache.best_f(), Some(3.0));
        assert_eq!(cache.best_x(), Some(vec![1.0].as_slice()));
    }

    #[test]
    fn test_cache_training_data() {
        let mut cache = EvaluationCache::new();
        cache.record(vec![1.0, 2.0], 5.0);
        cache.record(vec![3.0, 4.0], 7.0);

        let (x_data, y_data) = cache.training_data();
        assert_eq!(x_data.len(), 2);
        assert_eq!(y_data.len(), 2);
        assert_eq!(x_data[0], vec![1.0, 2.0]);
        assert_eq!(y_data[1], 7.0);
    }

    #[test]
    fn test_cache_merge() {
        let mut a = EvaluationCache::new();
        a.record(vec![0.0], 10.0);
        a.record(vec![1.0], 5.0);

        let mut b = EvaluationCache::new();
        b.record(vec![2.0], 3.0); // best overall
        b.record(vec![3.0], 8.0);

        a.merge(&b);
        assert_eq!(a.len(), 4);
        assert_eq!(a.best_f(), Some(3.0));
    }

    #[test]
    fn test_cache_with_capacity() {
        let cache = EvaluationCache::with_capacity(1000);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_eval_record_clone() {
        let record = EvaluationRecord {
            x: vec![1.0, 2.0, 3.0],
            f: 42.0,
        };
        let cloned = record.clone();
        assert_eq!(cloned.x, vec![1.0, 2.0, 3.0]);
        assert_eq!(cloned.f, 42.0);
    }
}
