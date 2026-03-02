//! Security Provider Abstraction
//!
//! Generic security provider interface that ANY primal or service can implement.
//! BearDog is just ONE implementation - HSM, KMS, local keyring all possible.
//!
//! ## Philosophy: "Security Provider, Not BearDog"
//!
//! Code requests "security capability" via Universal Adapter.
//! Runtime discovers WHO provides it (beardog, HSM, KMS, etc.).
//! This module defines WHAT a security provider can do, not WHO provides it.
//!
//! ## Deep Debt Principles
//!
//! - ✅ **No hardcoding**: Generic trait, no primal names
//! - ✅ **Capability-based**: Request by features, not by provider name
//! - ✅ **Runtime discovery**: Use Universal Adapter to find provider
//! - ✅ **Pluggable**: Any implementation works
//! - ✅ **Testable**: Mock providers for testing
//!
//! ## Usage
//!
//! ```rust,ignore
//! use toadstool_distributed::security_provider::*;
//! use toadstool_common::universal_adapter::*;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Discover security provider via Universal Adapter
//! let adapter = UniversalAdapter::new().await?;
//! let handle = adapter.request_capability(
//!     CapabilityType::Security {
//!         features: vec![SecurityFeature::Encryption],
//!         min_trust_level: TrustLevel::High,
//!     }
//! ).await?;
//!
//! // Get provider instance (discovered at runtime!)
//! let provider = SecurityProviderFactory::create_from_handle(&handle).await?;
//!
//! // Use provider (don't care who provides it!)
//! let encrypted = provider.encrypt(b"sensitive data").await?;
//! # Ok(())
//! # }
//! ```

pub mod factory;
pub mod local_keyring;
pub mod provider;
pub mod software_hsm;
pub mod tcp_provider;
pub mod types;
pub mod unix_socket_provider;

// BearDog implementation (ONE of many possible implementations)
pub mod beardog_impl;

pub use factory::*;
pub use provider::*;
pub use types::*;

// Re-export BearDog implementation
pub use beardog_impl::BearDogSecurityProvider;
pub use local_keyring::LocalKeyringProvider;
pub use software_hsm::SoftwareHsmProvider;
