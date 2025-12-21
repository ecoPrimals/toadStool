//! Trait implementations for programmer placeholders
//!
//! Modern, idiomatic placeholder implementations for future embedded systems support.

use async_trait::async_trait;
use crate::{ToadStoolResult, ProgrammingInterfaceType, ProgrammingInterface, LegacyArchitecture};
use super::types::{ProgrammerInterface as ProgrammerTrait, VerificationResult};
use super::programmers::{GenericProgrammer, EPROMProgrammer};

/// Macro for implementing placeholder programmers (modern, DRY approach)
macro_rules! impl_placeholder_programmer {
    ($programmer:ty, $name:expr, $interface:expr) => {
        #[async_trait]
        impl ProgrammerTrait for $programmer {
            fn name(&self) -> &str {
                $name
            }

            fn supported_interfaces(&self) -> Vec<ProgrammingInterfaceType> {
                vec![$interface]
            }

            async fn initialize(&mut self, _config: &ProgrammingInterface) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn connect(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn disconnect(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn read_memory(&mut self, _address: u32, length: u32) -> ToadStoolResult<Vec<u8>> {
                // Placeholder: return zeros
                Ok(vec![0u8; length as usize])
            }

            async fn write_memory(&mut self, _address: u32, _data: &[u8]) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn erase_memory(&mut self, _address: u32, _length: u32) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn verify_memory(&mut self, _address: u32, _data: &[u8]) -> ToadStoolResult<VerificationResult> {
                // Placeholder: always verify successfully
                Ok(VerificationResult {
                    verified: true,
                    mismatches: vec![],
                    verification_time: std::time::Duration::from_millis(1),
                })
            }

            async fn get_target_info(&mut self) -> ToadStoolResult<crate::embedded::types::TargetInfo> {
                // Placeholder: return minimal target info
                Ok(crate::embedded::types::TargetInfo {
                    name: "Placeholder Target".to_string(),
                    architecture: LegacyArchitecture::MOS6502,
                    flash_size: 0,
                    ram_size: 0,
                    eeprom_size: None,
                    cpu_speed: 0,
                    features: vec![],
                })
            }
        }
    };
}

// Implement for all programmer types
impl_placeholder_programmer!(GenericProgrammer, "Generic Programmer (Placeholder)", ProgrammingInterfaceType::ISP);
impl_placeholder_programmer!(EPROMProgrammer, "EPROM Programmer (Placeholder)", ProgrammingInterfaceType::ParallelPort);

