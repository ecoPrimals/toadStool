// SPDX-License-Identifier: AGPL-3.0-or-later
//! Where a VBIOS keeps its init scripts — discovered from the ROM itself.
//!
//! This is the single place that answers "which scripts does this ROM have, and
//! where do they start and end?". Both the interpreter and the register-write
//! scanner consume it, so the two cannot form different opinions about the same
//! image.
//!
//! # Why this exists
//!
//! It previously did not, and the two consumers each derived script locations
//! their own way. On a measured Tesla K80 (GK210) image the interpreter walked
//! all six scripts the table advertises, while the scanner took only the first
//! table entry and scanned forward to the end of the ROM. Entry `[2]` on that
//! part lives at `0x65ff`, *below* the first entry at `0x9271`, so scanning
//! forward could never reach it — 32 register writes were invisible to one
//! consumer and visible to the other.
//!
//! # Capability, not card name
//!
//! Nothing here asks which GPU it is looking at. The ROM advertises its own
//! layout in the BIT 'I' entry and this module reports what was found. A
//! device ID would be a second source of truth that can disagree with the
//! artifact actually being parsed.
//!
//! Reference: nouveau `nvkm/subdev/bios/init.c` (Ben Skeggs, Red Hat)

use crate::error::DevinitError;

use super::super::vbios::BitTable;

/// A `data_size` at or above this means BIT 'I' carries the extended fields
/// (PMU firmware pointers) in addition to the init-table pointer.
///
/// Kepler-era images measure `0x12`; Maxwell and later measure `0x1c` or more.
/// The threshold is a property of the table, not of any particular part.
const BIT_I_EXTENDED_MIN: u16 = 0x1c;

/// Upper bound on entries walked in the init-script table.
///
/// The table is NUL-terminated; this only bounds a corrupt image that never
/// terminates. Measured images use a single-digit number of scripts.
const MAX_SCRIPT_ENTRIES: usize = 64;

/// What the BIT 'I' entry advertises about its own layout.
///
/// These are properties read out of the ROM, not inferences about which card
/// produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitICapability {
    /// BIT 'I' entry version field.
    pub version: u8,
    /// Size of the BIT 'I' data payload, in bytes.
    pub data_size: u16,
    /// Whether the payload is large enough to carry the extended PMU firmware
    /// fields. Short payloads carry only the init-table pointer.
    pub carries_pmu_fields: bool,
}

impl BitICapability {
    /// Whether this image uses the shorter opcode encodings.
    ///
    /// Images without the extended BIT 'I' fields also predate several opcode
    /// stride changes, so this doubles as the encoding selector. It is derived
    /// from the table rather than from a device ID for the reason described in
    /// the module docs.
    #[must_use]
    pub const fn uses_short_opcode_strides(&self) -> bool {
        !self.carries_pmu_fields
    }
}

/// One init script, with a discovered extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptEntry {
    /// Index in the BIT 'I' init-script table.
    pub index: usize,
    /// ROM offset where the script begins.
    pub offset: usize,
    /// Bytes from `offset` to the start of the next script in ROM order, or to
    /// the end of the ROM for the last one.
    ///
    /// This is an upper bound on the script's extent, not its true length —
    /// scripts terminate on an opcode, not on a byte count. It exists so a
    /// scanner can stop somewhere principled instead of running to the end of
    /// the image.
    pub extent: usize,
}

/// The init-script table discovered in a VBIOS image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptTable {
    /// ROM offset of the `u16` entry array.
    pub table_offset: usize,
    /// Discovered scripts, in table order.
    pub entries: Vec<ScriptEntry>,
    /// What BIT 'I' said about itself.
    pub capability: BitICapability,
}

impl ScriptTable {
    /// Discover the init-script table by reading the ROM's own BIT 'I' entry.
    ///
    /// # Errors
    ///
    /// Returns an error if BIT cannot be parsed, the 'I' entry is missing or
    /// truncated, the init-table pointer is out of range, or the table holds no
    /// usable entries.
    pub fn discover(rom: &[u8]) -> Result<Self, DevinitError> {
        let bit = BitTable::parse(rom)?;
        let bit_i = bit.find(b'I').ok_or(DevinitError::BitINotFound)?;

        let i_off = bit_i.data_offset as usize;
        let Some(table_offset) = read_u16(rom, i_off).map(usize::from) else {
            return Err(DevinitError::BitIDataTooShort);
        };

        let capability = BitICapability {
            version: bit_i.version,
            data_size: bit_i.data_size,
            carries_pmu_fields: bit_i.data_size >= BIT_I_EXTENDED_MIN,
        };

        if table_offset == 0 || table_offset + 2 > rom.len() {
            return Err(DevinitError::InterpreterInitTablesInvalid);
        }

        // Walk the NUL-terminated table of u16 script offsets.
        let mut offsets = Vec::new();
        for index in 0..MAX_SCRIPT_ENTRIES {
            let Some(offset) = read_u16(rom, table_offset + index * 2).map(usize::from) else {
                break;
            };
            if offset == 0 {
                break;
            }
            if offset >= rom.len() {
                tracing::debug!(
                    index,
                    offset = format!("{offset:#06x}"),
                    "init-script entry points outside the ROM; stopping table walk"
                );
                break;
            }
            offsets.push(offset);
        }

        if offsets.is_empty() {
            return Err(DevinitError::NoBootScriptsInBitI);
        }

        // An entry's extent runs to whichever script starts next *in ROM order*,
        // which is not table order: measured images list entries out of address
        // order, so sorting is required for the bound to mean anything.
        let mut ascending: Vec<usize> = offsets.clone();
        ascending.sort_unstable();
        ascending.dedup();

        let entries = offsets
            .iter()
            .enumerate()
            .map(|(index, &offset)| {
                let next = ascending
                    .iter()
                    .copied()
                    .find(|&candidate| candidate > offset)
                    .unwrap_or(rom.len());
                ScriptEntry {
                    index,
                    offset,
                    extent: next.saturating_sub(offset),
                }
            })
            .collect();

        let table = Self {
            table_offset,
            entries,
            capability,
        };

        tracing::debug!(
            table_offset = format!("{table_offset:#06x}"),
            scripts = table.entries.len(),
            data_size = capability.data_size,
            carries_pmu_fields = capability.carries_pmu_fields,
            "discovered VBIOS init-script table"
        );

        Ok(table)
    }

    /// Script entry points in table order.
    #[cfg(test)]
    fn offsets(&self) -> Vec<usize> {
        self.entries.iter().map(|e| e.offset).collect()
    }
}

/// Read a little-endian `u16`, or `None` if it would run past the end.
fn read_u16(rom: &[u8], offset: usize) -> Option<u16> {
    let bytes = rom.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal ROM carrying a BIT header, an 'I' entry, and a script
    /// table at a known offset.
    fn synth_rom(data_size: u16, script_offsets: &[u16]) -> Vec<u8> {
        let mut rom = vec![0u8; 0x4000];

        let bit_at = 0x100usize;
        let i_data_at = 0x200usize;
        let table_at = 0x300usize;

        rom[bit_at..bit_at + 5].copy_from_slice(&[0xff, 0xb8, b'B', b'I', b'T']);
        rom[bit_at + 8] = 12; // header size
        rom[bit_at + 9] = 6; // entry size
        rom[bit_at + 10] = 1; // entry count

        let e = bit_at + 12;
        rom[e] = b'I';
        rom[e + 1] = 1; // version
        rom[e + 2..e + 4].copy_from_slice(&data_size.to_le_bytes());
        rom[e + 4..e + 6].copy_from_slice(&u16::try_from(i_data_at).unwrap().to_le_bytes());

        // BIT 'I' data[0:2] -> script table
        rom[i_data_at..i_data_at + 2]
            .copy_from_slice(&u16::try_from(table_at).unwrap().to_le_bytes());

        for (k, off) in script_offsets.iter().enumerate() {
            let at = table_at + k * 2;
            rom[at..at + 2].copy_from_slice(&off.to_le_bytes());
        }

        rom
    }

    #[test]
    fn discovers_all_entries_in_table_order() {
        let rom = synth_rom(0x12, &[0x2000, 0x1000, 0x3000]);
        let table = ScriptTable::discover(&rom).expect("discover");
        assert_eq!(table.offsets(), vec![0x2000, 0x1000, 0x3000]);
        assert_eq!(table.table_offset, 0x300);
    }

    /// The bug this module exists to prevent: a consumer that starts at the
    /// first table entry and scans forward never reaches an earlier script.
    #[test]
    fn entry_below_the_first_is_still_discovered() {
        let rom = synth_rom(0x12, &[0x2000, 0x0900, 0x3000]);
        let table = ScriptTable::discover(&rom).expect("discover");
        assert!(
            table.offsets().contains(&0x0900),
            "a script below the first table entry must still be discovered"
        );
    }

    /// Extent runs to the next script in *address* order, not table order.
    #[test]
    fn extent_bounds_by_next_script_in_rom_order() {
        let rom = synth_rom(0x12, &[0x2000, 0x1000, 0x3000]);
        let table = ScriptTable::discover(&rom).expect("discover");
        let by_offset = |o: usize| {
            table
                .entries
                .iter()
                .find(|e| e.offset == o)
                .copied()
                .expect("entry")
        };
        assert_eq!(by_offset(0x1000).extent, 0x1000, "0x1000 -> 0x2000");
        assert_eq!(by_offset(0x2000).extent, 0x1000, "0x2000 -> 0x3000");
        assert_eq!(
            by_offset(0x3000).extent,
            rom.len() - 0x3000,
            "last script runs to end of ROM"
        );
    }

    #[test]
    fn short_bit_i_reports_no_pmu_fields_and_short_strides() {
        let rom = synth_rom(0x12, &[0x1000]);
        let cap = ScriptTable::discover(&rom).expect("discover").capability;
        assert!(!cap.carries_pmu_fields);
        assert!(cap.uses_short_opcode_strides());
        assert_eq!(cap.data_size, 0x12);
    }

    #[test]
    fn extended_bit_i_reports_pmu_fields() {
        let rom = synth_rom(0x20, &[0x1000]);
        let cap = ScriptTable::discover(&rom).expect("discover").capability;
        assert!(cap.carries_pmu_fields);
        assert!(!cap.uses_short_opcode_strides());
    }

    #[test]
    fn table_terminates_on_zero() {
        let rom = synth_rom(0x12, &[0x1000, 0x2000, 0x0000, 0x3000]);
        let table = ScriptTable::discover(&rom).expect("discover");
        assert_eq!(table.offsets(), vec![0x1000, 0x2000]);
    }

    #[test]
    fn entry_pointing_outside_the_rom_stops_the_walk() {
        let rom = synth_rom(0x12, &[0x1000, 0xF000]);
        let table = ScriptTable::discover(&rom).expect("discover");
        assert_eq!(table.offsets(), vec![0x1000]);
    }

    #[test]
    fn empty_table_is_an_error_not_an_empty_success() {
        let rom = synth_rom(0x12, &[0x0000]);
        assert!(ScriptTable::discover(&rom).is_err());
    }

    fn fixture(name: &str) -> Option<Vec<u8>> {
        std::fs::read(format!(
            "{}/../../../testdata/vbios/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .ok()
    }

    /// Measured GK210 layout, pinned against the real image.
    ///
    /// The numbers here are read off a Tesla K80 VBIOS dump, not chosen: BIT
    /// 'I' is version 1 with an 0x12 payload, the init-script table sits at
    /// `0x5046`, and it lists six scripts. Entry `[2]` is at `0x65ff`, below
    /// entry `[0]` at `0x9271` — the out-of-order case that broke the scanner.
    ///
    /// Fixture is vendor firmware and is gitignored; the test skips without it.
    #[test]
    fn kepler_rom_layout_is_discovered_as_measured() {
        let Some(rom) = fixture("k80_gk210.rom") else {
            eprintln!("skipping: testdata/vbios/k80_gk210.rom not present");
            return;
        };

        let table = ScriptTable::discover(&rom).expect("K80 ROM must yield a script table");

        assert_eq!(table.table_offset, 0x5046, "measured init-table pointer");
        assert_eq!(table.entries.len(), 6, "measured script count");
        assert_eq!(table.capability.data_size, 0x12);
        assert_eq!(table.capability.version, 1);
        assert!(!table.capability.carries_pmu_fields);
        assert!(table.capability.uses_short_opcode_strides());

        let offsets = table.offsets();
        assert_eq!(
            offsets,
            vec![0x9271, 0x9ba3, 0x65ff, 0xb6d6, 0xb6d7, 0xb848],
            "scripts are listed out of address order on this part"
        );

        // The regression itself: a consumer starting at entry [0] and scanning
        // forward can never reach entry [2].
        let first = offsets[0];
        assert!(
            offsets.iter().any(|&o| o < first),
            "this ROM must exercise the out-of-order case, or the test proves nothing"
        );
    }

    /// Extents must partition the scripts rather than each running to ROM end.
    #[test]
    fn kepler_extents_do_not_overlap_the_next_script() {
        let Some(rom) = fixture("k80_gk210.rom") else {
            eprintln!("skipping: testdata/vbios/k80_gk210.rom not present");
            return;
        };

        let table = ScriptTable::discover(&rom).expect("discover");
        let mut sorted: Vec<_> = table.entries.clone();
        sorted.sort_by_key(|e| e.offset);

        for pair in sorted.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(
                a.offset + a.extent <= b.offset,
                "script at {:#06x} (extent {}) runs into the next at {:#06x}",
                a.offset,
                a.extent,
                b.offset
            );
        }
    }
}
