// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trait implementations for embedded emulators (MOS 6502 and Zilog Z80 cores).
//!
//! See DEBT.md `D-EMBEDDED-EMULATOR` for GDB / remote-debug transport tracking.

use std::collections::HashMap;
use std::future::{Future, ready};

use crate::{EmbeddedConfig, LegacyArchitecture, SpecialtyRuntimeError, ToadStoolResult};
use toadstool::ToadStoolError;

use super::emulators::{Emulator6502, EmulatorZ80};
use super::errors::EmbeddedEmulatorError;
use super::types::{CpuRegisters, EmbeddedEmulator as EmulatorTrait, EmulationStatus};

fn emulator_err(e: EmbeddedEmulatorError) -> ToadStoolError {
    SpecialtyRuntimeError::from(e).into()
}

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

fn copy_mem_range(mem: &[u8], addr: u32, len: u32) -> Result<Vec<u8>, EmbeddedEmulatorError> {
    let end = u64::from(addr).checked_add(u64::from(len)).ok_or_else(|| {
        EmbeddedEmulatorError::NotReady {
            detail: "memory read overflow".into(),
        }
    })?;
    if end > 65536 {
        return Err(EmbeddedEmulatorError::NotReady {
            detail: "memory read past 64K".into(),
        });
    }
    let start = addr as usize;
    let end = end as usize;
    Ok(mem[start..end].to_vec())
}

fn write_mem_range(mem: &mut [u8], addr: u32, data: &[u8]) -> Result<(), EmbeddedEmulatorError> {
    let end = u64::from(addr)
        .checked_add(data.len() as u64)
        .ok_or_else(|| EmbeddedEmulatorError::NotReady {
            detail: "memory write overflow".into(),
        })?;
    if end > 65536 {
        return Err(EmbeddedEmulatorError::NotReady {
            detail: "memory write past 64K".into(),
        });
    }
    let start = addr as usize;
    mem[start..start + data.len()].copy_from_slice(data);
    Ok(())
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

#[cfg(test)]
mod tests {
    use crate::{
        EmbeddedConfig, LegacyArchitecture, MemoryLayout, ProgrammingInterface,
        ProgrammingInterfaceType,
    };
    use std::collections::HashMap;

    use crate::embedded::emulators::{Emulator6502, EmulatorZ80};
    use crate::embedded::types::{CpuRegisters, EmbeddedEmulator, EmulationStatus};

    fn minimal_embedded_config() -> EmbeddedConfig {
        EmbeddedConfig {
            architecture: LegacyArchitecture::MOS6502,
            memory_layout: MemoryLayout {
                rom_regions: vec![],
                ram_regions: vec![],
                io_regions: vec![],
            },
            peripherals: vec![],
            programming_interface: ProgrammingInterface {
                interface_type: ProgrammingInterfaceType::ISP,
                connection_params: HashMap::new(),
            },
        }
    }

    fn z80_embedded_config() -> EmbeddedConfig {
        EmbeddedConfig {
            architecture: LegacyArchitecture::ZilogZ80,
            memory_layout: MemoryLayout {
                rom_regions: vec![],
                ram_regions: vec![],
                io_regions: vec![],
            },
            peripherals: vec![],
            programming_interface: ProgrammingInterface {
                interface_type: ProgrammingInterfaceType::ISP,
                connection_params: HashMap::new(),
            },
        }
    }

    fn assert_serde_json_stable<T>(value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
    {
        let json = serde_json::to_string(value).expect("serde_json serialize");
        let back: T = serde_json::from_str(&json).expect("serde_json deserialize");
        let json_again = serde_json::to_string(&back).expect("serde_json re-serialize");
        assert_eq!(json, json_again);
    }

    #[test]
    fn emulator_6502_new_default_debug() {
        let a = Emulator6502::new();
        let b = Emulator6502::default();
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        let s = format!("{a:?}");
        assert!(s.contains("Emulator6502"), "{s}");
    }

    #[test]
    fn emulator_z80_new_default_debug() {
        let a = EmulatorZ80::new();
        let b = EmulatorZ80::default();
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        let s = format!("{a:?}");
        assert!(s.contains("EmulatorZ80"), "{s}");
    }

    #[test]
    fn serde_roundtrip_types_used_by_emulator_trait() {
        let cfg = minimal_embedded_config();
        assert_serde_json_stable(&cfg);
        let pi = ProgrammingInterface {
            interface_type: ProgrammingInterfaceType::ISP,
            connection_params: HashMap::from([("port".to_string(), "/dev/ttyUSB0".to_string())]),
        };
        assert_serde_json_stable(&pi);
        assert_serde_json_stable(&ProgrammingInterfaceType::Parallel);
        let regs = CpuRegisters {
            general_purpose: HashMap::from([("A".to_string(), 0x42)]),
            program_counter: 0x8000,
            stack_pointer: 0x100,
            status_register: 0,
            special: HashMap::new(),
        };
        assert_serde_json_stable(&regs);
        assert_serde_json_stable(&EmulationStatus::Running);
        assert_serde_json_stable(&EmulationStatus::Stopped);
        assert_serde_json_stable(&EmulationStatus::Breakpoint { address: 0x2000 });
        assert_serde_json_stable(&EmulationStatus::Error {
            message: "boom".to_string(),
        });
    }

    #[test]
    fn emulator_6502_trait_name_and_architectures() {
        let e = Emulator6502::new();
        assert_eq!(EmbeddedEmulator::name(&e), "6502 Emulator");
        assert_eq!(
            EmbeddedEmulator::supported_architectures(&e),
            vec![LegacyArchitecture::MOS6502]
        );
    }

    #[test]
    fn emulator_z80_trait_name_and_architectures() {
        let e = EmulatorZ80::new();
        assert_eq!(EmbeddedEmulator::name(&e), "Z80 Emulator");
        assert_eq!(
            EmbeddedEmulator::supported_architectures(&e),
            vec![LegacyArchitecture::ZilogZ80]
        );
    }

    #[tokio::test]
    async fn emulator_6502_runs_loaded_rom() {
        let mut e = Emulator6502::new();
        let cfg = minimal_embedded_config();
        e.initialize(&cfg).await.expect("init");
        e.load_rom(&[0xA9, 0x42, 0xEA], 0x0400).await.expect("load");
        e.cpu.mem[0xFFFC] = 0x00;
        e.cpu.mem[0xFFFD] = 0x04;
        e.cpu.reset();
        e.start().await.expect("start");
        e.step().await.expect("step");
        let regs = e.read_registers().await.expect("regs");
        assert_eq!(regs.general_purpose.get("A"), Some(&0x42));
    }

    #[tokio::test]
    async fn emulator_6502_breakpoint_stops() {
        let mut e = Emulator6502::new();
        e.initialize(&minimal_embedded_config())
            .await
            .expect("init");
        e.load_rom(&[0xEA, 0xEA], 0x0400).await.expect("load");
        e.cpu.mem[0xFFFC] = 0x00;
        e.cpu.mem[0xFFFD] = 0x04;
        e.cpu.reset();
        e.set_breakpoint(0x0401).await.expect("bp");
        e.start().await.expect("start");
        e.step().await.expect("s1");
        let st = e.get_status().await.expect("st");
        assert!(matches!(
            st,
            EmulationStatus::Breakpoint { address: 0x0401 }
        ));
    }

    #[tokio::test]
    async fn emulator_z80_step() {
        let mut e = EmulatorZ80::new();
        e.initialize(&z80_embedded_config()).await.expect("init");
        e.load_rom(&[0x3E, 0x07], 0x0000).await.expect("load");
        e.start().await.expect("start");
        e.step().await.expect("step");
        let regs = e.read_registers().await.expect("regs");
        assert_eq!(regs.general_purpose.get("A"), Some(&7));
    }
}
