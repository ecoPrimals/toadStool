//! Neural network training with backpropagation

use crate::network::SimpleNetwork;
use anyhow::{Context, Result};
use ndarray::{Array1, Array2};
use rand::seq::SliceRandom;

/// Training hyperparameters
pub struct TrainingConfig {
    pub learning_rate: f32,
    pub batch_size: usize,
    pub epochs: usize,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            batch_size: 32,
            epochs: 5,
        }
    }
}

/// Training statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrainingStats {
    pub epoch: usize,
    pub train_loss: f32,
    pub train_accuracy: f32,
    pub test_accuracy: f32,
}

impl SimpleNetwork {
    /// Train the network using backpropagation and SGD
    pub fn train(
        &mut self,
        train_images: &Array2<f32>,
        train_labels: &Array1<u8>,
        test_images: &Array2<f32>,
        test_labels: &Array1<u8>,
        config: &TrainingConfig,
    ) -> Result<Vec<TrainingStats>> {
        let mut rng = rand::thread_rng();
        let mut stats = Vec::new();

        let num_samples = train_images.nrows();
        let mut indices: Vec<usize> = (0..num_samples).collect();

        println!("Training network...");
        println!("  Samples: {num_samples}");
        println!("  Batch size: {}", config.batch_size);
        println!("  Epochs: {}", config.epochs);
        println!("  Learning rate: {}", config.learning_rate);
        println!();

        for epoch in 0..config.epochs {
            // Shuffle training data
            indices.shuffle(&mut rng);

            let mut epoch_loss = 0.0;
            let mut correct = 0;
            let num_batches = num_samples.div_ceil(config.batch_size);

            for batch_idx in 0..num_batches {
                let batch_start = batch_idx * config.batch_size;
                let batch_end = (batch_start + config.batch_size).min(num_samples);
                let batch_indices = &indices[batch_start..batch_end];

                // Forward + backward pass for batch
                let (loss, batch_correct) = self.train_batch(
                    train_images,
                    train_labels,
                    batch_indices,
                    config.learning_rate,
                )?;

                epoch_loss += loss;
                correct += batch_correct;

                if (batch_idx + 1) % 100 == 0 {
                    print!(
                        "\r  Epoch {}/{}: batch {}/{}",
                        epoch + 1,
                        config.epochs,
                        batch_idx + 1,
                        num_batches
                    );
                }
            }

            let train_loss = epoch_loss / num_batches as f32;
            let train_accuracy = correct as f32 / num_samples as f32;

            // Test accuracy
            let test_accuracy = self.accuracy(test_images, test_labels)?;

            println!(
                "\r  Epoch {}/{}: loss={:.4}, train_acc={:.2}%, test_acc={:.2}%",
                epoch + 1,
                config.epochs,
                train_loss,
                train_accuracy * 100.0,
                test_accuracy * 100.0
            );

            stats.push(TrainingStats {
                epoch: epoch + 1,
                train_loss,
                train_accuracy,
                test_accuracy,
            });
        }

        println!();
        Ok(stats)
    }

    /// Train on a single batch
    fn train_batch(
        &mut self,
        images: &Array2<f32>,
        labels: &Array1<u8>,
        batch_indices: &[usize],
        learning_rate: f32,
    ) -> Result<(f32, usize)> {
        let batch_size = batch_indices.len();

        // Accumulate gradients
        let mut dw1 = Array2::<f32>::zeros(self.w1.raw_dim());
        let mut db1 = Array1::<f32>::zeros(self.b1.raw_dim());
        let mut dw2 = Array2::<f32>::zeros(self.w2.raw_dim());
        let mut db2 = Array1::<f32>::zeros(self.b2.raw_dim());

        let mut batch_loss = 0.0;
        let mut correct = 0;

        for &idx in batch_indices {
            let image = images.row(idx).to_owned();
            let label = labels[idx];

            // Forward pass
            let z1 = image.dot(&self.w1) + &self.b1;
            let a1 = z1.mapv(|x| x.max(0.0)); // ReLU
            let z2 = a1.dot(&self.w2) + &self.b2;

            // Softmax
            let exp_z2 = z2.mapv(|x| x.exp());
            let sum_exp = exp_z2.sum();
            let output = &exp_z2 / sum_exp;

            // Loss (cross-entropy)
            let target_prob = output[label as usize];
            batch_loss -= target_prob.ln();

            // Accuracy
            let (predicted, _) = self.predict(&output);
            if predicted == label as usize {
                correct += 1;
            }

            // Backward pass
            // dL/dz2 (softmax + cross-entropy derivative)
            let mut dz2 = output.clone();
            dz2[label as usize] -= 1.0;

            // Gradients for w2 and b2
            let dw2_sample = a1
                .insert_axis(ndarray::Axis(1))
                .dot(&dz2.view().insert_axis(ndarray::Axis(0)));
            dw2 = dw2 + dw2_sample;
            db2 = &db2 + &dz2;

            // Backprop to hidden layer
            let da1 = dz2.dot(&self.w2.t());
            let dz1 = &da1 * &z1.mapv(|x| if x > 0.0 { 1.0 } else { 0.0 }); // ReLU derivative

            // Gradients for w1 and b1
            let dw1_sample = image
                .insert_axis(ndarray::Axis(1))
                .dot(&dz1.view().insert_axis(ndarray::Axis(0)));
            dw1 = dw1 + dw1_sample;
            db1 = &db1 + &dz1;
        }

        // Average gradients
        let scale = learning_rate / batch_size as f32;
        self.w1 = &self.w1 - &(dw1 * scale);
        self.b1 = &self.b1 - &(db1 * scale);
        self.w2 = &self.w2 - &(dw2 * scale);
        self.b2 = &self.b2 - &(db2 * scale);

        Ok((batch_loss / batch_size as f32, correct))
    }

    /// Save trained weights to file
    pub fn save_weights(&self, path: &str) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;

        // Save dimensions
        writeln!(file, "{} {}", self.w1.nrows(), self.w1.ncols())?;
        writeln!(file, "{} {}", self.w2.nrows(), self.w2.ncols())?;

        // Save w1
        for row in self.w1.rows() {
            for &val in row {
                write!(file, "{val} ")?;
            }
            writeln!(file)?;
        }

        // Save b1
        for &val in &self.b1 {
            write!(file, "{val} ")?;
        }
        writeln!(file)?;

        // Save w2
        for row in self.w2.rows() {
            for &val in row {
                write!(file, "{val} ")?;
            }
            writeln!(file)?;
        }

        // Save b2
        for &val in &self.b2 {
            write!(file, "{val} ")?;
        }
        writeln!(file)?;

        println!("✓ Saved weights to {path}");
        Ok(())
    }

    /// Load trained weights from file
    pub fn load_weights(path: &str) -> Result<Self> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // Helper to parse line of numbers
        let parse_line = |line: String, context: &str| -> Result<Vec<f32>> {
            line.split_whitespace()
                .map(|s| {
                    s.parse::<f32>()
                        .with_context(|| format!("Failed to parse '{s}' as f32 in {context}"))
                })
                .collect()
        };

        // Helper to parse dimensions
        let parse_dims = |line: String, context: &str| -> Result<Vec<usize>> {
            line.split_whitespace()
                .map(|s| {
                    s.parse::<usize>()
                        .with_context(|| format!("Failed to parse '{s}' as dimension in {context}"))
                })
                .collect()
        };

        // Read dimensions
        let dims1 = parse_dims(
            lines
                .next()
                .ok_or_else(|| anyhow::anyhow!("Missing w1 dimensions"))??,
            "w1 dimensions",
        )?;
        let dims2 = parse_dims(
            lines
                .next()
                .ok_or_else(|| anyhow::anyhow!("Missing w2 dimensions"))??,
            "w2 dimensions",
        )?;

        // Validate dimensions
        anyhow::ensure!(
            dims1.len() == 2,
            "w1 dimensions must have 2 values, got {}",
            dims1.len()
        );
        anyhow::ensure!(
            dims2.len() == 2,
            "w2 dimensions must have 2 values, got {}",
            dims2.len()
        );

        // Read w1
        let mut w1_data = Vec::new();
        for i in 0..dims1[0] {
            let line = lines
                .next()
                .ok_or_else(|| anyhow::anyhow!("Missing w1 row {}/{}", i + 1, dims1[0]))??;
            let row = parse_line(line, &format!("w1 row {}", i + 1))?;
            anyhow::ensure!(
                row.len() == dims1[1],
                "w1 row {} has {} values, expected {}",
                i + 1,
                row.len(),
                dims1[1]
            );
            w1_data.extend(row);
        }
        let w1 = Array2::from_shape_vec((dims1[0], dims1[1]), w1_data)
            .with_context(|| format!("Failed to create w1 array with shape {dims1:?}"))?;

        // Read b1
        let b1_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("Missing b1 bias vector"))??;
        let b1_data = parse_line(b1_line, "b1 bias")?;
        let b1 = Array1::from_vec(b1_data);

        // Read w2
        let mut w2_data = Vec::new();
        for i in 0..dims2[0] {
            let line = lines
                .next()
                .ok_or_else(|| anyhow::anyhow!("Missing w2 row {}/{}", i + 1, dims2[0]))??;
            let row = parse_line(line, &format!("w2 row {}", i + 1))?;
            anyhow::ensure!(
                row.len() == dims2[1],
                "w2 row {} has {} values, expected {}",
                i + 1,
                row.len(),
                dims2[1]
            );
            w2_data.extend(row);
        }
        let w2 = Array2::from_shape_vec((dims2[0], dims2[1]), w2_data)
            .with_context(|| format!("Failed to create w2 array with shape {dims2:?}"))?;

        // Read b2
        let b2_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("Missing b2 bias vector"))??;
        let b2_data = parse_line(b2_line, "b2 bias")?;
        let b2 = Array1::from_vec(b2_data);

        println!("✓ Loaded weights from {path}");
        Ok(Self { w1, b1, w2, b2 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_shapes() {
        let mut network = SimpleNetwork::new();
        let images = Array2::from_elem((10, 784), 0.5);
        let labels = Array1::from_vec(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let config = TrainingConfig {
            learning_rate: 0.01,
            batch_size: 5,
            epochs: 1,
        };

        let stats = network
            .train(&images, &labels, &images, &labels, &config)
            .unwrap();
        assert_eq!(stats.len(), 1);
    }
}
