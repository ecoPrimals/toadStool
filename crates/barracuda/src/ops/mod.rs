//! Operations module - Pure WGSL operations
//!
//! **Pure WGSL Architecture**:
//! - WGSL shaders ONLY (no CPU code!)
//! - wgpu handles execution on any device
//! - Single implementation per operation
//! - Zero duplication

// Activation operations
pub mod relu;
pub mod gelu;
pub mod sigmoid;
pub mod tanh;
pub mod softmax;
pub mod swish;
pub mod elu;
pub mod mish;
pub mod selu;
pub mod leaky_relu;
pub mod hardswish;
pub mod softplus;

// Element-wise operations
pub mod add;
pub mod sub;
pub mod mul;
pub mod div;
pub mod abs;
pub mod sqrt;
pub mod exp;
pub mod pow;
pub mod clamp;
pub mod log;
pub mod neg;
pub mod reciprocal;
pub mod sign;

// Comparison operations
pub mod eq;
pub mod gt;
pub mod lt;

// Trigonometric operations
pub mod cos;
pub mod sin;

// Rounding operations
pub mod floor;
pub mod ceil;
pub mod round;

// Reduction operations
pub mod sum;
pub mod mean;
pub mod max;
pub mod min;
pub mod variance;
pub mod std;
pub mod norm;
pub mod prod;

// Shape operations
pub mod transpose;
pub mod concat;
pub mod slice;
pub mod pad;

// Selection and manipulation operations
pub mod argmax;
pub mod squeeze;
pub mod unsqueeze;
pub mod where_op;

// Neuromorphic operations
pub mod layer_norm;
pub mod batch_norm;
pub mod dropout;
pub mod gather;
pub mod scatter;
pub mod topk;
pub mod cast;
pub mod maxpool2d;
pub mod avgpool2d;
pub mod adaptive_avgpool2d;
pub mod adaptive_maxpool2d;
pub mod global_maxpool;
pub mod matmul;
pub mod conv2d;
pub mod embedding;

// Utility operations
pub mod one_hot;
pub mod broadcast;
pub mod fill;
pub mod repeat;
pub mod flip;
pub mod cumsum;

// Loss functions
pub mod mse_loss;
pub mod cross_entropy;
pub mod binary_cross_entropy;
pub mod l1_loss;
pub mod focal_loss;
pub mod dice_loss;
pub mod huber_loss;
pub mod mae_loss;

// Advanced normalization
pub mod rmsnorm;
pub mod instancenorm;
pub mod groupnorm;

// Convolution variants
pub mod conv1d;
pub mod conv3d;
pub mod depthwise_conv2d;
pub mod transposed_conv2d;

// Advanced operations
pub mod batch_matmul;
pub mod global_avgpool;
pub mod split;
pub mod dotproduct;
pub mod map;
pub mod filter;
pub mod scan;
pub mod reduce;
pub mod matmul_tiled;

// Optimizers
pub mod sgd;
pub mod rmsprop;
pub mod nadam;
pub mod adam;
pub mod adagrad;
pub mod adadelta;

// Attention mechanisms
pub mod scaled_dot_product_attention;
pub mod multi_head_attention;

// RNN/LSTM cells
pub mod lstm_cell;
pub mod gru_cell;
pub mod rnn_cell;
pub mod bi_lstm;

// Advanced activations
pub mod prelu;
pub mod glu;
pub mod softsign;
pub mod tanhshrink;

// Utility operations (extended)
pub mod layer_scale;
pub mod channel_shuffle;
pub mod pixel_shuffle;
pub mod upsample;
pub mod take;
pub mod put;
pub mod masked_fill;
pub mod roll;
pub mod reshape;

/// Re-exports
pub use relu::ReLU;
pub use gelu::GELU;
pub use sigmoid::Sigmoid;
pub use tanh::Tanh;
pub use softmax::Softmax;
pub use swish::Swish;
pub use elu::ELU;
pub use mish::Mish;
pub use selu::SELU;
pub use leaky_relu::LeakyReLU;
pub use hardswish::HardSwish;
pub use add::Add;
pub use sub::Sub;
pub use mul::Mul;
pub use div::Div;
pub use abs::Abs;
pub use sqrt::Sqrt;
pub use exp::Exp;
pub use pow::Pow;
pub use clamp::Clamp;
pub use log::Log;
pub use neg::Neg;
pub use reciprocal::Reciprocal;
pub use sign::Sign;
pub use eq::Eq;
pub use gt::Gt;
pub use lt::Lt;
pub use cos::Cos;
pub use sin::Sin;
pub use floor::Floor;
pub use ceil::Ceil;
pub use round::Round;
pub use sum::Sum;
pub use mean::Mean;
pub use max::Max;
pub use min::Min;
pub use variance::Variance;
pub use std::Std;
pub use norm::Norm;
pub use prod::Prod;
pub use transpose::Transpose;
pub use concat::Concat;
pub use slice::Slice;
pub use pad::Pad;
pub use argmax::Argmax;
pub use squeeze::Squeeze;
pub use unsqueeze::Unsqueeze;
pub use where_op::Where;
pub use layer_norm::LayerNorm;
pub use batch_norm::BatchNorm;
pub use dropout::Dropout;
pub use gather::Gather;
pub use scatter::Scatter;
// topk exports functions, not struct
pub use cast::Cast;
pub use maxpool2d::MaxPool2D;
pub use avgpool2d::AvgPool2D;
pub use matmul::MatMul;
pub use conv2d::Conv2D;
pub use embedding::Embedding;
pub use softplus::Softplus;
pub use one_hot::OneHot;
pub use broadcast::Broadcast;
pub use fill::Fill;
pub use repeat::Repeat;
pub use flip::Flip;
pub use cumsum::Cumsum;
pub use mse_loss::MseLoss;
pub use cross_entropy::CrossEntropy;
pub use binary_cross_entropy::BinaryCrossEntropy;
pub use l1_loss::L1Loss;
pub use rmsnorm::RMSNorm;
pub use instancenorm::InstanceNorm;
pub use groupnorm::GroupNorm;
pub use conv1d::Conv1D;
pub use conv3d::Conv3D;
pub use depthwise_conv2d::DepthwiseConv2D;
pub use transposed_conv2d::TransposedConv2D;
pub use batch_matmul::BatchMatMul;
pub use global_avgpool::GlobalAvgPool;
pub use global_maxpool::GlobalMaxPool;
pub use split::Split;
pub use adaptive_avgpool2d::AdaptiveAvgPool2D;
pub use adaptive_maxpool2d::AdaptiveMaxPool2D;
pub use focal_loss::FocalLoss;
pub use dice_loss::DiceLoss;
pub use huber_loss::HuberLoss;
pub use sgd::SGD;
pub use rmsprop::RMSprop;
pub use nadam::Nadam;
pub use adam::Adam;
pub use adagrad::AdaGrad;
pub use adadelta::AdaDelta;
pub use mae_loss::MAELoss;
pub use dotproduct::DotProduct;
pub use map::{Map, MapOperation};
pub use filter::{Filter, FilterOperation};
pub use scan::Scan;
pub use reduce::{Reduce, ReduceOperation};
pub use matmul_tiled::MatmulTiled;
// Note: Reshape is already defined in tensor.rs
