// SPDX-License-Identifier: AGPL-3.0-or-later

//! Typed BAR0 register reads.
//!
//! # Why this exists
//!
//! NVIDIA GPUs signal failure in-band. A read that cannot be served comes
//! back as a plausible-looking `u32` rather than an error, so the raw value
//! is ambiguous: `0xFFFF_FFFF` might be a register with every bit set, or a
//! device that is not answering at all.
//!
//! [`super::pri::is_pri_fault`] has encoded these patterns for a long time,
//! but it is a free function that callers must remember to invoke. Three
//! times that memory has failed, each time producing a confident wrong
//! answer rather than an error:
//!
//! | Date | Sentinel | Read as | Consequence |
//! |------|----------|---------|-------------|
//! | Aug 13 | `0xBADFxxxx` | warm FECS state | skipped `pgraph_reset` and `falcon_boot`, reported `compute_ready` |
//! | Aug 16 | `0xFFFF_FFFF` | PMC with 32 engines | popcount 32 cleared the `< 8` cold gate; whole classification ran on garbage |
//! | Aug 16 | `0xBADF_5040` | live FECS | promoted to Tier 2 "full shader dispatch" with FECS dead |
//!
//! Each was fixed at the call site, and the next call site repeated it. The
//! fault is not in the checks but in the shape of the data: a `u32` invites
//! arithmetic, and `count_ones()` on a sentinel is silently meaningless.
//!
//! [`RegisterRead`] removes the invitation. There is no way to get a number
//! out of it without acknowledging that the read may not have produced one.
//!
//! # Bus failure is not a PRI fault
//!
//! `is_pri_fault` groups `0xFFFF_FFFF` with `0xBADFxxxx`, which conflates two
//! very different situations:
//!
//! - `0xBADF_5040` on FECS: the device is alive and answering; *that engine*
//!   is clock- or security-gated. Other registers remain meaningful.
//! - `0xFFFF_FFFF` on PMC_ENABLE: nothing is answering. The device is in
//!   D3hot, or memory decode is off. **No** register read is meaningful, and
//!   continuing to probe only manufactures more garbage.
//!
//! Conflating them is how a sleeping Titan V was classified as cold rather
//! than as unmeasured. This type keeps them apart.

use super::pri;

/// The outcome of reading a BAR0 register.
///
/// Construct with [`RegisterRead::classify`]. Extract data with
/// [`RegisterRead::valid`], which yields `None` for anything that is not a
/// real value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterRead {
    /// A real value from responsive hardware.
    Valid(u32),

    /// All-ones: the device did not answer.
    ///
    /// Device-wide, not register-specific. Probing further is pointless
    /// until the device is woken or decode is restored.
    BusFailure,

    /// `0xBADFxxxx` / `0xBAD0xxxx`: this register is gated.
    ///
    /// The device is alive. The low half encodes the gating reason. Other
    /// registers may still be valid.
    PriFault(u32),

    /// The read itself failed (mapping error, EIO), producing no value.
    Unread,
}

impl RegisterRead {
    /// Classify a raw value read from BAR0.
    #[must_use]
    pub const fn classify(raw: u32) -> Self {
        if raw == 0xFFFF_FFFF {
            return Self::BusFailure;
        }
        if (raw & 0xFFFF_0000) == 0xBADF_0000
            || (raw & 0xFFFF_0000) == 0xBAD0_0000
            || raw == 0xDEAD_DEAD
        {
            return Self::PriFault(raw);
        }
        Self::Valid(raw)
    }

    /// Classify a fallible read, mapping the error case to [`Self::Unread`].
    #[must_use]
    pub fn from_result<E>(res: Result<u32, E>) -> Self {
        res.map_or(Self::Unread, Self::classify)
    }

    /// The value, if the read produced one.
    ///
    /// This is the only way to obtain a number, so a sentinel cannot reach
    /// arithmetic without the caller deciding what to do about it.
    #[must_use]
    pub const fn valid(self) -> Option<u32> {
        match self {
            Self::Valid(v) => Some(v),
            _ => None,
        }
    }

    /// The underlying value for logging, including sentinels.
    ///
    /// For diagnostics only. Never use this for a liveness decision.
    #[must_use]
    pub const fn raw(self) -> Option<u32> {
        match self {
            Self::Valid(v) | Self::PriFault(v) => Some(v),
            Self::BusFailure => Some(0xFFFF_FFFF),
            Self::Unread => None,
        }
    }

    /// Whether the device itself failed to answer.
    #[must_use]
    pub const fn is_bus_failure(self) -> bool {
        matches!(self, Self::BusFailure)
    }

    /// Whether this specific register is gated.
    #[must_use]
    pub const fn is_pri_fault(self) -> bool {
        matches!(self, Self::PriFault(_))
    }

    /// Whether the read produced usable data.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        matches!(self, Self::Valid(_))
    }

    /// Whether this register indicates a live, initialized unit.
    ///
    /// Zero is treated as not-alive: a responsive but uninitialized register
    /// reads zero, which is a real value yet not evidence of liveness.
    #[must_use]
    pub const fn is_alive(self) -> bool {
        matches!(self, Self::Valid(v) if v != 0)
    }

    /// Population count of a valid read, `None` otherwise.
    ///
    /// The counterpart to the bug this type exists to prevent. Engine-count
    /// checks of the form `pmc.count_ones() >= N` treat `0xFFFF_FFFF` as 32
    /// enabled engines, so a sleeping device passes every warmth threshold in
    /// the codebase. Callers wanting a scalar should use
    /// `count_ones().unwrap_or(0)`: a device that did not answer has no
    /// demonstrable engines, so zero is both safe and true.
    #[must_use]
    pub const fn count_ones(self) -> Option<u32> {
        match self {
            Self::Valid(v) => Some(v.count_ones()),
            _ => None,
        }
    }

    /// A short human-readable state, for step details and logs.
    #[must_use]
    pub fn describe(self) -> String {
        match self {
            Self::Valid(v) => format!("{v:#010x}"),
            Self::BusFailure => "bus-failure (all-ones, device not answering)".into(),
            Self::PriFault(v) => format!("pri-fault ({v:#010x})"),
            Self::Unread => "unread (read failed)".into(),
        }
    }
}

/// Bridge for call sites still holding a raw `u32`.
///
/// Agrees with [`pri::is_pri_fault`] on what counts as a fault, while
/// preserving the bus-failure distinction that function collapses.
#[must_use]
pub fn is_fault_raw(raw: u32) -> bool {
    debug_assert_eq!(
        !RegisterRead::classify(raw).is_valid(),
        pri::is_pri_fault(raw),
        "RegisterRead and is_pri_fault must agree on {raw:#010x}"
    );
    !RegisterRead::classify(raw).is_valid()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ones_is_bus_failure_not_pri_fault() {
        let r = RegisterRead::classify(0xFFFF_FFFF);
        assert_eq!(r, RegisterRead::BusFailure);
        assert!(r.is_bus_failure());
        assert!(!r.is_pri_fault(), "bus failure must stay distinguishable");
    }

    #[test]
    fn badf_is_pri_fault_not_bus_failure() {
        let r = RegisterRead::classify(0xBADF_5040);
        assert!(r.is_pri_fault());
        assert!(!r.is_bus_failure(), "a gated register does not mean a dead device");
    }

    /// The Aug 16 bug in one assertion: a sentinel must never reach
    /// `count_ones()`.
    #[test]
    fn sentinels_cannot_be_counted() {
        for raw in [0xFFFF_FFFFu32, 0xBADF_5040, 0xBAD0_0200, 0xDEAD_DEAD] {
            assert!(
                RegisterRead::classify(raw).valid().is_none(),
                "{raw:#010x} must not yield a countable value"
            );
        }
        // The real PMC_ENABLE observed after waking the Titan V.
        assert_eq!(
            RegisterRead::classify(0x5FEC_DFF1).valid().map(u32::count_ones),
            Some(23)
        );
    }

    #[test]
    fn zero_is_valid_but_not_alive() {
        let r = RegisterRead::classify(0);
        assert!(r.is_valid(), "zero is a real read");
        assert!(!r.is_alive(), "but an uninitialized unit is not alive");
        assert_eq!(r.valid(), Some(0));
    }

    #[test]
    fn unread_yields_nothing() {
        let r = RegisterRead::from_result(Err::<u32, &str>("EIO"));
        assert_eq!(r, RegisterRead::Unread);
        assert_eq!(r.valid(), None);
        assert_eq!(r.raw(), None);
        assert!(!r.is_alive());
    }

    #[test]
    fn from_result_classifies_ok_values() {
        assert_eq!(
            RegisterRead::from_result(Ok::<u32, &str>(0x1234)),
            RegisterRead::Valid(0x1234)
        );
        assert_eq!(
            RegisterRead::from_result(Ok::<u32, &str>(0xFFFF_FFFF)),
            RegisterRead::BusFailure
        );
    }

    #[test]
    fn agrees_with_legacy_pri_fault_predicate() {
        for raw in [
            0x0000_0000,
            0x0000_0001,
            0x5FEC_DFF1,
            0xFFFF_FFFF,
            0xBADF_5040,
            0xBAD0_0200,
            0xDEAD_DEAD,
            0x1001_1111,
        ] {
            assert_eq!(
                is_fault_raw(raw),
                pri::is_pri_fault(raw),
                "divergence on {raw:#010x}"
            );
        }
    }

    #[test]
    fn describe_is_unambiguous() {
        assert_eq!(RegisterRead::classify(0x1234).describe(), "0x00001234");
        assert!(RegisterRead::classify(0xFFFF_FFFF).describe().contains("bus-failure"));
        assert!(RegisterRead::classify(0xBADF_5040).describe().contains("pri-fault"));
    }
}

#[cfg(test)]
mod engine_count_tests {
    use super::*;

    /// Every warmth threshold in the codebase is of the form
    /// `count_ones() >= N`. All-ones must not clear any of them.
    #[test]
    fn all_ones_clears_no_warmth_threshold() {
        let count = RegisterRead::classify(0xFFFF_FFFF)
            .count_ones()
            .unwrap_or(0);
        assert_eq!(count, 0, "a device that did not answer has no live engines");
        for threshold in [8u32, 10, 16] {
            assert!(count < threshold, "must not pass the >= {threshold} gate");
        }
        // The raw value would have passed all of them.
        assert_eq!(0xFFFF_FFFFu32.count_ones(), 32);
    }

    /// Real readings still count normally.
    #[test]
    fn valid_readings_count_normally() {
        // Titan V, warm, after nouveau handoff.
        assert_eq!(
            RegisterRead::classify(0x5FEC_DFF1).count_ones(),
            Some(23)
        );
        // Tesla K80, genuinely cold.
        assert_eq!(
            RegisterRead::classify(0xC000_2020).count_ones(),
            Some(4)
        );
        assert_eq!(RegisterRead::classify(0).count_ones(), Some(0));
    }

    #[test]
    fn faults_and_unread_have_no_count() {
        assert_eq!(RegisterRead::classify(0xBADF_5040).count_ones(), None);
        assert_eq!(RegisterRead::from_result(Err::<u32, ()>(())).count_ones(), None);
    }
}
