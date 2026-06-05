// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serial port transport via the `serialport` crate.
//!
//! Gated behind feature `serial-transport` (opt-in; not in `default`).

/// User-facing message when serial transport is disabled at compile time.
pub const SERIAL_TRANSPORT_UNAVAILABLE: &str =
    "Serial transport unavailable; enable `serial-transport` feature";

#[cfg(feature = "serial-transport")]
mod inner {
    /// Whether this build includes the `serialport` transport dependency.
    #[must_use]
    pub const fn serial_transport_available() -> bool {
        true
    }
}

#[cfg(feature = "serial-transport")]
pub use inner::serial_transport_available;

#[cfg(not(feature = "serial-transport"))]
mod feature_disabled {
    /// Whether this build includes the `serialport` transport dependency.
    #[must_use]
    pub const fn serial_transport_available() -> bool {
        false
    }
}

#[cfg(not(feature = "serial-transport"))]
pub use feature_disabled::serial_transport_available;
