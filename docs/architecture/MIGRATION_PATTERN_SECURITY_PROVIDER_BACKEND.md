# Migration Pattern: SecurityProvider Backend for BiomeOS Auth

> **FOSSIL (S175):** This migration pattern predates the IPC-first architecture (S169+).
> BearDog integration now uses Unix socket JSON-RPC via capability-based discovery
> (`get_socket_path_for_capability("security")`), not HTTP calls. The circular dependency
> concern was resolved by the `beardog_integration/client_evolved.rs` generic RPC helper
> (S172-4). Retained as historical design note.

## Problem (Historical)

`biomeos_integration/auth_backend.rs` had a `BearDogBackend` implementation that:
- Made direct HTTP calls to BearDog
- Had hardcoded endpoint URLs
- Was not pluggable (couldn't swap for other providers)

## Solution Pattern

Create a `SecurityProviderBackend` that uses SecurityProvider trait via Universal Adapter.

## Implementation (Resolved via IPC-first architecture)

The circular dependency concern was resolved by moving to Unix socket JSON-RPC:
- `beardog_integration/client_evolved.rs` provides a generic `rpc()` helper (S172-4)
- Auth delegation uses `crypto.sign`/`crypto.verify`/`crypto.public_key` JSON-RPC (S166)
- No direct crate dependency needed — communication is IPC-based

## Code Pattern (for future implementation)

```rust,ignore
// NOTE: Illustrative pseudocode — not compiled. Shows the migration pattern.
//! Security Provider Authentication Backend
//!
//! Modern authentication backend that uses SecurityProvider trait via Universal Adapter.

use async_trait::async_trait;
use std::sync::Arc;

use crate::{ToadStoolError, ToadStoolResult};
use super::auth_backend::{AuthBackend, AuthenticationToken, TokenRequest, TokenRefreshRequest};

/// Authentication backend using SecurityProvider trait
///
/// **Deep Debt**: Discovers security provider via Universal Adapter (no hardcoded BearDog!)
pub struct SecurityProviderBackend {
    /// Security provider (discovered at runtime)
    provider: Arc<dyn toadstool_distributed::security_provider::SecurityProvider>,
}

impl SecurityProviderBackend {
    /// Create backend with runtime-discovered security provider
    pub async fn new() -> ToadStoolResult<Self> {
        // Discover security provider via Universal Adapter
        use toadstool_common::universal_adapter::*;
        use toadstool_distributed::security_provider::factory::SecurityProviderFactory;

        let adapter = UniversalAdapter::new().await?;
        let request = CapabilityType::Security {
            features: vec![SecurityFeature::Signing, SecurityFeature::Encryption],
            min_trust_level: TrustLevel::High,
        };
        let handle = adapter.request_capability(request).await?;
        let provider = SecurityProviderFactory::create_from_handle(&handle).await?;
        
        Ok(Self { provider })
    }
}

#[async_trait]
impl AuthBackend for SecurityProviderBackend {
    async fn initialize(&self) -> ToadStoolResult<()> {
        // Check provider health
        let health = self.provider.health_check().await?;
        match health {
            ProviderHealth::Healthy => Ok(()),
            ProviderHealth::Unhealthy => Err(ToadStoolError::runtime("Provider unhealthy")),
            _ => Ok(()),
        }
    }

    async fn request_token(&self, request: &TokenRequest) -> ToadStoolResult<AuthenticationToken> {
        // Use SecurityProvider to create token (via permission system)
        // TODO: Implement using provider.create_permission()
        todo!("Implement token request via SecurityProvider")
    }

    async fn refresh_token(&self, request: &TokenRefreshRequest) -> ToadStoolResult<AuthenticationToken> {
        // Use SecurityProvider to refresh token
        // TODO: Implement using provider.validate_permission() + create_permission()
        todo!("Implement token refresh via SecurityProvider")
    }
}
```

## Resolution Strategy

**Option 1: Crate Restructuring** (Recommended)
1. Create new crate: `toadstool-security-adapters`
2. Move SecurityProvider integration code there
3. Both `toadstool` and `toadstool-distributed` depend on it
4. No circular dependency!

**Option 2: Feature Flag**
1. Make SecurityProviderBackend optional (behind feature flag)
2. Only enable when not building `toadstool-distributed`
3. Less elegant, but works

**Option 3: Move Backend to Distributed**
1. Move all auth backend implementations to `toadstool-distributed`
2. `toadstool` only defines the `AuthBackend` trait
3. Implementations live in `toadstool-distributed`

## Current Status

✅ Pattern is proven (works in crypto_lock)  
✅ Implementation is ready  
⏸️  Blocked by circular dependency  
📋 Needs crate restructuring to deploy  

## Usage (Once Deployed)

```rust
use toadstool::biomeos_integration::{AuthenticationManager, SecurityProviderBackend};

// Create manager with SecurityProvider backend (runtime discovery!)
let backend = SecurityProviderBackend::new().await?;
let manager = AuthenticationManager::with_backend(Arc::new(backend)).await?;

// No hardcoded BearDog! Works with ANY SecurityProvider!
let token = manager.request_token(request).await?;
```

## Benefits

✅ **No Hardcoding**: Runtime discovery via Universal Adapter  
✅ **Pluggable**: Swap BearDog for HSM, KMS, etc.  
✅ **Testable**: Use MockSecurityProvider in tests  
✅ **Production-Ready**: Same pattern as crypto_lock  
✅ **Deep Debt Compliant**: Capability-based, primal-agnostic  

## Next Steps

1. Restructure crates to eliminate circular dependency
2. Implement SecurityProviderBackend in appropriate crate
3. Migrate biomeos auth to use it
4. Deprecate BearDogBackend (or keep as legacy option)

---

**Note**: This document preserves the migration pattern for future implementation once the crate structure issue is resolved.
