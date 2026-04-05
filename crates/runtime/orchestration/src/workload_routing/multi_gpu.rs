// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_sysmon::pcie_topology::PcieTopologyGraph;

pub(crate) fn evaluate_group(gpus: &[u32], topology: &PcieTopologyGraph) -> (bool, u64) {
    let mut all_shared_switch = true;
    let mut min_bw = u64::MAX;

    for i in 0..gpus.len() {
        for j in (i + 1)..gpus.len() {
            let bw = topology.effective_bandwidth_bps(gpus[i], gpus[j]);
            min_bw = min_bw.min(bw);

            if let Some(pair) = topology.pair(gpus[i], gpus[j]) {
                if pair.common_bridge.is_none() || pair.hops > 1 {
                    all_shared_switch = false;
                }
            } else {
                all_shared_switch = false;
            }
        }
    }

    (all_shared_switch, min_bw)
}

pub(crate) fn combinations(items: &[u32], k: usize) -> Vec<Vec<u32>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.len() < k {
        return vec![];
    }

    let mut result = Vec::new();
    for (i, &item) in items.iter().enumerate() {
        for mut rest in combinations(&items[i + 1..], k - 1) {
            rest.insert(0, item);
            result.push(rest);
        }
    }
    result
}
