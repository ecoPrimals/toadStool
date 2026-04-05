// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

/// Security level for encryption
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// Basic encryption (software-based)
    Standard,
    /// Enhanced encryption (genetic keys, entropy mixing)
    Enhanced,
    /// Hardware security module required
    HardwareSecured,
}
