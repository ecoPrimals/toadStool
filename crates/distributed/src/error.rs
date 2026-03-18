// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error types for the distributed crate

use thiserror::Error;

/// Errors from the distributed computing layer.
#[derive(Error, Debug)]
pub enum DistributedError {
    /// TOADSTOOL_ENDPOINT env var not set; required for discovery.
    #[error("TOADSTOOL_ENDPOINT not set - primal must know its own endpoint for discovery")]
    ToadstoolEndpointNotSet,

    /// Songbird service registration failed with the given reason.
    #[error("Songbird registration failed: {0}")]
    SongbirdRegistration(String),

    /// Workload cannot be converted to UniversalJob without scheduler.
    #[error("Workload conversion to UniversalJob requires scheduler integration")]
    WorkloadConversionRequiresScheduler,

    /// JSON or other serialization failed.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toadstool_endpoint_not_set_display() {
        let err = DistributedError::ToadstoolEndpointNotSet;
        assert!(err.to_string().contains("TOADSTOOL_ENDPOINT"));
    }

    #[test]
    fn songbird_registration_display() {
        let err = DistributedError::SongbirdRegistration("connection refused".to_string());
        assert!(err.to_string().contains("Songbird registration failed"));
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn workload_conversion_display() {
        let err = DistributedError::WorkloadConversionRequiresScheduler;
        assert!(err.to_string().contains("scheduler"));
    }

    #[test]
    fn serialization_error_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let err: DistributedError = json_err.into();
        assert!(err.to_string().contains("Serialization"));
    }
}
