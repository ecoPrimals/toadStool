// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for hw-learn crate (S155 expansion):
//! - distiller/classify.rs (GpuGen, classify_events, classify_register)
//! - knowledge/ (KnowledgeStore, amd_baseline, arch_map)
//! - observer/ (ObserveConfig, TraceMode, TraceEvent, TraceObserver)

#![allow(clippy::pedantic)]

use hw_learn::distiller::classify::{
    GpuGen, classify_events, classify_register, classify_register_for_gen,
};
use hw_learn::distiller::{
    DriverKind, GpuArch, InitRecipe, InitStep, RegFunction, Vendor, VerifyCheck,
};
use hw_learn::knowledge::{KnowledgeStore, amd_baseline, arch_map, export_recipe, import_recipe};
use hw_learn::observer::{
    GpuSelector, ObserveConfig, ObserveError, ObserveResult, RpcDirection, TraceEvent,
    TraceEventKind, TraceMode, TraceObserver,
};

// -----------------------------------------------------------------------------
// distiller/classify.rs — GpuGen, classify_events, classify_register
// -----------------------------------------------------------------------------

#[test]
fn classify_gpu_gen_from_chip() {
    assert_eq!(GpuGen::from_chip("gm200"), GpuGen::Maxwell);
    assert_eq!(GpuGen::from_chip("GP100"), GpuGen::Pascal);
    assert_eq!(GpuGen::from_chip("gv100"), GpuGen::Volta);
    assert_eq!(GpuGen::from_chip("TU102"), GpuGen::Turing);
    assert_eq!(GpuGen::from_chip("ga102"), GpuGen::Ampere);
    assert_eq!(GpuGen::from_chip("unknown"), GpuGen::Unknown);
}

#[test]
fn classify_register_nvidia_ranges() {
    assert_eq!(classify_register(0x0002_0000), RegFunction::PowerGate);
    assert_eq!(classify_register(0x0006_0000), RegFunction::ClockEnable);
    assert_eq!(classify_register(0x0010_0000), RegFunction::MemoryConfig);
    assert_eq!(classify_register(0x0040_0000), RegFunction::EngineReset);
    assert_eq!(classify_register(0x0000_0100), RegFunction::InterruptEnable);
}

#[test]
fn classify_register_amd_ranges() {
    assert_eq!(classify_register(0x0000_2100), RegFunction::EngineReset);
    assert_eq!(classify_register(0x0000_D500), RegFunction::PowerGate);
    assert_eq!(classify_register(0x0001_6000), RegFunction::ThermalConfig);
}

#[test]
fn classify_register_for_gen_volta() {
    let r = classify_register_for_gen(0x0002_0000, GpuGen::Volta);
    assert_eq!(r, RegFunction::ClockEnable);
}

#[test]
fn classify_register_for_gen_turing() {
    let r = classify_register_for_gen(0x0050_0000, GpuGen::Turing);
    assert_eq!(r, RegFunction::EngineReset);
}

#[test]
fn classify_register_unknown_offset() {
    assert_eq!(classify_register(0xFFFF_FFFF), RegFunction::Unknown);
}

#[test]
fn classify_events_empty() {
    let events: Vec<TraceEvent> = vec![];
    let classified = classify_events(&events, None);
    assert!(classified.is_empty());
}

#[test]
fn classify_events_with_register_write() {
    let events = vec![TraceEvent {
        timestamp_us: 0,
        kind: TraceEventKind::RegisterWrite {
            offset: 0x0040_0000,
            value: 1,
            width: 4,
        },
        context: "test".to_string(),
    }];
    let classified = classify_events(&events, Some("GA102"));
    assert_eq!(classified.len(), 1);
    assert_eq!(classified[0].function, RegFunction::EngineReset);
}

// -----------------------------------------------------------------------------
// knowledge/amd_baseline.rs — amd_gfx10_compute_init, UniversalInitPhase
// -----------------------------------------------------------------------------

#[test]
fn amd_baseline_recipe_structure() {
    let recipe = amd_baseline::amd_gfx10_compute_init();
    assert_eq!(recipe.source_arch.vendor, Vendor::Amd);
    assert_eq!(recipe.target_arch.compute_class, "gfx1030");
    assert!(recipe.confidence > 0.99);
    assert!(!recipe.steps.is_empty());
}

#[test]
fn amd_baseline_universal_init_phase_from_reg_function() {
    use amd_baseline::UniversalInitPhase;
    assert!(matches!(
        UniversalInitPhase::from_reg_function(RegFunction::PowerGate),
        UniversalInitPhase::Power
    ));
    assert!(matches!(
        UniversalInitPhase::from_reg_function(RegFunction::MemoryConfig),
        UniversalInitPhase::Memory
    ));
    assert!(matches!(
        UniversalInitPhase::from_reg_function(RegFunction::EngineReset),
        UniversalInitPhase::Engine
    ));
}

#[test]
fn amd_baseline_universal_init_phase_display() {
    use amd_baseline::UniversalInitPhase;
    assert_eq!(UniversalInitPhase::Probe.to_string(), "1. Probe");
    assert_eq!(UniversalInitPhase::Verify.to_string(), "7. Verify");
}

#[test]
fn amd_baseline_universal_init_phase_all() {
    use amd_baseline::UniversalInitPhase;
    assert_eq!(UniversalInitPhase::ALL.len(), 7);
}

// -----------------------------------------------------------------------------
// knowledge/arch_map.rs — ArchMapping, stable_registers, architecture_similarity
// -----------------------------------------------------------------------------

fn test_volta_arch() -> GpuArch {
    GpuArch {
        vendor: Vendor::Nvidia,
        generation: "Volta".into(),
        chip: "GV100".into(),
        compute_class: "sm70".into(),
    }
}

fn test_ada_arch() -> GpuArch {
    GpuArch {
        vendor: Vendor::Nvidia,
        generation: "Ada".into(),
        chip: "AD104".into(),
        compute_class: "sm89".into(),
    }
}

fn test_navi_arch() -> GpuArch {
    GpuArch {
        vendor: Vendor::Amd,
        generation: "RDNA2".into(),
        chip: "Navi21".into(),
        compute_class: "gfx1030".into(),
    }
}

#[test]
fn arch_map_similarity_same_arch() {
    let v = test_volta_arch();
    assert!((arch_map::architecture_similarity(&v, &v) - 1.0).abs() < 1e-9);
}

#[test]
fn arch_map_similarity_cross_vendor() {
    let sim = arch_map::architecture_similarity(&test_volta_arch(), &test_navi_arch());
    assert!(sim < 0.2);
}

#[test]
fn arch_map_stable_registers() {
    let nvidia = arch_map::stable_registers(Vendor::Nvidia);
    assert!(!nvidia.is_empty());
    let amd = arch_map::stable_registers(Vendor::Amd);
    assert!(!amd.is_empty());
    let intel = arch_map::stable_registers(Vendor::Intel);
    assert!(!intel.is_empty());
}

#[test]
fn arch_map_arch_mapping_translate() {
    let mut mapping = arch_map::ArchMapping::new(test_ada_arch(), test_volta_arch());
    assert!(mapping.is_empty());
    mapping.add_translation(0x100, 0x200);
    assert_eq!(mapping.translate(0x100), Some(0x200));
    assert_eq!(mapping.translate(0x999), None);
}

#[test]
fn arch_map_architectures_compatible() {
    assert!(arch_map::architectures_compatible(
        &test_volta_arch(),
        &test_ada_arch()
    ));
    assert!(!arch_map::architectures_compatible(
        &test_volta_arch(),
        &test_navi_arch()
    ));
}

// -----------------------------------------------------------------------------
// knowledge/mod.rs — KnowledgeStore, export_recipe, import_recipe
// -----------------------------------------------------------------------------

fn test_recipe() -> InitRecipe {
    InitRecipe {
        source_arch: test_volta_arch(),
        source_driver: DriverKind::Nouveau,
        target_arch: test_volta_arch(),
        steps: vec![
            InitStep::RegisterWrite {
                offset: 0x20000,
                value: 1,
                function: RegFunction::PowerGate,
            },
            InitStep::Verify {
                check: VerifyCheck::ComputeReadback,
            },
        ],
        confidence: 0.5,
        description: "s155 test recipe".into(),
    }
}

#[test]
fn knowledge_store_open_and_lookup() {
    let dir = std::env::temp_dir().join("hw_learn_s155_test");
    let _ = std::fs::remove_dir_all(&dir);

    let mut store = KnowledgeStore::open(&dir).unwrap();
    let recipe = test_recipe();
    let id = store.store(&recipe).unwrap();
    assert!(!id.is_empty());

    let entries = store.lookup(&test_volta_arch());
    assert!(!entries.is_empty());

    let best = store.best_recipe(&test_volta_arch());
    assert_eq!(best.unwrap(), id);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn knowledge_export_import_roundtrip() {
    let recipe = test_recipe();
    let json = export_recipe(&recipe).unwrap();
    let imported = import_recipe(&json).unwrap();
    assert_eq!(imported.steps.len(), recipe.steps.len());
    assert_eq!(imported.description, recipe.description);
}

// -----------------------------------------------------------------------------
// observer/ — ObserveConfig, TraceMode, TraceEvent, TraceObserver
// -----------------------------------------------------------------------------

#[test]
fn observer_trace_mode_debug() {
    let m = TraceMode::MmioTrace;
    let s = format!("{m:?}");
    assert!(s.contains("Mmio") || s.contains("Trace"));

    let m = TraceMode::GspRpc;
    let s = format!("{m:?}");
    assert!(s.contains("Gsp") || s.contains("Rpc"));
}

#[test]
fn observer_gpu_selector_debug() {
    let s = GpuSelector::CardIndex(0);
    let ds = format!("{s:?}");
    assert!(ds.contains("Card") || ds.contains("0"));

    let s = GpuSelector::Auto;
    let ds = format!("{s:?}");
    assert!(ds.contains("Auto"));
}

#[test]
fn observer_rpc_direction_debug() {
    let d = RpcDirection::HostToGsp;
    let s = format!("{d:?}");
    assert!(s.contains("Host") || s.contains("Gsp"));
}

#[test]
fn observer_trace_event_serialization() {
    let evt = TraceEvent {
        timestamp_us: 123,
        kind: TraceEventKind::RegisterWrite {
            offset: 0x1000,
            value: 42,
            width: 4,
        },
        context: "test".to_string(),
    };
    let json = serde_json::to_string(&evt).unwrap();
    let restored: TraceEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.timestamp_us, 123);
}

#[test]
fn observer_observe_missing_trace_path_returns_error() {
    let config = ObserveConfig {
        gpu_selector: GpuSelector::Auto,
        mode: TraceMode::MmioTrace,
        trace_path: None,
        trigger_compute: false,
    };
    let result = TraceObserver::observe(&config);
    assert!(result.is_err());
    if let Err(ObserveError::TraceUnavailable(msg)) = result {
        assert!(msg.contains("trace_path") || msg.contains("mmiotrace"));
    }
}

#[test]
fn observer_observe_result_construction() {
    let result = ObserveResult {
        gpu_id: "card0".to_string(),
        driver: "nouveau".to_string(),
        events: vec![],
        compute_triggered: false,
        duration_us: 1000,
    };
    let s = format!("{result:?}");
    assert!(s.contains("card0"));
}
