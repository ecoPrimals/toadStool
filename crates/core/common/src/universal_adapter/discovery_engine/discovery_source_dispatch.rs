// SPDX-License-Identifier: AGPL-3.0-or-later

//! Static dispatch for [`super::DiscoverySource`] implementations.

#[cfg(feature = "mdns")]
use super::MDnsSource;
#[cfg(test)]
use super::test_mocks::{
    DedupMockSource, FailingMixedMockSource, FailingMockSource, FastOkMockSource, OkMockSource,
    SlowMockSource,
};
use super::{DiscoverySource, EnvironmentSource, LocalRegistrySource};
use crate::ToadStoolResult;
use crate::universal_adapter::capability_types::CapabilityInfo;

/// Known [`DiscoverySource`] implementations for the universal adapter discovery engine.
pub enum DiscoverySourceDispatch {
    /// mDNS / DNS-SD (feature `mdns`).
    #[cfg(feature = "mdns")]
    Mdns(MDnsSource),
    /// Environment variables (`TOADSTOOL_*_PROVIDER`).
    Environment(EnvironmentSource),
    /// Local `registry.json` under the XDG config tree.
    LocalRegistry(LocalRegistrySource),
    /// Test-only: duplicate-ID scenario (`crate::universal_adapter::discovery_engine::test_mocks`).
    #[cfg(test)]
    TestDedup(DedupMockSource),
    /// Test-only: always errors (`test_mocks::FailingMockSource`).
    #[cfg(test)]
    TestFailing(FailingMockSource),
    /// Test-only: never completes (`test_mocks::SlowMockSource`).
    #[cfg(test)]
    TestSlow(SlowMockSource),
    /// Test-only: single OK provider (`test_mocks::OkMockSource`).
    #[cfg(test)]
    TestOk(OkMockSource),
    /// Test-only: mixed-source failure (`test_mocks::FailingMixedMockSource`).
    #[cfg(test)]
    TestFailingMixed(FailingMixedMockSource),
    /// Test-only: fast OK for timeout ordering (`test_mocks::FastOkMockSource`).
    #[cfg(test)]
    TestFastOk(FastOkMockSource),
}

impl DiscoverySource for DiscoverySourceDispatch {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        match self {
            #[cfg(feature = "mdns")]
            Self::Mdns(s) => s.discover().await,
            Self::Environment(s) => s.discover().await,
            Self::LocalRegistry(s) => s.discover().await,
            #[cfg(test)]
            Self::TestDedup(s) => s.discover().await,
            #[cfg(test)]
            Self::TestFailing(s) => s.discover().await,
            #[cfg(test)]
            Self::TestSlow(s) => s.discover().await,
            #[cfg(test)]
            Self::TestOk(s) => s.discover().await,
            #[cfg(test)]
            Self::TestFailingMixed(s) => s.discover().await,
            #[cfg(test)]
            Self::TestFastOk(s) => s.discover().await,
        }
    }

    fn name(&self) -> &str {
        match self {
            #[cfg(feature = "mdns")]
            Self::Mdns(s) => s.name(),
            Self::Environment(s) => s.name(),
            Self::LocalRegistry(s) => s.name(),
            #[cfg(test)]
            Self::TestDedup(s) => s.name(),
            #[cfg(test)]
            Self::TestFailing(s) => s.name(),
            #[cfg(test)]
            Self::TestSlow(s) => s.name(),
            #[cfg(test)]
            Self::TestOk(s) => s.name(),
            #[cfg(test)]
            Self::TestFailingMixed(s) => s.name(),
            #[cfg(test)]
            Self::TestFastOk(s) => s.name(),
        }
    }
}
