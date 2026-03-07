// SPDX-License-Identifier: AGPL-3.0-or-later
//! Generic NPU Parameter Controller — hotSpring absorption (S94).
//!
//! Provides the trait for NPU-driven autonomous parameter tuning.
//! The pattern: observe metrics → extract features → NPU predicts → safety
//! clamp → apply. Springs implement this for their specific physics.
//!
//! ## Ownership
//!
//! - **toadStool**: Defines the generic trait and safety clamp infrastructure.
//! - **Springs**: Implement the trait for domain-specific tuning (e.g. HMC
//!   step size, CG tolerance, learning rate).
//! - **barraCuda**: Dispatches compute; unaware of the controller.

use std::fmt::Debug;

/// A single proxy measurement from the simulation state.
///
/// NPU controllers observe these to make parameter suggestions.
/// Generalizes hotSpring's [`ProxyFeatures`] pattern for adaptive simulation steering.
#[derive(Debug, Clone)]
pub struct ProxyFeature {
    /// Feature name (e.g. `acceptance_rate`, `residual_norm`, `energy_drift`).
    pub name: &'static str,
    /// Current value.
    pub value: f64,
    /// Optional target value (controller tries to steer toward this).
    pub target: Option<f64>,
    /// Weight for multi-feature observation (1.0 = default).
    pub weight: f64,
}

impl ProxyFeature {
    /// Create a proxy feature with name and value.
    #[must_use]
    pub fn new(name: &'static str, value: f64) -> Self {
        Self {
            name,
            value,
            target: None,
            weight: 1.0,
        }
    }

    /// Set the target value the controller should steer toward.
    #[must_use]
    pub fn with_target(mut self, target: f64) -> Self {
        self.target = Some(target);
        self
    }

    /// Set the weight for multi-feature observation.
    #[must_use]
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Collection of proxy features for a single observation.
pub type ProxyFeatureSet = Vec<ProxyFeature>;

/// A parameter suggestion from the NPU controller.
///
/// Generic over the parameter type `P` so springs can define their own
/// parameter structs (e.g. `HmcParams { dt: f64, n_md: usize }`).
#[derive(Debug, Clone)]
pub struct ParameterSuggestion<P: Debug + Clone> {
    /// The suggested parameters.
    pub params: P,
    /// Confidence in [0.0, 1.0]. Controllers should only apply suggestions
    /// above their trust threshold.
    pub confidence: f64,
    /// Whether the suggestion came from the NPU model or a heuristic fallback.
    pub source: SuggestionSource,
}

/// Where the parameter suggestion originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionSource {
    /// NPU model prediction (ESN, SNN, etc.).
    NpuModel,
    /// Heuristic fallback when the model isn't trusted.
    Heuristic,
    /// Default / initial values.
    Default,
}

/// Safety clamp configuration for parameter suggestions.
///
/// Prevents the NPU from suggesting values outside safe bounds.
#[derive(Debug, Clone)]
pub struct SafetyClamp<P: Debug + Clone> {
    /// Minimum allowed parameter values.
    pub min: P,
    /// Maximum allowed parameter values.
    pub max: P,
}

/// Errors from NPU parameter control operations.
#[derive(Debug, thiserror::Error)]
pub enum ControllerError {
    /// NPU is unavailable (not connected, powered down, etc.).
    #[error("NPU unavailable: {0}")]
    NpuUnavailable(String),

    /// Feature extraction failed.
    #[error("feature extraction failed: {0}")]
    FeatureExtraction(String),

    /// The model hasn't been trained yet.
    #[error("model not trained")]
    ModelNotTrained,

    /// Other controller-specific errors.
    #[error("{0}")]
    Other(String),
}

/// Generic NPU parameter controller trait.
///
/// Implementors define:
/// - What metrics to observe (`Observation`)
/// - What parameters to tune (`Params`)
/// - How to extract features and interpret NPU output
///
/// toadStool provides the infrastructure; springs provide the physics.
pub trait NpuParameterController: Debug + Send {
    /// The observation/metric type fed to the controller.
    type Observation: Debug + Send;
    /// The parameter type the controller tunes.
    type Params: Debug + Clone + Send;

    /// Feed an observation to the controller (e.g. acceptance rate, residual).
    ///
    /// # Errors
    /// Returns error if the NPU is unavailable or feature extraction fails.
    fn observe(&mut self, observation: Self::Observation) -> Result<(), ControllerError>;

    /// Request a parameter suggestion based on accumulated observations.
    ///
    /// Returns `None` if insufficient data or the model isn't ready.
    ///
    /// # Errors
    /// Returns error if the NPU dispatch fails.
    fn suggest(&self) -> Result<Option<ParameterSuggestion<Self::Params>>, ControllerError>;

    /// Get the safety clamp bounds.
    fn safety_clamp(&self) -> &SafetyClamp<Self::Params>;

    /// Minimum confidence threshold for applying suggestions.
    /// Default: 0.3 (from hotSpring's `HEAD_TRUST_THRESHOLD`).
    fn trust_threshold(&self) -> f64 {
        0.3
    }

    /// Whether the controller's model is trained and ready.
    fn is_ready(&self) -> bool;

    /// Reset accumulated observations (e.g. at the start of a new run).
    fn reset(&mut self);
}

/// Controller for adaptive simulation parameter tuning.
///
/// Absorbs the hotSpring `npu_worker` pattern as a generic primitive.
/// Combines [`NpuParameterController`] with simulation-specific lifecycle
/// and proxy-feature observation.
pub trait AdaptiveSimulationController: Debug + Send {
    /// The parameter set being tuned (e.g. timestep, `n_md`, temperature).
    type Params: Debug + Clone + Send;

    /// Feed proxy features from the current simulation state.
    ///
    /// # Errors
    ///
    /// Returns [`ControllerError`] if the controller cannot process the features.
    fn observe_features(&mut self, features: &[ProxyFeature]) -> Result<(), ControllerError>;

    /// Get parameter suggestion based on accumulated observations.
    ///
    /// # Errors
    ///
    /// Returns [`ControllerError`] if the controller state is invalid.
    fn suggest_params(&self) -> Result<Option<ParameterSuggestion<Self::Params>>, ControllerError>;

    /// Whether the controller has enough data to make suggestions.
    fn is_warmed_up(&self) -> bool;

    /// Reset state for a new simulation run.
    fn reset(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestParams {
        step_size: f64,
    }

    #[test]
    fn test_suggestion_source() {
        let suggestion = ParameterSuggestion {
            params: TestParams { step_size: 0.01 },
            confidence: 0.8,
            source: SuggestionSource::NpuModel,
        };
        assert_eq!(suggestion.source, SuggestionSource::NpuModel);
        assert!(suggestion.confidence > 0.3);
    }

    #[test]
    fn test_safety_clamp() {
        let clamp = SafetyClamp {
            min: TestParams { step_size: 0.001 },
            max: TestParams { step_size: 0.02 },
        };
        assert!(clamp.min.step_size < clamp.max.step_size);
    }

    #[test]
    fn test_controller_error_display() {
        let err = ControllerError::NpuUnavailable("powered down".into());
        assert!(err.to_string().contains("powered down"));

        let err = ControllerError::ModelNotTrained;
        assert!(err.to_string().contains("not trained"));
    }

    #[test]
    fn test_proxy_feature_construction() {
        let feature = ProxyFeature::new("acceptance_rate", 0.75);
        assert_eq!(feature.name, "acceptance_rate");
        assert!((feature.value - 0.75).abs() < f64::EPSILON);
        assert!(feature.target.is_none());
        assert!((feature.weight - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_proxy_feature_builder_pattern() {
        let feature = ProxyFeature::new("residual_norm", 1e-6)
            .with_target(1e-8)
            .with_weight(2.0);
        assert_eq!(feature.name, "residual_norm");
        assert!((feature.value - 1e-6).abs() < f64::EPSILON);
        assert_eq!(feature.target, Some(1e-8));
        assert!((feature.weight - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_proxy_feature_set_type_alias() {
        let set: ProxyFeatureSet = vec![
            ProxyFeature::new("energy_drift", 0.001),
            ProxyFeature::new("acceptance_rate", 0.8).with_target(0.7),
        ];
        assert_eq!(set.len(), 2);
        assert_eq!(set[0].name, "energy_drift");
        assert_eq!(set[1].name, "acceptance_rate");
    }
}
