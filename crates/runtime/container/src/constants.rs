// SPDX-License-Identifier: AGPL-3.0-or-later
//! Container runtime policy defaults.

/// Start of the default application port range allowed by container network policy.
pub const APP_PORT_RANGE_START: u16 = 8000;
/// End of the default application port range allowed by container network policy.
pub const APP_PORT_RANGE_END: u16 = 8999;
/// Start of the default development port range allowed by container network policy.
pub const DEV_PORT_RANGE_START: u16 = 3000;
/// End of the default development port range allowed by container network policy.
pub const DEV_PORT_RANGE_END: u16 = 3999;
