// SPDX-License-Identifier: AGPL-3.0-or-later
//! Connection management for ecosystem services
//!
//! This module handles maintaining connections to ecosystem services,
//! including connection pooling, retry logic, and state tracking.

// ConnectionManager removed - was defined but never constructed.
// Connection state is managed directly in EcosystemIntegrator
// via the connections HashMap field for better ergonomics.

// get_local_address() REMOVED - was part of deprecated hardcoded integration.
// New code uses capability-based adapters in ecosystem::adapters instead.
