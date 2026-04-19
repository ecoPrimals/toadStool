// SPDX-License-Identifier: AGPL-3.0-or-later

use std::future::{Future, ready};

use crate::{ProgrammingInterface, ProgrammingInterfaceType, ToadStoolResult};

use super::super::errors::EmbeddedProgrammerError;
use super::super::programmers::GenericProgrammer;
use super::super::protocol_engine::{
    self, avr_isp_erase_sequence, avr_validate_flash_range, pic_validate_flash_range,
};
use super::super::types::{ProgrammerInterface as ProgrammerTrait, TargetInfo};
use super::init::{init_generic_programmer, target_info_avr, target_info_pic};
use super::programmer_err;

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
                    let _ = super::super::protocol_engine::pic18_icsp_read_program_memory_ops(
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
                    let _ = super::super::protocol_engine::pic18_icsp_connect_sequence();
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
