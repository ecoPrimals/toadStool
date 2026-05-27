// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::path::Path;

use super::apply::{apply_single_patch, RET_OPCODE};
use super::identity::rename_module_identity;
use super::types::{PatchError, PatchSet, PatchStrategy, PatchTarget};

#[test]
fn volta_patch_set_targets_correct_functions() {
    let ps = PatchSet::volta_warm_handoff();
    assert_eq!(ps.module_name, "nouveau");
    assert_eq!(ps.targets.len(), 8);

    let names: Vec<&str> = ps.targets.iter().map(|t| t.symbol.as_str()).collect();
    assert!(names.contains(&"gf100_gr_fini"));
    assert!(names.contains(&"nvkm_pmu_fini"));
    assert!(names.contains(&"nvkm_mc_disable"));
    assert!(names.contains(&"nvkm_mc_reset"));
    assert!(names.contains(&"gk104_fifo_fini"));
    assert!(names.contains(&"gk104_clkgate_fini"));
    assert!(names.contains(&"nvkm_therm_clkgate_fini"));
    assert!(names.contains(&"g84_therm_fini"));
}

#[test]
fn kepler_patch_set_targets_correct_functions() {
    let ps = PatchSet::kepler_warm_handoff();
    assert_eq!(ps.module_name, "nouveau");
    assert_eq!(ps.targets.len(), 5);
}

#[test]
fn nvidia_patch_set_targets_correct_functions() {
    let ps = PatchSet::nvidia_warm_handoff();
    assert_eq!(ps.module_name, "nvidia");
    assert_eq!(ps.targets.len(), 17);

    let names: Vec<&str> = ps.targets.iter().map(|t| t.symbol.as_str()).collect();
    // Teardown NOPs
    assert!(names.contains(&"nv_pci_remove"));
    assert!(names.contains(&"gpuStateUnload_IMPL"));
    assert!(names.contains(&"gpuStateDestroy_IMPL"));
    assert!(names.contains(&"_deviceTeardown"));
    assert!(names.contains(&"clTeardown_IMPL"));
    assert!(names.contains(&"fecsBufferTeardown"));
    // Co-load isolation NOPs
    assert!(names.contains(&"nv_cap_init"));
    assert!(names.contains(&"nv_cap_drv_init"));
    assert!(names.contains(&"nv_procfs_init"));
    assert!(names.contains(&"nv_cap_procfs_init"));
    assert!(names.contains(&"nvlink_core_init"));
    assert!(names.contains(&"nvswitch_init"));
    assert!(names.contains(&"nv_acpi_init"));

    assert!(ps.targets.iter().all(|t| matches!(
        t.strategy,
        PatchStrategy::RetAtEntry | PatchStrategy::Ret1AtEntry | PatchStrategy::NopCallAt(_)
    )));
}

#[test]
fn by_name_resolves_known_sets() {
    assert!(PatchSet::by_name("volta_warm_handoff").is_some());
    assert!(PatchSet::by_name("kepler_warm_handoff").is_some());
    assert!(PatchSet::by_name("nvidia_warm_handoff").is_some());
    assert!(PatchSet::by_name("nonexistent").is_none());
}

#[test]
fn patch_strategy_serde_roundtrip() {
    let ps = PatchSet::volta_warm_handoff();
    let json = serde_json::to_string(&ps).unwrap();
    let back: PatchSet = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, "volta_warm_handoff");
    assert_eq!(back.targets.len(), 8);
}

#[test]
fn apply_single_patch_patches_ret_after_ftrace() {
    // Simulate a minimal function: e8 00 00 00 00 55 (call + push rbp)
    let mut bytes = vec![0xe8, 0x00, 0x00, 0x00, 0x00, 0x55, 0x48, 0x89];
    let len = bytes.len();
    let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

    let target = PatchTarget {
        symbol: "test_fn".into(),
        strategy: PatchStrategy::RetAfterFtrace,
    };

    let result = apply_single_patch(
        &mut bytes,
        len,
        &symbols,
        &target,
        Path::new("test.ko"),
        0,
    )
    .unwrap();

    assert!(result.applied);
    assert_eq!(result.offset, Some(5));
    assert_eq!(bytes[5], RET_OPCODE);
}

#[test]
fn apply_single_patch_rejects_missing_ftrace() {
    let mut bytes = vec![0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56];
    let len = bytes.len();
    let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

    let target = PatchTarget {
        symbol: "test_fn".into(),
        strategy: PatchStrategy::RetAfterFtrace,
    };

    let result = apply_single_patch(
        &mut bytes,
        len,
        &symbols,
        &target,
        Path::new("test.ko"),
        0,
    );

    assert!(matches!(result, Err(PatchError::NoFtraceCallSite { .. })));
}

#[test]
fn apply_single_patch_rejects_missing_symbol() {
    let mut bytes = vec![0xe8, 0x00, 0x00, 0x00, 0x00, 0x55];
    let len = bytes.len();
    let symbols: HashMap<String, u64> = HashMap::new();

    let target = PatchTarget {
        symbol: "missing_fn".into(),
        strategy: PatchStrategy::RetAfterFtrace,
    };

    let result = apply_single_patch(
        &mut bytes,
        len,
        &symbols,
        &target,
        Path::new("test.ko"),
        0,
    );

    assert!(matches!(result, Err(PatchError::SymbolNotFound { .. })));
}

#[test]
fn apply_single_patch_accepts_nop_sled() {
    let mut bytes = vec![0x90, 0x90, 0x90, 0x90, 0x90, 0x55, 0x48, 0x89];
    let len = bytes.len();
    let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

    let target = PatchTarget {
        symbol: "test_fn".into(),
        strategy: PatchStrategy::RetAfterFtrace,
    };

    let result = apply_single_patch(&mut bytes, len, &symbols, &target, Path::new("test.ko"), 0)
        .unwrap();

    assert!(result.applied);
    assert_eq!(result.offset, Some(5));
    assert_eq!(bytes[5], RET_OPCODE);
    assert!(result.detail.contains("nop-padded"));
}

#[test]
fn apply_single_patch_accepts_zero_pad() {
    let mut bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x55, 0x48, 0x89];
    let len = bytes.len();
    let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

    let target = PatchTarget {
        symbol: "test_fn".into(),
        strategy: PatchStrategy::RetAfterFtrace,
    };

    let result = apply_single_patch(&mut bytes, len, &symbols, &target, Path::new("test.ko"), 0)
        .unwrap();

    assert!(result.applied);
    assert_eq!(result.offset, Some(5));
    assert_eq!(bytes[5], RET_OPCODE);
    assert!(result.detail.contains("nop-padded"));
}

#[test]
fn apply_single_patch_accepts_multibyte_nop() {
    let mut bytes = vec![0x0f, 0x1f, 0x44, 0x00, 0x00, 0x55, 0x48, 0x89];
    let len = bytes.len();
    let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

    let target = PatchTarget {
        symbol: "test_fn".into(),
        strategy: PatchStrategy::RetAfterFtrace,
    };

    let result = apply_single_patch(&mut bytes, len, &symbols, &target, Path::new("test.ko"), 0)
        .unwrap();

    assert!(result.applied);
    assert_eq!(result.offset, Some(5));
    assert_eq!(bytes[5], RET_OPCODE);
    assert!(result.detail.contains("nop-padded"));
}

#[test]
fn apply_single_patch_rejects_mid_instruction() {
    let mut bytes = vec![0xe5, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56];
    let len = bytes.len();
    let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

    let target = PatchTarget {
        symbol: "test_fn".into(),
        strategy: PatchStrategy::RetAfterFtrace,
    };

    let result = apply_single_patch(&mut bytes, len, &symbols, &target, Path::new("test.ko"), 0);
    assert!(matches!(result, Err(PatchError::NoFtraceCallSite { found: 0xe5, .. })));
}

#[test]
fn rename_module_identity_replaces_nul_bounded() {
    let mut data = vec![0u8; 32];
    data[0] = 0;
    data[1..7].copy_from_slice(b"nvidia");
    data[7] = 0;
    data[8..14].copy_from_slice(b"nvidia");
    data[14] = b'=';

    let count = rename_module_identity(&mut data, "nvidia", "nvsov").unwrap();
    assert_eq!(count, 2);
    assert_eq!(&data[1..6], b"nvsov");
    assert_eq!(data[6], 0); // NUL-padded since "nvsov" is shorter
    assert_eq!(&data[8..13], b"nvsov");
}

#[test]
fn rename_rejects_longer_new_name() {
    let mut data = vec![0u8; 16];
    let result = rename_module_identity(&mut data, "nv", "nvidia_sovereign_extended");
    assert!(result.is_err());
}

#[test]
fn patch_set_min_applied_default_serde() {
    // When min_applied is absent from JSON, it defaults to 1
    let json = r#"{"name":"test","module_name":"test","targets":[]}"#;
    let ps: PatchSet = serde_json::from_str(json).unwrap();
    assert_eq!(ps.min_applied, 1);
}

#[test]
fn patch_set_min_applied_explicit_serde() {
    let json = r#"{"name":"test","module_name":"test","targets":[],"min_applied":3}"#;
    let ps: PatchSet = serde_json::from_str(json).unwrap();
    assert_eq!(ps.min_applied, 3);
}
