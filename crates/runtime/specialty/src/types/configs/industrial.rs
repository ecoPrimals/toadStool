// SPDX-License-Identifier: AGPL-3.0-or-later
//! Industrial control systems configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Industrial system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialConfig {
    /// System type
    pub system_type: IndustrialSystemType,
    /// Communication protocols
    pub protocols: Vec<IndustrialProtocol>,
    /// Device configuration
    pub devices: Vec<IndustrialDevice>,
    /// Safety configuration
    pub safety_config: SafetyConfig,
}

/// Industrial system types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndustrialSystemType {
    /// PLC (Programmable Logic Controller)
    PLC,
    /// SCADA (Supervisory Control And Data Acquisition)
    SCADA,
    /// DCS (Distributed Control System)
    DCS,
    /// HMI (Human Machine Interface)
    HMI,
    /// MES (Manufacturing Execution System)
    MES,
    /// Custom industrial system type.
    Custom {
        /// System type name.
        name: String,
    },
}

/// Industrial communication protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndustrialProtocol {
    /// Modbus RTU
    ModbusRTU,
    /// Modbus TCP
    ModbusTCP,
    /// Profibus
    Profibus,
    /// Profinet
    Profinet,
    /// `DeviceNet`
    DeviceNet,
    /// `ControlNet`
    ControlNet,
    /// EtherNet/IP
    EtherNetIP,
    /// CAN bus
    CANBus,
    /// Foundation Fieldbus
    FoundationFieldbus,
    /// HART
    HART,
    /// AS-Interface
    ASInterface,
    /// Custom industrial protocol.
    Custom {
        /// Protocol name.
        name: String,
    },
}

/// Industrial device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialDevice {
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: IndustrialDeviceType,
    /// Device address
    pub address: String,
    /// Communication protocol
    pub protocol: IndustrialProtocol,
    /// Device parameters
    pub parameters: HashMap<String, String>,
}

/// Industrial device types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndustrialDeviceType {
    /// Input/Output module
    IOModule,
    /// Sensor
    Sensor,
    /// Actuator
    Actuator,
    /// Motor drive
    MotorDrive,
    /// Valve
    Valve,
    /// Transmitter
    Transmitter,
    /// Controller
    Controller,
    /// Custom industrial device type.
    Custom {
        /// Device type name.
        name: String,
    },
}

/// Safety configuration for industrial systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Safety integrity level
    pub sil_level: SILLevel,
    /// Safety functions
    pub safety_functions: Vec<SafetyFunction>,
    /// Emergency stop configuration
    pub emergency_stop: EmergencyStopConfig,
}

/// Safety Integrity Level (SIL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SILLevel {
    /// SIL 1
    SIL1,
    /// SIL 2
    SIL2,
    /// SIL 3
    SIL3,
    /// SIL 4
    SIL4,
}

/// Safety function configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyFunction {
    /// Function name
    pub name: String,
    /// Function type
    pub function_type: SafetyFunctionType,
    /// Response time
    pub response_time: Duration,
    /// Test interval
    pub test_interval: Duration,
}

/// Safety function types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyFunctionType {
    /// Emergency stop
    EmergencyStop,
    /// Safety door
    SafetyDoor,
    /// Light curtain
    LightCurtain,
    /// Pressure sensitive mat
    PressureMat,
    /// Two-hand control
    TwoHandControl,
    /// Custom safety function.
    Custom {
        /// Function type name.
        name: String,
    },
}

/// Emergency stop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyStopConfig {
    /// Emergency stop devices
    pub devices: Vec<String>,
    /// Response time
    pub response_time: Duration,
    /// Reset procedure
    pub reset_procedure: ResetProcedure,
}

/// Reset procedure types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResetProcedure {
    /// Automatic reset
    Automatic,
    /// Manual reset
    Manual,
    /// Key reset
    KeyReset,
    /// Custom reset procedure.
    Custom {
        /// Procedure name.
        name: String,
    },
}
