// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dispatch enums for embedded toolchains, programmers, and emulators.

use std::future::Future;
use std::path::{Path, PathBuf};

#[cfg(feature = "embedded-placeholder-impls")]
use crate::embedded::emulators::{Emulator6502, EmulatorZ80};
#[cfg(feature = "embedded-placeholder-impls")]
use crate::embedded::programmers::{EPROMProgrammer, GenericProgrammer};
use crate::embedded::errors::{EmbeddedEmulatorError, EmbeddedProgrammerError};
use crate::embedded::toolchains::{
    Toolchain6502, Toolchain8051, Toolchain8080, Toolchain8086, Toolchain68000, ToolchainZ80,
};
use toadstool::ToadStoolError;

use crate::{
    EmbeddedConfig, LegacyArchitecture, MemoryLayout, ProgrammingInterface, SpecialtyRuntimeError,
    ToadStoolResult,
};

#[cfg(not(feature = "embedded-placeholder-impls"))]
fn programmer_unregistered() -> ToadStoolError {
    SpecialtyRuntimeError::from(EmbeddedProgrammerError::AdapterNotRegistered).into()
}

#[cfg(not(feature = "embedded-placeholder-impls"))]
fn emulator_unregistered() -> ToadStoolError {
    SpecialtyRuntimeError::from(EmbeddedEmulatorError::AdapterNotRegistered).into()
}

use super::interfaces::{
    CpuRegisters, EmbeddedEmulator, EmulationStatus, ProgrammerInterface, TargetInfo,
};
use super::job::{OutputFileType, SourceFile};
use super::toolchain::{CompilationResult, EmbeddedToolchain, LinkResult, MemoryMap};

/// Enum over all [`EmbeddedToolchain`] implementations.
#[derive(Debug)]
pub enum EmbeddedToolchainDispatch {
    /// MOS 6502 toolchain.
    Toolchain6502(Toolchain6502),
    /// Zilog Z80 toolchain.
    ToolchainZ80(ToolchainZ80),
    /// Intel 8080 toolchain.
    Toolchain8080(Toolchain8080),
    /// Intel 8051 toolchain.
    Toolchain8051(Toolchain8051),
    /// Intel 8086 toolchain.
    Toolchain8086(Toolchain8086),
    /// Motorola 68000 toolchain.
    Toolchain68000(Toolchain68000),
}

impl EmbeddedToolchain for EmbeddedToolchainDispatch {
    fn name(&self) -> &'static str {
        match self {
            EmbeddedToolchainDispatch::Toolchain6502(t) => t.name(),
            EmbeddedToolchainDispatch::ToolchainZ80(t) => t.name(),
            EmbeddedToolchainDispatch::Toolchain8080(t) => t.name(),
            EmbeddedToolchainDispatch::Toolchain8051(t) => t.name(),
            EmbeddedToolchainDispatch::Toolchain8086(t) => t.name(),
            EmbeddedToolchainDispatch::Toolchain68000(t) => t.name(),
        }
    }

    fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
        match self {
            EmbeddedToolchainDispatch::Toolchain6502(t) => t.supported_architectures(),
            EmbeddedToolchainDispatch::ToolchainZ80(t) => t.supported_architectures(),
            EmbeddedToolchainDispatch::Toolchain8080(t) => t.supported_architectures(),
            EmbeddedToolchainDispatch::Toolchain8051(t) => t.supported_architectures(),
            EmbeddedToolchainDispatch::Toolchain8086(t) => t.supported_architectures(),
            EmbeddedToolchainDispatch::Toolchain68000(t) => t.supported_architectures(),
        }
    }

    fn initialize<'a>(
        &'a mut self,
        config: &'a EmbeddedConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                EmbeddedToolchainDispatch::Toolchain6502(t) => t.initialize(config).await,
                EmbeddedToolchainDispatch::ToolchainZ80(t) => t.initialize(config).await,
                EmbeddedToolchainDispatch::Toolchain8080(t) => t.initialize(config).await,
                EmbeddedToolchainDispatch::Toolchain8051(t) => t.initialize(config).await,
                EmbeddedToolchainDispatch::Toolchain8086(t) => t.initialize(config).await,
                EmbeddedToolchainDispatch::Toolchain68000(t) => t.initialize(config).await,
            }
        }
    }

    fn compile<'a>(
        &'a self,
        sources: &'a [SourceFile],
        output_path: &'a Path,
    ) -> impl Future<Output = ToadStoolResult<CompilationResult>> + Send + 'a {
        async move {
            match self {
                EmbeddedToolchainDispatch::Toolchain6502(t) => {
                    t.compile(sources, output_path).await
                }
                EmbeddedToolchainDispatch::ToolchainZ80(t) => t.compile(sources, output_path).await,
                EmbeddedToolchainDispatch::Toolchain8080(t) => {
                    t.compile(sources, output_path).await
                }
                EmbeddedToolchainDispatch::Toolchain8051(t) => {
                    t.compile(sources, output_path).await
                }
                EmbeddedToolchainDispatch::Toolchain8086(t) => {
                    t.compile(sources, output_path).await
                }
                EmbeddedToolchainDispatch::Toolchain68000(t) => {
                    t.compile(sources, output_path).await
                }
            }
        }
    }

    fn link<'a>(
        &'a self,
        objects: &'a [PathBuf],
        output_path: &'a Path,
        memory_layout: &'a MemoryLayout,
    ) -> impl Future<Output = ToadStoolResult<LinkResult>> + Send + 'a {
        async move {
            match self {
                EmbeddedToolchainDispatch::Toolchain6502(t) => {
                    t.link(objects, output_path, memory_layout).await
                }
                EmbeddedToolchainDispatch::ToolchainZ80(t) => {
                    t.link(objects, output_path, memory_layout).await
                }
                EmbeddedToolchainDispatch::Toolchain8080(t) => {
                    t.link(objects, output_path, memory_layout).await
                }
                EmbeddedToolchainDispatch::Toolchain8051(t) => {
                    t.link(objects, output_path, memory_layout).await
                }
                EmbeddedToolchainDispatch::Toolchain8086(t) => {
                    t.link(objects, output_path, memory_layout).await
                }
                EmbeddedToolchainDispatch::Toolchain68000(t) => {
                    t.link(objects, output_path, memory_layout).await
                }
            }
        }
    }

    fn generate_rom_image<'a>(
        &'a self,
        executable: &'a Path,
        format: OutputFileType,
    ) -> impl Future<Output = ToadStoolResult<Vec<u8>>> + Send + 'a {
        async move {
            match self {
                EmbeddedToolchainDispatch::Toolchain6502(t) => {
                    t.generate_rom_image(executable, format).await
                }
                EmbeddedToolchainDispatch::ToolchainZ80(t) => {
                    t.generate_rom_image(executable, format).await
                }
                EmbeddedToolchainDispatch::Toolchain8080(t) => {
                    t.generate_rom_image(executable, format).await
                }
                EmbeddedToolchainDispatch::Toolchain8051(t) => {
                    t.generate_rom_image(executable, format).await
                }
                EmbeddedToolchainDispatch::Toolchain8086(t) => {
                    t.generate_rom_image(executable, format).await
                }
                EmbeddedToolchainDispatch::Toolchain68000(t) => {
                    t.generate_rom_image(executable, format).await
                }
            }
        }
    }

    fn disassemble<'a>(
        &'a self,
        binary: &'a [u8],
        start_address: u32,
    ) -> impl Future<Output = ToadStoolResult<String>> + Send + 'a {
        async move {
            match self {
                EmbeddedToolchainDispatch::Toolchain6502(t) => {
                    t.disassemble(binary, start_address).await
                }
                EmbeddedToolchainDispatch::ToolchainZ80(t) => {
                    t.disassemble(binary, start_address).await
                }
                EmbeddedToolchainDispatch::Toolchain8080(t) => {
                    t.disassemble(binary, start_address).await
                }
                EmbeddedToolchainDispatch::Toolchain8051(t) => {
                    t.disassemble(binary, start_address).await
                }
                EmbeddedToolchainDispatch::Toolchain8086(t) => {
                    t.disassemble(binary, start_address).await
                }
                EmbeddedToolchainDispatch::Toolchain68000(t) => {
                    t.disassemble(binary, start_address).await
                }
            }
        }
    }

    fn create_memory_map<'a>(
        &'a self,
        executable: &'a Path,
    ) -> impl Future<Output = ToadStoolResult<MemoryMap>> + Send + 'a {
        async move {
            match self {
                EmbeddedToolchainDispatch::Toolchain6502(t) => {
                    t.create_memory_map(executable).await
                }
                EmbeddedToolchainDispatch::ToolchainZ80(t) => t.create_memory_map(executable).await,
                EmbeddedToolchainDispatch::Toolchain8080(t) => {
                    t.create_memory_map(executable).await
                }
                EmbeddedToolchainDispatch::Toolchain8051(t) => {
                    t.create_memory_map(executable).await
                }
                EmbeddedToolchainDispatch::Toolchain8086(t) => {
                    t.create_memory_map(executable).await
                }
                EmbeddedToolchainDispatch::Toolchain68000(t) => {
                    t.create_memory_map(executable).await
                }
            }
        }
    }
}

/// Enum over all [`ProgrammerInterface`] implementations.
#[derive(Debug)]
pub enum ProgrammerInterfaceDispatch {
    /// Generic ISP-style programmer.
    #[cfg(feature = "embedded-placeholder-impls")]
    Generic(GenericProgrammer),
    /// EPROM programmer.
    #[cfg(feature = "embedded-placeholder-impls")]
    Eprom(EPROMProgrammer),
    /// Sentinel when [`embedded-placeholder-impls`] is disabled.
    #[cfg(not(feature = "embedded-placeholder-impls"))]
    Unregistered,
}

#[cfg_attr(
    not(feature = "embedded-placeholder-impls"),
    allow(unused_variables, reason = "trait params unused when no adapter is registered")
)]
impl ProgrammerInterface for ProgrammerInterfaceDispatch {
    fn name(&self) -> &'static str {
        match self {
            #[cfg(feature = "embedded-placeholder-impls")]
            ProgrammerInterfaceDispatch::Generic(p) => p.name(),
            #[cfg(feature = "embedded-placeholder-impls")]
            ProgrammerInterfaceDispatch::Eprom(p) => p.name(),
            #[cfg(not(feature = "embedded-placeholder-impls"))]
            ProgrammerInterfaceDispatch::Unregistered => "unregistered",
        }
    }

    fn supported_interfaces(&self) -> Vec<crate::ProgrammingInterfaceType> {
        match self {
            #[cfg(feature = "embedded-placeholder-impls")]
            ProgrammerInterfaceDispatch::Generic(p) => p.supported_interfaces(),
            #[cfg(feature = "embedded-placeholder-impls")]
            ProgrammerInterfaceDispatch::Eprom(p) => p.supported_interfaces(),
            #[cfg(not(feature = "embedded-placeholder-impls"))]
            ProgrammerInterfaceDispatch::Unregistered => vec![],
        }
    }

    fn initialize<'a>(
        &'a mut self,
        config: &'a ProgrammingInterface,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Generic(p) => p.initialize(config).await,
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Eprom(p) => p.initialize(config).await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                ProgrammerInterfaceDispatch::Unregistered => Err(programmer_unregistered()),
            }
        }
    }

    fn connect(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Generic(p) => p.connect().await,
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Eprom(p) => p.connect().await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                ProgrammerInterfaceDispatch::Unregistered => Err(programmer_unregistered()),
            }
        }
    }

    fn disconnect(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Generic(p) => p.disconnect().await,
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Eprom(p) => p.disconnect().await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                ProgrammerInterfaceDispatch::Unregistered => Err(programmer_unregistered()),
            }
        }
    }

    fn read_memory(
        &mut self,
        address: u32,
        length: u32,
    ) -> impl Future<Output = ToadStoolResult<Vec<u8>>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Generic(p) => p.read_memory(address, length).await,
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Eprom(p) => p.read_memory(address, length).await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                ProgrammerInterfaceDispatch::Unregistered => Err(programmer_unregistered()),
            }
        }
    }

    fn write_memory<'a>(
        &'a mut self,
        address: u32,
        data: &'a [u8],
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Generic(p) => p.write_memory(address, data).await,
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Eprom(p) => p.write_memory(address, data).await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                ProgrammerInterfaceDispatch::Unregistered => Err(programmer_unregistered()),
            }
        }
    }

    fn erase_memory(
        &mut self,
        address: u32,
        length: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Generic(p) => p.erase_memory(address, length).await,
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Eprom(p) => p.erase_memory(address, length).await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                ProgrammerInterfaceDispatch::Unregistered => Err(programmer_unregistered()),
            }
        }
    }

    fn verify_memory<'a>(
        &'a mut self,
        address: u32,
        expected_data: &'a [u8],
    ) -> impl Future<Output = ToadStoolResult<bool>> + Send + 'a {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Generic(p) => {
                    p.verify_memory(address, expected_data).await
                }
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Eprom(p) => {
                    p.verify_memory(address, expected_data).await
                }
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                ProgrammerInterfaceDispatch::Unregistered => Err(programmer_unregistered()),
            }
        }
    }

    fn get_target_info(&self) -> impl Future<Output = ToadStoolResult<TargetInfo>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Generic(p) => p.get_target_info().await,
                #[cfg(feature = "embedded-placeholder-impls")]
                ProgrammerInterfaceDispatch::Eprom(p) => p.get_target_info().await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                ProgrammerInterfaceDispatch::Unregistered => Err(programmer_unregistered()),
            }
        }
    }
}

/// Enum over all [`EmbeddedEmulator`] implementations.
#[derive(Debug)]
pub enum EmbeddedEmulatorDispatch {
    /// 6502 emulator stub.
    #[cfg(feature = "embedded-placeholder-impls")]
    Emulator6502(Emulator6502),
    /// Z80 emulator stub.
    #[cfg(feature = "embedded-placeholder-impls")]
    EmulatorZ80(EmulatorZ80),
    /// Sentinel when [`embedded-placeholder-impls`] is disabled.
    #[cfg(not(feature = "embedded-placeholder-impls"))]
    Unregistered,
}

#[cfg_attr(
    not(feature = "embedded-placeholder-impls"),
    allow(unused_variables, reason = "trait params unused when no adapter is registered")
)]
impl EmbeddedEmulator for EmbeddedEmulatorDispatch {
    fn name(&self) -> &'static str {
        match self {
            #[cfg(feature = "embedded-placeholder-impls")]
            EmbeddedEmulatorDispatch::Emulator6502(e) => e.name(),
            #[cfg(feature = "embedded-placeholder-impls")]
            EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.name(),
            #[cfg(not(feature = "embedded-placeholder-impls"))]
            EmbeddedEmulatorDispatch::Unregistered => "unregistered",
        }
    }

    fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
        match self {
            #[cfg(feature = "embedded-placeholder-impls")]
            EmbeddedEmulatorDispatch::Emulator6502(e) => e.supported_architectures(),
            #[cfg(feature = "embedded-placeholder-impls")]
            EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.supported_architectures(),
            #[cfg(not(feature = "embedded-placeholder-impls"))]
            EmbeddedEmulatorDispatch::Unregistered => vec![],
        }
    }

    fn initialize<'a>(
        &'a mut self,
        config: &'a EmbeddedConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => e.initialize(config).await,
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.initialize(config).await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }

    fn load_rom<'a>(
        &'a mut self,
        rom_data: &'a [u8],
        load_address: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => {
                    e.load_rom(rom_data, load_address).await
                }
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => {
                    e.load_rom(rom_data, load_address).await
                }
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }

    fn start(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => e.start().await,
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.start().await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }

    fn stop(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => e.stop().await,
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.stop().await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }

    fn step(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => e.step().await,
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.step().await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }

    fn set_breakpoint(
        &mut self,
        address: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => e.set_breakpoint(address).await,
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.set_breakpoint(address).await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }

    fn clear_breakpoint(
        &mut self,
        address: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => e.clear_breakpoint(address).await,
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.clear_breakpoint(address).await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }

    fn read_registers(&self) -> impl Future<Output = ToadStoolResult<CpuRegisters>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => e.read_registers().await,
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.read_registers().await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }

    fn write_registers<'a>(
        &'a mut self,
        registers: &'a CpuRegisters,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => e.write_registers(registers).await,
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.write_registers(registers).await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }

    fn read_memory(
        &self,
        address: u32,
        length: u32,
    ) -> impl Future<Output = ToadStoolResult<Vec<u8>>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => e.read_memory(address, length).await,
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.read_memory(address, length).await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }

    fn write_memory<'a>(
        &'a mut self,
        address: u32,
        data: &'a [u8],
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => e.write_memory(address, data).await,
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.write_memory(address, data).await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }

    fn get_status(&self) -> impl Future<Output = ToadStoolResult<EmulationStatus>> + Send + '_ {
        async move {
            match self {
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::Emulator6502(e) => e.get_status().await,
                #[cfg(feature = "embedded-placeholder-impls")]
                EmbeddedEmulatorDispatch::EmulatorZ80(e) => e.get_status().await,
                #[cfg(not(feature = "embedded-placeholder-impls"))]
                EmbeddedEmulatorDispatch::Unregistered => Err(emulator_unregistered()),
            }
        }
    }
}
