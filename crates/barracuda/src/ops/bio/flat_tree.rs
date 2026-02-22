// SPDX-License-Identifier: AGPL-3.0-only

//! Generic CSR FlatTree for phylogenetic and classification tree dispatch.
//!
//! A `FlatTree` stores tree topology in Compressed Sparse Row (CSR) format:
//! parent indices, branch lengths, and per-node metadata. This is the common
//! layout consumed by GPU shaders (Felsenstein pruning, UniFrac propagation,
//! bootstrap resampling, NJ clustering, DTL reconciliation).
//!
//! Provenance: wetSpring metagenomics + neuralSpring metalForge → toadStool

/// Generic CSR tree with parent-indexed topology.
///
/// Nodes are numbered `0..n_nodes`. Leaves occupy indices `0..n_leaves`.
/// Internal nodes occupy `n_leaves..n_nodes`. The root has `parent[root] == -1`.
#[derive(Debug, Clone)]
pub struct FlatTree {
    /// Parent index for each node; `-1` for the root.
    pub parent: Vec<i32>,
    /// Branch length (edge weight) for each node.
    pub branch_length: Vec<f64>,
    /// Number of leaf nodes (always stored at indices `0..n_leaves`).
    pub n_leaves: usize,
}

impl FlatTree {
    /// Total number of nodes in the tree.
    pub fn n_nodes(&self) -> usize {
        self.parent.len()
    }

    /// Validate structural invariants.
    pub fn validate(&self) -> Result<(), &'static str> {
        let n = self.n_nodes();
        if self.branch_length.len() != n {
            return Err("branch_length length mismatch");
        }
        if self.n_leaves > n {
            return Err("n_leaves exceeds n_nodes");
        }
        let root_count = self.parent.iter().filter(|&&p| p < 0).count();
        if root_count != 1 {
            return Err("tree must have exactly one root");
        }
        for (i, &p) in self.parent.iter().enumerate() {
            if p >= 0 && p as usize >= n {
                return Err("parent index out of bounds");
            }
            if p >= 0 && p as usize == i {
                return Err("self-loop detected");
            }
        }
        Ok(())
    }

    /// Convert to GPU-ready buffers (parent as `i32`, branch_length as `f64`).
    pub fn to_gpu_arrays(&self) -> (Vec<i32>, Vec<f64>) {
        (self.parent.clone(), self.branch_length.clone())
    }

    /// Build tree levels bottom-up for multi-pass GPU dispatch.
    ///
    /// Returns a vector of levels, each containing node indices for that level.
    /// Level 0 = leaves, last level = root.
    pub fn bottom_up_levels(&self) -> Vec<Vec<usize>> {
        let n = self.n_nodes();
        let mut depth = vec![0u32; n];
        let mut max_depth = 0u32;

        // Compute depths top-down
        let root = self.parent.iter().position(|&p| p < 0).unwrap_or(0);
        depth[root] = 0;

        // BFS from root
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(root);
        let mut children: Vec<Vec<usize>> = vec![vec![]; n];
        for (i, &p) in self.parent.iter().enumerate() {
            if p >= 0 {
                children[p as usize].push(i);
            }
        }
        while let Some(node) = queue.pop_front() {
            for &child in &children[node] {
                depth[child] = depth[node] + 1;
                max_depth = max_depth.max(depth[child]);
                queue.push_back(child);
            }
        }

        // Group by depth, reverse for bottom-up
        let mut levels: Vec<Vec<usize>> = vec![vec![]; (max_depth + 1) as usize];
        for (i, &d) in depth.iter().enumerate() {
            levels[d as usize].push(i);
        }
        levels.reverse();
        levels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> FlatTree {
        // Simple tree: 3 leaves (0,1,2), 2 internal (3,4=root)
        //       4
        //      / \
        //     3   2
        //    / \
        //   0   1
        FlatTree {
            parent: vec![3, 3, 4, 4, -1],
            branch_length: vec![0.1, 0.2, 0.3, 0.15, 0.0],
            n_leaves: 3,
        }
    }

    #[test]
    fn validate_ok() {
        assert!(sample_tree().validate().is_ok());
    }

    #[test]
    fn validate_bad_root_count() {
        let mut t = sample_tree();
        t.parent[3] = -1; // two roots
        assert!(t.validate().is_err());
    }

    #[test]
    fn bottom_up_levels_correct() {
        let t = sample_tree();
        let levels = t.bottom_up_levels();
        // Deepest level first (leaves 0,1), then (leaf 2, internal 3), then root 4
        assert_eq!(levels.len(), 3);
        assert!(levels[0].contains(&0));
        assert!(levels[0].contains(&1));
        assert!(levels.last().unwrap().contains(&4));
    }
}
