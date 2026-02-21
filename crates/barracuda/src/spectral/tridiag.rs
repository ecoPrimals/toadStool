// SPDX-License-Identifier: AGPL-3.0-only

//! Sturm bisection eigensolve for symmetric tridiagonal matrices.
//!
//! Counts eigenvalues below a given value using LDLT factorization (Sturm
//! sequence) and finds all eigenvalues via bisection.
//!
//! Provenance: hotSpring v0.6.0 (Kachkovskiy spectral theory)

/// Count eigenvalues of a symmetric tridiagonal matrix strictly less than λ.
///
/// Uses the LDLT factorization (Sturm sequence): the number of negative
/// pivots equals the number of eigenvalues below λ.
///
/// - `diagonal`: main diagonal d[0..n]
/// - `off_diag`: sub/super-diagonal e[0..n-1]
pub fn sturm_count(diagonal: &[f64], off_diag: &[f64], lambda: f64) -> usize {
    let n = diagonal.len();
    if n == 0 {
        return 0;
    }

    let mut count = 0;
    let mut q = diagonal[0] - lambda;
    if q < 0.0 {
        count += 1;
    }

    for i in 1..n {
        let q_safe = if q.abs() < 1e-300 {
            if q >= 0.0 {
                1e-300
            } else {
                -1e-300
            }
        } else {
            q
        };
        q = (diagonal[i] - lambda) - off_diag[i - 1] * off_diag[i - 1] / q_safe;
        if q < 0.0 {
            count += 1;
        }
    }
    count
}

/// Find all eigenvalues of a symmetric tridiagonal matrix via Sturm bisection.
///
/// Returns eigenvalues sorted in ascending order. Complexity: O(N² log(1/ε)).
/// Exact to machine precision for well-separated eigenvalues.
pub fn find_all_eigenvalues(diagonal: &[f64], off_diag: &[f64]) -> Vec<f64> {
    let n = diagonal.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![diagonal[0]];
    }

    // Gershgorin bounds
    let mut lo = f64::MAX;
    let mut hi = f64::MIN;
    for i in 0..n {
        let e_left = if i > 0 { off_diag[i - 1].abs() } else { 0.0 };
        let e_right = if i < n - 1 { off_diag[i].abs() } else { 0.0 };
        lo = lo.min(diagonal[i] - e_left - e_right);
        hi = hi.max(diagonal[i] + e_left + e_right);
    }
    lo -= 1.0;
    hi += 1.0;

    let mut eigenvalues = Vec::with_capacity(n);
    for k in 0..n {
        let mut a = lo;
        let mut b = hi;
        for _ in 0..200 {
            let mid = 0.5 * (a + b);
            if (b - a) < 2.0 * f64::EPSILON * mid.abs().max(1.0) {
                break;
            }
            if sturm_count(diagonal, off_diag, mid) <= k {
                a = mid;
            } else {
                b = mid;
            }
        }
        eigenvalues.push(0.5 * (a + b));
    }
    eigenvalues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sturm_count_identity_2x2() {
        let d = [1.0, 3.0];
        let e = [-1.0];
        assert_eq!(sturm_count(&d, &e, 0.0), 0);
        assert_eq!(sturm_count(&d, &e, 1.0), 1);
        assert_eq!(sturm_count(&d, &e, 4.0), 2);
    }

    #[test]
    fn eigenvalues_clean_chain() {
        let n = 50;
        let d = vec![0.0; n];
        let e = vec![-1.0; n - 1];
        let evals = find_all_eigenvalues(&d, &e);

        assert_eq!(evals.len(), n);

        for k in 1..=n {
            let exact = 2.0 * (k as f64 * std::f64::consts::PI / (n as f64 + 1.0)).cos();
            let closest = evals
                .iter()
                .map(|&ev| (ev - exact).abs())
                .fold(f64::MAX, f64::min);
            assert!(
                closest < 1e-10,
                "k={k}, exact={exact:.6}, closest error={closest:.2e}"
            );
        }
    }
}
