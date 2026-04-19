// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{LegacyArchitecture, ProgrammingInterface};

use super::super::chip_database::{
    avr_by_name, parallel_eprom_size_by_name, pic_by_name, validate_avr_isp_clock,
    validate_avr_signature_db, validate_avr_supply, validate_pic_clock, validate_pic_supply,
};
use super::super::errors::EmbeddedProgrammerError;
use super::super::programmers::{EpromProgrammerInner, GenericProgrammerInner};
use super::super::types::TargetInfo;
use super::{get_str, parse_signature_hex, parse_u32, parse_u64};

pub(super) fn init_generic_programmer(
    config: &ProgrammingInterface,
) -> Result<GenericProgrammerInner, EmbeddedProgrammerError> {
    let params = &config.connection_params;
    let family = get_str(params, "family")?;
    let chip_name = get_str(params, "chip")?;
    let clock_hz = parse_u64(params, "clock_hz")?;
    let voltage_mv = parse_u32(params, "voltage_mv")?;

    match family.to_ascii_lowercase().as_str() {
        "avr" => {
            let avr = avr_by_name(chip_name).ok_or_else(|| {
                EmbeddedProgrammerError::ConfigurationInvalid {
                    detail: format!("unknown AVR device {chip_name:?} (use chip_database name)"),
                }
            })?;
            validate_avr_isp_clock(avr, clock_hz)?;
            validate_avr_supply(avr, voltage_mv)?;
            if let Some(sig_s) = params.get("signature") {
                let sig = parse_signature_hex(sig_s)?;
                validate_avr_signature_db(sig, Some(chip_name))?;
            }
            Ok(GenericProgrammerInner {
                avr: Some(avr),
                pic: None,
                clock_hz,
                voltage_mv,
                connected: false,
                engine: super::super::protocol_engine::ProtocolEngine::new(),
            })
        }
        "pic" => {
            let pic = pic_by_name(chip_name).ok_or_else(|| {
                EmbeddedProgrammerError::ConfigurationInvalid {
                    detail: format!("unknown PIC device {chip_name:?}"),
                }
            })?;
            validate_pic_clock(pic, clock_hz)?;
            validate_pic_supply(pic, voltage_mv)?;
            Ok(GenericProgrammerInner {
                avr: None,
                pic: Some(pic),
                clock_hz,
                voltage_mv,
                connected: false,
                engine: super::super::protocol_engine::ProtocolEngine::new(),
            })
        }
        _ => Err(EmbeddedProgrammerError::ConfigurationInvalid {
            detail: format!("unsupported family {family:?} (expected avr or pic)"),
        }),
    }
}

pub(super) fn init_eprom_programmer(
    config: &ProgrammingInterface,
) -> Result<EpromProgrammerInner, EmbeddedProgrammerError> {
    let params = &config.connection_params;
    let chip = get_str(params, "chip")?;
    let voltage_mv = parse_u32(params, "voltage_mv")?;
    let size = if params.contains_key("size_bytes") {
        parse_u32(params, "size_bytes")?
    } else {
        parallel_eprom_size_by_name(chip).ok_or_else(|| {
            EmbeddedProgrammerError::ConfigurationInvalid {
                detail: format!(
                    "unknown EPROM {chip:?} (set size_bytes or use a known part number)"
                ),
            }
        })?
    };
    if !(4_750..=5_250).contains(&voltage_mv) {
        return Err(EmbeddedProgrammerError::VoltageIncompatible {
            chip_family: "parallel-eprom",
            detail: format!("nominal {voltage_mv} mV not in 5 V EPROM programming range"),
        });
    }
    Ok(EpromProgrammerInner {
        device_name: chip.to_string(),
        size_bytes: size,
        voltage_mv,
        connected: false,
        engine: super::super::protocol_engine::ProtocolEngine::new(),
    })
}

pub(super) fn target_info_avr(
    chip: &super::super::chip_database::AvrChipInfo,
    clock_hz: u64,
) -> TargetInfo {
    TargetInfo {
        name: chip.name.to_string(),
        architecture: LegacyArchitecture::Avr8bit,
        flash_size: chip.flash_size,
        ram_size: 0,
        eeprom_size: Some(chip.eeprom_size),
        cpu_speed: clock_hz.min(u64::from(u32::MAX)) as u32,
        features: vec!["isp".to_string(), "avr".to_string()],
    }
}

pub(super) fn target_info_pic(
    chip: &super::super::chip_database::PicChipInfo,
    clock_hz: u64,
) -> TargetInfo {
    TargetInfo {
        name: chip.name.to_string(),
        architecture: LegacyArchitecture::Pic16bit,
        flash_size: chip.flash_size,
        ram_size: 0,
        eeprom_size: None,
        cpu_speed: clock_hz.min(u64::from(u32::MAX)) as u32,
        features: vec!["icsp".to_string(), "pic18".to_string()],
    }
}

pub(super) fn target_info_eprom(inner: &EpromProgrammerInner) -> TargetInfo {
    TargetInfo {
        name: inner.device_name.clone(),
        architecture: LegacyArchitecture::Intel8051,
        flash_size: inner.size_bytes,
        ram_size: 0,
        eeprom_size: None,
        cpu_speed: 0,
        features: vec![
            "parallel".to_string(),
            "eprom".to_string(),
            format!("{}mV", inner.voltage_mv),
        ],
    }
}
