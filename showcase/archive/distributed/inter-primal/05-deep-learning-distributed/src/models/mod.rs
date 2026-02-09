// Model architectures

pub mod resnet18;

use anyhow::Result;
use tch::nn;

pub trait Model {
    /// Forward pass
    fn forward(&self, xs: &tch::Tensor) -> tch::Tensor;
    
    /// Get the number of parameters
    fn num_parameters(&self) -> i64;
}

/// Create a model by name
pub fn create_model(name: &str, vs: &nn::Path, num_classes: i64) -> Result<Box<dyn Model>> {
    match name {
        "resnet18" => Ok(Box::new(resnet18::ResNet18::new(vs, num_classes))),
        _ => anyhow::bail!("Unknown model: {}", name),
    }
}

