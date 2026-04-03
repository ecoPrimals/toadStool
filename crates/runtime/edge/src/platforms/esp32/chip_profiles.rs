// SPDX-License-Identifier: AGPL-3.0-only
//! Per-variant resource maps and capability lists for ESP32 chips.

use super::super::{
    EdgeDeviceResources, ESP32Framework, ESP32Variant, NetworkInterface, NetworkInterfaceType,
};

/// Get chip-specific resources
pub(crate) fn get_chip_resources(chip: &ESP32Variant) -> EdgeDeviceResources {
    match chip {
        ESP32Variant::ESP32 => EdgeDeviceResources {
            cpu_cores: 2,
            cpu_frequency_mhz: 240,
            memory_bytes: 520192, // 520KB SRAM
            storage_bytes: 4194304, // 4MB Flash (typical)
            network_interfaces: vec![
                NetworkInterface {
                    name: "WiFi".to_string(),
                    interface_type: NetworkInterfaceType::WiFi,
                    mac_address: None,
                    ip_address: None,
                    is_connected: false,
                    speed_mbps: Some(150),
                },
                NetworkInterface {
                    name: "Bluetooth".to_string(),
                    interface_type: NetworkInterfaceType::Bluetooth,
                    mac_address: None,
                    ip_address: None,
                    is_connected: false,
                    speed_mbps: Some(2),
                },
            ],
            gpio_pins: 39,
            analog_pins: 18,
            pwm_pins: 16,
            i2c_buses: 2,
            spi_buses: 4,
            uart_ports: 3,
        },
        ESP32Variant::ESP32S2 => EdgeDeviceResources {
            cpu_cores: 1,
            cpu_frequency_mhz: 240,
            memory_bytes: 327680, // 320KB SRAM
            storage_bytes: 4194304,
            network_interfaces: vec![NetworkInterface {
                name: "WiFi".to_string(),
                interface_type: NetworkInterfaceType::WiFi,
                mac_address: None,
                ip_address: None,
                is_connected: false,
                speed_mbps: Some(150),
            }],
            gpio_pins: 43,
            analog_pins: 20,
            pwm_pins: 14,
            i2c_buses: 2,
            spi_buses: 4,
            uart_ports: 2,
        },
        ESP32Variant::ESP32S3 => EdgeDeviceResources {
            cpu_cores: 2,
            cpu_frequency_mhz: 240,
            memory_bytes: 524288, // 512KB SRAM
            storage_bytes: 8388608, // 8MB Flash (typical)
            network_interfaces: vec![
                NetworkInterface {
                    name: "WiFi".to_string(),
                    interface_type: NetworkInterfaceType::WiFi,
                    mac_address: None,
                    ip_address: None,
                    is_connected: false,
                    speed_mbps: Some(150),
                },
                NetworkInterface {
                    name: "Bluetooth".to_string(),
                    interface_type: NetworkInterfaceType::Bluetooth,
                    mac_address: None,
                    ip_address: None,
                    is_connected: false,
                    speed_mbps: Some(2),
                },
            ],
            gpio_pins: 45,
            analog_pins: 20,
            pwm_pins: 14,
            i2c_buses: 2,
            spi_buses: 4,
            uart_ports: 3,
        },
        ESP32Variant::ESP32C3 => EdgeDeviceResources {
            cpu_cores: 1,
            cpu_frequency_mhz: 160,
            memory_bytes: 409600, // 400KB SRAM
            storage_bytes: 4194304,
            network_interfaces: vec![
                NetworkInterface {
                    name: "WiFi".to_string(),
                    interface_type: NetworkInterfaceType::WiFi,
                    mac_address: None,
                    ip_address: None,
                    is_connected: false,
                    speed_mbps: Some(150),
                },
                NetworkInterface {
                    name: "Bluetooth".to_string(),
                    interface_type: NetworkInterfaceType::Bluetooth,
                    mac_address: None,
                    ip_address: None,
                    is_connected: false,
                    speed_mbps: Some(2),
                },
            ],
            gpio_pins: 22,
            analog_pins: 6,
            pwm_pins: 6,
            i2c_buses: 1,
            spi_buses: 3,
            uart_ports: 2,
        },
        _ => EdgeDeviceResources {
            cpu_cores: 1,
            cpu_frequency_mhz: 160,
            memory_bytes: 327680,
            storage_bytes: 4194304,
            network_interfaces: vec![],
            gpio_pins: 20,
            analog_pins: 8,
            pwm_pins: 8,
            i2c_buses: 1,
            spi_buses: 2,
            uart_ports: 2,
        },
    }
}

/// Get chip and framework specific capabilities
pub(crate) fn get_chip_capabilities(chip: &ESP32Variant, framework: &ESP32Framework) -> Vec<String> {
    let mut capabilities = vec![
        "gpio_control".to_string(),
        "analog_input".to_string(),
        "pwm_output".to_string(),
        "i2c_communication".to_string(),
        "spi_communication".to_string(),
        "uart_communication".to_string(),
        "interrupt_handling".to_string(),
        "timer_control".to_string(),
        "nvs_storage".to_string(),
        "deep_sleep".to_string(),
        "watchdog_timer".to_string(),
    ];

    if !matches!(chip, ESP32Variant::ESP32H2) {
        capabilities.extend(vec![
            "wifi_connectivity".to_string(),
            "wifi_ap_mode".to_string(),
            "wifi_sta_mode".to_string(),
            "wifi_mesh".to_string(),
            "network_communication".to_string(),
            "http_server".to_string(),
            "https_server".to_string(),
            "websocket_server".to_string(),
            "mqtt_client".to_string(),
            "ota_updates".to_string(),
        ]);
    }

    if matches!(
        chip,
        ESP32Variant::ESP32 | ESP32Variant::ESP32S3 | ESP32Variant::ESP32C3
    ) {
        capabilities.extend(vec![
            "bluetooth_classic".to_string(),
            "bluetooth_le".to_string(),
            "ble_advertising".to_string(),
            "ble_scanning".to_string(),
            "ble_mesh".to_string(),
        ]);
    }

    match framework {
        ESP32Framework::ESPIDF => {
            capabilities.extend(vec![
                "freertos".to_string(),
                "lwip_stack".to_string(),
                "mbedtls".to_string(),
                "fatfs".to_string(),
                "spiffs".to_string(),
                "nvs_flash".to_string(),
            ]);
        }
        ESP32Framework::Arduino => {
            capabilities.extend(vec![
                "arduino_libraries".to_string(),
                "serial_monitor".to_string(),
                "arduino_ota".to_string(),
            ]);
        }
        ESP32Framework::MicroPython => {
            capabilities.extend(vec![
                "python_interpreter".to_string(),
                "micropython_modules".to_string(),
                "repl_console".to_string(),
            ]);
        }
        ESP32Framework::Rust => {
            capabilities.extend(vec![
                "rust_std".to_string(),
                "no_std_support".to_string(),
                "embedded_hal".to_string(),
            ]);
        }
        _ => {}
    }

    capabilities
}
