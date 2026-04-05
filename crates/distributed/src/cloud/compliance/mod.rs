// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compliance and security enforcement
//!
//! This module validates data sovereignty, security tier requirements (encryption, audit logging),
//! and resource isolation. Returns structured compliance reports with pass/fail per check.

mod security_tier;
#[cfg(test)]
mod tests;
mod validation;

/// Minimum security posture (standard vs high) for compliance checks.
pub use security_tier::SecurityTier;
/// Compliance reports, per-check results, and the cloud compliance enforcer.
pub use validation::{
    CheckResult, CloudComplianceEnforcer, ComplianceCheck, ComplianceError, ComplianceReport,
};
