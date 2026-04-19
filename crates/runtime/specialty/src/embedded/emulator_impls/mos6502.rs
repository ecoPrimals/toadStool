// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::future::{Future, ready};

use crate::{EmbeddedConfig, LegacyArchitecture, ToadStoolResult};

use super::super::emulators::Emulator6502;
use super::super::errors::EmbeddedEmulatorError;
use super::super::types::{CpuRegisters, EmbeddedEmulator as EmulatorTrait, EmulationStatus};

use super::{copy_mem_range, emulator_err, write_mem_range};

fn regs6502(e: &Emulator6502) -> CpuRegisters {
    let mut gp = HashMap::new();
    gp.insert("A".to_string(), u32::from(e.cpu.a));
    gp.insert("X".to_string(), u32::from(e.cpu.x));
    gp.insert("Y".to_string(), u32::from(e.cpu.y));
    CpuRegisters {
        general_purpose: gp,
        program_counter: u32::from(e.cpu.pc),
        stack_pointer: u32::from(e.cpu.sp),
        status_register: u32::from(e.cpu.p),
        special: HashMap::new(),
    }
}

fn apply6502(e: &mut Emulator6502, regs: &CpuRegisters) {
    e.cpu.pc = regs.program_counter.min(0xFFFF) as u16;
    e.cpu.sp = regs.stack_pointer.min(0xFF) as u8;
    e.cpu.p = regs.status_register.min(0xFF) as u8;
    if let Some(&a) = regs.general_purpose.get("A") {
        e.cpu.a = a.min(255) as u8;
    }
    if let Some(&x) = regs.general_purpose.get("X") {
        e.cpu.x = x.min(255) as u8;
    }
    if let Some(&y) = regs.general_purpose.get("Y") {
        e.cpu.y = y.min(255) as u8;
    }
}

impl EmulatorTrait for Emulator6502 {
    fn name(&self) -> &'static str {
        "6502 Emulator"
    }

    fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
        vec![LegacyArchitecture::MOS6502]
    }

    fn initialize<'a>(
        &'a mut self,
        config: &'a EmbeddedConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        ready(
            (|| {
                if config.architecture != LegacyArchitecture::MOS6502 {
                    return Err(EmbeddedEmulatorError::NotReady {
                        detail: format!("expected MOS6502, got {:?}", config.architecture),
                    });
                }
                self.initialized = true;
                self.cpu.reset();
                self.status = EmulationStatus::Stopped;
                Ok(())
            })()
            .map_err(emulator_err),
        )
    }

    fn load_rom<'a>(
        &'a mut self,
        rom_data: &'a [u8],
        load_address: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        ready(
            (|| {
                if !self.initialized {
                    return Err(EmbeddedEmulatorError::NotReady {
                        detail: "initialize() first".into(),
                    });
                }
                let end = load_address
                    .checked_add(rom_data.len() as u32)
                    .ok_or_else(|| EmbeddedEmulatorError::NotReady {
                        detail: "ROM load overflow".into(),
                    })?;
                if end > 65536 {
                    return Err(EmbeddedEmulatorError::NotReady {
                        detail: "ROM extends past 64K".into(),
                    });
                }
                self.cpu.load(load_address as u16, rom_data);
                self.image_loaded = true;
                Ok(())
            })()
            .map_err(emulator_err),
        )
    }

    fn start(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        ready(
            (|| {
                if !self.initialized || !self.image_loaded {
                    return Err(EmbeddedEmulatorError::NotReady {
                        detail: "initialize and load_rom first".into(),
                    });
                }
                self.running = true;
                self.status = EmulationStatus::Running;
                Ok(())
            })()
            .map_err(emulator_err),
        )
    }

    fn stop(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        ready({
            self.running = false;
            if !matches!(
                self.status,
                EmulationStatus::Breakpoint { .. } | EmulationStatus::Error { .. }
            ) {
                self.status = EmulationStatus::Stopped;
            }
            Ok(())
        })
    }

    fn step(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        ready(
            (|| {
                if !self.initialized || !self.image_loaded {
                    return Err(EmbeddedEmulatorError::NotReady {
                        detail: "initialize and load_rom first".into(),
                    });
                }
                self.cpu.step();
                let pc = u32::from(self.cpu.pc);
                if self.breakpoints.contains(&pc) {
                    self.running = false;
                    self.status = EmulationStatus::Breakpoint { address: pc };
                } else if self.running {
                    self.status = EmulationStatus::Running;
                }
                Ok(())
            })()
            .map_err(emulator_err),
        )
    }

    fn set_breakpoint(
        &mut self,
        address: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        ready({
            self.breakpoints.insert(address);
            Ok(())
        })
    }

    fn clear_breakpoint(
        &mut self,
        address: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        ready({
            self.breakpoints.remove(&address);
            Ok(())
        })
    }

    fn read_registers(&self) -> impl Future<Output = ToadStoolResult<CpuRegisters>> + Send + '_ {
        ready(
            (|| {
                if !self.initialized {
                    return Err(EmbeddedEmulatorError::NotReady {
                        detail: "initialize() first".into(),
                    });
                }
                Ok(regs6502(self))
            })()
            .map_err(emulator_err),
        )
    }

    fn write_registers<'a>(
        &'a mut self,
        registers: &'a CpuRegisters,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        ready(
            (|| {
                if !self.initialized {
                    return Err(EmbeddedEmulatorError::NotReady {
                        detail: "initialize() first".into(),
                    });
                }
                apply6502(self, registers);
                Ok(())
            })()
            .map_err(emulator_err),
        )
    }

    fn read_memory(
        &self,
        address: u32,
        length: u32,
    ) -> impl Future<Output = ToadStoolResult<Vec<u8>>> + Send + '_ {
        ready(
            (|| {
                if !self.initialized {
                    return Err(EmbeddedEmulatorError::NotReady {
                        detail: "initialize() first".into(),
                    });
                }
                copy_mem_range(&self.cpu.mem, address, length)
            })()
            .map_err(emulator_err),
        )
    }

    fn write_memory<'a>(
        &'a mut self,
        address: u32,
        data: &'a [u8],
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        ready(
            (|| {
                if !self.initialized {
                    return Err(EmbeddedEmulatorError::NotReady {
                        detail: "initialize() first".into(),
                    });
                }
                write_mem_range(&mut self.cpu.mem, address, data)
            })()
            .map_err(emulator_err),
        )
    }

    fn get_status(&self) -> impl Future<Output = ToadStoolResult<EmulationStatus>> + Send + '_ {
        ready(
            (|| {
                if !self.initialized {
                    return Err(EmbeddedEmulatorError::NotReady {
                        detail: "initialize() first".into(),
                    });
                }
                Ok(self.status.clone())
            })()
            .map_err(emulator_err),
        )
    }
}
