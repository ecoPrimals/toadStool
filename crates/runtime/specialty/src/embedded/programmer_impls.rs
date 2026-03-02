//! Trait implementations for embedded programmers
//!
//! Error-returning stubs until hardware-specific implementations are available.
//! All hardware operations return `not_supported` to clearly communicate status.

use async_trait::async_trait;

use crate::{LegacyArchitecture, ProgrammingInterface, ProgrammingInterfaceType, ToadStoolResult};
use toadstool::ToadStoolError;

use super::programmers::{EPROMProgrammer, GenericProgrammer};
use super::types::{ProgrammerInterface as ProgrammerTrait, VerificationResult};

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

            async fn read_memory(&mut self, _address: u32, _length: u32) -> ToadStoolResult<Vec<u8>> {
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
            ) -> ToadStoolResult<VerificationResult> {
                Err(not_implemented("Memory verify"))
            }

            async fn get_target_info(
                &mut self,
            ) -> ToadStoolResult<crate::embedded::types::TargetInfo> {
                Err(not_implemented("Target info"))
            }
        }
    };
}

impl_programmer_stub!(GenericProgrammer, "Generic Programmer", ProgrammingInterfaceType::ISP);
impl_programmer_stub!(EPROMProgrammer, "EPROM Programmer", ProgrammingInterfaceType::ParallelPort);

