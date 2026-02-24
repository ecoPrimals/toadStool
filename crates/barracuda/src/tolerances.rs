// SPDX-License-Identifier: AGPL-3.0-or-later

//! Centralized validation tolerances with mathematical justification.
//!
//! Every tolerance threshold used in validation is defined here.
//! No ad-hoc magic numbers. Imitates the neuralSpring hotSpring pattern.

/// Tolerance descriptor with absolute and relative bounds plus justification.
#[derive(Debug, Clone, Copy)]
pub struct Tolerance {
    pub name: &'static str,
    pub abs_tol: f64,
    pub rel_tol: f64,
    pub justification: &'static str,
}

/// Check whether `computed` matches `expected` within the tolerance.
///
/// Uses combined absolute-or-relative: passes if
/// `|computed - expected| <= abs_tol` or
/// `|computed - expected| <= rel_tol * max(|expected|, 1.0)`.
#[must_use]
pub fn check(computed: f64, expected: f64, tol: &Tolerance) -> bool {
    if !computed.is_finite() || !expected.is_finite() {
        return computed == expected;
    }
    let abs = (computed - expected).abs();
    if abs <= tol.abs_tol {
        return true;
    }
    let scale = expected.abs().max(1.0);
    abs <= tol.rel_tol * scale
}

// ═══════════════════════════════════════════════════════════════════
// linalg tolerances
// ═══════════════════════════════════════════════════════════════════

/// Matmul: dot-product accumulation O(√n) rounding for inner dim n.
pub const LINALG_MATMUL: Tolerance = Tolerance {
    name: "linalg_matmul",
    abs_tol: 1e-10,
    rel_tol: 1e-10,
    justification: "f64 dot-product accumulation; √n rounding for inner dim n",
};

/// Transpose: pure data movement, no arithmetic.
pub const LINALG_TRANSPOSE: Tolerance = Tolerance {
    name: "linalg_transpose",
    abs_tol: 1e-14,
    rel_tol: 1e-14,
    justification: "exact data movement; only f64 representation",
};

/// Frobenius norm: single-pass reduction.
pub const LINALG_FROBENIUS: Tolerance = Tolerance {
    name: "linalg_frobenius",
    abs_tol: 1e-10,
    rel_tol: 1e-10,
    justification: "f64 sum-of-squares reduction; accumulation order",
};

// ═══════════════════════════════════════════════════════════════════
// reduction tolerances
// ═══════════════════════════════════════════════════════════════════

/// Sum: Kahan or simple accumulation.
pub const REDUCTION_SUM: Tolerance = Tolerance {
    name: "reduction_sum",
    abs_tol: 1e-12,
    rel_tol: 1e-12,
    justification: "f64 accumulation; O(n) rounding for n elements",
};

/// Mean: sum / n.
pub const REDUCTION_MEAN: Tolerance = Tolerance {
    name: "reduction_mean",
    abs_tol: 1e-12,
    rel_tol: 1e-12,
    justification: "f64 sum then division; machine precision",
};

/// Variance: two-pass mean then residual sum.
pub const REDUCTION_VARIANCE: Tolerance = Tolerance {
    name: "reduction_variance",
    abs_tol: 1e-10,
    rel_tol: 1e-10,
    justification: "two-pass mean; catastrophic cancellation in subtraction",
};

/// Logsumexp: max-subtract + exp + sum + log.
pub const REDUCTION_LOGSUMEXP: Tolerance = Tolerance {
    name: "reduction_logsumexp",
    abs_tol: 1e-10,
    rel_tol: 1e-10,
    justification: "numerically stable; exp/log round-trip",
};

// ═══════════════════════════════════════════════════════════════════
// bio tolerances
// ═══════════════════════════════════════════════════════════════════

/// HMM forward: log-likelihood from T matrix-vector products.
pub const BIO_HMM: Tolerance = Tolerance {
    name: "bio_hmm",
    abs_tol: 1e-8,
    rel_tol: 1e-8,
    justification: "forward-backward accumulates rounding from T steps",
};

/// Allele frequency: per-locus variance across populations.
pub const BIO_ALLELE_FREQ: Tolerance = Tolerance {
    name: "bio_allele_freq",
    abs_tol: 1e-6,
    rel_tol: 1e-6,
    justification: "mean/variance over populations; f64 two-pass",
};

/// Nucleotide diversity: pairwise differences.
pub const BIO_NUCLEOTIDE_DIVERSITY: Tolerance = Tolerance {
    name: "bio_nucleotide_diversity",
    abs_tol: 1e-8,
    rel_tol: 1e-8,
    justification: "pairwise counting; exact arithmetic on integers",
};

// ═══════════════════════════════════════════════════════════════════
// special tolerances
// ═══════════════════════════════════════════════════════════════════

/// erf: polynomial/Chebyshev approximation.
pub const SPECIAL_ERF: Tolerance = Tolerance {
    name: "special_erf",
    abs_tol: 1e-10,
    rel_tol: 1e-10,
    justification: "A&S 7.1.26; ~6 digits accuracy in f64",
};

/// Gamma: Lanczos approximation.
pub const SPECIAL_GAMMA: Tolerance = Tolerance {
    name: "special_gamma",
    abs_tol: 1e-10,
    rel_tol: 1e-10,
    justification: "Lanczos; ~12 digits for x in [0.5, 2]",
};

/// Bessel: polynomial approximations.
pub const SPECIAL_BESSEL: Tolerance = Tolerance {
    name: "special_bessel",
    abs_tol: 1e-6,
    rel_tol: 1e-6,
    justification: "A&S 9.4.1-9.4.6; ~6 digits in f64",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_abs_tol() {
        assert!(check(1.0, 1.0 + 1e-15, &LINALG_TRANSPOSE));
        assert!(!check(1.0, 1.0 + 1e-10, &LINALG_TRANSPOSE));
    }

    #[test]
    fn check_rel_tol() {
        assert!(check(100.0, 100.0 + 1e-8, &LINALG_MATMUL));
        assert!(!check(100.0, 100.0 + 1e-5, &LINALG_MATMUL));
    }

    #[test]
    fn check_zero_expected() {
        assert!(check(1e-15, 0.0, &LINALG_TRANSPOSE));
    }

    #[test]
    fn check_nan_rejects() {
        assert!(!check(f64::NAN, 1.0, &LINALG_MATMUL));
        assert!(!check(1.0, f64::NAN, &LINALG_MATMUL));
    }

    #[test]
    fn check_infinity() {
        assert!(check(f64::INFINITY, f64::INFINITY, &LINALG_MATMUL));
    }

    #[test]
    fn tolerances_have_finite_values() {
        let tols = [
            LINALG_MATMUL,
            LINALG_TRANSPOSE,
            LINALG_FROBENIUS,
            REDUCTION_SUM,
            REDUCTION_MEAN,
            REDUCTION_VARIANCE,
            REDUCTION_LOGSUMEXP,
            BIO_HMM,
            BIO_ALLELE_FREQ,
            BIO_NUCLEOTIDE_DIVERSITY,
            SPECIAL_ERF,
            SPECIAL_GAMMA,
            SPECIAL_BESSEL,
        ];
        for t in &tols {
            assert!(t.abs_tol.is_finite(), "{} abs_tol must be finite", t.name);
            assert!(t.rel_tol.is_finite(), "{} rel_tol must be finite", t.name);
            assert!(!t.name.is_empty(), "tolerance name must not be empty");
            assert!(
                !t.justification.is_empty(),
                "justification must not be empty"
            );
        }
    }
}
