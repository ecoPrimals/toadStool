//! Capability-based service discovery and invocation
//!
//! This module implements capability-based service interaction where services
//! are discovered and invoked based on **what they can do** (capabilities),
//! not **what they are called** (service names).
//!
//! # Core Principle
//! **"We don't care if it's BearDog, AWS KMS, or HSM. We care that it can sign with Ed25519."**

pub mod registry;
pub mod resolver;
pub mod taxonomy;

pub use registry::{CapabilityRegistry, ServiceProvider};
pub use resolver::CapabilityResolver;
pub use taxonomy::{CapabilityId, StandardCapability};
