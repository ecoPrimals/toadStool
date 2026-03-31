// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use uuid::Uuid;

use super::*;
use crate::types::configs::embedded::PeripheralType;
use crate::{
    JobStatus, LegacyArchitecture, MemoryLayout, MemoryPermissions, MemoryRegionType,
    ProgrammingInterface, ProgrammingInterfaceType,
};
use std::collections::HashMap as StdHashMap;

/// Asserts JSON serialization is stable across serialize → deserialize → serialize.
fn assert_serde_json_stable<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    let json = serde_json::to_string(value).expect("serde_json serialize");
    let back: T = serde_json::from_str(&json).expect("serde_json deserialize");
    let json_again = serde_json::to_string(&back).expect("serde_json re-serialize");
    assert_eq!(
        json, json_again,
        "serde round-trip must preserve JSON representation"
    );
}

fn sample_memory_layout() -> MemoryLayout {
    MemoryLayout {
        rom_regions: vec![],
        ram_regions: vec![],
        io_regions: vec![],
    }
}

fn sample_programming_interface() -> ProgrammingInterface {
    ProgrammingInterface {
        interface_type: ProgrammingInterfaceType::ISP,
        connection_params: StdHashMap::new(),
    }
}

#[test]
fn memory_usage_default_matches_region_usage_defaults() {
    let u = MemoryUsage::default();
    let z = RegionUsage::default();
    assert_eq!(u.rom_usage.used, z.used);
    assert_eq!(u.rom_usage.total, z.total);
    assert!((u.rom_usage.percentage - z.percentage).abs() < f32::EPSILON);
    assert_eq!(u.ram_usage.used, z.used);
    assert!(u.eeprom_usage.is_none());
}

#[test]
fn region_usage_default_is_zeroed() {
    let r = RegionUsage::default();
    assert_eq!(r.used, 0);
    assert_eq!(r.total, 0);
    assert!((r.percentage - 0.0).abs() < f32::EPSILON);
}

#[test]
fn embedded_job_serde_roundtrip() {
    let job = EmbeddedJob {
        job_id: Uuid::nil(),
        target_architecture: LegacyArchitecture::MOS6502,
        job_type: EmbeddedJobType::Compilation {
            language: EmbeddedLanguage::Assembly,
            optimization: OptimizationLevel::Size,
            debug_info: true,
        },
        source_files: vec![SourceFile {
            path: PathBuf::from("src/main.asm"),
            file_type: SourceFileType::Assembly,
            content: "nop".to_string(),
            include_paths: vec![PathBuf::from("inc")],
            defines: HashMap::from([("BOARD".to_string(), "1".to_string())]),
        }],
        memory_layout: sample_memory_layout(),
        programming_interface: sample_programming_interface(),
        status: JobStatus::Queued,
        output_files: vec![OutputFile {
            path: PathBuf::from("out.bin"),
            file_type: OutputFileType::Binary,
            size: 4,
            load_address: Some(0x8000),
            execution_address: Some(0x8000),
        }],
        compilation_log: String::new(),
        programming_log: String::new(),
        start_time: None,
        end_time: None,
    };
    assert_serde_json_stable(&job);
}

#[test]
fn embedded_job_type_variants_roundtrip() {
    let cases = vec![
        EmbeddedJobType::Compilation {
            language: EmbeddedLanguage::C,
            optimization: OptimizationLevel::Speed,
            debug_info: false,
        },
        EmbeddedJobType::Programming {
            target_memory: MemoryRegionType::Flash,
            verify: true,
            erase_first: false,
        },
        EmbeddedJobType::Debugging {
            debug_interface: DebugInterface::JTAG,
            breakpoints: vec![Breakpoint {
                address: 0x1000,
                breakpoint_type: BreakpointType::Code,
                condition: None,
                hit_count: 0,
                enabled: true,
            }],
        },
        EmbeddedJobType::Emulation {
            emulator_type: EmulatorType::Software,
            rom_image: vec![0xea, 0x4c],
        },
        EmbeddedJobType::MemoryDump {
            start_address: 0,
            length: 256,
        },
        EmbeddedJobType::PeripheralTest {
            peripheral: PeripheralType::UART,
            test_type: PeripheralTestType::Functional,
        },
    ];
    for job_type in cases {
        assert_serde_json_stable(&job_type);
    }
}

#[test]
fn embedded_language_variants_roundtrip() {
    for lang in [
        EmbeddedLanguage::Assembly,
        EmbeddedLanguage::C,
        EmbeddedLanguage::CPlusPlus,
        EmbeddedLanguage::BASIC,
        EmbeddedLanguage::Pascal,
        EmbeddedLanguage::Forth,
        EmbeddedLanguage::MachineCode,
    ] {
        assert_serde_json_stable(&lang);
    }
}

#[test]
fn optimization_level_variants_roundtrip() {
    for level in [
        OptimizationLevel::None,
        OptimizationLevel::Size,
        OptimizationLevel::Speed,
        OptimizationLevel::Debug,
    ] {
        assert_serde_json_stable(&level);
    }
}

#[test]
fn debug_interface_variants_roundtrip() {
    for iface in [
        DebugInterface::ICE,
        DebugInterface::JTAG,
        DebugInterface::SWD,
        DebugInterface::BDM,
        DebugInterface::Serial,
        DebugInterface::Software,
    ] {
        assert_serde_json_stable(&iface);
    }
}

#[test]
fn breakpoint_type_variants_roundtrip() {
    for t in [
        BreakpointType::Code,
        BreakpointType::DataRead,
        BreakpointType::DataWrite,
        BreakpointType::DataAccess,
    ] {
        assert_serde_json_stable(&t);
    }
}

#[test]
fn emulator_type_variants_roundtrip() {
    for t in [
        EmulatorType::Software,
        EmulatorType::Hardware,
        EmulatorType::InCircuit,
    ] {
        assert_serde_json_stable(&t);
    }
}

#[test]
fn peripheral_test_type_variants_roundtrip() {
    for t in [
        PeripheralTestType::Functional,
        PeripheralTestType::Performance,
        PeripheralTestType::Stress,
        PeripheralTestType::Compliance,
    ] {
        assert_serde_json_stable(&t);
    }
}

#[test]
fn source_file_type_variants_roundtrip() {
    for t in [
        SourceFileType::C,
        SourceFileType::CPlusPlus,
        SourceFileType::Assembly,
        SourceFileType::Header,
        SourceFileType::LinkerScript,
        SourceFileType::Configuration,
    ] {
        assert_serde_json_stable(&t);
    }
}

#[test]
fn output_file_type_variants_roundtrip() {
    for t in [
        OutputFileType::Binary,
        OutputFileType::IntelHex,
        OutputFileType::MotorolaS,
        OutputFileType::ELF,
        OutputFileType::Object,
        OutputFileType::Library,
        OutputFileType::Map,
        OutputFileType::Listing,
    ] {
        assert_serde_json_stable(&t);
    }
}

#[test]
fn message_type_variants_roundtrip() {
    for t in [
        MessageType::Error,
        MessageType::Warning,
        MessageType::Info,
        MessageType::Debug,
    ] {
        assert_serde_json_stable(&t);
    }
}

#[test]
fn symbol_type_variants_roundtrip() {
    for t in [
        SymbolType::Function,
        SymbolType::Variable,
        SymbolType::Constant,
        SymbolType::Label,
        SymbolType::Section,
    ] {
        assert_serde_json_stable(&t);
    }
}

#[test]
fn section_type_variants_roundtrip() {
    let cases = vec![
        SectionType::Code,
        SectionType::Data,
        SectionType::BSS,
        SectionType::ReadOnlyData,
        SectionType::Stack,
        SectionType::Heap,
        SectionType::Custom {
            name: ".vectors".to_string(),
        },
    ];
    for s in cases {
        assert_serde_json_stable(&s);
    }
}

#[test]
fn compilation_result_serde_roundtrip() {
    let result = CompilationResult {
        success: true,
        output_files: vec![],
        messages: vec![CompilerMessage {
            message_type: MessageType::Warning,
            source_file: Some(PathBuf::from("a.c")),
            line_number: Some(10),
            column_number: None,
            message: "unused".to_string(),
        }],
        compilation_time: Duration::from_millis(100),
        memory_usage: MemoryUsage::default(),
    };
    assert_serde_json_stable(&result);
}

#[test]
fn link_result_serde_roundtrip() {
    let result = LinkResult {
        success: false,
        executable: Some(PathBuf::from("a.out")),
        memory_map: Some(MemoryMap {
            regions: vec![],
            symbols: vec![],
            sections: vec![],
        }),
        messages: vec![LinkerMessage {
            message_type: MessageType::Error,
            section: Some(".text".to_string()),
            symbol: Some("_start".to_string()),
            message: "undefined".to_string(),
        }],
        link_time: Duration::from_millis(50),
    };
    assert_serde_json_stable(&result);
}

#[test]
fn memory_map_region_and_symbol_serde_roundtrip() {
    let region = MemoryMapRegion {
        name: "flash".to_string(),
        start_address: 0,
        end_address: 0xffff,
        size: 0x10000,
        region_type: MemoryRegionType::Flash,
        permissions: MemoryPermissions {
            read: true,
            write: false,
            execute: true,
        },
    };
    assert_serde_json_stable(&region);

    let sym = Symbol {
        name: "main".to_string(),
        address: 0x200,
        size: 4,
        symbol_type: SymbolType::Function,
        section: Some(".text".to_string()),
    };
    assert_serde_json_stable(&sym);
}

#[test]
fn section_struct_serde_roundtrip() {
    let section = Section {
        name: ".data".to_string(),
        start_address: 0x1000,
        size: 0x100,
        section_type: SectionType::Data,
        alignment: 4,
    };
    assert_serde_json_stable(&section);
}

#[test]
fn target_info_serde_roundtrip() {
    let info = TargetInfo {
        name: "demo".to_string(),
        architecture: LegacyArchitecture::ZilogZ80,
        flash_size: 32 * 1024,
        ram_size: 8 * 1024,
        eeprom_size: Some(1024),
        cpu_speed: 16_000_000,
        features: vec!["uart".to_string()],
    };
    assert_serde_json_stable(&info);
}

#[test]
fn cpu_registers_serde_roundtrip() {
    let mut gp = StdHashMap::new();
    gp.insert("a".to_string(), 0x42);
    let regs = CpuRegisters {
        general_purpose: gp,
        program_counter: 0x100,
        stack_pointer: 0x200,
        status_register: 0,
        special: StdHashMap::new(),
    };
    assert_serde_json_stable(&regs);
}

#[test]
fn emulation_status_variants_roundtrip() {
    let cases = vec![
        EmulationStatus::Running,
        EmulationStatus::Stopped,
        EmulationStatus::Breakpoint { address: 0x400 },
        EmulationStatus::Error {
            message: "halt".to_string(),
        },
    ];
    for s in cases {
        assert_serde_json_stable(&s);
    }
}

#[test]
fn peripheral_status_serde_roundtrip() {
    let mut regs = StdHashMap::new();
    regs.insert("sr".to_string(), 1);
    let ps = PeripheralStatus {
        name: "uart0".to_string(),
        peripheral_type: PeripheralType::UART,
        status: "idle".to_string(),
        registers: regs,
        interrupt_status: false,
    };
    assert_serde_json_stable(&ps);
}

#[test]
fn debug_formatting_is_non_empty() {
    let job = EmbeddedJob {
        job_id: Uuid::new_v4(),
        target_architecture: LegacyArchitecture::Intel8086,
        job_type: EmbeddedJobType::Compilation {
            language: EmbeddedLanguage::Assembly,
            optimization: OptimizationLevel::None,
            debug_info: false,
        },
        source_files: vec![],
        memory_layout: sample_memory_layout(),
        programming_interface: sample_programming_interface(),
        status: JobStatus::Running,
        output_files: vec![],
        compilation_log: String::new(),
        programming_log: String::new(),
        start_time: None,
        end_time: None,
    };
    let dbg = format!("{job:?}");
    assert!(dbg.contains("EmbeddedJob"));
    assert!(dbg.len() > 20);
}
