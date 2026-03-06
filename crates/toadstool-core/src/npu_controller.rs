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
}
