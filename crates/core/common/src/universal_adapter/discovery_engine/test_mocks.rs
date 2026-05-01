// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test-only discovery sources for unit tests (see [`super::DiscoverySourceDispatch`]).

use std::collections::HashMap;

use super::DiscoverySource;
use crate::universal_adapter::capability_types::{
    CapabilityInfo, CapabilityType, HealthStatus, ServiceEndpoint,
};
use crate::{ToadStoolError, ToadStoolResult};

/// Duplicate provider IDs (deduplication coverage).
#[doc(hidden)]
pub struct DedupMockSource;

impl DiscoverySource for DedupMockSource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        Ok(vec![
            CapabilityInfo {
                provider_id: "dup-1".to_string(),
                capability: CapabilityType::Storage {
                    features: vec![],
                    min_throughput_mbps: None,
                },
                metadata: HashMap::new(),
                endpoint: ServiceEndpoint::Http("http://a".to_string()),
                health: HealthStatus::Unknown,
            },
            CapabilityInfo {
                provider_id: "dup-1".to_string(),
                capability: CapabilityType::Storage {
                    features: vec![],
                    min_throughput_mbps: None,
                },
                metadata: HashMap::new(),
                endpoint: ServiceEndpoint::Http("http://b".to_string()),
                health: HealthStatus::Unknown,
            },
        ])
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}

/// Always fails discovery (error-path coverage).
#[doc(hidden)]
pub struct FailingMockSource;

impl DiscoverySource for FailingMockSource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        Err(ToadStoolError::configuration("config error".to_string()))
    }

    fn name(&self) -> &'static str {
        "failing"
    }
}

/// Never completes (timeout coverage).
#[doc(hidden)]
pub struct SlowMockSource;

impl DiscoverySource for SlowMockSource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        std::future::pending::<ToadStoolResult<Vec<CapabilityInfo>>>().await
    }

    fn name(&self) -> &'static str {
        "slow"
    }
}

/// Returns one compute provider (mixed-source coverage).
#[doc(hidden)]
pub struct OkMockSource;

impl DiscoverySource for OkMockSource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        Ok(vec![CapabilityInfo {
            provider_id: "ok-1".to_string(),
            capability: CapabilityType::Compute {
                features: vec![],
                min_memory_gb: None,
            },
            metadata: HashMap::new(),
            endpoint: ServiceEndpoint::Http("http://ok:0".to_string()),
            health: HealthStatus::Unknown,
        }])
    }

    fn name(&self) -> &'static str {
        "ok"
    }
}

/// Fails with a distinct configuration message (mixed with [`OkMockSource`]).
#[doc(hidden)]
pub struct FailingMixedMockSource;

impl DiscoverySource for FailingMixedMockSource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        Err(ToadStoolError::configuration("fail".to_string()))
    }

    fn name(&self) -> &'static str {
        "fail"
    }
}

/// Fast success (timeout ordering coverage with [`SlowMockSource`]).
#[doc(hidden)]
pub struct FastOkMockSource;

impl DiscoverySource for FastOkMockSource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        Ok(vec![CapabilityInfo {
            provider_id: "fast".to_string(),
            capability: CapabilityType::Compute {
                features: vec![],
                min_memory_gb: None,
            },
            metadata: HashMap::new(),
            endpoint: ServiceEndpoint::Http("http://fast:0".to_string()),
            health: HealthStatus::Unknown,
        }])
    }

    fn name(&self) -> &'static str {
        "fast"
    }
}
