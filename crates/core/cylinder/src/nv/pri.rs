// SPDX-License-Identifier: AGPL-3.0-or-later

//! PRI (Private Register Interface) fault detection utilities.
//!
//! NVIDIA GPUs return distinctive error patterns when a BAR0 register read
//! hits a clock-gated, security-gated, or non-existent register. These
//! patterns are architecture-stable (Fermi through Blackwell) and form the
//! foundation of all hardware probing in the sovereign compute pipeline.
//!
//! This module provides the single source of truth for error pattern
//! classification, used by `gr_init`, `driver_probe`, `pmu_init`,
//! `warm_capture`, `bar_cartography`, and `sovereign_init`.

/// Check whether a BAR0 register value is a PRI fault pattern.
///
/// PRI faults occur when reading a register that is:
/// - Clock-gated (`0xBADF_xxxx` — the `xxxx` encodes the gating reason)
/// - Security-gated (`0xBAD0_xxxx`)
/// - Non-existent / bus error (`0xFFFF_FFFF`)
/// - Read failure sentinel (`0xDEAD_DEAD` — our `unwrap_or` value)
pub fn is_pri_fault(val: u32) -> bool {
    val == 0xFFFF_FFFF
        || (val & 0xFFFF_0000) == 0xBADF_0000
        || (val & 0xFFFF_0000) == 0xBAD0_0000
        || val == 0xDEAD_DEAD
}

/// Check whether a register value is an error pattern OR zero.
///
/// Useful for "alive" register counting where zero-valued registers
/// are considered uninitialized rather than responsive.
pub fn is_error_or_zero(val: u32) -> bool {
    val == 0 || is_pri_fault(val)
}

/// Find the BAR0 domain name for a given offset.
///
/// `domains` is a slice of `(name, start, end)` tuples where each
/// entry covers `[start, end)`. Returns `"UNKNOWN"` if no domain matches.
pub fn domain_for_offset(offset: usize, domains: &[(&str, usize, usize)]) -> String {
    for &(name, start, end) in domains {
        if offset >= start && offset < end {
            return name.to_string();
        }
    }
    "UNKNOWN".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pri_fault_patterns() {
        assert!(is_pri_fault(0xFFFF_FFFF));
        assert!(is_pri_fault(0xBADF_5040));
        assert!(is_pri_fault(0xBADF_1002));
        assert!(is_pri_fault(0xBAD0_0200));
        assert!(is_pri_fault(0xDEAD_DEAD));
    }

    #[test]
    fn non_fault_values() {
        assert!(!is_pri_fault(0));
        assert!(!is_pri_fault(1));
        assert!(!is_pri_fault(0x5fec_dff1));
        assert!(!is_pri_fault(0x1234_5678));
    }

    #[test]
    fn error_or_zero_includes_zero() {
        assert!(is_error_or_zero(0));
        assert!(is_error_or_zero(0xFFFF_FFFF));
        assert!(is_error_or_zero(0xBADF_1002));
        assert!(!is_error_or_zero(1));
        assert!(!is_error_or_zero(0x5fec_dff1));
    }

    #[test]
    fn domain_lookup() {
        let domains = &[
            ("PMC", 0x0000_0000, 0x0000_1000),
            ("PFIFO", 0x0000_2000, 0x0000_4000),
            ("PGRAPH", 0x0040_0000, 0x0042_0000),
        ];
        assert_eq!(domain_for_offset(0x200, domains), "PMC");
        assert_eq!(domain_for_offset(0x2200, domains), "PFIFO");
        assert_eq!(domain_for_offset(0x400700, domains), "PGRAPH");
        assert_eq!(domain_for_offset(0xFFFF_FF00, domains), "UNKNOWN");
    }

    #[test]
    fn domain_boundary() {
        let domains = &[("A", 0x1000, 0x2000)];
        assert_eq!(domain_for_offset(0x1000, domains), "A");
        assert_eq!(domain_for_offset(0x1FFF, domains), "A");
        assert_eq!(domain_for_offset(0x2000, domains), "UNKNOWN");
        assert_eq!(domain_for_offset(0x0FFF, domains), "UNKNOWN");
    }
}
