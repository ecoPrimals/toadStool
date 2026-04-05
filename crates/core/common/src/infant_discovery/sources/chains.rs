// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! Preset ordered chains of [`EndpointSource`] for production and development.

use std::sync::Arc;

use crate::infant_discovery::capabilities::EndpointSource;

use super::environment::EnvironmentSource;
use super::fallback::FallbackSource;
use super::mdns::MDNSSource;
use super::service_mesh::ServiceMeshSource;

/// Create standard production source chain
#[must_use]
pub fn production_sources() -> Vec<Arc<dyn EndpointSource>> {
    vec![
        Arc::new(EnvironmentSource::default()),
        Arc::new(ServiceMeshSource::new()),
        Arc::new(MDNSSource::new()),
        Arc::new(FallbackSource::new()),
    ]
}

/// Create development source chain (faster fallbacks)
#[must_use]
pub fn development_sources() -> Vec<Arc<dyn EndpointSource>> {
    vec![
        Arc::new(EnvironmentSource::default()),
        Arc::new(FallbackSource::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_source_chain() {
        let sources = production_sources();
        assert_eq!(sources.len(), 4);
        assert_eq!(sources[0].source_name(), "environment");
        assert_eq!(sources[3].source_name(), "fallback");
    }

    #[test]
    fn test_development_sources() {
        let sources = development_sources();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].source_name(), "environment");
        assert_eq!(sources[1].source_name(), "fallback");
    }

    #[test]
    fn test_production_sources_order() {
        let sources = production_sources();
        assert_eq!(sources[0].source_name(), "environment");
        assert_eq!(sources[1].source_name(), "service_mesh");
        assert_eq!(sources[2].source_name(), "mdns");
        assert_eq!(sources[3].source_name(), "fallback");
    }
}
