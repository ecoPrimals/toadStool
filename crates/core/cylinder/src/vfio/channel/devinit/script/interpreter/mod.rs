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

/// Number of RAM-restrict groups from VBIOS M (rammap) table header.
///
/// BIT 'M' data\[0:2\] points to the rammap table. The table header's
/// `snr` field (offset +4 for version < 0x10, offset +4 for version >= 0x10)
/// gives the number of sub-entries per timing entry, which equals the
/// RAM-restrict group count used by opcodes 0x87, 0x88, 0x8F.
pub(crate) fn ram_restrict_group_count(rom: &[u8]) -> usize {
    if let Ok(bit) = BitTable::parse(rom)
        && let Some(m) = bit.find(b'M')
    {
        let m_off = m.data_offset as usize;
        if m_off + 2 <= rom.len() {
            let tbl_ptr = u16::from_le_bytes([rom[m_off], rom[m_off + 1]]) as usize;
            if tbl_ptr != 0 && tbl_ptr + 5 <= rom.len() {
                let snr = rom[tbl_ptr + 4] as usize;
                if snr > 0 && snr <= 16 {
                    return snr;
                }
            }
        }
    }
    4
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
    let bit = BitTable::parse(rom)?;
    let bit_i = bit.find(b'I').ok_or(DevinitError::BitINotFound)?;

    let i_off = bit_i.data_offset as usize;
    if i_off + 2 > rom.len() {
        return Err(DevinitError::BitIDataTooShort);
    }

    // Detect BIOS generation from BIT I data size:
    // Kepler (GK110/GK210) has 18-byte BIT I (no PMU script pointers).
    // Maxwell+ has >=28-byte BIT I with extended PMU firmware fields.
    let bios_gen = if bit_i.data_size < 0x1c {
        BiosGeneration::Kepler
    } else {
        BiosGeneration::MaxwellPlus
    };
    tracing::debug!(
        data_size = bit_i.data_size,
        gen = ?bios_gen,
        "VBIOS generation detected"
    );

    // BIT I data[0:2] is the "init tables base" — a table of u16 script
    // offsets.  Each entry points directly to an init script in the ROM.
    // nouveau: nvbios_init_table(bios, n) reads rd16(tbl + n * 2).
    let script_table = u16::from_le_bytes([rom[i_off], rom[i_off + 1]]) as usize;

    if script_table == 0 || script_table + 2 > rom.len() {
        return Err(DevinitError::InterpreterInitTablesInvalid);
    }

    tracing::debug!(
        script_table = format!("{script_table:#06x}"),
        gen = ?bios_gen,
        "VBIOS interpreter entry points"
    );

    let mut combined_stats = InterpreterStats::default();
    let mut script_idx = 0;

    loop {
        let entry_off = script_table + script_idx * 2;
        if entry_off + 2 > rom.len() {
            break;
        }
        let script_off = u16::from_le_bytes([rom[entry_off], rom[entry_off + 1]]) as usize;
        if script_off == 0 || script_off >= rom.len() {
            break;
        }

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
            script_idx += 1;
            if script_idx > 50 {
                break;
            }
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

        script_idx += 1;
        if script_idx > 50 {
            break;
        }
    }

    tracing::info!(
        scripts = script_idx,
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

#[cfg(test)]
mod ram_restrict_tests {
    use super::ram_restrict_group_count;
    use crate::vfio::channel::devinit::vbios::BitTable;

    fn rom_with_bit_m(m_data_off: usize, count: u8) -> Vec<u8> {
        let bit_off = 0x100;
        let tbl_off = m_data_off + 16;
        let mut rom = vec![0u8; tbl_off + 16];
        rom[bit_off..bit_off + 5].copy_from_slice(&[0xFF, 0xB8, b'B', b'I', b'T']);
        rom[bit_off + 9] = 6;
        rom[bit_off + 10] = 1;
        let e0 = bit_off + 12;
        rom[e0] = b'M';
        rom[e0 + 1] = 1;
        rom[e0 + 2..e0 + 4].copy_from_slice(&0x10u16.to_le_bytes());
        rom[e0 + 4..e0 + 6].copy_from_slice(&(m_data_off as u16).to_le_bytes());
        // M data[0:2] = pointer to rammap table
        rom[m_data_off..m_data_off + 2].copy_from_slice(&(tbl_off as u16).to_le_bytes());
        // Rammap table header: snr (ram restrict groups) at offset +4
        rom[tbl_off + 4] = count;
        rom
    }

    #[test]
    fn ram_restrict_default_without_m() {
        let rom = vec![0u8; 4096];
        assert_eq!(ram_restrict_group_count(&rom), 4);
    }

    #[test]
    fn ram_restrict_from_bit_m_table() {
        let rom = rom_with_bit_m(0x400, 8);
        assert!(BitTable::parse(&rom).is_ok());
        assert_eq!(ram_restrict_group_count(&rom), 8);
    }
}

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
        if stats.ops_executed > 0 {
            let pct = stats.unknown_opcodes.len() * 100 / stats.ops_executed;
            if pct > MAX_UNKNOWN_PERCENT {
                assert!(
                    bus.writes().is_empty(),
                    "a {pct}% unknown parse issued {} writes — the guard failed",
                    bus.writes().len()
                );
            }
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
