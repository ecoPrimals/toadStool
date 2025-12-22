// Dataset loaders

pub mod cifar10;

use anyhow::Result;

/// Dataset trait
pub trait DataLoader {
    fn train_images(&self) -> &tch::Tensor;
    fn train_labels(&self) -> &tch::Tensor;
    fn test_images(&self) -> &tch::Tensor;
    fn test_labels(&self) -> &tch::Tensor;
    fn num_classes(&self) -> i64;
    fn labels(&self) -> Vec<String>;
}

/// Load a dataset by name
pub fn load_dataset(name: &str, data_dir: &str) -> Result<Box<dyn DataLoader>> {
    match name {
        "cifar10" => Ok(Box::new(cifar10::Cifar10::load(data_dir)?)),
        _ => anyhow::bail!("Unknown dataset: {}", name),
    }
}

