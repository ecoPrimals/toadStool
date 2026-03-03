// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real MNIST data loader - no mocks

use anyhow::{Context, Result};
use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::GzDecoder;
use ndarray::{Array1, Array2};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// MNIST dataset
pub struct MnistDataset {
    pub images: Array2<f32>, // (N, 784)
    pub labels: Array1<u8>,  // (N,)
}

impl MnistDataset {
    /// Load MNIST dataset from files
    pub fn load<P: AsRef<Path>>(images_path: P, labels_path: P) -> Result<Self> {
        let images = Self::load_images(images_path)?;
        let labels = Self::load_labels(labels_path)?;

        if images.nrows() != labels.len() {
            anyhow::bail!(
                "Image count ({}) doesn't match label count ({})",
                images.nrows(),
                labels.len()
            );
        }

        Ok(Self { images, labels })
    }

    /// Load MNIST images (idx3-ubyte format)
    fn load_images<P: AsRef<Path>>(path: P) -> Result<Array2<f32>> {
        let file = File::open(path.as_ref())
            .context(format!("Failed to open {}", path.as_ref().display()))?;

        let mut reader: Box<dyn Read> =
            if path.as_ref().extension().and_then(|s| s.to_str()) == Some("gz") {
                Box::new(GzDecoder::new(file))
            } else {
                Box::new(file)
            };

        // Read header
        let magic = reader.read_u32::<BigEndian>()?;
        if magic != 0x00000803 {
            anyhow::bail!("Invalid MNIST image file magic: {magic:#x}");
        }

        let num_images = reader.read_u32::<BigEndian>()? as usize;
        let num_rows = reader.read_u32::<BigEndian>()? as usize;
        let num_cols = reader.read_u32::<BigEndian>()? as usize;

        if num_rows != 28 || num_cols != 28 {
            anyhow::bail!("Expected 28x28 images, got {num_rows}x{num_cols}");
        }

        // Read pixel data
        let mut buffer = vec![0u8; num_images * num_rows * num_cols];
        reader.read_exact(&mut buffer)?;

        // Convert to float32 and normalize to [0, 1]
        let images: Vec<f32> = buffer.iter().map(|&x| x as f32 / 255.0).collect();

        // Reshape to (N, 784)
        Array2::from_shape_vec((num_images, num_rows * num_cols), images)
            .context("Failed to reshape image array")
    }

    /// Load MNIST labels (idx1-ubyte format)
    fn load_labels<P: AsRef<Path>>(path: P) -> Result<Array1<u8>> {
        let file = File::open(path.as_ref())
            .context(format!("Failed to open {}", path.as_ref().display()))?;

        let mut reader: Box<dyn Read> =
            if path.as_ref().extension().and_then(|s| s.to_str()) == Some("gz") {
                Box::new(GzDecoder::new(file))
            } else {
                Box::new(file)
            };

        // Read header
        let magic = reader.read_u32::<BigEndian>()?;
        if magic != 0x00000801 {
            anyhow::bail!("Invalid MNIST label file magic: {magic:#x}");
        }

        let num_labels = reader.read_u32::<BigEndian>()? as usize;

        // Read labels
        let mut buffer = vec![0u8; num_labels];
        reader.read_exact(&mut buffer)?;

        Ok(Array1::from_vec(buffer))
    }

    /// Get number of samples
    pub fn len(&self) -> usize {
        self.images.nrows()
    }

    /// Get a single sample
    pub fn get(&self, index: usize) -> Option<(Array1<f32>, u8)> {
        if index >= self.len() {
            return None;
        }

        let image = self.images.row(index).to_owned();
        let label = self.labels[index];

        Some((image, label))
    }

    /// Get a batch of samples
    pub fn batch(&self, start: usize, size: usize) -> Option<(Array2<f32>, Array1<u8>)> {
        let end = (start + size).min(self.len());
        if start >= end {
            return None;
        }

        let images = self.images.slice(ndarray::s![start..end, ..]).to_owned();
        let labels = self.labels.slice(ndarray::s![start..end]).to_owned();

        Some((images, labels))
    }
}

/// Download MNIST dataset if not present
///
/// **Note**: reqwest was removed as part of Pure Rust evolution.
/// Please download MNIST files manually from <http://yann.lecun.com/exdb/mnist/>
/// Or use the download-mnist binary (requires adding reqwest to Cargo.toml).
pub async fn download_mnist<P: AsRef<Path>>(_data_dir: P) -> Result<()> {
    anyhow::bail!(
        "download_mnist requires manual MNIST download.\n\
         \n\
         Download files from: <http://yann.lecun.com/exdb/mnist/>\n\
         Files needed:\n\
         - train-images-idx3-ubyte.gz\n\
         - train-labels-idx1-ubyte.gz\n\
         - t10k-images-idx3-ubyte.gz\n\
         - t10k-labels-idx1-ubyte.gz\n\
         \n\
         Or use: cargo run --bin download-mnist (requires adding reqwest)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Run manually: cargo test --release -- --ignored
    fn test_load_mnist() {
        let dataset = MnistDataset::load(
            "data/mnist/train-images-idx3-ubyte.gz",
            "data/mnist/train-labels-idx1-ubyte.gz",
        )
        .unwrap();

        assert_eq!(dataset.len(), 60000);
        assert_eq!(dataset.images.ncols(), 784);

        let (image, label) = dataset.get(0).unwrap();
        assert_eq!(image.len(), 784);
        assert!(label < 10);
        assert!(image.iter().all(|&x| x >= 0.0 && x <= 1.0));
    }
}
