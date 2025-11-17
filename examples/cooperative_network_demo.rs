//! Cooperative Network Revolution Demo
//! Shows how giving creates exponential returns through network effects

use std::collections::HashMap;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Node {
    name: String,
    contributed: u64,
    received: u64,
}

#[derive(Debug)]
#[allow(dead_code)]
struct CooperativeNetwork {
    nodes: HashMap<String, Node>,
    total_contributions: u64,
}

impl CooperativeNetwork {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            total_contributions: 0,
        }
    }

    fn add_node(&mut self, name: String) {
        let node = Node {
            name: name.clone(),
            contributed: 0,
            received: 0,
        };
        self.nodes.insert(name, node);
    }

    fn contribute(&mut self, node_name: &str, amount: u64) {
        if let Some(node) = self.nodes.get_mut(node_name) {
            node.contributed += amount;
            self.total_contributions += amount;
        }
        self.distribute_rewards();
    }

    fn distribute_rewards(&mut self) {
        let network_multiplier = 1.0 + (self.total_contributions as f64 / 1000.0);

        // Calculate values before mutable iteration to avoid borrow conflict
        let nodes_len = self.nodes.len();
        let total_contributions = self.total_contributions as f64;

        for node in self.nodes.values_mut() {
            let base_reward = node.contributed as f64 * network_multiplier;
            let network_bonus = total_contributions / nodes_len as f64;
            node.received = (base_reward + network_bonus * 0.5) as u64;
        }
    }

    fn show_status(&self) {
        println!("\n🌐 COOPERATIVE NETWORK STATUS");
        println!("Total Contributions: {}", self.total_contributions);
        println!(
            "Network Multiplier: {:.2}x",
            1.0 + (self.total_contributions as f64 / 1000.0)
        );

        for (name, node) in &self.nodes {
            let multiplier = if node.contributed > 0 {
                node.received as f64 / node.contributed as f64
            } else {
                1.0
            };
            println!(
                "{}: Gave {} → Got {} ({}x return)",
                name, node.contributed, node.received, multiplier
            );
        }
    }
}

fn main() {
    println!("🚀 TOADSTOOL COOPERATIVE NETWORK REVOLUTION");
    println!("═══════════════════════════════════════════");
    println!("✅ Cooperative Model: Give → Get Back MORE");
    println!("🚀 Network Effects: Everyone benefits");
    println!("♾️ No Cap: Unlimited potential");

    let mut network = CooperativeNetwork::new();

    // Add nodes
    network.add_node("Alice".to_string());
    network.add_node("Bob".to_string());
    network.add_node("Carol".to_string());

    println!("\n🤝 COOPERATIVE CONTRIBUTIONS:");

    // Contributions create network effects
    network.contribute("Alice", 1000);
    println!("Alice contributes 1000 compute units");

    network.contribute("Bob", 800);
    println!("Bob contributes 800 compute units");

    network.contribute("Carol", 1200);
    println!("Carol contributes 1200 compute units");

    network.show_status();

    println!("\n🎯 KEY INSIGHT: Everyone gets back MORE than they gave!");
    println!("💡 This is COOPERATIVE NETWORK EFFECTS in action!");
    println!("🌟 Pure Rust ecosystem: Always FREE");
    println!("🔒 BearDog crypto locks: Protect the cooperative value");
    println!("🎉 This is the future of computing!");
}
