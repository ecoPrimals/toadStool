// SPDX-License-Identifier: AGPL-3.0-or-later
//! Enum dispatch for [`super::traits::UniversalPrimalProvider`].

use std::future::Future;

use crate::ToadStoolResult;

use super::provider::ToadStoolPrimalProvider;
use super::requests::{PrimalEndpoints, PrimalRequest, PrimalResponse};
use super::traits::UniversalPrimalProvider;
use super::types::{PrimalCapability, PrimalContext, PrimalHealth, PrimalType};

/// Production-oriented [`UniversalPrimalProvider`] dispatch (see workspace async trait migration).
pub enum UniversalPrimalProviderDispatch {
    /// Built-in `ToadStool` primal provider.
    ToadStool(ToadStoolPrimalProvider),
}

impl UniversalPrimalProvider for UniversalPrimalProviderDispatch {
    fn primal_id(&self) -> &str {
        match self {
            Self::ToadStool(p) => p.primal_id(),
        }
    }

    fn instance_id(&self) -> &str {
        match self {
            Self::ToadStool(p) => p.instance_id(),
        }
    }

    fn context(&self) -> &PrimalContext {
        match self {
            Self::ToadStool(p) => p.context(),
        }
    }

    fn primal_type(&self) -> PrimalType {
        match self {
            Self::ToadStool(p) => p.primal_type(),
        }
    }

    fn capabilities(&self) -> Vec<PrimalCapability> {
        match self {
            Self::ToadStool(p) => p.capabilities(),
        }
    }

    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        match self {
            Self::ToadStool(p) => p.health_check(),
        }
    }

    fn endpoints(&self) -> PrimalEndpoints {
        match self {
            Self::ToadStool(p) => p.endpoints(),
        }
    }

    fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> impl Future<Output = ToadStoolResult<PrimalResponse>> + Send + '_ {
        match self {
            Self::ToadStool(p) => p.handle_primal_request(request),
        }
    }

    fn initialize(
        &mut self,
        config: serde_json::Value,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        match self {
            Self::ToadStool(p) => p.initialize(config),
        }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        match self {
            Self::ToadStool(p) => p.shutdown(),
        }
    }

    fn can_serve_context(&self, context: &PrimalContext) -> bool {
        match self {
            Self::ToadStool(p) => p.can_serve_context(context),
        }
    }
}
