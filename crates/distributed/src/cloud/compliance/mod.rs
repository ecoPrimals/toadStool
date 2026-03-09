// SPDX-License-Identifier: AGPL-3.0-only
//! Compliance and security enforcement
//!
//! This module validates data sovereignty, security tier requirements (encryption, audit logging),
//! and resource isolation. Returns structured compliance reports with pass/fail per check.

mod security_tier;
#[cfg(test)]
mod tests;
mod validation;

pub use security_tier::SecurityTier;
pub use validation::{
    CheckResult, CloudComplianceEnforcer, ComplianceCheck, ComplianceError, ComplianceReport,
};
