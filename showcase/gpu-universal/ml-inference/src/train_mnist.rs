//! Train MNIST classifier - Real training with validation

use anyhow::Result;
use ml_inference_showcase::{
    mnist::MnistDataset, network::SimpleNetwork, training::TrainingConfig,
};

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  MNIST Training - Real Backpropagation                  ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Load training data
    println!("Loading training dataset...");
    let train_data = MnistDataset::load(
        "data/mnist/train-images-idx3-ubyte.gz",
        "data/mnist/train-labels-idx1-ubyte.gz",
    )?;
    println!("✓ Loaded {} training samples", train_data.len());

    // Load test data
    println!("Loading test dataset...");
    let test_data = MnistDataset::load(
        "data/mnist/t10k-images-idx3-ubyte.gz",
        "data/mnist/t10k-labels-idx1-ubyte.gz",
    )?;
    println!("✓ Loaded {} test samples", test_data.len());
    println!();

    // Create network
    println!("Initializing neural network...");
    let mut network = SimpleNetwork::new();
    println!("✓ Network ready (784 -> 128 -> 10)");
    println!();

    // Training configuration
    let config = TrainingConfig {
        learning_rate: 0.1,
        batch_size: 64,
        epochs: 10,
    };

    // Train
    let stats = network.train(
        &train_data.images,
        &train_data.labels,
        &test_data.images,
        &test_data.labels,
        &config,
    )?;

    // Final results
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Training Complete!                                      ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    let final_stats = stats.last().unwrap();
    println!("Final Results:");
    println!(
        "  Train accuracy: {:.2}%",
        final_stats.train_accuracy * 100.0
    );
    println!(
        "  Test accuracy:  {:.2}%",
        final_stats.test_accuracy * 100.0
    );
    println!();

    // Save trained weights
    network.save_weights("models/mnist_trained.weights")?;

    // Save training stats
    let stats_json = serde_json::to_string_pretty(&stats)?;
    std::fs::create_dir_all("results")?;
    std::fs::write("results/training_stats.json", stats_json)?;
    println!("✓ Training statistics saved to results/training_stats.json");
    println!();

    if final_stats.test_accuracy > 0.90 {
        println!("🎉 SUCCESS! Model achieves >90% accuracy!");
        println!("   This proves backpropagation works correctly.");
    } else {
        println!("⚠️  Warning: Accuracy below 90%");
        println!("   Try training longer or adjusting hyperparameters.");
    }

    Ok(())
}
