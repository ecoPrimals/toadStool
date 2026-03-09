// SPDX-License-Identifier: AGPL-3.0-only
//! Universal service adapters - capability-based service interaction
//!
//! This module provides adapters that interact with services based on their
//! **capabilities**, not their names. Services are discovered dynamically and
//! invoked through protocol-agnostic interfaces.
//!
//! # Philosophy
//! **"We don't know BearDog, NestGate, or Songbird. We know capabilities."**
//!
//! Instead of hardcoded service connections, we discover services that provide
//! the capabilities we need and interact with them through standard protocols.

pub mod coordination;
pub mod crypto;
pub mod factory;
pub mod storage;
pub mod universal;

pub use coordination::CoordinationAdapter;
pub use crypto::CryptoAdapter;
pub use factory::AdapterFactory;
pub use storage::StorageAdapter;
pub use universal::UniversalServiceAdapter;
