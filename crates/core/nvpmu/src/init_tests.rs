// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

struct FakeRegAccess {
    registers: std::collections::HashMap<u64, u32>,
}

impl FakeRegAccess {
    fn new() -> Self {
        Self {
            registers: std::collections::HashMap::new(),
        }
    }
}

impl RegisterAccess for FakeRegAccess {
    fn read_u32(&self, offset: u64) -> std::result::Result<u32, String> {
        Ok(*self.registers.get(&offset).unwrap_or(&0))
    }

    fn write_u32(&mut self, offset: u64, value: u32) -> std::result::Result<(), String> {
        self.registers.insert(offset, value);
        Ok(())
    }
}

#[test]
fn snapshot_captures_register_values() {
    let mut access = FakeRegAccess::new();
    access.registers.insert(0x100, 0xAABB_CCDD);
    access.registers.insert(0x200, 0x1122_3344);

    let recipe = InitRecipe {
        source_arch: GpuArch {
            vendor: Vendor::Nvidia,
            generation: String::new(),
            chip: "test".into(),
            compute_class: String::new(),
        },
        source_driver: DriverKind::Nouveau,
        target_arch: GpuArch {
            vendor: Vendor::Nvidia,
            generation: String::new(),
            chip: "test".into(),
            compute_class: String::new(),
        },
        steps: vec![
            InitStep::RegisterWrite {
                offset: 0x100,
                value: 0xFFFF_FFFF,
                function: RegFunction::Unknown,
            },
            InitStep::RegisterWrite {
                offset: 0x200,
                value: 0x0000_0000,
                function: RegFunction::Unknown,
            },
        ],
        confidence: 1.0,
        description: "test recipe".into(),
    };

    let snapshot = RegisterSnapshot::capture(&recipe, &access);
    assert_eq!(snapshot.len(), 2);

    access.registers.insert(0x100, 0xFFFF_FFFF);
    access.registers.insert(0x200, 0x0000_0000);

    let ok = snapshot.rollback(&mut access);
    assert!(ok);
    assert_eq!(access.registers[&0x100], 0xAABB_CCDD);
    assert_eq!(access.registers[&0x200], 0x1122_3344);
}

#[test]
fn empty_snapshot_rollback_succeeds() {
    let mut access = FakeRegAccess::new();
    let recipe = InitRecipe {
        source_arch: GpuArch {
            vendor: Vendor::Nvidia,
            generation: String::new(),
            chip: "test".into(),
            compute_class: String::new(),
        },
        source_driver: DriverKind::Nouveau,
        target_arch: GpuArch {
            vendor: Vendor::Nvidia,
            generation: String::new(),
            chip: "test".into(),
            compute_class: String::new(),
        },
        steps: vec![],
        confidence: 1.0,
        description: "empty".into(),
    };

    let snapshot = RegisterSnapshot::capture(&recipe, &access);
    assert!(snapshot.is_empty());
    assert!(snapshot.rollback(&mut access));
}

#[test]
fn to_init_recipe_converts_legacy_format() {
    let recipe = Recipe {
        chip: "gv100".into(),
        steps: vec![RecipeStep {
            offset: 0x100,
            value: 0xAA,
            width: 4,
            delay_us: Some(10),
        }],
        verify_reads: vec![VerifyRead {
            offset: 0x200,
            expected_mask: 0xFF,
            expected_value: 0x42,
        }],
    };

    let init = to_init_recipe(&recipe);
    assert_eq!(init.target_arch.chip, "gv100");
    assert_eq!(init.steps.len(), 3);
}

#[test]
fn apply_recipe_parses_legacy_json() {
    let json = r#"{"chip":"gv100","steps":[],"verify_reads":[]}"#;
    let mut access = FakeRegAccess::new();
    let result = apply_recipe(json, &mut access).unwrap();
    assert!(result.success);
    assert_eq!(result.chip, "gv100");
}
