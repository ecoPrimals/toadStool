// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    missing_docs,
    reason = "chip metadata tables are self-documenting data definitions"
)]
//! Built-in chip metadata for ISP / ICSP validation (signatures, voltage, timing).

use super::errors::EmbeddedProgrammerError;

/// 24-bit device signature (AVR: three id bytes; PIC: device id layout varies).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceSignature(pub u32);

impl DeviceSignature {
    pub const fn from_bytes(b0: u8, b1: u8, b2: u8) -> Self {
        Self(((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32))
    }

    pub fn from_u32(raw: u32) -> Self {
        Self(raw & 0xFF_FFFF)
    }
}

/// AVR ISP chip row (subset of datasheet fields used for validation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvrChipInfo {
    /// Marketing name.
    pub name: &'static str,
    /// 24-bit signature.
    pub signature: DeviceSignature,
    /// Flash size in bytes.
    pub flash_size: u32,
    /// Flash page size for page programming.
    pub flash_page_size: u32,
    /// EEPROM size in bytes (0 = none).
    pub eeprom_size: u32,
    /// Minimum ISP SPI clock (Hz).
    pub isp_clock_min_hz: u64,
    /// Maximum ISP SPI clock (Hz).
    pub isp_clock_max_hz: u64,
    /// Minimum Vcc for ISP (mV).
    pub supply_mv_min: u32,
    /// Maximum Vcc for ISP (mV).
    pub supply_mv_max: u32,
    /// Minimum chip erase time (ms) — host should delay at least this long after erase cmd.
    pub chip_erase_ms_min: u32,
    /// Typical bulk erase upper bound for validation (ms).
    pub chip_erase_ms_max: u32,
}

/// PIC18 / ICSP device entry (device id is 16-bit for many families; stored in low 16 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PicChipInfo {
    pub name: &'static str,
    /// Device id as used in validation (typically 0x0000_IDDD).
    pub device_id: u32,
    pub flash_size: u32,
    pub flash_page_size: u32,
    pub icsp_clock_min_hz: u64,
    pub icsp_clock_max_hz: u64,
    pub supply_mv_min: u32,
    pub supply_mv_max: u32,
}

const AVR_TABLE: &[AvrChipInfo] = &[
    AvrChipInfo {
        name: "ATmega328P",
        signature: DeviceSignature::from_bytes(0x1E, 0x95, 0x0F),
        flash_size: 32 * 1024,
        flash_page_size: 128,
        eeprom_size: 1024,
        isp_clock_min_hz: 125_000,
        isp_clock_max_hz: 4_000_000,
        supply_mv_min: 4_750,
        supply_mv_max: 5_250,
        chip_erase_ms_min: 9,
        chip_erase_ms_max: 50,
    },
    AvrChipInfo {
        name: "ATmega328",
        signature: DeviceSignature::from_bytes(0x1E, 0x95, 0x14),
        flash_size: 32 * 1024,
        flash_page_size: 128,
        eeprom_size: 1024,
        isp_clock_min_hz: 125_000,
        isp_clock_max_hz: 4_000_000,
        supply_mv_min: 4_750,
        supply_mv_max: 5_250,
        chip_erase_ms_min: 9,
        chip_erase_ms_max: 50,
    },
    AvrChipInfo {
        name: "ATmega2560",
        signature: DeviceSignature::from_bytes(0x1E, 0x98, 0x03),
        flash_size: 256 * 1024,
        flash_page_size: 256,
        eeprom_size: 4 * 1024,
        isp_clock_min_hz: 125_000,
        isp_clock_max_hz: 2_000_000,
        supply_mv_min: 4_750,
        supply_mv_max: 5_250,
        chip_erase_ms_min: 9,
        chip_erase_ms_max: 100,
    },
    AvrChipInfo {
        name: "ATtiny85",
        signature: DeviceSignature::from_bytes(0x1E, 0x93, 0x0B),
        flash_size: 8 * 1024,
        flash_page_size: 64,
        eeprom_size: 512,
        isp_clock_min_hz: 125_000,
        isp_clock_max_hz: 4_000_000,
        supply_mv_min: 2_700,
        supply_mv_max: 5_500,
        chip_erase_ms_min: 9,
        chip_erase_ms_max: 50,
    },
];

const PIC_TABLE: &[PicChipInfo] = &[
    PicChipInfo {
        name: "PIC18F4550",
        device_id: 0x0000_4A20,
        flash_size: 32 * 1024,
        flash_page_size: 64,
        icsp_clock_min_hz: 100_000,
        icsp_clock_max_hz: 10_000_000,
        supply_mv_min: 4_750,
        supply_mv_max: 5_250,
    },
    PicChipInfo {
        name: "PIC18F4620",
        device_id: 0x0000_4A40,
        flash_size: 64 * 1024,
        flash_page_size: 64,
        icsp_clock_min_hz: 100_000,
        icsp_clock_max_hz: 10_000_000,
        supply_mv_min: 4_750,
        supply_mv_max: 5_250,
    },
];

/// Look up AVR chip by exact 24-bit signature.
pub fn avr_by_signature(sig: DeviceSignature) -> Option<&'static AvrChipInfo> {
    AVR_TABLE.iter().find(|c| c.signature == sig)
}

/// Look up AVR chip by name (case-insensitive).
pub fn avr_by_name(name: &str) -> Option<&'static AvrChipInfo> {
    let n = name.trim();
    AVR_TABLE.iter().find(|c| c.name.eq_ignore_ascii_case(n))
}

/// True if signature matches a known AVR in the database.
pub fn avr_signature_known_u32(signature: u32) -> bool {
    avr_by_signature(DeviceSignature::from_u32(signature)).is_some()
}

/// Validate signature against family and optional expected chip name.
pub fn validate_avr_signature_db(
    signature: u32,
    chip_name: Option<&str>,
) -> Result<&'static AvrChipInfo, EmbeddedProgrammerError> {
    let sig = DeviceSignature::from_u32(signature);
    let row =
        avr_by_signature(sig).ok_or_else(|| EmbeddedProgrammerError::ChipSignatureUnknown {
            signature,
            family: "avr".into(),
        })?;
    if let Some(name) = chip_name {
        let expect =
            avr_by_name(name).ok_or_else(|| EmbeddedProgrammerError::ChipSignatureUnknown {
                signature,
                family: format!("avr:{name}"),
            })?;
        if expect.signature != sig {
            return Err(EmbeddedProgrammerError::ChipSignatureUnknown {
                signature,
                family: format!("avr (expected {})", expect.name),
            });
        }
        Ok(expect)
    } else {
        Ok(row)
    }
}

/// Validate ISP clock against chip-specific bounds.
pub fn validate_avr_isp_clock(
    chip: &AvrChipInfo,
    clock_hz: u64,
) -> Result<(), EmbeddedProgrammerError> {
    if clock_hz == 0 {
        return Err(EmbeddedProgrammerError::TimingInvalid {
            detail: "clock_hz must be non-zero".into(),
        });
    }
    if !(chip.isp_clock_min_hz..=chip.isp_clock_max_hz).contains(&clock_hz) {
        return Err(EmbeddedProgrammerError::TimingInvalid {
            detail: format!(
                "clock_hz {clock_hz} outside [{}, {}] for {}",
                chip.isp_clock_min_hz, chip.isp_clock_max_hz, chip.name
            ),
        });
    }
    Ok(())
}

/// Validate supply voltage (mV) for AVR chip.
pub fn validate_avr_supply(
    chip: &AvrChipInfo,
    nominal_mv: u32,
) -> Result<(), EmbeddedProgrammerError> {
    if !(chip.supply_mv_min..=chip.supply_mv_max).contains(&nominal_mv) {
        return Err(EmbeddedProgrammerError::VoltageIncompatible {
            chip_family: "avr-isp",
            detail: format!(
                "nominal {nominal_mv} mV not in [{}, {}] for {}",
                chip.supply_mv_min, chip.supply_mv_max, chip.name
            ),
        });
    }
    Ok(())
}

pub fn pic_by_device_id(id: u32) -> Option<&'static PicChipInfo> {
    PIC_TABLE.iter().find(|c| c.device_id == id)
}

pub fn pic_by_name(name: &str) -> Option<&'static PicChipInfo> {
    let n = name.trim();
    PIC_TABLE.iter().find(|c| c.name.eq_ignore_ascii_case(n))
}

pub fn validate_pic_clock(
    chip: &PicChipInfo,
    clock_hz: u64,
) -> Result<(), EmbeddedProgrammerError> {
    if clock_hz == 0 {
        return Err(EmbeddedProgrammerError::TimingInvalid {
            detail: "clock_hz must be non-zero".into(),
        });
    }
    if !(chip.icsp_clock_min_hz..=chip.icsp_clock_max_hz).contains(&clock_hz) {
        return Err(EmbeddedProgrammerError::TimingInvalid {
            detail: format!(
                "clock_hz {clock_hz} outside [{}, {}] for {}",
                chip.icsp_clock_min_hz, chip.icsp_clock_max_hz, chip.name
            ),
        });
    }
    Ok(())
}

pub fn validate_pic_supply(
    chip: &PicChipInfo,
    nominal_mv: u32,
) -> Result<(), EmbeddedProgrammerError> {
    if !(chip.supply_mv_min..=chip.supply_mv_max).contains(&nominal_mv) {
        return Err(EmbeddedProgrammerError::VoltageIncompatible {
            chip_family: "pic-icsp",
            detail: format!(
                "nominal {nominal_mv} mV not in [{}, {}] for {}",
                chip.supply_mv_min, chip.supply_mv_max, chip.name
            ),
        });
    }
    Ok(())
}

/// Parallel EPROM size by common part number (bytes).
pub fn parallel_eprom_size_by_name(name: &str) -> Option<u32> {
    match name.trim().to_ascii_uppercase().as_str() {
        "27C64" | "2764" => Some(8 * 1024),
        "27C128" => Some(16 * 1024),
        "27C256" => Some(32 * 1024),
        "27C512" => Some(64 * 1024),
        // Industry "27C010" = 128 KiB UV EPROM
        "27C010" | "27C1001" => Some(128 * 1024),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atmega328p_signature_lookup() {
        let s = DeviceSignature::from_bytes(0x1E, 0x95, 0x0F);
        let c = avr_by_signature(s).expect("mega328p");
        assert_eq!(c.name, "ATmega328P");
        assert_eq!(c.flash_size, 32 * 1024);
    }

    #[test]
    fn attiny85_user_example() {
        let raw = 0x001E_930B_u32;
        let c = avr_by_signature(DeviceSignature::from_u32(raw)).expect("tiny85");
        assert_eq!(c.name, "ATtiny85");
    }

    #[test]
    fn mega328_non_p_distinct_from_p() {
        let p = avr_by_signature(DeviceSignature::from_bytes(0x1E, 0x95, 0x0F)).unwrap();
        let np = avr_by_signature(DeviceSignature::from_bytes(0x1E, 0x95, 0x14)).unwrap();
        assert_ne!(p.signature, np.signature);
    }
}
