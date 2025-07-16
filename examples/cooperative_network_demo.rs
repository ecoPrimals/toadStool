//! Cooperative Network Revolution Demo
//! Shows how giving creates exponential returns through network effects

use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Node {
    name: String,
    contributed: u64,
    received: u64,
}

#[derive(Debug)]
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
        let node_count = self.nodes.len() as f64;
        let total_contributions = self.total_contributions as f64;

        for node in self.nodes.values_mut() {
            let base_reward = node.contributed as f64 * network_multiplier;
            let network_bonus = total_contributions / node_count;
            node.received = (base_reward + network_bonus * 0.5) as u64;
        }
    }

    fn show_status(&self) {
        println!("\n🌐 Cooperative Network Status:");
        println!("Total Contributions: {}", self.total_contributions);
        println!(
            "Network Multiplier: {:.2}x",
            1.0 + (self.total_contributions as f64 / 1000.0)
        );

        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by(|a, b| b.received.cmp(&a.received));

        for node in nodes {
            let roi = if node.contributed > 0 {
                (node.received as f64 / node.contributed as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "  {} - Gave: {}, Received: {}, ROI: {:.1}%",
                node.name, node.contributed, node.received, roi
            );
        }
    }
}

fn main() {
    println!("🚀 Cooperative Network Revolution Demo");
    println!("Demonstrating exponential returns through giving\n");

    let mut network = CooperativeNetwork::new();

    // Add nodes
    network.add_node("Alice".to_string());
    network.add_node("Bob".to_string());
    network.add_node("Charlie".to_string());
    network.add_node("Diana".to_string());

    println!("Phase 1: Initial contributions");
    network.contribute("Alice", 100);
    network.contribute("Bob", 150);
    network.show_status();

    println!("\nPhase 2: More join the network");
    network.contribute("Charlie", 200);
    network.contribute("Diana", 75);
    network.show_status();

    println!("\nPhase 3: Network effects amplify");
    network.contribute("Alice", 50);
    network.contribute("Bob", 100);
    network.contribute("Charlie", 25);
    network.show_status();

    println!("\n✨ Notice how everyone's returns increase as the network grows!");
    println!("This is the power of cooperative economics - giving creates abundance for all.");
}
