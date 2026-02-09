/// MNIST data loading - reuse from ml-inference showcase

use anyhow::{Context, Result};
use ndarray::{Array1, Array3};
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct MnistDataset {
    pub train_images: Array3<f32>,
    pub train_labels: Array1<u8>,
    pub test_images: Array3<f32>,
    pub test_labels: Array1<u8>,
}

impl MnistDataset {
    pub fn load(data_dir: &Path) -> Result<Self> {
        println!("Loading MNIST dataset from {:?}", data_dir);

        let train_images = Self::load_images(&data_dir.join("train-images-idx3-ubyte.gz"))?;
        let train_labels = Self::load_labels(&data_dir.join("train-labels-idx1-ubyte.gz"))?;
        let test_images = Self::load_images(&data_dir.join("t10k-images-idx3-ubyte.gz"))?;
        let test_labels = Self::load_labels(&data_dir.join("t10k-labels-idx1-ubyte.gz"))?;

        println!("✅ Loaded MNIST:");
        println!("   Train: {} images", train_images.shape()[0]);
        println!("   Test:  {} images", test_images.shape()[0]);

        Ok(Self {
            train_images,
            train_labels,
            test_images,
            test_labels,
        })
    }

    fn load_images(path: &Path) -> Result<Array3<f32>> {
        let file = File::open(path).context(format!("Failed to open image file: {:?}", path))?;
        let mut decoder = GzDecoder::new(file);
        let mut buffer = Vec::new();
        decoder.read_to_end(&mut buffer).context("Failed to read image gzip data")?;

        // Parse IDX format
        let magic = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        if magic != 2051 {
            anyhow::bail!("Invalid magic number in image file: {}", magic);
        }

        let n_images = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]) as usize;
        let n_rows = u32::from_be_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]) as usize;
        let n_cols = u32::from_be_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]) as usize;

        let pixel_data = &buffer[16..];
        let pixels: Vec<f32> = pixel_data.iter().map(|&x| x as f32 / 255.0).collect();

        Array3::from_shape_vec((n_images, n_rows, n_cols), pixels)
            .context("Failed to reshape image data")
    }

    fn load_labels(path: &Path) -> Result<Array1<u8>> {
        let file = File::open(path).context(format!("Failed to open label file: {:?}", path))?;
        let mut decoder = GzDecoder::new(file);
        let mut buffer = Vec::new();
        decoder.read_to_end(&mut buffer).context("Failed to read label gzip data")?;

        // Parse IDX format
        let magic = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        if magic != 2049 {
            anyhow::bail!("Invalid magic number in label file: {}", magic);
        }

        let n_labels = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]) as usize;

        Ok(Array1::from_vec(buffer[8..8 + n_labels].to_vec()))
    }

    pub fn partition(&self, start_idx: usize, end_idx: usize) -> Result<(Array3<f32>, Array1<u8>)> {
        let end_idx = end_idx.min(self.train_images.shape()[0]);
        
        if start_idx >= end_idx {
            anyhow::bail!("Invalid partition: start {} >= end {}", start_idx, end_idx);
        }

        let images = self.train_images.slice(ndarray::s![start_idx..end_idx, .., ..]).to_owned();
        let labels = self.train_labels.slice(ndarray::s![start_idx..end_idx]).to_owned();

        Ok((images, labels))
    }
}

