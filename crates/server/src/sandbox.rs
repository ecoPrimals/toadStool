// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security sandbox integration (feature `sandbox`).
//!
//! Re-exports [`toadstool-security-sandbox`] and [`toadstool-security-policies`]
//! for workload isolation when the server is built with `--features sandbox`.

pub use toadstool_security_policies::{
    FilePolicyManager, PolicyManager, PolicyManagerConfig, SecurityPolicy,
};
pub use toadstool_security_sandbox::{
    CrossPlatformSandboxManager, SandboxConfig, SandboxManager, SandboxSpec, SandboxStatus,
};
