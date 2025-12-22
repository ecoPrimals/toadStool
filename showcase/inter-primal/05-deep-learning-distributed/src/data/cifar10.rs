// CIFAR-10 dataset loader
// 60K 32x32 color images in 10 classes

use super::DataLoader;
use anyhow::Result;
use std::path::Path;
use tch::{vision, Tensor};

pub struct Cifar10 {
    train_images: Tensor,
    train_labels: Tensor,
    test_images: Tensor,
    test_labels: Tensor,
}

impl Cifar10 {
    pub fn load<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref();
        
        tracing::info!("Loading CIFAR-10 from {:?}", data_dir);
        
        // Load using tch's vision module
        let dataset = vision::cifar::load_dir(data_dir)
            .map_err(|e| anyhow::anyhow!("Failed to load CIFAR-10: {}", e))?;
        
        // Normalize images to [0, 1] range and convert to CHW format
        let train_images = dataset.train_images
            .to_kind(tch::Kind::Float)
            .g_div_scalar(255.0)
            .view([-1, 3, 32, 32]); // NCHW format
        
        let test_images = dataset.test_images
            .to_kind(tch::Kind::Float)
            .g_div_scalar(255.0)
            .view([-1, 3, 32, 32]);
        
        let train_labels = dataset.train_labels;
        let test_labels = dataset.test_labels;
        
        tracing::info!(
            "Loaded CIFAR-10: {} train, {} test images",
            train_images.size()[0],
            test_images.size()[0]
        );
        
        Ok(Self {
            train_images,
            train_labels,
            test_images,
            test_labels,
        })
    }
    
    /// Apply data augmentation for training
    pub fn augment(&self, images: &Tensor) -> Tensor {
        // Random horizontal flip
        let flipped = images.flip(&[3]); // Flip along width
        let mask = Tensor::rand(&[images.size()[0], 1, 1, 1], (tch::Kind::Float, images.device()))
            .gt(0.5);
        
        let augmented = mask.where_self(images, &flipped);
        
        // Random crop (4 pixels padding, then crop back to 32x32)
        // For simplicity, we'll skip this for now and just use horizontal flip
        
        augmented
    }
}

impl DataLoader for Cifar10 {
    fn train_images(&self) -> &Tensor {
        &self.train_images
    }
    
    fn train_labels(&self) -> &Tensor {
        &self.train_labels
    }
    
    fn test_images(&self) -> &Tensor {
        &self.test_images
    }
    
    fn test_labels(&self) -> &Tensor {
        &self.test_labels
    }
    
    fn num_classes(&self) -> i64 {
        10
    }
    
    fn labels(&self) -> Vec<String> {
        vec![
            "airplane".to_string(),
            "automobile".to_string(),
            "bird".to_string(),
            "cat".to_string(),
            "deer".to_string(),
            "dog".to_string(),
            "frog".to_string(),
            "horse".to_string(),
            "ship".to_string(),
            "truck".to_string(),
        ]
    }
}

