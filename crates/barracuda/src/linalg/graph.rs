//! Graph and spectral utilities for linear algebra
//!
//! - [`graph_laplacian`]: Compute graph Laplacian L = D - A from adjacency matrix
//! - [`effective_rank`]: Effective rank via Shannon entropy of eigenvalue spectrum

/// Compute the graph Laplacian from a flat row-major adjacency matrix.
///
/// L = D - A where D is the degree matrix (diagonal with row sums).
#[must_use]
pub fn graph_laplacian(adjacency: &[f64], n: usize) -> Vec<f64> {
    let mut laplacian = vec![0.0; n * n];
    for i in 0..n {
        let degree: f64 = (0..n).map(|j| adjacency[i * n + j]).sum();
        laplacian[i * n + i] = degree;
        for j in 0..n {
            laplacian[i * n + j] -= adjacency[i * n + j];
        }
    }
    laplacian
}

/// Effective rank via Shannon entropy of normalized eigenvalue spectrum.
///
/// rank_eff = exp(H) where H = -sum(p_i * log(p_i)).
#[must_use]
pub fn effective_rank(eigenvalues: &[f64]) -> f64 {
    let abs_vals: Vec<f64> = eigenvalues.iter().map(|&ev| ev.abs()).collect();
    let total: f64 = abs_vals.iter().sum();
    if total < 1e-300 {
        return 0.0;
    }
    let mut entropy = 0.0;
    for &v in &abs_vals {
        let p = v / total;
        if p > 1e-300 {
            entropy -= p * p.ln();
        }
    }
    entropy.exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_laplacian_row_sums_zero() {
        // 3-node path: 0--1--2
        let adj = vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let l = graph_laplacian(&adj, 3);
        for i in 0..3 {
            let row_sum: f64 = (0..3).map(|j| l[i * 3 + j]).sum();
            assert!(
                row_sum.abs() < 1e-14,
                "row {} sum = {}, expected 0",
                i,
                row_sum
            );
        }
    }

    #[test]
    fn test_graph_laplacian_identity() {
        // 2x2 complete graph: both nodes connected
        // A = [[0,1],[1,0]] -> D = [[1,0],[0,1]], L = D - A = [[1,-1],[-1,1]]
        let adj = vec![0.0, 1.0, 1.0, 0.0];
        let l = graph_laplacian(&adj, 2);
        assert!((l[0] - 1.0).abs() < 1e-14);
        assert!((l[1] - (-1.0)).abs() < 1e-14);
        assert!((l[2] - (-1.0)).abs() < 1e-14);
        assert!((l[3] - 1.0).abs() < 1e-14);
    }

    #[test]
    fn test_graph_laplacian_empty() {
        // Zero adjacency (disconnected graph)
        let adj = vec![0.0, 0.0, 0.0, 0.0];
        let l = graph_laplacian(&adj, 2);
        assert_eq!(l, vec![0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_effective_rank_full() {
        // Equal eigenvalues -> full rank (entropy max, rank_eff = n)
        let ev = vec![1.0, 1.0, 1.0, 1.0];
        let r = effective_rank(&ev);
        assert!((r - 4.0).abs() < 1e-10, "expected 4, got {}", r);
    }

    #[test]
    fn test_effective_rank_single() {
        // One nonzero -> rank 1
        let ev = vec![1.0, 0.0, 0.0];
        let r = effective_rank(&ev);
        assert!((r - 1.0).abs() < 1e-10, "expected 1, got {}", r);
    }

    #[test]
    fn test_effective_rank_zero() {
        // All zeros -> 0
        let ev = vec![0.0, 0.0, 0.0];
        let r = effective_rank(&ev);
        assert!((r - 0.0).abs() < 1e-14, "expected 0, got {}", r);
    }
}
