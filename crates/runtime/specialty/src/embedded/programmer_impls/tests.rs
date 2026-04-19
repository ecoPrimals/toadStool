// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{LegacyArchitecture, ProgrammingInterface, ProgrammingInterfaceType, ToadStoolError};
use std::collections::HashMap;

use crate::embedded::programmers::{EPROMProgrammer, GenericProgrammer};
use crate::embedded::types::{ProgrammerInterface, TargetInfo};

fn avr_config() -> ProgrammingInterface {
    ProgrammingInterface {
        interface_type: ProgrammingInterfaceType::ISP,
        connection_params: HashMap::from([
            ("family".to_string(), "avr".to_string()),
            ("chip".to_string(), "ATmega328P".to_string()),
            ("clock_hz".to_string(), "250000".to_string()),
            ("voltage_mv".to_string(), "5000".to_string()),
        ]),
    }
}

fn assert_not_supported_programmer(err: &ToadStoolError) {
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("not supported"),
        "expected not-supported wording, got: {msg}"
    );
}

fn assert_serde_json_stable<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    let v1 = serde_json::to_value(value).expect("serde_json to_value");
    let back: T = serde_json::from_value(v1.clone()).expect("serde_json from_value");
    let v2 = serde_json::to_value(&back).expect("serde_json to_value roundtrip");
    assert_eq!(v1, v2);
}

#[test]
fn generic_programmer_new_default_debug() {
    let a = GenericProgrammer::new();
    let b = GenericProgrammer::default();
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    assert!(format!("{a:?}").contains("GenericProgrammer"));
}

#[test]
fn eprom_programmer_new_default_debug() {
    let a = EPROMProgrammer::new();
    let b = EPROMProgrammer::default();
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    assert!(format!("{a:?}").contains("EPROMProgrammer"));
}

#[test]
fn serde_roundtrip_types_used_by_programmer_trait() {
    let intf = avr_config();
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
async fn generic_programmer_init_connect_without_transport() {
    let mut p = GenericProgrammer::new();
    p.initialize(&avr_config()).await.expect("init");
    p.connect().await.expect("connect");
    p.disconnect().await.expect("disconnect");
}

#[tokio::test]
async fn generic_programmer_read_needs_transport_after_connect() {
    let mut p = GenericProgrammer::new();
    p.initialize(&avr_config()).await.expect("init");
    p.connect().await.expect("connect");
    let err = p.read_memory(0, 4).await.expect_err("read");
    assert_not_supported_programmer(&err);
    assert!(
        err.to_string().contains("transport") || err.to_string().contains("Transport"),
        "{}",
        err
    );
}

#[tokio::test]
async fn generic_programmer_init_fails_without_keys() {
    let mut p = GenericProgrammer::new();
    let bad = ProgrammingInterface {
        interface_type: ProgrammingInterfaceType::ISP,
        connection_params: HashMap::new(),
    };
    let err = p.initialize(&bad).await.expect_err("init");
    assert_not_supported_programmer(&err);
}

#[tokio::test]
async fn eprom_programmer_lifecycle() {
    let mut p = EPROMProgrammer::new();
    let cfg = ProgrammingInterface {
        interface_type: ProgrammingInterfaceType::Parallel,
        connection_params: HashMap::from([
            ("chip".to_string(), "27C512".to_string()),
            ("voltage_mv".to_string(), "5000".to_string()),
        ]),
    };
    p.initialize(&cfg).await.expect("init");
    p.connect().await.expect("connect");
    let err = p.read_memory(0, 4).await.expect_err("read");
    assert_not_supported_programmer(&err);
    p.disconnect().await.expect("disconnect");
}
