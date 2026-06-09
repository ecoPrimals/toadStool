// SPDX-License-Identifier: AGPL-3.0-or-later
//! BAR cartography types — register classification and region grouping.

use std::collections::BTreeMap;

/// Classification of a single register's access behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterAccess {
    /// Register reads and writes back successfully.
    ReadWrite,
    /// Register reads a value but writes don't change it.
    ReadOnly,
    /// Register reads zero or a constant; writes may trigger effects.
    WriteOnly,
    /// Writing changes behavior (interrupt clears, triggers, etc.).
    Trigger,
    /// Register returns an error pattern (0xFFFFFFFF, 0xBADFxxxx, etc.).
    Dead,
}

/// Pattern observed in a register's read value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterPattern {
    /// Register always returns the same non-zero value.
    Constant(u32),
    /// Register value changes between reads (counters, timers, status).
    Dynamic,
    /// Register returns an error signature (PRI timeout, dead device).
    ErrorPattern(u32),
    /// Register returns zero.
    Zeros,
}

/// A contiguous region of registers with similar behavior.
#[derive(Debug, Clone)]
pub struct RegisterRegion {
    /// Start offset in BAR space.
    pub start: usize,
    /// End offset (exclusive) in BAR space.
    pub end: usize,
    /// Human-readable domain name (e.g., "PMC", "PFIFO", "PFB").
    pub name: Option<String>,
    /// Predominant access type in this region.
    pub access: RegisterAccess,
    /// Predominant value pattern.
    pub pattern: RegisterPattern,
    /// Number of responsive (non-dead) registers in this region.
    pub responsive_count: usize,
    /// Number of dead/error registers.
    pub dead_count: usize,
}

/// Complete BAR0 scan result.
#[derive(Debug, Clone)]
pub struct BarMap {
    /// Which BAR was scanned (typically 0).
    pub bar_index: u8,
    /// Total BAR size in bytes.
    pub size: usize,
    /// Discovered register regions.
    pub regions: Vec<RegisterRegion>,
    /// Total responsive bytes (non-error, non-dead).
    pub responsive_bytes: usize,
    /// Total error/dead bytes.
    pub error_bytes: usize,
    /// Per-offset classification for detailed queries.
    pub register_map: BTreeMap<usize, RegisterProbe>,
}

/// Result of probing a single register offset.
#[derive(Debug, Clone, Copy)]
pub struct RegisterProbe {
    /// BAR offset.
    pub offset: usize,
    /// Value read on first access.
    pub read1: u32,
    /// Value read on second access (for dynamic detection).
    pub read2: u32,
    /// Whether write-readback succeeded (if safe to test).
    pub writable: Option<bool>,
    /// Classified access type.
    pub access: RegisterAccess,
    /// Classified value pattern.
    pub pattern: RegisterPattern,
}

/// Known domain map entry for labeling discovered regions.
pub struct DomainHint {
    pub start: usize,
    pub end: usize,
    pub name: &'static str,
}

/// Difference between two BarMap scans (e.g., cold vs warm).
#[derive(Debug, Clone)]
pub struct BarMapDiff {
    /// Registers that were dead in `before` but alive in `after`.
    pub woke_up: Vec<(usize, u32)>,
    /// Registers that were alive in `before` but dead in `after`.
    pub went_dead: Vec<(usize, u32)>,
    /// Registers alive in both but with different values.
    pub value_changed: Vec<(usize, u32, u32)>,
    /// Registers alive in both with same values.
    pub unchanged: usize,
}
