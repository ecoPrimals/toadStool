// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trait implementations for embedded programmers
//!
//! ISP / ICSP protocol sequencing and chip validation are implemented in pure Rust; any operation
//! that would require talking to a USB/serial/parallel adapter returns
//! [`EmbeddedProgrammerError::TransportNotConfigured`] after validation succeeds.
//!
//! See DEBT.md `D-EMBEDDED-PROGRAMMER` for hardware transport tracking.

use std::collections::HashMap;
use std::future::{Future, ready};

use crate::{
    LegacyArchitecture, ProgrammingInterface, ProgrammingInterfaceType, SpecialtyRuntimeError,
    ToadStoolResult,
};
use toadstool::ToadStoolError;

use super::chip_database::{
    avr_by_name, parallel_eprom_size_by_name, pic_by_name, validate_avr_isp_clock,
    validate_avr_signature_db, validate_avr_supply, validate_pic_clock, validate_pic_supply,
};
use super::errors::EmbeddedProgrammerError;
use super::programmers::{
    EPROMProgrammer, EpromProgrammerInner, GenericProgrammer, GenericProgrammerInner,
};
use super::protocol_engine::{
    self, avr_isp_erase_sequence, avr_validate_flash_range, parallel_eprom_read_block,
    pic_validate_flash_range,
};
use super::types::{ProgrammerInterface as ProgrammerTrait, TargetInfo};

fn programmer_err(e: EmbeddedProgrammerError) -> ToadStoolError {
    SpecialtyRuntimeError::from(e).into()
}

fn get_str<'a>(
    params: &'a HashMap<String, String>,
    key: &str,
) -> Result<&'a str, EmbeddedProgrammerError> {
    params.get(key).map(String::as_str).ok_or_else(|| {
        EmbeddedProgrammerError::ConfigurationInvalid {
            detail: format!("missing connection parameter `{key}`"),
        }
    })
}

fn parse_u64(params: &HashMap<String, String>, key: &str) -> Result<u64, EmbeddedProgrammerError> {
    let s = get_str(params, key)?;
    s.parse::<u64>()
        .map_err(|_| EmbeddedProgrammerError::ConfigurationInvalid {
            detail: format!("`{key}` must be a decimal integer (got {s:?})"),
        })
}

fn parse_u32(params: &HashMap<String, String>, key: &str) -> Result<u32, EmbeddedProgrammerError> {
    let s = get_str(params, key)?;
    s.parse::<u32>()
        .map_err(|_| EmbeddedProgrammerError::ConfigurationInvalid {
            detail: format!("`{key}` must be a decimal integer (got {s:?})"),
        })
}

fn parse_signature_hex(s: &str) -> Result<u32, EmbeddedProgrammerError> {
    let t = s.trim();
    let t = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    u32::from_str_radix(t, 16)
        .map(|v| v & 0xFF_FFFF)
        .map_err(|_| EmbeddedProgrammerError::ConfigurationInvalid {
            detail: format!("invalid hex signature {s:?}"),
        })
}

fn init_generic_programmer(
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
                engine: super::protocol_engine::ProtocolEngine::new(),
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
                engine: super::protocol_engine::ProtocolEngine::new(),
            })
        }
        _ => Err(EmbeddedProgrammerError::ConfigurationInvalid {
            detail: format!("unsupported family {family:?} (expected avr or pic)"),
        }),
    }
}

fn init_eprom_programmer(
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
        engine: super::protocol_engine::ProtocolEngine::new(),
    })
}

fn target_info_avr(chip: &super::chip_database::AvrChipInfo, clock_hz: u64) -> TargetInfo {
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

fn target_info_pic(chip: &super::chip_database::PicChipInfo, clock_hz: u64) -> TargetInfo {
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

fn target_info_eprom(inner: &EpromProgrammerInner) -> TargetInfo {
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

impl ProgrammerTrait for GenericProgrammer {
    fn name(&self) -> &'static str {
        "Generic Programmer"
    }

    fn supported_interfaces(&self) -> Vec<ProgrammingInterfaceType> {
        vec![ProgrammingInterfaceType::ISP]
    }

    fn initialize<'a>(
        &'a mut self,
        config: &'a ProgrammingInterface,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        ready(
            init_generic_programmer(config)
                .map(|i| {
                    self.inner = Some(i);
                })
                .map_err(programmer_err),
        )
    }

    fn connect(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        ready(
            (|| {
                let s = self.inner.as_mut().ok_or_else(|| {
                    EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "initialize() must succeed before connect()".into(),
                    }
                })?;
                if s.avr.is_some() {
                    s.engine.build_avr_connect_probe();
                } else if s.pic.is_some() {
                    s.engine.build_pic_connect();
                }
                s.connected = true;
                Ok(())
            })()
            .map_err(programmer_err),
        )
    }

    fn disconnect(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        ready({
            if let Some(s) = self.inner.as_mut() {
                s.connected = false;
            }
            Ok(())
        })
    }

    fn read_memory(
        &mut self,
        address: u32,
        length: u32,
    ) -> impl Future<Output = ToadStoolResult<Vec<u8>>> + Send + '_ {
        ready(
            (|| {
                let s = self.inner.as_ref().ok_or_else(|| {
                    EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "initialize() must succeed before read_memory()".into(),
                    }
                })?;
                if !s.connected {
                    return Err(EmbeddedProgrammerError::DeviceNotConnected {
                        device_id: "generic_isp:not_connected".into(),
                    });
                }
                if let Some(avr) = s.avr {
                    let mut eng = protocol_engine::ProtocolEngine::new();
                    eng.build_avr_read_flash(avr, address, length)?;
                } else if let Some(pic) = s.pic {
                    pic_validate_flash_range(pic, address, length)?;
                    let _ = super::protocol_engine::pic18_icsp_read_program_memory_ops(
                        length / 2 + length % 2,
                    );
                } else {
                    return Err(EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "internal: no family".into(),
                    });
                }
                Err(EmbeddedProgrammerError::TransportNotConfigured { interface: "ISP" })
            })()
            .map_err(programmer_err),
        )
    }

    fn write_memory<'a>(
        &'a mut self,
        address: u32,
        data: &'a [u8],
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        ready(
            (|| {
                let s = self.inner.as_mut().ok_or_else(|| {
                    EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "initialize() must succeed before write_memory()".into(),
                    }
                })?;
                if !s.connected {
                    return Err(EmbeddedProgrammerError::DeviceNotConnected {
                        device_id: "generic_isp:not_connected".into(),
                    });
                }
                if let Some(avr) = s.avr {
                    protocol_engine::avr_validate_page_write(avr, address, data.len())?;
                    s.engine.build_avr_program_page(avr, address, data)?;
                } else if let Some(pic) = s.pic {
                    pic_validate_flash_range(pic, address, data.len() as u32)?;
                } else {
                    return Err(EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "internal: no family".into(),
                    });
                }
                Err(EmbeddedProgrammerError::TransportNotConfigured { interface: "ISP" })
            })()
            .map_err(programmer_err),
        )
    }

    fn erase_memory(
        &mut self,
        address: u32,
        length: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        ready(
            (|| {
                let s = self.inner.as_mut().ok_or_else(|| {
                    EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "initialize() must succeed before erase_memory()".into(),
                    }
                })?;
                if !s.connected {
                    return Err(EmbeddedProgrammerError::DeviceNotConnected {
                        device_id: "generic_isp:not_connected".into(),
                    });
                }
                if let Some(avr) = s.avr {
                    if address != 0 || length != avr.flash_size {
                        return Err(EmbeddedProgrammerError::OperationNotSupported {
                            detail: "AVR ISP bulk erase is whole flash only".into(),
                        });
                    }
                    let _seq = avr_isp_erase_sequence(avr);
                } else if let Some(_pic) = s.pic {
                    let _ = super::protocol_engine::pic18_icsp_connect_sequence();
                } else {
                    return Err(EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "internal: no family".into(),
                    });
                }
                Err(EmbeddedProgrammerError::TransportNotConfigured { interface: "ISP" })
            })()
            .map_err(programmer_err),
        )
    }

    fn verify_memory<'a>(
        &'a mut self,
        address: u32,
        expected_data: &'a [u8],
    ) -> impl Future<Output = ToadStoolResult<bool>> + Send + 'a {
        ready(
            (|| {
                let s = self.inner.as_mut().ok_or_else(|| {
                    EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "initialize() must succeed before verify_memory()".into(),
                    }
                })?;
                if !s.connected {
                    return Err(EmbeddedProgrammerError::DeviceNotConnected {
                        device_id: "generic_isp:not_connected".into(),
                    });
                }
                if let Some(avr) = s.avr {
                    avr_validate_flash_range(avr, address, expected_data.len() as u32)?;
                    let mut eng = protocol_engine::ProtocolEngine::new();
                    eng.build_avr_read_flash(avr, address, expected_data.len() as u32)?;
                } else if let Some(pic) = s.pic {
                    pic_validate_flash_range(pic, address, expected_data.len() as u32)?;
                } else {
                    return Err(EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "internal: no family".into(),
                    });
                }
                Err(EmbeddedProgrammerError::TransportNotConfigured { interface: "ISP" })
            })()
            .map_err(programmer_err),
        )
    }

    fn get_target_info(&self) -> impl Future<Output = ToadStoolResult<TargetInfo>> + Send + '_ {
        ready(
            (|| {
                let s = self.inner.as_ref().ok_or_else(|| {
                    EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "initialize() must succeed before get_target_info()".into(),
                    }
                })?;
                if let Some(avr) = s.avr {
                    Ok(target_info_avr(avr, s.clock_hz))
                } else if let Some(pic) = s.pic {
                    Ok(target_info_pic(pic, s.clock_hz))
                } else {
                    Err(EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "internal: no family".into(),
                    })
                }
            })()
            .map_err(programmer_err),
        )
    }
}

impl ProgrammerTrait for EPROMProgrammer {
    fn name(&self) -> &'static str {
        "EPROM Programmer"
    }

    fn supported_interfaces(&self) -> Vec<ProgrammingInterfaceType> {
        vec![ProgrammingInterfaceType::Parallel]
    }

    fn initialize<'a>(
        &'a mut self,
        config: &'a ProgrammingInterface,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        ready(
            init_eprom_programmer(config)
                .map(|i| {
                    self.inner = Some(i);
                })
                .map_err(programmer_err),
        )
    }

    fn connect(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        ready(
            (|| {
                let s = self.inner.as_mut().ok_or_else(|| {
                    EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "initialize() must succeed before connect()".into(),
                    }
                })?;
                let _ = parallel_eprom_read_block(0, 1, s.size_bytes)?;
                s.connected = true;
                Ok(())
            })()
            .map_err(programmer_err),
        )
    }

    fn disconnect(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        ready({
            if let Some(s) = self.inner.as_mut() {
                s.connected = false;
            }
            Ok(())
        })
    }

    fn read_memory(
        &mut self,
        address: u32,
        length: u32,
    ) -> impl Future<Output = ToadStoolResult<Vec<u8>>> + Send + '_ {
        ready(
            (|| {
                let s = self.inner.as_ref().ok_or_else(|| {
                    EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "initialize() must succeed before read_memory()".into(),
                    }
                })?;
                if !s.connected {
                    return Err(EmbeddedProgrammerError::DeviceNotConnected {
                        device_id: "parallel_eprom:not_connected".into(),
                    });
                }
                let _ = parallel_eprom_read_block(address, length, s.size_bytes)?;
                Err(EmbeddedProgrammerError::TransportNotConfigured {
                    interface: "Parallel",
                })
            })()
            .map_err(programmer_err),
        )
    }

    fn write_memory<'a>(
        &'a mut self,
        address: u32,
        data: &'a [u8],
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        ready(
            (|| {
                let s = self.inner.as_ref().ok_or_else(|| {
                    EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "initialize() must succeed before write_memory()".into(),
                    }
                })?;
                if !s.connected {
                    return Err(EmbeddedProgrammerError::DeviceNotConnected {
                        device_id: "parallel_eprom:not_connected".into(),
                    });
                }
                let _ = parallel_eprom_read_block(address, data.len() as u32, s.size_bytes)?;
                Err(EmbeddedProgrammerError::TransportNotConfigured {
                    interface: "Parallel",
                })
            })()
            .map_err(programmer_err),
        )
    }

    fn erase_memory(
        &mut self,
        _address: u32,
        _length: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        ready(Err(programmer_err(
            EmbeddedProgrammerError::OperationNotSupported {
                detail: "UV EPROM erasure is not performed through this software adapter".into(),
            },
        )))
    }

    fn verify_memory<'a>(
        &'a mut self,
        address: u32,
        expected_data: &'a [u8],
    ) -> impl Future<Output = ToadStoolResult<bool>> + Send + 'a {
        ready(
            (|| {
                let s = self.inner.as_ref().ok_or_else(|| {
                    EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "initialize() must succeed before verify_memory()".into(),
                    }
                })?;
                if !s.connected {
                    return Err(EmbeddedProgrammerError::DeviceNotConnected {
                        device_id: "parallel_eprom:not_connected".into(),
                    });
                }
                let _ =
                    parallel_eprom_read_block(address, expected_data.len() as u32, s.size_bytes)?;
                Err(EmbeddedProgrammerError::TransportNotConfigured {
                    interface: "Parallel",
                })
            })()
            .map_err(programmer_err),
        )
    }

    fn get_target_info(&self) -> impl Future<Output = ToadStoolResult<TargetInfo>> + Send + '_ {
        ready(
            (|| {
                let s = self.inner.as_ref().ok_or_else(|| {
                    EmbeddedProgrammerError::ConfigurationInvalid {
                        detail: "initialize() must succeed before get_target_info()".into(),
                    }
                })?;
                Ok(target_info_eprom(s))
            })()
            .map_err(programmer_err),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProgrammingInterface, ProgrammingInterfaceType};
    use std::collections::HashMap;

    use crate::embedded::programmers::{EPROMProgrammer, GenericProgrammer};
    use crate::embedded::types::{ProgrammerInterface, TargetInfo};

    fn avr_config() -> ProgrammingInterface {
        ProgrammingInterface {
            interface_type: ProgrammingInterfaceType::ISP,
            connection_params: HashMap::from([
                ("family".to_string(), "avr".to_string()),
                ("chip".to_string(), "ATmega328P".to_string()),
                ("clock_hz".to_string(), "250000".to_string()),
                ("voltage_mv".to_string(), "5000".to_string()),
            ]),
        }
    }

    fn assert_not_supported_programmer(err: &ToadStoolError) {
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("not supported"),
            "expected not-supported wording, got: {msg}"
        );
    }

    fn assert_serde_json_stable<T>(value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
    {
        let v1 = serde_json::to_value(value).expect("serde_json to_value");
        let back: T = serde_json::from_value(v1.clone()).expect("serde_json from_value");
        let v2 = serde_json::to_value(&back).expect("serde_json to_value roundtrip");
        assert_eq!(v1, v2);
    }

    #[test]
    fn generic_programmer_new_default_debug() {
        let a = GenericProgrammer::new();
        let b = GenericProgrammer::default();
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert!(format!("{a:?}").contains("GenericProgrammer"));
    }

    #[test]
    fn eprom_programmer_new_default_debug() {
        let a = EPROMProgrammer::new();
        let b = EPROMProgrammer::default();
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert!(format!("{a:?}").contains("EPROMProgrammer"));
    }

    #[test]
    fn serde_roundtrip_types_used_by_programmer_trait() {
        let intf = avr_config();
        assert_serde_json_stable(&intf);
        assert_serde_json_stable(&ProgrammingInterfaceType::Parallel);
        assert_serde_json_stable(&ProgrammingInterfaceType::Custom {
            name: "foo".to_string(),
        });
        let info = TargetInfo {
            name: "mcu".to_string(),
            architecture: LegacyArchitecture::Avr8bit,
            flash_size: 32_768,
            ram_size: 2_048,
            eeprom_size: Some(1024),
            cpu_speed: 16_000_000,
            features: vec!["spi".to_string()],
        };
        assert_serde_json_stable(&info);
    }

    #[test]
    fn generic_programmer_trait_name_and_interfaces() {
        let p = GenericProgrammer::new();
        assert_eq!(ProgrammerInterface::name(&p), "Generic Programmer");
        let v = ProgrammerInterface::supported_interfaces(&p);
        assert_eq!(v.len(), 1);
        assert!(matches!(v[0], ProgrammingInterfaceType::ISP));
    }

    #[test]
    fn eprom_programmer_trait_name_and_interfaces() {
        let p = EPROMProgrammer::new();
        assert_eq!(ProgrammerInterface::name(&p), "EPROM Programmer");
        let v = ProgrammerInterface::supported_interfaces(&p);
        assert_eq!(v.len(), 1);
        assert!(matches!(v[0], ProgrammingInterfaceType::Parallel));
    }

    #[tokio::test]
    async fn generic_programmer_init_connect_without_transport() {
        let mut p = GenericProgrammer::new();
        p.initialize(&avr_config()).await.expect("init");
        p.connect().await.expect("connect");
        p.disconnect().await.expect("disconnect");
    }

    #[tokio::test]
    async fn generic_programmer_read_needs_transport_after_connect() {
        let mut p = GenericProgrammer::new();
        p.initialize(&avr_config()).await.expect("init");
        p.connect().await.expect("connect");
        let err = p.read_memory(0, 4).await.expect_err("read");
        assert_not_supported_programmer(&err);
        assert!(
            err.to_string().contains("transport") || err.to_string().contains("Transport"),
            "{}",
            err
        );
    }

    #[tokio::test]
    async fn generic_programmer_init_fails_without_keys() {
        let mut p = GenericProgrammer::new();
        let bad = ProgrammingInterface {
            interface_type: ProgrammingInterfaceType::ISP,
            connection_params: HashMap::new(),
        };
        let err = p.initialize(&bad).await.expect_err("init");
        assert_not_supported_programmer(&err);
    }

    #[tokio::test]
    async fn eprom_programmer_lifecycle() {
        let mut p = EPROMProgrammer::new();
        let cfg = ProgrammingInterface {
            interface_type: ProgrammingInterfaceType::Parallel,
            connection_params: HashMap::from([
                ("chip".to_string(), "27C512".to_string()),
                ("voltage_mv".to_string(), "5000".to_string()),
            ]),
        };
        p.initialize(&cfg).await.expect("init");
        p.connect().await.expect("connect");
        let err = p.read_memory(0, 4).await.expect_err("read");
        assert_not_supported_programmer(&err);
        p.disconnect().await.expect("disconnect");
    }
}
