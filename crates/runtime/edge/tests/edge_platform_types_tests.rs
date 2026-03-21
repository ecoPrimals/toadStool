// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for edge platform types

use toadstool_runtime_edge::platforms::*;
use toadstool::IsolationLevel;

// ============================================================================
// ArduinoBoard Tests
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
fn test_arduino_board_due() {
    let board = ArduinoBoard::Due;
    assert_eq!(board, ArduinoBoard::Due);
}

#[test]
fn test_arduino_board_portenta() {
    let board = ArduinoBoard::Portenta;
    assert_eq!(board, ArduinoBoard::Portenta);
}

#[test]
fn test_arduino_board_clone() {
    let board = ArduinoBoard::Nano33IoT;
    let cloned = board.clone();
    assert_eq!(board, cloned);
}

// ============================================================================
// ESP32Variant Tests
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
fn test_esp32_variant_clone() {
    let variant = ESP32Variant::ESP32S3;
    let cloned = variant.clone();
    assert_eq!(variant, cloned);
}

// ============================================================================
// ESP32Framework Tests
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

// ============================================================================
// EdgePlatform Tests
// ============================================================================

#[test]
fn test_edge_platform_arduino() {
    let platform = EdgePlatform::Arduino {
        board: ArduinoBoard::Uno,
        version: "1.8.19".to_string(),
    };
    
    if let EdgePlatform::Arduino { board, version } = platform {
        assert_eq!(board, ArduinoBoard::Uno);
        assert_eq!(version, "1.8.19");
    } else {
        panic!("Expected Arduino variant");
    }
}

#[test]
fn test_edge_platform_esp32() {
    let platform = EdgePlatform::ESP32 {
        chip: ESP32Variant::ESP32S3,
        framework: ESP32Framework::ESPIDF,
    };
    
    if let EdgePlatform::ESP32 { chip, framework } = platform {
        assert_eq!(chip, ESP32Variant::ESP32S3);
        assert_eq!(framework, ESP32Framework::ESPIDF);
    } else {
        panic!("Expected ESP32 variant");
    }
}

#[test]
fn test_edge_platform_raspberry_pi() {
    let platform = EdgePlatform::RaspberryPi {
        model: PiModel::Pi4,
        os: PiOS::RaspberryPiOS,
    };
    
    if let EdgePlatform::RaspberryPi { model, .. } = platform {
        assert_eq!(model, PiModel::Pi4);
    } else {
        panic!("Expected RaspberryPi variant");
    }
}

#[test]
fn test_edge_platform_beaglebone() {
    let platform = EdgePlatform::BeagleBone {
        variant: BeagleBoneVariant::Black,
    };
    
    if let EdgePlatform::BeagleBone { variant } = platform {
        assert_eq!(variant, BeagleBoneVariant::Black);
    } else {
        panic!("Expected BeagleBone variant");
    }
}

#[test]
fn test_edge_platform_industrial() {
    let platform = EdgePlatform::Industrial {
        system_type: IndustrialSystemType::PLC,
        protocol: IndustrialProtocol::Modbus,
    };
    
    if let EdgePlatform::Industrial { system_type, protocol } = platform {
        assert_eq!(system_type, IndustrialSystemType::PLC);
        assert_eq!(protocol, IndustrialProtocol::Modbus);
    } else {
        panic!("Expected Industrial variant");
    }
}

#[test]
fn test_edge_platform_microcontroller() {
    let platform = EdgePlatform::Microcontroller {
        architecture: MicrocontrollerArch::ARM,
        vendor: "STMicroelectronics".to_string(),
        model: "STM32F4".to_string(),
    };
    
    if let EdgePlatform::Microcontroller { architecture, vendor, model } = platform {
        assert_eq!(architecture, MicrocontrollerArch::ARM);
        assert_eq!(vendor, "STMicroelectronics");
        assert_eq!(model, "STM32F4");
    } else {
        panic!("Expected Microcontroller variant");
    }
}

#[test]
fn test_edge_platform_linux_edge() {
    let platform = EdgePlatform::LinuxEdge {
        architecture: "aarch64".to_string(),
        kernel_version: "5.15.0".to_string(),
    };
    
    if let EdgePlatform::LinuxEdge { architecture, kernel_version } = platform {
        assert_eq!(architecture, "aarch64");
        assert_eq!(kernel_version, "5.15.0");
    } else {
        panic!("Expected LinuxEdge variant");
    }
}

#[test]
fn test_edge_platform_clone() {
    let platform = EdgePlatform::Arduino {
        board: ArduinoBoard::Nano,
        version: "1.8.0".to_string(),
    };
    
    let cloned = platform.clone();
    assert_eq!(platform, cloned);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_all_arduino_boards() {
    let boards = vec![
        ArduinoBoard::Uno,
        ArduinoBoard::Nano,
        ArduinoBoard::Mega2560,
        ArduinoBoard::Leonardo,
        ArduinoBoard::Micro,
        ArduinoBoard::Due,
        ArduinoBoard::MKR1000,
        ArduinoBoard::MKRZero,
        ArduinoBoard::Portenta,
        ArduinoBoard::Nano33IoT,
        ArduinoBoard::Nano33BLE,
        ArduinoBoard::MKRWiFi1010,
    ];
    
    assert_eq!(boards.len(), 12);
}

#[test]
fn test_all_esp32_variants() {
    let variants = vec![
        ESP32Variant::ESP32,
        ESP32Variant::ESP32S2,
        ESP32Variant::ESP32S3,
        ESP32Variant::ESP32C3,
        ESP32Variant::ESP32C6,
        ESP32Variant::ESP32H2,
        ESP32Variant::ESP32P4,
    ];
    
    assert_eq!(variants.len(), 7);
}

#[test]
fn test_all_esp32_frameworks() {
    let frameworks = vec![
        ESP32Framework::ESPIDF,
        ESP32Framework::Arduino,
        ESP32Framework::PlatformIO,
        ESP32Framework::MicroPython,
    ];
    
    assert_eq!(frameworks.len(), 4);
}

#[test]
fn test_edge_platform_serialization() {
    let platform = EdgePlatform::ESP32 {
        chip: ESP32Variant::ESP32C3,
        framework: ESP32Framework::Arduino,
    };
    
    let json = serde_json::to_string(&platform).expect("Failed to serialize");
    let deserialized: EdgePlatform = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(platform, deserialized);
}

#[test]
fn test_arduino_advanced_boards() {
    let advanced = vec![
        ArduinoBoard::Portenta,
        ArduinoBoard::Nano33IoT,
        ArduinoBoard::Nano33BLE,
        ArduinoBoard::MKRWiFi1010,
    ];
    
    assert_eq!(advanced.len(), 4);
}

