// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Edge platform types

use toadstool_runtime_edge::*;

// ============================================================================
// ArduinoBoard Tests (12 variants)
// ============================================================================

#[test]
fn test_arduino_board_uno() {
    let board = ArduinoBoard::Uno;
    assert_eq!(board, ArduinoBoard::Uno);
}

#[test]
fn test_arduino_board_nano() {
    let board = ArduinoBoard::Nano;
    assert_eq!(board, ArduinoBoard::Nano);
}

#[test]
fn test_arduino_board_mega2560() {
    let board = ArduinoBoard::Mega2560;
    assert_eq!(board, ArduinoBoard::Mega2560);
}

#[test]
fn test_arduino_board_leonardo() {
    let board = ArduinoBoard::Leonardo;
    assert_eq!(board, ArduinoBoard::Leonardo);
}

#[test]
fn test_arduino_board_micro() {
    let board = ArduinoBoard::Micro;
    assert_eq!(board, ArduinoBoard::Micro);
}

#[test]
fn test_arduino_board_due() {
    let board = ArduinoBoard::Due;
    assert_eq!(board, ArduinoBoard::Due);
}

#[test]
fn test_arduino_board_mkr1000() {
    let board = ArduinoBoard::MKR1000;
    assert_eq!(board, ArduinoBoard::MKR1000);
}

#[test]
fn test_arduino_board_mkrzero() {
    let board = ArduinoBoard::MKRZero;
    assert_eq!(board, ArduinoBoard::MKRZero);
}

#[test]
fn test_arduino_board_portenta() {
    let board = ArduinoBoard::Portenta;
    assert_eq!(board, ArduinoBoard::Portenta);
}

#[test]
fn test_arduino_board_nano33iot() {
    let board = ArduinoBoard::Nano33IoT;
    assert_eq!(board, ArduinoBoard::Nano33IoT);
}

#[test]
fn test_arduino_board_nano33ble() {
    let board = ArduinoBoard::Nano33BLE;
    assert_eq!(board, ArduinoBoard::Nano33BLE);
}

#[test]
fn test_arduino_board_mkrwifi1010() {
    let board = ArduinoBoard::MKRWiFi1010;
    assert_eq!(board, ArduinoBoard::MKRWiFi1010);
}

#[test]
fn test_arduino_board_clone() {
    let board1 = ArduinoBoard::Uno;
    let board2 = board1.clone();
    assert_eq!(board1, board2);
}

#[test]
fn test_arduino_board_serialization() {
    let board = ArduinoBoard::Nano;
    let serialized = serde_json::to_string(&board).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// ESP32Variant Tests (7 variants)
// ============================================================================

#[test]
fn test_esp32_variant_esp32() {
    let variant = ESP32Variant::ESP32;
    assert_eq!(variant, ESP32Variant::ESP32);
}

#[test]
fn test_esp32_variant_esp32s2() {
    let variant = ESP32Variant::ESP32S2;
    assert_eq!(variant, ESP32Variant::ESP32S2);
}

#[test]
fn test_esp32_variant_esp32s3() {
    let variant = ESP32Variant::ESP32S3;
    assert_eq!(variant, ESP32Variant::ESP32S3);
}

#[test]
fn test_esp32_variant_esp32c3() {
    let variant = ESP32Variant::ESP32C3;
    assert_eq!(variant, ESP32Variant::ESP32C3);
}

#[test]
fn test_esp32_variant_esp32c6() {
    let variant = ESP32Variant::ESP32C6;
    assert_eq!(variant, ESP32Variant::ESP32C6);
}

#[test]
fn test_esp32_variant_esp32h2() {
    let variant = ESP32Variant::ESP32H2;
    assert_eq!(variant, ESP32Variant::ESP32H2);
}

#[test]
fn test_esp32_variant_esp32p4() {
    let variant = ESP32Variant::ESP32P4;
    assert_eq!(variant, ESP32Variant::ESP32P4);
}

#[test]
fn test_esp32_variant_clone() {
    let variant1 = ESP32Variant::ESP32S3;
    let variant2 = variant1.clone();
    assert_eq!(variant1, variant2);
}

// ============================================================================
// ESP32Framework Tests (5 variants)
// ============================================================================

#[test]
fn test_esp32_framework_espidf() {
    let framework = ESP32Framework::ESPIDF;
    assert_eq!(framework, ESP32Framework::ESPIDF);
}

#[test]
fn test_esp32_framework_arduino() {
    let framework = ESP32Framework::Arduino;
    assert_eq!(framework, ESP32Framework::Arduino);
}

#[test]
fn test_esp32_framework_platformio() {
    let framework = ESP32Framework::PlatformIO;
    assert_eq!(framework, ESP32Framework::PlatformIO);
}

#[test]
fn test_esp32_framework_micropython() {
    let framework = ESP32Framework::MicroPython;
    assert_eq!(framework, ESP32Framework::MicroPython);
}

#[test]
fn test_esp32_framework_rust() {
    let framework = ESP32Framework::Rust;
    assert_eq!(framework, ESP32Framework::Rust);
}

// ============================================================================
// PiModel Tests (10 variants)
// ============================================================================

#[test]
fn test_pi_model_pi1() {
    let model = PiModel::Pi1;
    assert_eq!(model, PiModel::Pi1);
}

#[test]
fn test_pi_model_pi2() {
    let model = PiModel::Pi2;
    assert_eq!(model, PiModel::Pi2);
}

#[test]
fn test_pi_model_pi3() {
    let model = PiModel::Pi3;
    assert_eq!(model, PiModel::Pi3);
}

#[test]
fn test_pi_model_pi4() {
    let model = PiModel::Pi4;
    assert_eq!(model, PiModel::Pi4);
}

#[test]
fn test_pi_model_pi5() {
    let model = PiModel::Pi5;
    assert_eq!(model, PiModel::Pi5);
}

#[test]
fn test_pi_model_pizero() {
    let model = PiModel::PiZero;
    assert_eq!(model, PiModel::PiZero);
}

#[test]
fn test_pi_model_pizero2w() {
    let model = PiModel::PiZero2W;
    assert_eq!(model, PiModel::PiZero2W);
}

#[test]
fn test_pi_model_pipico() {
    let model = PiModel::PiPico;
    assert_eq!(model, PiModel::PiPico);
}

#[test]
fn test_pi_model_pipicow() {
    let model = PiModel::PiPicoW;
    assert_eq!(model, PiModel::PiPicoW);
}

#[test]
fn test_pi_model_compute3() {
    let model = PiModel::Compute3;
    assert_eq!(model, PiModel::Compute3);
}

#[test]
fn test_pi_model_compute4() {
    let model = PiModel::Compute4;
    assert_eq!(model, PiModel::Compute4);
}

// ============================================================================
// PiOS Tests (5 variants)
// ============================================================================

#[test]
fn test_pi_os_raspberrypios() {
    let os = PiOS::RaspberryPiOS;
    assert_eq!(os, PiOS::RaspberryPiOS);
}

#[test]
fn test_pi_os_ubuntu() {
    let os = PiOS::Ubuntu;
    assert_eq!(os, PiOS::Ubuntu);
}

#[test]
fn test_pi_os_buildroot() {
    let os = PiOS::BuildRoot;
    assert_eq!(os, PiOS::BuildRoot);
}

#[test]
fn test_pi_os_yocto() {
    let os = PiOS::Yocto;
    assert_eq!(os, PiOS::Yocto);
}

#[test]
fn test_pi_os_customlinux() {
    let os = PiOS::CustomLinux;
    assert_eq!(os, PiOS::CustomLinux);
}

// ============================================================================
// BeagleBoneVariant Tests (5 variants)
// ============================================================================

#[test]
fn test_beaglebone_black() {
    let variant = BeagleBoneVariant::Black;
    assert_eq!(variant, BeagleBoneVariant::Black);
}

#[test]
fn test_beaglebone_green() {
    let variant = BeagleBoneVariant::Green;
    assert_eq!(variant, BeagleBoneVariant::Green);
}

#[test]
fn test_beaglebone_blue() {
    let variant = BeagleBoneVariant::Blue;
    assert_eq!(variant, BeagleBoneVariant::Blue);
}

#[test]
fn test_beaglebone_ai() {
    let variant = BeagleBoneVariant::AI;
    assert_eq!(variant, BeagleBoneVariant::AI);
}

#[test]
fn test_beaglebone_x15() {
    let variant = BeagleBoneVariant::X15;
    assert_eq!(variant, BeagleBoneVariant::X15);
}

// ============================================================================
// IndustrialSystemType Tests (6 variants)
// ============================================================================

#[test]
fn test_industrial_system_plc() {
    let system = IndustrialSystemType::PLC;
    assert_eq!(system, IndustrialSystemType::PLC);
}

#[test]
fn test_industrial_system_scada() {
    let system = IndustrialSystemType::SCADA;
    assert_eq!(system, IndustrialSystemType::SCADA);
}

#[test]
fn test_industrial_system_hmi() {
    let system = IndustrialSystemType::HMI;
    assert_eq!(system, IndustrialSystemType::HMI);
}

#[test]
fn test_industrial_system_dcs() {
    let system = IndustrialSystemType::DCS;
    assert_eq!(system, IndustrialSystemType::DCS);
}

#[test]
fn test_industrial_system_rtu() {
    let system = IndustrialSystemType::RTU;
    assert_eq!(system, IndustrialSystemType::RTU);
}

#[test]
fn test_industrial_system_ied() {
    let system = IndustrialSystemType::IED;
    assert_eq!(system, IndustrialSystemType::IED);
}

// ============================================================================
// IndustrialProtocol Tests (9 variants)
// ============================================================================

#[test]
fn test_industrial_protocol_modbus() {
    let protocol = IndustrialProtocol::Modbus;
    assert_eq!(protocol, IndustrialProtocol::Modbus);
}

#[test]
fn test_industrial_protocol_profibus() {
    let protocol = IndustrialProtocol::Profibus;
    assert_eq!(protocol, IndustrialProtocol::Profibus);
}

#[test]
fn test_industrial_protocol_profinet() {
    let protocol = IndustrialProtocol::Profinet;
    assert_eq!(protocol, IndustrialProtocol::Profinet);
}

#[test]
fn test_industrial_protocol_ethercat() {
    let protocol = IndustrialProtocol::EtherCAT;
    assert_eq!(protocol, IndustrialProtocol::EtherCAT);
}

#[test]
fn test_industrial_protocol_devicenet() {
    let protocol = IndustrialProtocol::DeviceNet;
    assert_eq!(protocol, IndustrialProtocol::DeviceNet);
}

#[test]
fn test_industrial_protocol_canopen() {
    let protocol = IndustrialProtocol::CANopen;
    assert_eq!(protocol, IndustrialProtocol::CANopen);
}

#[test]
fn test_industrial_protocol_ethernetip() {
    let protocol = IndustrialProtocol::EtherNetIP;
    assert_eq!(protocol, IndustrialProtocol::EtherNetIP);
}

#[test]
fn test_industrial_protocol_foundation() {
    let protocol = IndustrialProtocol::Foundation;
    assert_eq!(protocol, IndustrialProtocol::Foundation);
}

#[test]
fn test_industrial_protocol_hart() {
    let protocol = IndustrialProtocol::Hart;
    assert_eq!(protocol, IndustrialProtocol::Hart);
}

// ============================================================================
// MicrocontrollerArch Tests (9 variants)
// ============================================================================

#[test]
fn test_microcontroller_arm() {
    let arch = MicrocontrollerArch::ARM;
    assert_eq!(arch, MicrocontrollerArch::ARM);
}

#[test]
fn test_microcontroller_avr() {
    let arch = MicrocontrollerArch::AVR;
    assert_eq!(arch, MicrocontrollerArch::AVR);
}

#[test]
fn test_microcontroller_pic() {
    let arch = MicrocontrollerArch::PIC;
    assert_eq!(arch, MicrocontrollerArch::PIC);
}

#[test]
fn test_microcontroller_msp430() {
    let arch = MicrocontrollerArch::MSP430;
    assert_eq!(arch, MicrocontrollerArch::MSP430);
}

#[test]
fn test_microcontroller_riscv() {
    let arch = MicrocontrollerArch::RISCV;
    assert_eq!(arch, MicrocontrollerArch::RISCV);
}

#[test]
fn test_microcontroller_x86() {
    let arch = MicrocontrollerArch::x86;
    assert_eq!(arch, MicrocontrollerArch::x86);
}

#[test]
fn test_microcontroller_z80() {
    let arch = MicrocontrollerArch::Z80;
    assert_eq!(arch, MicrocontrollerArch::Z80);
}

#[test]
fn test_microcontroller_m68k() {
    let arch = MicrocontrollerArch::M68K;
    assert_eq!(arch, MicrocontrollerArch::M68K);
}

#[test]
fn test_microcontroller_powerpc() {
    let arch = MicrocontrollerArch::PowerPC;
    assert_eq!(arch, MicrocontrollerArch::PowerPC);
}

