// SPDX-License-Identifier: AGPL-3.0-or-later
//! BAR map display and JSON serialization.

use std::fmt::Write as FmtWrite;

use super::types::{BarMap, BarMapDiff, RegisterAccess};

impl BarMap {
    /// Print a human-readable summary.
    pub fn print_summary(&self) {
        let total = self.responsive_bytes + self.error_bytes;
        let pct = if total > 0 {
            (self.responsive_bytes as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let mut s = String::new();
        let _ = writeln!(
            &mut s,
            "╠══ BAR{} CARTOGRAPHY ═══════════════════════════════════════╣",
            self.bar_index
        );
        let _ = writeln!(
            &mut s,
            "║ Scanned: {} KB | Responsive: {} KB ({pct:.1}%) | Dead: {} KB",
            total / 1024,
            self.responsive_bytes / 1024,
            self.error_bytes / 1024,
        );
        let _ = writeln!(&mut s, "║ Regions: {}", self.regions.len());
        for region in &self.regions {
            let name = region.name.as_deref().unwrap_or("???");
            let _ = writeln!(
                &mut s,
                "║   {name:<16} {:#08x}–{:#08x} ({} regs) alive={} dead={} {:?}",
                region.start,
                region.end,
                (region.end - region.start) / 4,
                region.responsive_count,
                region.dead_count,
                region.access,
            );
        }
        tracing::info!(summary = %s, bar_index = self.bar_index, "BAR cartography");
    }

    /// Export as a serializable map for JSON persistence.
    pub fn to_json_value(&self) -> serde_json::Value {
        use serde_json::json;
        let regions: Vec<serde_json::Value> = self
            .regions
            .iter()
            .map(|r| {
                json!({
                    "name": r.name,
                    "start": format!("{:#x}", r.start),
                    "end": format!("{:#x}", r.end),
                    "access": format!("{:?}", r.access),
                    "responsive": r.responsive_count,
                    "dead": r.dead_count,
                })
            })
            .collect();
        json!({
            "bar_index": self.bar_index,
            "size_bytes": self.size,
            "responsive_bytes": self.responsive_bytes,
            "error_bytes": self.error_bytes,
            "region_count": self.regions.len(),
            "regions": regions,
        })
    }
}

impl BarMapDiff {
    /// Print a human-readable summary.
    pub fn print_summary(&self) {
        let mut s = String::new();
        let _ = writeln!(
            &mut s,
            "╠══ BAR MAP DIFF ════════════════════════════════════════════╣"
        );
        let _ = writeln!(
            &mut s,
            "║ Woke up:       {} registers (dead → alive)",
            self.woke_up.len()
        );
        let _ = writeln!(
            &mut s,
            "║ Went dead:     {} registers (alive → dead)",
            self.went_dead.len()
        );
        let _ = writeln!(
            &mut s,
            "║ Value changed: {} registers",
            self.value_changed.len()
        );
        let _ = writeln!(&mut s, "║ Unchanged:     {} registers", self.unchanged);
        if !self.woke_up.is_empty() {
            let _ = writeln!(&mut s, "║ ─── Woke up (first 20) ───");
            for &(off, val) in self.woke_up.iter().take(20) {
                let _ = writeln!(&mut s, "║   [{off:#08x}] → {val:#010x}");
            }
        }
        if !self.value_changed.is_empty() {
            let _ = writeln!(&mut s, "║ ─── Changed (first 20) ───");
            for &(off, before, after) in self.value_changed.iter().take(20) {
                let _ = writeln!(&mut s, "║   [{off:#08x}] {before:#010x} → {after:#010x}");
            }
        }
        tracing::info!(summary = %s, "BAR map diff");
    }

    /// Export as JSON.
    pub fn to_json_value(&self) -> serde_json::Value {
        use serde_json::json;
        json!({
            "woke_up": self.woke_up.len(),
            "went_dead": self.went_dead.len(),
            "value_changed": self.value_changed.len(),
            "unchanged": self.unchanged,
            "woke_up_registers": self.woke_up.iter().map(|(o, v)| json!({
                "offset": format!("{o:#x}"),
                "value": format!("{v:#010x}"),
            })).collect::<Vec<_>>(),
            "value_changed_registers": self.value_changed.iter().map(|(o, b, a)| json!({
                "offset": format!("{o:#x}"),
                "before": format!("{b:#010x}"),
                "after": format!("{a:#010x}"),
            })).collect::<Vec<_>>(),
        })
    }
}

/// Diff two BarMap scans to discover what changes between states.
pub fn diff_bar_maps(before: &BarMap, after: &BarMap) -> BarMapDiff {
    let mut woke_up = Vec::new();
    let mut went_dead = Vec::new();
    let mut value_changed = Vec::new();
    let mut unchanged = 0usize;

    let all_offsets: std::collections::BTreeSet<usize> = before
        .register_map
        .keys()
        .chain(after.register_map.keys())
        .copied()
        .collect();

    for offset in all_offsets {
        let b = before.register_map.get(&offset);
        let a = after.register_map.get(&offset);

        match (b, a) {
            (Some(bp), Some(ap)) => {
                let b_dead = bp.access == RegisterAccess::Dead;
                let a_dead = ap.access == RegisterAccess::Dead;
                match (b_dead, a_dead) {
                    (true, false) => woke_up.push((offset, ap.read1)),
                    (false, true) => went_dead.push((offset, bp.read1)),
                    (false, false) if bp.read1 != ap.read1 => {
                        value_changed.push((offset, bp.read1, ap.read1));
                    }
                    _ => unchanged += 1,
                }
            }
            (None, Some(ap)) if ap.access != RegisterAccess::Dead => {
                woke_up.push((offset, ap.read1));
            }
            (Some(bp), None) if bp.access != RegisterAccess::Dead => {
                went_dead.push((offset, bp.read1));
            }
            _ => unchanged += 1,
        }
    }

    BarMapDiff {
        woke_up,
        went_dead,
        value_changed,
        unchanged,
    }
}
