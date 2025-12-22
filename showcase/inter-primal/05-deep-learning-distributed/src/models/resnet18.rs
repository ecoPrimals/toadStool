// ResNet-18 implementation
// Deep Residual Learning for Image Recognition (He et al., 2015)

use super::Model;
use tch::nn::{self, ModuleT};
use tch::Tensor;

/// Basic residual block
#[derive(Debug)]
struct BasicBlock {
    conv1: nn::Conv2D,
    bn1: nn::BatchNorm,
    conv2: nn::Conv2D,
    bn2: nn::BatchNorm,
    shortcut: Option<(nn::Conv2D, nn::BatchNorm)>,
}

impl BasicBlock {
    fn new(
        vs: &nn::Path,
        in_channels: i64,
        out_channels: i64,
        stride: i64,
    ) -> Self {
        let conv1 = nn::conv2d(
            vs / "conv1",
            in_channels,
            out_channels,
            3,
            nn::ConvConfig {
                stride,
                padding: 1,
                bias: false,
                ..Default::default()
            },
        );
        let bn1 = nn::batch_norm2d(vs / "bn1", out_channels, Default::default());
        
        let conv2 = nn::conv2d(
            vs / "conv2",
            out_channels,
            out_channels,
            3,
            nn::ConvConfig {
                stride: 1,
                padding: 1,
                bias: false,
                ..Default::default()
            },
        );
        let bn2 = nn::batch_norm2d(vs / "bn2", out_channels, Default::default());
        
        // Shortcut connection
        let shortcut = if stride != 1 || in_channels != out_channels {
            let conv = nn::conv2d(
                vs / "shortcut" / "0",
                in_channels,
                out_channels,
                1,
                nn::ConvConfig {
                    stride,
                    bias: false,
                    ..Default::default()
                },
            );
            let bn = nn::batch_norm2d(
                vs / "shortcut" / "1",
                out_channels,
                Default::default(),
            );
            Some((conv, bn))
        } else {
            None
        };
        
        Self {
            conv1,
            bn1,
            conv2,
            bn2,
            shortcut,
        }
    }
}

impl ModuleT for BasicBlock {
    fn forward_t(&self, xs: &Tensor, train: bool) -> Tensor {
        let identity = xs.shallow_clone();
        
        let out = xs.apply(&self.conv1);
        let out = out.apply_t(&self.bn1, train);
        let out = out.relu();
        
        let out = out.apply(&self.conv2);
        let out = out.apply_t(&self.bn2, train);
        
        let out = if let Some((ref conv, ref bn)) = self.shortcut {
            let shortcut_out = identity.apply(conv).apply_t(bn, train);
            out + shortcut_out
        } else {
            out + identity
        };
        
        out.relu()
    }
}

/// ResNet-18 architecture
pub struct ResNet18 {
    conv1: nn::Conv2D,
    bn1: nn::BatchNorm,
    layer1: Vec<BasicBlock>,
    layer2: Vec<BasicBlock>,
    layer3: Vec<BasicBlock>,
    layer4: Vec<BasicBlock>,
    fc: nn::Linear,
}

impl ResNet18 {
    pub fn new(vs: &nn::Path, num_classes: i64) -> Self {
        // Initial convolution
        let conv1 = nn::conv2d(
            vs / "conv1",
            3,
            64,
            7,
            nn::ConvConfig {
                stride: 2,
                padding: 3,
                bias: false,
                ..Default::default()
            },
        );
        let bn1 = nn::batch_norm2d(vs / "bn1", 64, Default::default());
        
        // ResNet layers (each with 2 blocks for ResNet-18)
        let layer1 = vec![
            BasicBlock::new(&(vs / "layer1" / "0"), 64, 64, 1),
            BasicBlock::new(&(vs / "layer1" / "1"), 64, 64, 1),
        ];
        
        let layer2 = vec![
            BasicBlock::new(&(vs / "layer2" / "0"), 64, 128, 2),
            BasicBlock::new(&(vs / "layer2" / "1"), 128, 128, 1),
        ];
        
        let layer3 = vec![
            BasicBlock::new(&(vs / "layer3" / "0"), 128, 256, 2),
            BasicBlock::new(&(vs / "layer3" / "1"), 256, 256, 1),
        ];
        
        let layer4 = vec![
            BasicBlock::new(&(vs / "layer4" / "0"), 256, 512, 2),
            BasicBlock::new(&(vs / "layer4" / "1"), 512, 512, 1),
        ];
        
        // Final fully connected layer
        let fc = nn::linear(vs / "fc", 512, num_classes, Default::default());
        
        Self {
            conv1,
            bn1,
            layer1,
            layer2,
            layer3,
            layer4,
            fc,
        }
    }
    
    pub fn forward_t(&self, xs: &Tensor, train: bool) -> Tensor {
        // Initial conv + bn + relu + maxpool
        let out = xs.apply(&self.conv1);
        let out = out.apply_t(&self.bn1, train);
        let out = out.relu();
        let out = out.max_pool2d(&[3, 3], &[2, 2], &[1, 1], &[1, 1], false);
        
        // Layer 1
        let mut out = out;
        for block in &self.layer1 {
            out = block.forward_t(&out, train);
        }
        
        // Layer 2
        for block in &self.layer2 {
            out = block.forward_t(&out, train);
        }
        
        // Layer 3
        for block in &self.layer3 {
            out = block.forward_t(&out, train);
        }
        
        // Layer 4
        for block in &self.layer4 {
            out = block.forward_t(&out, train);
        }
        
        // Global average pooling
        let out = out.adaptive_avg_pool2d(&[1, 1]);
        let out = out.flat_view();
        
        // Final FC layer
        out.apply(&self.fc)
    }
}

impl Model for ResNet18 {
    fn forward(&self, xs: &Tensor) -> Tensor {
        self.forward_t(xs, false)
    }
    
    fn num_parameters(&self) -> i64 {
        // ResNet-18 has approximately 11.7M parameters
        11_700_000
    }
}

