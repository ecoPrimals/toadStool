// SPDX-License-Identifier: AGPL-3.0-only
//! Trait implementations for embedded programmers
//!
//! ## Planned / Future Implementation
//!
//! These programmer structs and trait implementations are **infrastructure placeholders**
//! for future hardware programmer support. They are registered in the embedded adapter
//! registry and satisfy the type system, but all operations (except no-op `disconnect`)
//! return [`crate::SpecialtyRuntimeError::EmbeddedProgrammerPlaceholder`] (mapped to
//! [`toadstool::ToadStoolError::not_supported`]) until hardware-specific implementations exist.
//!
//! Compile this module with Cargo feature **`embedded-placeholder-impls`** (enabled by default).
//!
//! **Tracking:** `# TODO(embedded-hw):` replace stubs when ISP/ICSP/parallel transports land.
//!
//! ## Architecture Notes
//!
//! - **GenericProgrammer**: ISP/ICSP interface; planned support for AVR, PIC, ARM
//! - **EPROMProgrammer**: Parallel port or USB; planned for 27xxx/28xxx EPROM/EEPROM
//!
//! Each programmer will require: transport layer (USB, parallel, serial), protocol
//! implementation (SPI, JTAG, proprietary), and device-specific algorithms.

use async_trait::async_trait;

use crate::{
    ProgrammingInterface, ProgrammingInterfaceType, SpecialtyRuntimeError, ToadStoolResult,
};
use toadstool::ToadStoolError;

use super::programmers::{EPROMProgrammer, GenericProgrammer};
use super::types::{ProgrammerInterface as ProgrammerTrait, TargetInfo};

const PROGRAMMER_PLACEHOLDER_DETAIL: &str =
    "requires hardware-specific programmer support (infrastructure placeholder; not implemented)";

fn programmer_placeholder_err(operation: &'static str) -> ToadStoolError {
    SpecialtyRuntimeError::EmbeddedProgrammerPlaceholder {
        operation,
        detail: PROGRAMMER_PLACEHOLDER_DETAIL,
    }
    .into()
}

/// Generates [`ProgrammerInterface`] impls that return structured placeholder errors (no panics).
///
/// **Tracking:** `# TODO(embedded-hw):` hardware programmer — see module-level docs.
macro_rules! impl_programmer_stub {
    ($programmer:ty, $name:expr, $interface:expr) => {
        // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
        #[async_trait]
        impl ProgrammerTrait for $programmer {
            fn name(&self) -> &'static str {
                $name
            }

            fn supported_interfaces(&self) -> Vec<ProgrammingInterfaceType> {
                vec![$interface]
            }

            async fn initialize(&mut self, _config: &ProgrammingInterface) -> ToadStoolResult<()> {
                Err(programmer_placeholder_err("Programmer initialization"))
            }

            async fn connect(&mut self) -> ToadStoolResult<()> {
                Err(programmer_placeholder_err("Programmer connect"))
            }

            async fn disconnect(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn read_memory(
                &mut self,
                _address: u32,
                _length: u32,
            ) -> ToadStoolResult<Vec<u8>> {
                Err(programmer_placeholder_err("Memory read"))
            }

            async fn write_memory(&mut self, _address: u32, _data: &[u8]) -> ToadStoolResult<()> {
                Err(programmer_placeholder_err("Memory write"))
            }

            async fn erase_memory(&mut self, _address: u32, _length: u32) -> ToadStoolResult<()> {
                Err(programmer_placeholder_err("Memory erase"))
            }

            async fn verify_memory(
                &mut self,
                _address: u32,
                _data: &[u8],
            ) -> ToadStoolResult<bool> {
                Err(programmer_placeholder_err("Memory verify"))
            }

            async fn get_target_info(&self) -> ToadStoolResult<TargetInfo> {
                Err(programmer_placeholder_err("Target info"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LegacyArchitecture, ProgrammingInterface, ProgrammingInterfaceType};
    use std::collections::HashMap;

    use crate::embedded::programmers::{EPROMProgrammer, GenericProgrammer};
    use crate::embedded::types::{ProgrammerInterface, TargetInfo};

    fn sample_programming_interface() -> ProgrammingInterface {
        ProgrammingInterface {
            interface_type: ProgrammingInterfaceType::ISP,
            connection_params: HashMap::new(),
        }
    }

    fn assert_not_supported_programmer(err: &ToadStoolError) {
        let msg = err.to_string();
        assert!(
            msg.contains("not supported"),
            "expected not-supported wording, got: {msg}"
        );
        assert!(
            msg.contains("hardware-specific") || msg.contains("Programmer"),
            "expected programmer stub reason, got: {msg}"
        );
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
    fn generic_programmer_new_default_debug() {
        let a = GenericProgrammer::new();
        let b = GenericProgrammer;
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert!(format!("{a:?}").contains("GenericProgrammer"));
    }

    #[test]
    fn eprom_programmer_new_default_debug() {
        let a = EPROMProgrammer::new();
        let b = EPROMProgrammer;
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert!(format!("{a:?}").contains("EPROMProgrammer"));
    }

    #[test]
    fn serde_roundtrip_types_used_by_programmer_trait() {
        let intf = sample_programming_interface();
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
    async fn generic_programmer_stub_returns_not_supported_except_disconnect() {
        let mut p = GenericProgrammer::new();
        assert_not_supported_programmer(
            &p.initialize(&sample_programming_interface())
                .await
                .expect_err("initialize"),
        );
        assert_not_supported_programmer(&p.connect().await.expect_err("connect"));
        p.disconnect().await.expect("disconnect");
        assert_not_supported_programmer(&p.read_memory(0, 4).await.expect_err("read_memory"));
        assert_not_supported_programmer(&p.write_memory(0, &[1]).await.expect_err("write_memory"));
        assert_not_supported_programmer(&p.erase_memory(0, 4).await.expect_err("erase_memory"));
        assert_not_supported_programmer(&p.verify_memory(0, &[1]).await.expect_err("verify"));
        assert_not_supported_programmer(&p.get_target_info().await.expect_err("get_target_info"));
    }

    #[tokio::test]
    async fn eprom_programmer_stub_returns_not_supported_except_disconnect() {
        let mut p = EPROMProgrammer::new();
        assert_not_supported_programmer(
            &p.initialize(&sample_programming_interface())
                .await
                .expect_err("initialize"),
        );
        assert_not_supported_programmer(&p.connect().await.expect_err("connect"));
        p.disconnect().await.expect("disconnect");
        assert_not_supported_programmer(&p.read_memory(0, 4).await.expect_err("read_memory"));
        assert_not_supported_programmer(&p.write_memory(0, &[1]).await.expect_err("write_memory"));
        assert_not_supported_programmer(&p.erase_memory(0, 4).await.expect_err("erase_memory"));
        assert_not_supported_programmer(&p.verify_memory(0, &[1]).await.expect_err("verify"));
        assert_not_supported_programmer(&p.get_target_info().await.expect_err("get_target_info"));
    }
}
