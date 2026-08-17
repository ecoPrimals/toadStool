// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(missing_docs, reason = "VBIOS script interpreter; full docs planned")]
//! VBIOS init script host-side interpreter — executes opcode stream via BAR0.
//!
//! Reference: nouveau nvkm/subdev/bios/init.c (Ben Skeggs, Red Hat)

mod opcodes;
mod pri;

use crate::error::DevinitError;
use crate::vfio::device::MappedBar;

use super::super::vbios::BitTable;
use super::discovery::ScriptTable;

/// Statistics from a VBIOS interpreter run.
#[derive(Debug, Clone, Default)]
pub struct InterpreterStats {
    pub ops_executed: usize,
    pub writes_applied: usize,
    pub writes_skipped_pri: usize,
    pub ops_skipped: usize,
    pub conditions_evaluated: usize,
    pub delays_total_us: u64,
    pub unknown_opcodes: Vec<(usize, u8)>,
    pub pri_faults: usize,
    pub pri_recoveries: usize,
    pub faulted_domains: Vec<String>,
}

/// BIOS generation determines opcode stride/semantic differences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BiosGeneration {
    /// Pre-Maxwell: shorter strides for some opcodes (GK110, GK210, etc.)
    Kepler,
    /// Maxwell and later: extended opcode formats.
    MaxwellPlus,
}

/// Everything outside the interpreter: the registers it drives and the clock
/// it waits on.
///
/// The VBIOS interpreter exists to be *wrong* in interesting ways — a misparse
/// makes it write plausible-looking garbage to real registers, which cost
/// three Tesla K80 dies on 2026-08-16. Debugging that against hardware means a
/// die per iteration and a reboot to get it back.
///
/// Behind this trait the same interpreter runs against a recorded bus with a
/// real ROM dump, in microseconds, with the writes captured instead of
/// applied. `delay_us` is here for the same reason: one Volta script asks for
/// ten seconds of sleeps, which is fine on hardware and absurd in a test.
pub trait ScriptBus {
    /// Read a BAR0 register. `None` if the access could not be performed.
    fn read_u32(&self, offset: usize) -> Option<u32>;

    /// Write a BAR0 register. Failures are not reported: the interpreter has
    /// no recovery for them beyond the PRI backpressure it already tracks.
    fn write_u32(&self, offset: usize, value: u32);

    /// Wait, as the script asked.
    fn delay_us(&self, usec: u64);
}

impl ScriptBus for MappedBar {
    fn read_u32(&self, offset: usize) -> Option<u32> {
        MappedBar::read_u32(self, offset).ok()
    }

    fn write_u32(&self, offset: usize, value: u32) {
        let _ = MappedBar::write_u32(self, offset, value);
    }

    fn delay_us(&self, usec: u64) {
        std::thread::sleep(std::time::Duration::from_micros(usec));
    }
}

/// State for the VBIOS init script interpreter.
struct VbiosInterpreter<'a> {
    bar0: &'a dyn ScriptBus,
    rom: &'a [u8],
    offset: usize,
    execute: bool,
    repeat_count: u8,
    repeat_offset: usize,
    nested: u32,
    stats: InterpreterStats,
    /// PRI backpressure: consecutive faults without a clean read.
    pri_consecutive_faults: u32,
    /// PRI backpressure: threshold before attempting bus recovery.
    pri_fault_threshold: u32,
    /// PRI backpressure: domain -> fault count. Domains with 3+ faults are skipped.
    pri_domain_faults: std::collections::HashMap<String, u32>,
    /// BIOS generation for opcode stride/semantic branching.
    bios_gen: BiosGeneration,
    /// Whether register writes actually reach the hardware.
    ///
    /// False during the validation pass. A desynced parse still "executes"
    /// opcodes — it just decodes them out of whatever bytes follow — and a
    /// write opcode built from garbage writes a garbage value to a garbage
    /// address. Those writes must not land before we know the parse is real.
    writes_armed: bool,
}

impl<'a> VbiosInterpreter<'a> {
    fn new(bar0: &'a dyn ScriptBus, rom: &'a [u8], start: usize, bios_gen: BiosGeneration) -> Self {
        Self::with_writes(bar0, rom, start, bios_gen, true)
    }

    /// Build an interpreter that parses without touching the hardware.
    fn validating(
        bar0: &'a dyn ScriptBus,
        rom: &'a [u8],
        start: usize,
        bios_gen: BiosGeneration,
    ) -> Self {
        Self::with_writes(bar0, rom, start, bios_gen, false)
    }

    fn with_writes(
        bar0: &'a dyn ScriptBus,
        rom: &'a [u8],
        start: usize,
        bios_gen: BiosGeneration,
        writes_armed: bool,
    ) -> Self {
        Self {
            bar0,
            rom,
            offset: start,
            execute: true,
            repeat_count: 0,
            repeat_offset: 0,
            nested: 0,
            stats: InterpreterStats::default(),
            pri_consecutive_faults: 0,
            pri_fault_threshold: 5,
            pri_domain_faults: std::collections::HashMap::new(),
            bios_gen,
            writes_armed,
        }
    }

    fn rd08(&self, off: usize) -> u8 {
        self.rom.get(off).copied().unwrap_or(0)
    }

    fn rd16(&self, off: usize) -> u16 {
        if off + 2 <= self.rom.len() {
            u16::from_le_bytes([self.rom[off], self.rom[off + 1]])
        } else {
            0
        }
    }

    fn rd32(&self, off: usize) -> u32 {
        if off + 4 <= self.rom.len() {
            u32::from_le_bytes([
                self.rom[off],
                self.rom[off + 1],
                self.rom[off + 2],
                self.rom[off + 3],
            ])
        } else {
            0
        }
    }

    /// Look up a condition from the VBIOS condition table.
    /// Returns true if the condition is met (register & mask == value).
    fn condition_met(&mut self, cond_table_off: usize, cond_idx: u8) -> bool {
        let entry_off = cond_table_off + (cond_idx as usize) * 12;
        if entry_off + 12 > self.rom.len() {
            return true; // unknown condition → execute anyway
        }
        let reg = self.rd32(entry_off);
        let mask = self.rd32(entry_off + 4);
        let value = self.rd32(entry_off + 8);
        if reg == 0 {
            return true;
        }
        let actual = self.bar0_rd32(reg);
        (actual & mask) == value
    }

    /// Resolve the "init tables" base pointer from BIT 'I' offset 0x00.
    fn find_init_tables_base(&self) -> usize {
        if let Ok(bit) = BitTable::parse(self.rom)
            && let Some(i_entry) = bit.find(b'I')
        {
            let i_off = i_entry.data_offset as usize;
            if i_off + 2 <= self.rom.len() {
                return self.rd16(i_off) as usize;
            }
        }
        0
    }

    /// Find the condition table offset from the init tables base.
    fn find_condition_table(&self) -> usize {
        let base = self.find_init_tables_base();
        if base == 0 || base + 8 > self.rom.len() {
            return 0;
        }
        self.rd16(base + 0x06) as usize
    }

    fn run(&mut self) -> Result<(), DevinitError> {
        let cond_table = self.find_condition_table();
        self.nested += 1;
        let max_ops = 50_000;

        while self.offset != 0 && self.stats.ops_executed < max_ops {
            let op = self.rd08(self.offset);
            self.stats.ops_executed += 1;

            opcodes::dispatch_opcode(self, op, cond_table)?;
        }

        self.nested -= 1;
        Ok(())
    }
}

/// Number of RAM-restrict groups, read directly out of the BIT 'M' entry.
///
/// This count sets the payload length of opcodes `0x87`, `0x88`, `0x8A` and
/// `0x8F`, which carry one `u32` per group. Get it wrong and the offset
/// advances by the wrong amount, so **every byte after the first such opcode
/// is decoded at the wrong boundary** — the rest of the script becomes noise.
///
/// The count is a field of the BIT 'M' data itself, at a version-dependent
/// offset (nouveau `nvbios_ramcfg_count`, `bios/ramcfg.c`):
///
/// | BIT 'M' version | Requires | Count is at |
/// |-----------------|----------|-------------|
/// | 1 | `data_size >= 5` | `data_offset + 2` |
/// | 2 | `data_size >= 3` | `data_offset + 0` |
///
/// It is **not** reached by dereferencing `data[0:2]` as a pointer to the
/// rammap table. An earlier version did that and then read `+4` of whatever it
/// landed on. On a Tesla K80 that produced 58, which failed the sanity bound
/// and fell through to a default of 4, where the true count is 8. Measured on
/// that ROM: the boot scripts decoded at **75% unknown opcodes with 4, and 15%
/// with 8** — the misparse that made the interpreter refuse to run, and which
/// had previously driven 196 writes decoded from noise into a live die.
pub(crate) fn ram_restrict_group_count(rom: &[u8]) -> usize {
    const DEFAULT_GROUPS: usize = 4;

    let Ok(bit) = BitTable::parse(rom) else {
        return DEFAULT_GROUPS;
    };
    let Some(m) = bit.find(b'M') else {
        return DEFAULT_GROUPS;
    };

    let base = m.data_offset as usize;
    let field = match m.version {
        1 if m.data_size >= 5 => base + 2,
        2 if m.data_size >= 3 => base,
        _ => return DEFAULT_GROUPS,
    };

    match rom.get(field) {
        Some(&count) if count > 0 => usize::from(count),
        _ => DEFAULT_GROUPS,
    }
}

/// Maximum share of unrecognised opcodes tolerated before a script is
/// treated as misparsed rather than merely unsupported.
///
/// A correctly located script decodes almost entirely; the handful of misses
/// are genuinely unimplemented opcodes. A misparsed one misses most of the
/// stream, because each bad decode advances the offset by the wrong amount
/// and everything after it is read at the wrong boundary.
const MAX_UNKNOWN_PERCENT: usize = 25;

/// Execute VBIOS init scripts from the host CPU via BAR0.
///
/// This is the sovereign alternative to PMU FALCON execution. It interprets
/// the boot script opcode stream directly, respecting control flow, conditions,
/// and delays. Approximately 50 opcodes are handled.
pub fn interpret_boot_scripts(
    bar0: &dyn ScriptBus,
    rom: &[u8],
) -> Result<InterpreterStats, DevinitError> {
    // Shared with the register-write scanner so the two cannot form different
    // opinions about which scripts this image contains.
    let table = ScriptTable::discover(rom)?;

    // The encoding is selected by what the ROM advertises about itself, not by
    // a device ID: the artifact being parsed is the only authority on how it is
    // encoded, and a second source can disagree with it.
    let bios_gen = if table.capability.uses_short_opcode_strides() {
        BiosGeneration::Kepler
    } else {
        BiosGeneration::MaxwellPlus
    };

    tracing::debug!(
        data_size = table.capability.data_size,
        script_table = format!("{:#06x}", table.table_offset),
        scripts = table.entries.len(),
        gen = ?bios_gen,
        "VBIOS interpreter entry points"
    );

    let mut combined_stats = InterpreterStats::default();

    for entry in &table.entries {
        let script_idx = entry.index;
        let script_off = entry.offset;

        tracing::debug!(
            script_idx,
            script_off = format!("{script_off:#06x}"),
            "VBIOS interpreter running init script"
        );

        // Validation pass: walk the stream with writes disarmed and see
        // whether we are actually decoding this script.
        //
        // On a Tesla K80 the interpreter reported 1044 ops of which 796 were
        // unknown — a 76% miss rate, meaning the byte stream was not being
        // parsed as the instructions it contains. It issued 196 register
        // writes anyway, decoded from that garbage, and wedged the die.
        // A parser that does not understand its input must not drive hardware.
        let mut check = VbiosInterpreter::validating(bar0, rom, script_off, bios_gen);
        let _ = check.run();
        let seen = check.stats.ops_executed;
        let unknown = check.stats.unknown_opcodes.len();

        if seen > 0 && unknown * 100 / seen > MAX_UNKNOWN_PERCENT {
            tracing::warn!(
                script_idx,
                script_off = format!("{script_off:#06x}"),
                ops = seen,
                unknown,
                pct = unknown * 100 / seen,
                "VBIOS: script does not decode — refusing to execute its writes"
            );
            combined_stats.ops_skipped += seen;
            combined_stats
                .unknown_opcodes
                .extend(check.stats.unknown_opcodes.clone());
            continue;
        }

        let mut interp = VbiosInterpreter::new(bar0, rom, script_off, bios_gen);
        match interp.run() {
            Ok(()) => {
                tracing::info!(
                    script_idx,
                    ops = interp.stats.ops_executed,
                    writes = interp.stats.writes_applied,
                    pri_skipped = interp.stats.writes_skipped_pri,
                    unknown = interp.stats.unknown_opcodes.len(),
                    pri_faults = interp.stats.pri_faults,
                    pri_recoveries = interp.stats.pri_recoveries,
                    "VBIOS init script completed"
                );
            }
            Err(e) => {
                tracing::error!(
                    script_idx,
                    error = %e,
                    pri_faults = interp.stats.pri_faults,
                    pri_recoveries = interp.stats.pri_recoveries,
                    "VBIOS init script failed"
                );
            }
        }
        combined_stats.ops_executed += interp.stats.ops_executed;
        combined_stats.writes_applied += interp.stats.writes_applied;
        combined_stats.writes_skipped_pri += interp.stats.writes_skipped_pri;
        combined_stats.ops_skipped += interp.stats.ops_skipped;
        combined_stats.conditions_evaluated += interp.stats.conditions_evaluated;
        combined_stats.delays_total_us += interp.stats.delays_total_us;
        combined_stats
            .unknown_opcodes
            .extend(interp.stats.unknown_opcodes.clone());
        combined_stats.pri_faults += interp.stats.pri_faults;
        combined_stats.pri_recoveries += interp.stats.pri_recoveries;
        for (domain, &count) in &interp.pri_domain_faults {
            if count >= 3 && !combined_stats.faulted_domains.contains(domain) {
                combined_stats.faulted_domains.push(domain.clone());
            }
        }
    }

    tracing::info!(
        scripts = table.entries.len(),
        ops = combined_stats.ops_executed,
        writes = combined_stats.writes_applied,
        pri_skipped = combined_stats.writes_skipped_pri,
        delays_ms = combined_stats.delays_total_us as f64 / 1000.0,
        unknown = combined_stats.unknown_opcodes.len(),
        "VBIOS interpreter total"
    );

    if combined_stats.pri_faults > 0 {
        tracing::warn!(
            faults = combined_stats.pri_faults,
            recoveries = combined_stats.pri_recoveries,
            faulted_domains = combined_stats.faulted_domains.len(),
            domains = ?combined_stats.faulted_domains,
            "PRI backpressure"
        );
    }

    if !combined_stats.unknown_opcodes.is_empty() {
        let first_few: Vec<_> = combined_stats.unknown_opcodes.iter().take(10).collect();
        tracing::debug!(opcodes = ?first_few, "unknown VBIOS opcodes");
    }

    Ok(combined_stats)
}

// The former tests here built a ROM in which the group count sat behind a
// rammap pointer, then asserted the reader found it. They passed because the
// fixture was constructed to match the reader's own misreading — the test and
// the code shared the bug, so the test could never see it. Replaced by
// `ram_restrict_tests` below, which pins the layout to real hardware data.

#[cfg(test)]
mod desync_guard_tests {
    use super::MAX_UNKNOWN_PERCENT;

    fn refuses(ops: usize, unknown: usize) -> bool {
        ops > 0 && unknown * 100 / ops > MAX_UNKNOWN_PERCENT
    }

    /// The measured K80 run: 1044 ops, 796 unknown. It issued 196 writes from
    /// this parse and wedged the die.
    #[test]
    fn k80_desynced_stream_is_refused() {
        assert!(refuses(1044, 796), "76% unknown must not drive writes");
    }

    /// A script with a few genuinely unimplemented opcodes still runs —
    /// refusing those would regress every working path.
    #[test]
    fn mostly_understood_script_still_executes() {
        assert!(!refuses(1044, 20));
        assert!(!refuses(100, 25), "exactly at threshold is allowed");
    }

    /// Just past the threshold is refused.
    #[test]
    fn just_over_threshold_is_refused() {
        assert!(refuses(100, 26));
    }

    /// An empty script is not a desync; there is nothing to misparse.
    #[test]
    fn empty_script_is_not_a_desync() {
        assert!(!refuses(0, 0));
    }
}

/// A [`ScriptBus`] that records instead of touching hardware.
///
/// Reads come from a seeded map, defaulting to zero, so conditional opcodes
/// take a deterministic path. Writes and delays are captured rather than
/// applied. This is what lets the interpreter be debugged against a real ROM
/// without spending a die per iteration.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct RecordingBus {
    seed: std::collections::HashMap<usize, u32>,
    writes: std::cell::RefCell<Vec<(usize, u32)>>,
    delayed_us: std::cell::Cell<u64>,
}

#[cfg(test)]
impl RecordingBus {
    fn writes(&self) -> Vec<(usize, u32)> {
        self.writes.borrow().clone()
    }
}

#[cfg(test)]
impl ScriptBus for RecordingBus {
    fn read_u32(&self, offset: usize) -> Option<u32> {
        Some(self.seed.get(&offset).copied().unwrap_or(0))
    }

    fn write_u32(&self, offset: usize, value: u32) {
        self.writes.borrow_mut().push((offset, value));
    }

    fn delay_us(&self, usec: u64) {
        self.delayed_us.set(self.delayed_us.get() + usec);
    }
}

#[cfg(test)]
mod ram_restrict_tests {
    use super::*;

    /// Build a minimal ROM carrying one BIT 'M' entry.
    fn rom_with_bit_m(version: u8, data: &[u8]) -> Vec<u8> {
        let mut rom = vec![0u8; 0x400];
        let bit = 0x100usize;
        rom[bit..bit + 5].copy_from_slice(&[0xFF, 0xB8, b'B', b'I', b'T']);
        rom[bit + 9] = 6; // entry_size
        rom[bit + 10] = 1; // entry_count

        let data_off = 0x200usize;
        let e = bit + 12;
        rom[e] = b'M';
        rom[e + 1] = version;
        rom[e + 2..e + 4].copy_from_slice(&(data.len() as u16).to_le_bytes());
        rom[e + 4..e + 6].copy_from_slice(&(data_off as u16).to_le_bytes());
        rom[data_off..data_off + data.len()].copy_from_slice(data);
        rom
    }

    /// The exact BIT 'M' data from the Tesla K80 (GK210) VBIOS: version 2,
    /// 17 bytes, first byte 8. Reading it as a rammap pointer instead yielded
    /// 58, which fell through a sanity bound to a default of 4 and desynced
    /// the boot scripts to 75% unknown opcodes.
    #[test]
    fn k80_bit_m_v2_group_count_is_eight() {
        let m_data = [
            0x08, 0x5b, 0x4e, 0xa7, 0x4f, 0xa5, 0x8d, 0x00, 0x00, 0xec, 0x8d, 0x00, 0x00, 0x3e,
            0x50, 0x00, 0x00,
        ];
        let rom = rom_with_bit_m(2, &m_data);
        assert_eq!(ram_restrict_group_count(&rom), 8);
    }

    /// Version 1 keeps the count at +2, not +0.
    #[test]
    fn v1_reads_the_count_at_offset_two() {
        let rom = rom_with_bit_m(1, &[0xAA, 0xBB, 0x06, 0x00, 0x00]);
        assert_eq!(ram_restrict_group_count(&rom), 6);
    }

    /// A version we do not know must not guess from an arbitrary byte.
    #[test]
    fn unknown_version_falls_back() {
        let rom = rom_with_bit_m(9, &[0x20, 0x20, 0x20, 0x20, 0x20]);
        assert_eq!(ram_restrict_group_count(&rom), 4);
    }

    /// Too short to contain the field: fall back rather than read past it.
    #[test]
    fn truncated_entry_falls_back() {
        assert_eq!(ram_restrict_group_count(&rom_with_bit_m(2, &[])), 4);
        assert_eq!(ram_restrict_group_count(&rom_with_bit_m(1, &[0, 0])), 4);
    }

    /// A zero count would make group-carrying opcodes zero-length and hang
    /// the walk in place.
    #[test]
    fn zero_count_falls_back() {
        let rom = rom_with_bit_m(2, &[0x00, 0x00, 0x00]);
        assert_eq!(ram_restrict_group_count(&rom), 4);
    }

    /// No BIT 'M' at all.
    #[test]
    fn missing_m_entry_falls_back() {
        assert_eq!(ram_restrict_group_count(&vec![0u8; 4096]), 4);
    }
}

#[cfg(test)]
mod offline_interpreter_tests {
    use super::*;

    fn fixture(name: &str) -> Option<Vec<u8>> {
        std::fs::read(format!(
            "{}/../../../testdata/vbios/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .ok()
    }

    /// The interpreter runs end-to-end with no GPU present.
    ///
    /// This is the capability the trait exists for. Before it, reproducing the
    /// K80 misparse meant wedging a die and rebooting.
    #[test]
    fn interpreter_runs_against_a_real_rom_without_hardware() {
        let Some(rom) = fixture("titanv_gv100.rom") else {
            return;
        };
        let bus = RecordingBus::default();
        let stats = interpret_boot_scripts(&bus, &rom).expect("interpreter must complete");

        eprintln!(
            "ops={} unknown={} writes_applied={} recorded={} delayed_us={}",
            stats.ops_executed,
            stats.unknown_opcodes.len(),
            stats.writes_applied,
            bus.writes().len(),
            bus.delayed_us.get()
        );

        // The desync guard must hold: a misparsed script contributes no writes.
        if let Some(pct) = (stats.unknown_opcodes.len() * 100).checked_div(stats.ops_executed)
            && pct > MAX_UNKNOWN_PERCENT
        {
            assert!(
                bus.writes().is_empty(),
                "a {pct}% unknown parse issued {} writes — the guard failed",
                bus.writes().len()
            );
        }
    }

    /// Kepler boot scripts must actually decode.
    ///
    /// The K80's six boot scripts previously walked at 75% unknown opcodes,
    /// because `ram_restrict_group_count` misread BIT 'M' and returned a
    /// fallback of 4 where the ROM says 8. Every `0x8F` then advanced the
    /// offset by the wrong number of payload words and the rest of the script
    /// decoded at the wrong boundary.
    ///
    /// Fixture is vendor firmware and is gitignored; the test skips without
    /// it. Dump with `read_vbios_prom` or from `BAR0 + 0x300000`.
    #[test]
    fn kepler_boot_scripts_decode() {
        let Some(rom) = fixture("k80_gk210.rom") else {
            eprintln!("skipping: testdata/vbios/k80_gk210.rom not present");
            return;
        };

        assert_eq!(
            ram_restrict_group_count(&rom),
            8,
            "K80 BIT 'M' is version 2 and its first data byte is 8"
        );

        let bus = RecordingBus::default();
        let stats = interpret_boot_scripts(&bus, &rom).expect("interpreter must complete");
        let unknown = stats.unknown_opcodes.len();
        let seen = stats.ops_executed + stats.ops_skipped;
        assert!(seen > 0, "no opcodes walked — script table not found");

        let pct = unknown * 100 / seen;
        let writes = bus.writes();
        eprintln!(
            "K80: ops={seen} unknown={unknown} ({pct}%) writes={}",
            writes.len()
        );
        for (offset, op) in &stats.unknown_opcodes {
            eprintln!("  unknown opcode {op:#04x} at ROM {offset:#06x}");
        }

        // `0x4D` (INIT_ZM_I2C_BYTE) is variable-length: 4 + count * 2. It was
        // fixed at 6, which on this image landed mid-payload at 0xb84e and
        // desynced the tail of the last script. Nothing may reintroduce a
        // constant there.
        assert!(
            !stats.unknown_opcodes.iter().any(|&(off, _)| off == 0xb84e),
            "0xb84e is unknown again — 0x4D has gone back to a fixed length"
        );

        // Well under the refusal threshold, not merely at it. A parse that
        // squeaks past 25% is still mostly noise; this one decodes cleanly.
        assert!(
            pct <= 2,
            "boot scripts decode at {pct}% unknown ({unknown}/{seen}); \
             this ROM is known to decode at 0% (2 residual opcodes), so a \
             regression here means an opcode length or table offset moved"
        );

        // A decode that produces nothing is not a decode. The scripts carry
        // roughly 300 register writes.
        assert!(
            writes.len() > 250,
            "only {} writes from {seen} opcodes — expected ~300; the walk is \
             terminating early even though what it did read parsed",
            writes.len()
        );

        // Sanity-check the targets rather than just the count: a correct
        // Kepler devinit touches the framebuffer and clock trees. Landing
        // entirely outside them would mean a plausible-looking decode of the
        // wrong bytes.
        let domains: std::collections::BTreeSet<usize> =
            writes.iter().map(|(r, _)| r & 0xFFF000).collect();
        for expected in [0x10f000, 0x137000] {
            assert!(
                domains.contains(&expected),
                "no writes to {expected:#08x}; decoded stream does not look \
                 like a framebuffer/clock init"
            );
        }
    }

    /// Whatever the interpreter does, it must not sleep for real in a test.
    /// One Volta script asks for ten seconds.
    #[test]
    fn delays_are_recorded_not_slept() {
        let bus = RecordingBus::default();
        let start = std::time::Instant::now();
        bus.delay_us(10_000_000);
        assert!(start.elapsed().as_millis() < 100);
        assert_eq!(bus.delayed_us.get(), 10_000_000);
    }
}
