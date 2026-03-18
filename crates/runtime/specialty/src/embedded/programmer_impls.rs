// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trait implementations for embedded programmers
//!
//! ## Planned / Future Implementation
//!
//! These programmer structs and trait implementations are **infrastructure placeholders**
//! for future hardware programmer support. They are registered in the embedded adapter
//! registry and satisfy the type system, but all operations (except no-op `disconnect`)
//! return `not_supported` until hardware-specific implementations are available.
//!
//! ## Architecture Notes
//!
//! - **GenericProgrammer**: ISP/ICSP interface; planned support for AVR, PIC, ARM
//! - **EPROMProgrammer**: Parallel port or USB; planned for 27xxx/28xxx EPROM/EEPROM
//!
//! Each programmer will require: transport layer (USB, parallel, serial), protocol
//! implementation (SPI, JTAG, proprietary), and device-specific algorithms.

use async_trait::async_trait;

use crate::{ProgrammingInterface, ProgrammingInterfaceType, ToadStoolResult};
use toadstool::ToadStoolError;

use super::programmers::{EPROMProgrammer, GenericProgrammer};
use super::types::{ProgrammerInterface as ProgrammerTrait, TargetInfo};

fn not_implemented(feature: &str) -> ToadStoolError {
    ToadStoolError::not_supported(format!(
        "{feature} not yet implemented; requires hardware-specific programmer support"
    ))
}

macro_rules! impl_programmer_stub {
    ($programmer:ty, $name:expr, $interface:expr) => {
        // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
        #[async_trait]
        impl ProgrammerTrait for $programmer {
            fn name(&self) -> &str {
                $name
            }

            fn supported_interfaces(&self) -> Vec<ProgrammingInterfaceType> {
                vec![$interface]
            }

            async fn initialize(&mut self, _config: &ProgrammingInterface) -> ToadStoolResult<()> {
                Err(not_implemented("Programmer initialization"))
            }

            async fn connect(&mut self) -> ToadStoolResult<()> {
                Err(not_implemented("Programmer connect"))
            }

            async fn disconnect(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn read_memory(
                &mut self,
                _address: u32,
                _length: u32,
            ) -> ToadStoolResult<Vec<u8>> {
                Err(not_implemented("Memory read"))
            }

            async fn write_memory(&mut self, _address: u32, _data: &[u8]) -> ToadStoolResult<()> {
                Err(not_implemented("Memory write"))
            }

            async fn erase_memory(&mut self, _address: u32, _length: u32) -> ToadStoolResult<()> {
                Err(not_implemented("Memory erase"))
            }

            async fn verify_memory(
                &mut self,
                _address: u32,
                _data: &[u8],
            ) -> ToadStoolResult<bool> {
                Err(not_implemented("Memory verify"))
            }

            async fn get_target_info(&self) -> ToadStoolResult<TargetInfo> {
                Err(not_implemented("Target info"))
            }
        }
    };
}

impl_programmer_stub!(
    GenericProgrammer,
    "Generic Programmer",
    ProgrammingInterfaceType::ISP
);
impl_programmer_stub!(
    EPROMProgrammer,
    "EPROM Programmer",
    ProgrammingInterfaceType::Parallel
);
