// SPDX-License-Identifier: AGPL-3.0-or-later
//! ⚠️ DEPRECATED MODULE - REMOVED
//!
//! **This module has been removed in favor of capability-based adapters.**
//!
//! # Migration Guide
//!
//! All service-specific modules violated the infant discovery principle by hardcoding
//! service names. The new capability-based adapter system has replaced them:
//!
//! ## Old Way (Removed)
//! ```rust,ignore
//! use toadstool::ecosystem::services::{beardog, songbird, nestgate};
//!
//! beardog::install_permissions(path, false).await?;
//! songbird::register(&addr, &reg).await?;
//! nestgate::connect_storage(&addr, &mount, None).await?;
//! ```
//!
//! ## New Way (Current)
//! ```rust,ignore
//! use toadstool::ecosystem::adapters::{CryptoAdapter, CoordinationAdapter, StorageAdapter};
//! use toadstool::ecosystem::adapters::AdapterFactory;
//!
//! let factory = AdapterFactory::new();
//! let crypto = factory.crypto_adapter()?;
//! let coordination = factory.coordination_adapter()?;
//! let storage = factory.storage_adapter()?;
//!
//! crypto.install_permissions(path, false).await?;
//! coordination.register_service(info).await?;
//! storage.mount_distributed_storage(requirements).await?;
//! ```
//!
//! ## Benefits
//! - **Service Agnostic**: Works with any service providing the required capabilities
//! - **No Hardcoding**: Zero hardcoded service names, ports, or endpoints
//! - **Infant Discovery**: Services discovered dynamically at runtime
//! - **Failover**: Automatic fallback to backup services
//! - **Future Proof**: Works with services that don't exist yet
//!
//! See:
//! - `crates/cli/src/ecosystem/adapters/` - New capability-based adapters
//! - `reports/sessions/nov-19-2025/HARDCODING_ELIMINATION_COMPLETE.md` - Migration results

#![deprecated(
    since = "0.1.0",
    note = "All service modules removed. Use `ecosystem::adapters` instead. \
            See module docs for migration guide."
)]

// All service-specific modules (beardog, songbird, nestgate) have been removed.
// Use the capability-based adapter system in `ecosystem::adapters` instead.
