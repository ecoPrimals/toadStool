// SPDX-License-Identifier: AGPL-3.0-or-later
//! Modbus industrial transport (RTU/TCP) via the `modbus` crate.
//!
//! Gated behind feature `modbus-transport` (opt-in; not in `default`).

#[cfg(feature = "modbus-transport")]
mod inner {
    use crate::{SpecialtyRuntimeError, ToadStoolResult};

    /// Whether this build includes the `modbus` transport dependency.
    #[must_use]
    pub const fn modbus_transport_available() -> bool {
        true
    }

    /// Read holding registers over Modbus (RTU/TCP wiring is future work).
    pub fn modbus_read_registers(
        _host: &str,
        _unit_id: u8,
        _address: u16,
        _count: u16,
    ) -> ToadStoolResult<Vec<u16>> {
        // Touch the optional crate so `cargo check --features modbus-transport` links it.
        let _ = std::any::type_name::<modbus::Error>();
        Err(SpecialtyRuntimeError::CommunicationError(
            "Modbus register read not yet implemented".into(),
        )
        .into())
    }
}

#[cfg(feature = "modbus-transport")]
pub use inner::{modbus_read_registers, modbus_transport_available};

#[cfg(not(feature = "modbus-transport"))]
mod feature_disabled {
    use crate::{SpecialtyRuntimeError, ToadStoolResult};

    /// Whether this build includes the `modbus` transport dependency.
    #[must_use]
    pub const fn modbus_transport_available() -> bool {
        false
    }

    /// Modbus register read when `modbus-transport` is disabled.
    pub fn modbus_read_registers(
        _host: &str,
        _unit_id: u8,
        _address: u16,
        _count: u16,
    ) -> ToadStoolResult<Vec<u16>> {
        Err(SpecialtyRuntimeError::CommunicationError(
            "Modbus transport unavailable; enable `modbus-transport` feature".into(),
        )
        .into())
    }
}

#[cfg(not(feature = "modbus-transport"))]
pub use feature_disabled::{modbus_read_registers, modbus_transport_available};
