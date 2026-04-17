// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure-Rust ISP / programming protocol helpers (transport-free).
//!
//! The bit-banged sequences and validation rules are unit-testable without USB or
//! serial hardware. Actual pin wiggling remains in future transport crates.

use super::chip_database::avr_signature_known_u32;
use super::errors::EmbeddedProgrammerError;

/// AVR ISP "Programming Enable" instruction payload (5 bits + padding).
pub const AVR_ISP_PROGRAMMING_ENABLE: u8 = 0b1010_1100;

/// ATmega328P signature bytes (order: high, middle, low) per Microchip DS40002061.
pub const ATMEGA328P_SIGNATURE: [u8; 3] = [0x1E, 0x95, 0x0F];

/// ATmega2560 signature.
pub const ATMEGA2560_SIGNATURE: [u8; 3] = [0x1E, 0x98, 0x03];

/// Low-level SPI-like bit operation for ISP (transport layer applies these to pins).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IspBitOp {
    /// Drive `/RESET` (active low).
    Reset(bool),
    /// SPI clock.
    Sck(bool),
    /// Shift MOSI bits MSB-first (`bits` uses only the low `bit_count` bits).
    MosiBits {
        /// Payload bits.
        bits: u8,
        /// Number of bits (1–8).
        bit_count: u8,
    },
    /// Delay between edges (microseconds).
    DelayUs(u32),
}

/// Validates ISP SPI clock against common AVR ICSP limits (~125 kHz minimum for 5 V targets).
///
/// Returns an error if `clock_hz` is zero or above a conservative high bound.
pub fn validate_isp_clock_hz(clock_hz: u64) -> Result<(), EmbeddedProgrammerError> {
    const MIN_HZ: u64 = 125_000;
    const MAX_HZ: u64 = 4_000_000;
    if clock_hz == 0 {
        return Err(EmbeddedProgrammerError::TimingInvalid {
            detail: "clock_hz must be non-zero".into(),
        });
    }
    if !(MIN_HZ..=MAX_HZ).contains(&clock_hz) {
        return Err(EmbeddedProgrammerError::TimingInvalid {
            detail: format!("clock_hz {clock_hz} outside [{MIN_HZ}, {MAX_HZ}]"),
        });
    }
    Ok(())
}

/// Returns whether `signature` matches a known AVR device id in the built-in list.
pub fn avr_signature_known(signature: u32) -> bool {
    avr_signature_known_u32(signature)
}

/// Validates a 24-bit AVR-style signature for the given family string.
pub fn validate_avr_chip_signature(
    signature: u32,
    family: &str,
) -> Result<(), EmbeddedProgrammerError> {
    if family.eq_ignore_ascii_case("avr") && !avr_signature_known(signature) {
        return Err(EmbeddedProgrammerError::ChipSignatureUnknown {
            signature,
            family: family.to_string(),
        });
    }
    Ok(())
}

/// Typical 5 V AVR ISP: I/O must be 5 V tolerant; reject 1.8 V-only configs.
pub fn validate_voltage_for_avr(nominal_mv: u32) -> Result<(), EmbeddedProgrammerError> {
    match nominal_mv {
        4_750..=5_250 => Ok(()),
        3_000..=3_600 => Err(EmbeddedProgrammerError::VoltageIncompatible {
            chip_family: "5v-avr",
            detail: "3.3 V IO may not meet V_IH on 5 V ATmega ISP; use level shifters".into(),
        }),
        _ => Err(EmbeddedProgrammerError::VoltageIncompatible {
            chip_family: "5v-avr",
            detail: format!("nominal {nominal_mv} mV not supported for stock ISP wiring"),
        }),
    }
}

/// Builds the SPI bit stream for AVR ISP "Programming Enable" (no transport I/O).
///
/// Caller maps [`IspBitOp`] to pin toggles. Sequence assumes `/RESET` was already
/// asserted low elsewhere.
pub fn avr_isp_programming_enable_sequence(
    clock_hz: u64,
) -> Result<Vec<IspBitOp>, EmbeddedProgrammerError> {
    validate_isp_clock_hz(clock_hz)?;
    let half_period_us = (1_000_000u64 / (2 * clock_hz.max(1))) as u32;
    let mut ops = Vec::with_capacity(64);
    ops.push(IspBitOp::DelayUs(half_period_us.max(1)));
    for bit in 0..8 {
        let bit_val = (AVR_ISP_PROGRAMMING_ENABLE >> (7 - bit)) & 1;
        ops.push(IspBitOp::MosiBits {
            bits: bit_val,
            bit_count: 1,
        });
        ops.push(IspBitOp::Sck(false));
        ops.push(IspBitOp::DelayUs(half_period_us.max(1)));
        ops.push(IspBitOp::Sck(true));
        ops.push(IspBitOp::DelayUs(half_period_us.max(1)));
    }
    ops.push(IspBitOp::Sck(false));
    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isp_clock_bounds() {
        assert!(validate_isp_clock_hz(250_000).is_ok());
        assert!(validate_isp_clock_hz(0).is_err());
        assert!(validate_isp_clock_hz(10_000_000).is_err());
    }

    #[test]
    fn avr_signature_match() {
        let sig = (u32::from(ATMEGA328P_SIGNATURE[0]) << 16)
            | (u32::from(ATMEGA328P_SIGNATURE[1]) << 8)
            | u32::from(ATMEGA328P_SIGNATURE[2]);
        assert!(avr_signature_known(sig));
        assert!(validate_avr_chip_signature(sig, "avr").is_ok());
        // ATmega328 (non-P) — distinct signature in chip database.
        let sig328 = 0x1E9514u32;
        assert!(avr_signature_known(sig328));
    }

    #[test]
    fn avr_signature_unknown() {
        let err = validate_avr_chip_signature(0x00_00_01, "avr").unwrap_err();
        assert!(matches!(
            err,
            EmbeddedProgrammerError::ChipSignatureUnknown { .. }
        ));
    }

    #[test]
    fn voltage_5v_ok() {
        assert!(validate_voltage_for_avr(5_000).is_ok());
    }

    #[test]
    fn voltage_3v3_warns() {
        let err = validate_voltage_for_avr(3_300).unwrap_err();
        assert!(matches!(
            err,
            EmbeddedProgrammerError::VoltageIncompatible { .. }
        ));
    }

    #[test]
    fn programming_enable_sequence_non_empty() {
        let ops = avr_isp_programming_enable_sequence(250_000).expect("sequence");
        assert!(!ops.is_empty());
        assert!(ops.iter().any(|o| matches!(o, IspBitOp::MosiBits { .. })));
    }
}
