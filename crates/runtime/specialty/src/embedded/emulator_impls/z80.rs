// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::future::{Future, ready};

use crate::{EmbeddedConfig, LegacyArchitecture, ToadStoolResult};

use super::super::emulators::EmulatorZ80;
use super::super::errors::EmbeddedEmulatorError;
use super::super::types::{CpuRegisters, EmbeddedEmulator as EmulatorTrait, EmulationStatus};

use super::{copy_mem_range, emulator_err, write_mem_range};

fn regsz80(e: &EmulatorZ80) -> CpuRegisters {
    let mut gp = HashMap::new();
    gp.insert("A".to_string(), u32::from(e.cpu.a));
    gp.insert("F".to_string(), u32::from(e.cpu.f));
    gp.insert("B".to_string(), u32::from(e.cpu.b));
    gp.insert("C".to_string(), u32::from(e.cpu.c));
    gp.insert("D".to_string(), u32::from(e.cpu.d));
    gp.insert("E".to_string(), u32::from(e.cpu.e));
    gp.insert("H".to_string(), u32::from(e.cpu.h));
    gp.insert("L".to_string(), u32::from(e.cpu.l));
    let mut sp = HashMap::new();
    sp.insert("IX".to_string(), u32::from(e.cpu.ix));
    sp.insert("IY".to_string(), u32::from(e.cpu.iy));
    CpuRegisters {
        general_purpose: gp,
        program_counter: u32::from(e.cpu.pc),
        stack_pointer: u32::from(e.cpu.sp),
        status_register: u32::from(e.cpu.f),
        special: sp,
    }
}

fn applyz80(e: &mut EmulatorZ80, regs: &CpuRegisters) {
    e.cpu.pc = regs.program_counter.min(0xFFFF) as u16;
    e.cpu.sp = regs.stack_pointer.min(0xFFFF) as u16;
    e.cpu.f = regs.status_register.min(0xFF) as u8;
    for (k, v) in &regs.general_purpose {
        let b = (*v).min(255) as u8;
        match k.as_str() {
            "A" => e.cpu.a = b,
            "F" => e.cpu.f = b,
            "B" => e.cpu.b = b,
            "C" => e.cpu.c = b,
            "D" => e.cpu.d = b,
            "E" => e.cpu.e = b,
            "H" => e.cpu.h = b,
            "L" => e.cpu.l = b,
            _ => {}
        }
    }
    if let Some(&ix) = regs.special.get("IX") {
        e.cpu.ix = ix.min(0xFFFF) as u16;
    }
    if let Some(&iy) = regs.special.get("IY") {
        e.cpu.iy = iy.min(0xFFFF) as u16;
    }
}

impl EmulatorTrait for EmulatorZ80 {
    fn name(&self) -> &'static str {
        "Z80 Emulator"
    }

    fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
        vec![LegacyArchitecture::ZilogZ80]
    }

    fn initialize<'a>(
        &'a mut self,
        config: &'a EmbeddedConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        ready(
            (|| {
                if config.architecture != LegacyArchitecture::ZilogZ80 {
                    return Err(EmbeddedEmulatorError::NotReady {
                        detail: format!("expected ZilogZ80, got {:?}", config.architecture),
                    });
                }
                self.initialized = true;
                self.cpu.halted = false;
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
                Ok(regsz80(self))
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
                applyz80(self, registers);
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
