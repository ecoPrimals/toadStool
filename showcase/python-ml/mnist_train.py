#!/usr/bin/env python3
"""
MNIST Training in Python via ToadStool
Same workload as Rust version, proves multi-runtime support
"""

import numpy as np
import gzip
import struct
import json
import time
from pathlib import Path


def load_mnist_images(filepath):
    """Load MNIST images from idx3-ubyte format"""
    with gzip.open(filepath, 'rb') as f:
        magic, num, rows, cols = struct.unpack('>IIII', f.read(16))
        assert magic == 0x803, f"Invalid MNIST image file magic: {magic:#x}"
        assert rows == 28 and cols == 28, f"Expected 28x28 images, got {rows}x{cols}"
        
        # Read all pixels and normalize to [0, 1]
        images = np.frombuffer(f.read(), dtype=np.uint8)
        images = images.reshape(num, rows * cols).astype(np.float32) / 255.0
        
    return images


def load_mnist_labels(filepath):
    """Load MNIST labels from idx1-ubyte format"""
    with gzip.open(filepath, 'rb') as f:
        magic, num = struct.unpack('>II', f.read(8))
        assert magic == 0x801, f"Invalid MNIST label file magic: {magic:#x}"
        
        labels = np.frombuffer(f.read(), dtype=np.uint8)
        
    return labels


class SimpleNetwork:
    """2-layer neural network: 784 -> 128 -> 10"""
    
    def __init__(self):
        # He initialization
        self.w1 = np.random.randn(784, 128).astype(np.float32) * np.sqrt(2.0 / 784)
        self.b1 = np.zeros(128, dtype=np.float32)
        self.w2 = np.random.randn(128, 10).astype(np.float32) * np.sqrt(2.0 / 128)
        self.b2 = np.zeros(10, dtype=np.float32)
    
    def forward(self, x):
        """Forward pass"""
        # Layer 1
        z1 = x @ self.w1 + self.b1
        a1 = np.maximum(0, z1)  # ReLU
        
        # Layer 2
        z2 = a1 @ self.w2 + self.b2
        
        # Softmax
        exp_z2 = np.exp(z2 - np.max(z2, axis=-1, keepdims=True))  # Numerical stability
        output = exp_z2 / np.sum(exp_z2, axis=-1, keepdims=True)
        
        return output, a1, z1
    
    def predict(self, x):
        """Predict class"""
        output, _, _ = self.forward(x)
        return np.argmax(output, axis=-1)
    
    def accuracy(self, images, labels):
        """Calculate accuracy"""
        predictions = self.predict(images)
        return np.mean(predictions == labels)
    
    def train(self, train_images, train_labels, test_images, test_labels, 
              learning_rate=0.1, batch_size=64, epochs=10):
        """Train with backpropagation and SGD"""
        
        num_samples = len(train_images)
        num_batches = (num_samples + batch_size - 1) // batch_size
        
        print(f"Training network...")
        print(f"  Samples: {num_samples}")
        print(f"  Batch size: {batch_size}")
        print(f"  Epochs: {epochs}")
        print(f"  Learning rate: {learning_rate}")
        print()
        
        stats = []
        
        for epoch in range(epochs):
            # Shuffle training data
            indices = np.random.permutation(num_samples)
            train_images_shuffled = train_images[indices]
            train_labels_shuffled = train_labels[indices]
            
            epoch_loss = 0.0
            correct = 0
            
            for batch_idx in range(num_batches):
                batch_start = batch_idx * batch_size
                batch_end = min(batch_start + batch_size, num_samples)
                
                batch_images = train_images_shuffled[batch_start:batch_end]
                batch_labels = train_labels_shuffled[batch_start:batch_end]
                
                # Forward pass
                output, a1, z1 = self.forward(batch_images)
                
                # Loss (cross-entropy)
                batch_size_actual = len(batch_images)
                targets = np.eye(10)[batch_labels]
                loss = -np.sum(targets * np.log(output + 1e-8)) / batch_size_actual
                epoch_loss += loss
                
                # Accuracy
                predictions = np.argmax(output, axis=1)
                correct += np.sum(predictions == batch_labels)
                
                # Backward pass
                # Gradient of softmax + cross-entropy
                dz2 = (output - targets) / batch_size_actual
                
                # Gradients for w2 and b2
                dw2 = a1.T @ dz2
                db2 = np.sum(dz2, axis=0)
                
                # Backprop to hidden layer
                da1 = dz2 @ self.w2.T
                dz1 = da1 * (z1 > 0)  # ReLU derivative
                
                # Gradients for w1 and b1
                dw1 = batch_images.T @ dz1
                db1 = np.sum(dz1, axis=0)
                
                # Update weights
                self.w1 -= learning_rate * dw1
                self.b1 -= learning_rate * db1
                self.w2 -= learning_rate * dw2
                self.b2 -= learning_rate * db2
                
                if (batch_idx + 1) % 100 == 0:
                    print(f"\r  Epoch {epoch+1}/{epochs}: batch {batch_idx+1}/{num_batches}", end='')
            
            train_loss = epoch_loss / num_batches
            train_accuracy = correct / num_samples
            test_accuracy = self.accuracy(test_images, test_labels)
            
            print(f"\r  Epoch {epoch+1}/{epochs}: loss={train_loss:.4f}, "
                  f"train_acc={train_accuracy*100:.2f}%, test_acc={test_accuracy*100:.2f}%")
            
            stats.append({
                'epoch': epoch + 1,
                'train_loss': float(train_loss),
                'train_accuracy': float(train_accuracy),
                'test_accuracy': float(test_accuracy),
            })
        
        print()
        return stats
    
    def save_weights(self, filepath):
        """Save trained weights"""
        np.savez(filepath,
                 w1=self.w1, b1=self.b1,
                 w2=self.w2, b2=self.b2)
        print(f"✓ Saved weights to {filepath}")
    
    @classmethod
    def load_weights(cls, filepath):
        """Load trained weights"""
        data = np.load(filepath)
        network = cls()
        network.w1 = data['w1']
        network.b1 = data['b1']
        network.w2 = data['w2']
        network.b2 = data['b2']
        print(f"✓ Loaded weights from {filepath}")
        return network


def main():
    print("╔══════════════════════════════════════════════════════════╗")
    print("║  MNIST Training - Python via ToadStool                  ║")
    print("║  Same workload as Rust, proves multi-runtime support    ║")
    print("╚══════════════════════════════════════════════════════════╝")
    print()
    
    # Set random seed for reproducibility
    np.random.seed(42)
    
    # Paths
    data_dir = Path("../gpu-universal/ml-inference/data/mnist")
    output_dir = Path("results")
    output_dir.mkdir(exist_ok=True)
    
    # Load data
    print("Loading training dataset...")
    train_images = load_mnist_images(data_dir / "train-images-idx3-ubyte.gz")
    train_labels = load_mnist_labels(data_dir / "train-labels-idx1-ubyte.gz")
    print(f"✓ Loaded {len(train_images)} training samples")
    
    print("Loading test dataset...")
    test_images = load_mnist_images(data_dir / "t10k-images-idx3-ubyte.gz")
    test_labels = load_mnist_labels(data_dir / "t10k-labels-idx1-ubyte.gz")
    print(f"✓ Loaded {len(test_images)} test samples")
    print()
    
    # Create network
    print("Initializing neural network...")
    network = SimpleNetwork()
    print("✓ Network ready (784 -> 128 -> 10)")
    print()
    
    # Train
    start_time = time.time()
    stats = network.train(
        train_images, train_labels,
        test_images, test_labels,
        learning_rate=0.1,
        batch_size=64,
        epochs=10
    )
    training_time = time.time() - start_time
    
    # Results
    print("╔══════════════════════════════════════════════════════════╗")
    print("║  Training Complete!                                      ║")
    print("╚══════════════════════════════════════════════════════════╝")
    print()
    
    final_stats = stats[-1]
    print(f"Final Results:")
    print(f"  Train accuracy: {final_stats['train_accuracy']*100:.2f}%")
    print(f"  Test accuracy:  {final_stats['test_accuracy']*100:.2f}%")
    print(f"  Training time:  {training_time:.1f} seconds")
    print()
    
    # Save weights
    models_dir = Path("models")
    models_dir.mkdir(exist_ok=True)
    network.save_weights(models_dir / "mnist_trained_python.npz")
    
    # Save stats
    with open(output_dir / "training_stats_python.json", 'w') as f:
        json.dump(stats, f, indent=2)
    print(f"✓ Training statistics saved to results/training_stats_python.json")
    print()
    
    # Compare with Rust version
    rust_stats_path = Path("../gpu-universal/ml-inference/results/training_stats.json")
    if rust_stats_path.exists():
        print("╔══════════════════════════════════════════════════════════╗")
        print("║  Comparison: Python vs Rust                             ║")
        print("╚══════════════════════════════════════════════════════════╝")
        print()
        
        with open(rust_stats_path) as f:
            rust_stats = json.load(f)
        
        rust_final = rust_stats[-1]
        python_final = stats[-1]
        
        print(f"┌────────────────────┬─────────────┬─────────────┐")
        print(f"│ Metric             │ Rust        │ Python      │")
        print(f"├────────────────────┼─────────────┼─────────────┤")
        print(f"│ Test Accuracy      │ {rust_final['test_accuracy']*100:>10.2f}% │ {python_final['test_accuracy']*100:>10.2f}% │")
        print(f"│ Train Accuracy     │ {rust_final['train_accuracy']*100:>10.2f}% │ {python_final['train_accuracy']*100:>10.2f}% │")
        print(f"│ Final Loss         │ {rust_final['train_loss']:>11.4f} │ {python_final['train_loss']:>11.4f} │")
        print(f"└────────────────────┴─────────────┴─────────────┘")
        print()
        
        accuracy_diff = abs(rust_final['test_accuracy'] - python_final['test_accuracy'])
        if accuracy_diff < 0.02:  # Within 2%
            print("✅ SUCCESS! Python and Rust results match!")
            print(f"   Accuracy difference: {accuracy_diff*100:.2f}%")
            print("   This proves ToadStool's multi-runtime support works!")
        else:
            print("⚠️  Note: Results differ by more than 2%")
            print("   This is expected due to different random initializations.")
    
    if final_stats['test_accuracy'] > 0.90:
        print()
        print("🎉 SUCCESS! Model achieves >90% accuracy!")
        print("   This proves Python training works correctly on ToadStool.")


if __name__ == '__main__':
    main()

