// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for legacy runtime configuration types
//!
//! Tests cover config types from `toadstool_runtime_specialty::types::configs::*`:
//! - Config creation and defaults
//! - Serialization/deserialization round-trips
//! - Individual config struct construction

use std::path::PathBuf;
use std::time::Duration;
use toadstool_runtime_specialty::types::configs::*;
use toadstool_runtime_specialty::{LegacyArchitecture, LegacySystemType};

#[test]
fn test_communication_settings_default() {
    let settings = CommunicationSettings::default();
    assert!(matches!(
        settings.connection_type,
        ConnectionType::LocalEmulation
    ));
    assert!(settings.timeouts.connection_timeout.as_secs() > 0);
    assert!(settings.retries.max_retries > 0);
    assert!(settings.authentication.is_none());
}

#[test]
fn test_communication_settings_serialization() {
    let settings = CommunicationSettings::default();
    let json = serde_json::to_string(&settings).unwrap();
    let deserialized: CommunicationSettings = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        deserialized.connection_type,
        ConnectionType::LocalEmulation
    ));
}

#[test]
fn test_connection_type_variants() {
    let serial = ConnectionType::DirectSerial {
        port: "/dev/ttyUSB0".to_string(),
        baud_rate: 9600,
    };
    let json = serde_json::to_string(&serial).unwrap();
    let round: ConnectionType = serde_json::from_str(&json).unwrap();
    assert!(matches!(round, ConnectionType::DirectSerial { .. }));

    let telnet = ConnectionType::Telnet {
        host: "localhost".to_string(),
        port: 23,
    };
    let json = serde_json::to_string(&telnet).unwrap();
    let round: ConnectionType = serde_json::from_str(&json).unwrap();
    assert!(matches!(round, ConnectionType::Telnet { .. }));
}

#[test]
fn test_authentication_settings() {
    use toadstool_runtime_specialty::types::configs::communication::{
        AuthenticationSettings, AuthenticationType,
    };
    let auth = AuthenticationSettings {
        auth_type: AuthenticationType::UsernamePassword,
        username: Some("admin".to_string()),
        password: Some("secret".to_string()),
        key_file: None,
        certificate: None,
    };
    assert!(matches!(
        auth.auth_type,
        AuthenticationType::UsernamePassword
    ));
    assert_eq!(auth.username.as_deref(), Some("admin"));
}

#[test]
fn test_config_emulation_config_roundtrip() {
    use std::collections::HashMap;
    let config = ConfigEmulationConfig {
        emulator_type: EmulatorType::SIMH,
        emulator_path: PathBuf::from("/usr/bin/simh"),
        parameters: HashMap::new(),
        rom_files: vec![],
        disk_images: vec![],
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: ConfigEmulationConfig = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized.emulator_type, EmulatorType::SIMH));
}

#[test]
fn test_emulator_type_variants() {
    assert!(matches!(EmulatorType::SIMH, EmulatorType::SIMH));
    assert!(matches!(EmulatorType::MAME, EmulatorType::MAME));
    let custom = EmulatorType::Custom {
        name: "my-emu".to_string(),
    };
    let json = serde_json::to_string(&custom).unwrap();
    let round: EmulatorType = serde_json::from_str(&json).unwrap();
    assert!(matches!(round, EmulatorType::Custom { .. }));
}

#[test]
fn test_compilation_target_format() {
    let formats = [
        CompilationTargetFormat::Executable,
        CompilationTargetFormat::Object,
        CompilationTargetFormat::ROMImage,
    ];
    for f in formats {
        let json = serde_json::to_string(&f).unwrap();
        let _round: CompilationTargetFormat = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_compilation_optimization_level() {
    let levels = [
        CompilationOptimizationLevel::None,
        CompilationOptimizationLevel::Standard,
        CompilationOptimizationLevel::Maximum,
    ];
    for l in levels {
        let json = serde_json::to_string(&l).unwrap();
        let _round: CompilationOptimizationLevel = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_toolchain_config() {
    let config = CompilationToolchainConfig {
        name: "gcc-arm".to_string(),
        path: PathBuf::from("/usr/bin"),
        compiler: "arm-gcc".to_string(),
        linker: "arm-ld".to_string(),
        assembler: "arm-as".to_string(),
        archiver: "arm-ar".to_string(),
        debugger: Some("arm-gdb".to_string()),
        target: "arm-none-eabi".to_string(),
        environment: std::collections::HashMap::new(),
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: CompilationToolchainConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.name, deserialized.name);
}

#[test]
fn test_session_config() {
    let config = SessionConfig {
        width: 80,
        height: 24,
        line_ending: LineEnding::Unix,
        encoding: CharacterEncoding::ASCII,
        flow_control: FlowControl::None,
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: SessionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.width, deserialized.width);
    assert_eq!(config.height, deserialized.height);
}

#[test]
fn test_terminal_type_variants() {
    assert!(matches!(
        ConfigTerminalType::VT100,
        ConfigTerminalType::VT100
    ));
    assert!(matches!(
        ConfigTerminalType::IBM3270,
        ConfigTerminalType::IBM3270
    ));
}

#[test]
fn test_management_job_priority() {
    use toadstool_runtime_specialty::types::configs::management::JobPriority as ManagementJobPriority;
    let priorities = [
        ManagementJobPriority::Low,
        ManagementJobPriority::Normal,
        ManagementJobPriority::High,
        ManagementJobPriority::Critical,
    ];
    for p in priorities {
        let json = serde_json::to_string(&p).unwrap();
        let _round: ManagementJobPriority = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_transfer_type() {
    use toadstool_runtime_specialty::types::configs::management::TransferType;
    let json = serde_json::to_string(&TransferType::Upload).unwrap();
    let round: TransferType = serde_json::from_str(&json).unwrap();
    assert!(matches!(round, TransferType::Upload));
}

#[test]
fn test_storage_disk_image() {
    let disk = DiskImage {
        name: "boot.img".to_string(),
        path: PathBuf::from("/images/boot.img"),
        image_type: DiskImageType::Raw,
        size: 360 * 1024,
        read_only: false,
    };
    let json = serde_json::to_string(&disk).unwrap();
    let deserialized: DiskImage = serde_json::from_str(&json).unwrap();
    assert_eq!(disk.name, deserialized.name);
}

#[test]
fn test_storage_rom_file() {
    let rom = ROMFile {
        name: "bios.rom".to_string(),
        path: PathBuf::from("/roms/bios.rom"),
        load_address: 0xF000,
        size: 8192,
        checksum: "abc123".to_string(),
    };
    let json = serde_json::to_string(&rom).unwrap();
    let deserialized: ROMFile = serde_json::from_str(&json).unwrap();
    assert_eq!(rom.load_address, deserialized.load_address);
}

#[test]
fn test_realtime_config() {
    let config = RealtimeConfig {
        rtos: RealtimeOS::VxWorks,
        scheduling_policy: SchedulingPolicy::Preemptive,
        tasks: vec![],
        interrupts: vec![],
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: RealtimeConfig = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized.rtos, RealtimeOS::VxWorks));
}

#[test]
fn test_task_config() {
    let task = TaskConfig {
        name: "main_task".to_string(),
        priority: 10,
        stack_size: 4096,
        period: Duration::from_millis(100),
        deadline: Duration::from_millis(100),
        function: "main".to_string(),
    };
    let json = serde_json::to_string(&task).unwrap();
    let deserialized: TaskConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(task.name, deserialized.name);
}

#[test]
fn test_industrial_config() {
    let config = IndustrialConfig {
        system_type: IndustrialSystemType::PLC,
        protocols: vec![IndustrialProtocol::ModbusTCP],
        devices: vec![],
        safety_config: SafetyConfig {
            sil_level: SILLevel::SIL2,
            safety_functions: vec![],
            emergency_stop: EmergencyStopConfig {
                devices: vec!["E-Stop-1".to_string()],
                response_time: Duration::from_millis(50),
                reset_procedure: ResetProcedure::Manual,
            },
        },
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: IndustrialConfig = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        deserialized.system_type,
        IndustrialSystemType::PLC
    ));
}

#[test]
fn test_embedded_config() {
    use toadstool_runtime_specialty::types::configs::communication::{
        ProgrammingInterface, ProgrammingInterfaceType,
    };
    let config = EmbeddedConfig {
        architecture: LegacyArchitecture::Intel8086,
        memory_layout: MemoryLayout {
            rom_regions: vec![],
            ram_regions: vec![],
            io_regions: vec![],
        },
        peripherals: vec![],
        programming_interface: ProgrammingInterface {
            interface_type: ProgrammingInterfaceType::JTAG,
            connection_params: std::collections::HashMap::new(),
        },
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: EmbeddedConfig = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        deserialized.architecture,
        LegacyArchitecture::Intel8086
    ));
}

#[test]
fn test_memory_region() {
    let region = MemoryRegion {
        name: "RAM".to_string(),
        start_address: 0x0000,
        end_address: 0xFFFF,
        region_type: MemoryRegionType::RAM,
        permissions: MemoryPermissions {
            read: true,
            write: true,
            execute: true,
        },
    };
    let json = serde_json::to_string(&region).unwrap();
    let deserialized: MemoryRegion = serde_json::from_str(&json).unwrap();
    assert_eq!(region.start_address, deserialized.start_address);
}

#[test]
fn test_mainframe_config() {
    use toadstool_runtime_specialty::types::configs::communication::MainframeConnectionType;
    use toadstool_runtime_specialty::types::configs::communication::{
        AuthenticationSettings, AuthenticationType, ConnectionSettings,
    };
    let config = MainframeConfig {
        system_type: LegacySystemType::IbmSystem360,
        connection: ConnectionSettings {
            host: "mainframe.example.com".to_string(),
            port: 23,
            connection_type: MainframeConnectionType::IBM3270,
            authentication: AuthenticationSettings {
                auth_type: AuthenticationType::None,
                username: None,
                password: None,
                key_file: None,
                certificate: None,
            },
        },
        datasets: std::collections::HashMap::new(),
        jcl_settings: JCLSettings {
            job_class: "A".to_string(),
            message_class: "0".to_string(),
            priority: 8,
            time_limit: Duration::from_hours(1),
            region_size: 4 * 1024 * 1024,
        },
        cobol_settings: COBOLSettings {
            compiler: "ibmcob".to_string(),
            compile_options: vec![],
            link_options: vec![],
            runtime_options: vec![],
        },
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: MainframeConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.connection.host, deserialized.connection.host);
}

#[test]
fn test_communication_settings_flattened_json() {
    let json = r#"{
        "connection_type": "LocalEmulation",
        "connection_timeout": "30s",
        "request_timeout": "60s",
        "read_timeout": "30s",
        "write_timeout": "30s",
        "max_retries": 3,
        "base_delay": "100ms",
        "max_delay": "30s",
        "backoff_multiplier": 2.0,
        "jitter_percent": 10.0
    }"#;
    let result: Result<CommunicationSettings, _> = serde_json::from_str(json);
    assert!(result.is_ok(), "Should deserialize: {:?}", result.err());
    let settings = result.unwrap();
    assert_eq!(
        settings.timeouts.connection_timeout,
        Duration::from_secs(30)
    );
    assert_eq!(settings.retries.max_retries, 3);
}
