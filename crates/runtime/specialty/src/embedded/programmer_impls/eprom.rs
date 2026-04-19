// SPDX-License-Identifier: AGPL-3.0-or-later

use std::future::{Future, ready};

use crate::{ProgrammingInterface, ProgrammingInterfaceType, ToadStoolResult};

use super::super::errors::EmbeddedProgrammerError;
use super::super::programmers::EPROMProgrammer;
use super::super::protocol_engine::parallel_eprom_read_block;
use super::super::types::{ProgrammerInterface as ProgrammerTrait, TargetInfo};
use super::init::{init_eprom_programmer, target_info_eprom};
use super::programmer_err;

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
