//! Operations module - Pure WGSL operations
//!
//! **Pure WGSL Architecture**:
//! - WGSL shaders ONLY (no CPU code!)
//! - wgpu handles execution on any device
//! - Single implementation per operation
//! - Zero duplication

// NPU Bridge - Phase 3 unified API (Tensor ↔ NPU conversion)
pub mod npu_bridge;

// Neuromorphic operations (NPU/GPU/CPU universal)
pub mod sparse_matmul_quantized;

// Attention mechanisms (Phase 4 - Universal Compute)
pub mod attention;
pub mod mha;
pub mod causal_attn;
pub mod sparse_attn;


// Homomorphic encryption operations (FHE - GPU accelerated)
pub mod fhe_and;
pub mod fhe_or;
pub mod fhe_poly_add;
pub mod fhe_poly_mul;
pub mod fhe_poly_sub;
pub mod fhe_xor;

// Activation operations
pub mod elu;
pub mod gelu;
pub mod hardswish;
pub mod leaky_relu;
pub mod mish;
pub mod relu;
pub mod selu;
pub mod sigmoid;
pub mod softmax;
pub mod softplus;
pub mod swish;
pub mod tanh;

// Element-wise operations
pub mod abs;
pub mod add;
pub mod clamp;
pub mod div;
pub mod exp;
pub mod log;
pub mod mul;
pub mod neg;
pub mod pow;
pub mod reciprocal;
pub mod sign;
pub mod sqrt;
pub mod sub;

// Comparison operations
pub mod eq;
pub mod gt;
pub mod lt;

// Trigonometric operations
pub mod cos;
pub mod sin;

// Rounding operations
pub mod ceil;
pub mod floor;
pub mod round;

// Reduction operations
pub mod max;
pub mod mean;
pub mod min;
pub mod norm;
pub mod prod;
pub mod std;
pub mod sum;
pub mod variance;

// Shape operations
pub mod concat;
pub mod pad;
pub mod slice;
pub mod transpose;

// Selection and manipulation operations
pub mod argmax;
pub mod squeeze;
pub mod unsqueeze;
pub mod where_op;

// Neuromorphic operations
pub mod adaptive_avgpool2d;
pub mod adaptive_maxpool2d;
pub mod avgpool2d;
pub mod batch_norm;
pub mod cast;
pub mod conv2d;
pub mod dropout;
pub mod embedding;
pub mod gather;
pub mod global_maxpool;
pub mod layer_norm;
pub mod matmul;
pub mod maxpool2d;
pub mod scatter;
pub mod topk;

// Utility operations
pub mod broadcast;
pub mod cumsum;
pub mod fill;
pub mod flip;
pub mod one_hot;
pub mod repeat;

// Loss functions
pub mod binary_cross_entropy;
pub mod cross_entropy;
pub mod dice_loss;
pub mod focal_loss;
pub mod huber_loss;
pub mod l1_loss;
pub mod mae_loss;
pub mod mse_loss;

// Advanced normalization
pub mod groupnorm;
pub mod instancenorm;
pub mod rmsnorm;

// Convolution variants
pub mod conv1d;
pub mod conv3d;
pub mod depthwise_conv2d;
pub mod transposed_conv2d;

// Advanced operations
pub mod batch_matmul;
pub mod dotproduct;
pub mod filter;
pub mod global_avgpool;
pub mod map;
pub mod matmul_tiled;
pub mod reduce;
pub mod scan;
pub mod split;

// Optimizers
pub mod adadelta;
pub mod adagrad;
pub mod adam;
pub mod nadam;
pub mod rmsprop;
pub mod sgd;

// Attention mechanisms
pub mod alibi_position;
pub mod causal_attention;
pub mod cross_attention;
pub mod flash_attention;
pub mod grouped_query_attention;
pub mod local_attention;
pub mod multi_head_attention;
pub mod rotary_embedding;
pub mod scaled_dot_product_attention;
pub mod sparse_attention;

// RNN/LSTM cells
pub mod bi_lstm;
pub mod gru_cell;
pub mod lstm_cell;
pub mod rnn_cell;

// Advanced activations
pub mod glu;
pub mod prelu;
pub mod softsign;
pub mod tanhshrink;

// Advanced convolutions
pub mod avgpool3d;
pub mod circular_pad2d;
pub mod dilated_conv2d;
pub mod grouped_conv2d;
pub mod maxpool3d;
pub mod reflection_pad2d;
pub mod replication_pad2d;
pub mod separable_conv2d;

// Advanced loss functions
pub mod contrastive_loss;
pub mod cosine_embedding_loss;
pub mod hinge_loss;
pub mod kl_divergence;
pub mod margin_ranking_loss;
pub mod multi_margin_loss;
pub mod triplet_loss;

// Advanced normalization
pub mod adaptive_instance_norm;
pub mod filter_response_norm;
pub mod local_response_norm;
pub mod spectral_normalization;
pub mod weight_normalization;

// Advanced utilities
pub mod affine_grid;
pub mod bincount;
pub mod bucketize;
pub mod cdist;
pub mod diag;
pub mod fold;
pub mod grid_sample;
pub mod histc;
pub mod index_select;
pub mod interpolate;
pub mod logsumexp;
pub mod masked_select;
pub mod nonzero;
pub mod normalize;
pub mod pdist;
pub mod renorm;
pub mod searchsorted;
pub mod trace;
pub mod tril;
pub mod triu;
pub mod unfold;
pub mod unique;

// Tensor manipulation
pub mod chunk;
pub mod expand;
pub mod flatten;
pub mod movedim;
pub mod narrow;
pub mod permute;
pub mod repeat_interleave;
pub mod stack;
pub mod tensor_split;
pub mod tile;

// Advanced matrix operations
pub mod cross_product;
pub mod determinant;
pub mod matrix_inverse;
pub mod matrix_power;
pub mod matrix_rank;
pub mod outer_product;
pub mod tensor_dot;

// Gradient operations
pub mod clip_grad_norm;
pub mod clip_grad_value;

// Quantization
pub mod dequantize;
pub mod fake_quantize;
pub mod quantize;

// Object detection
pub mod anchor_generator;
pub mod bbox_transform;
pub mod box_iou;
pub mod nms;
pub mod roi_align;
pub mod roi_pool;
pub mod soft_nms;

// Advanced pooling
pub mod adaptive_avg_pool1d;
pub mod adaptive_max_pool1d;
pub mod fractional_max_pool2d;
pub mod lp_pool2d;

// Enhanced losses
pub mod focal_loss_v2;
pub mod smooth_l1_loss;

// Utility operations (original)
pub mod channel_shuffle;
pub mod layer_scale;
pub mod masked_fill;
pub mod pixel_shuffle;
pub mod put;
pub mod reshape;
pub mod roll;
pub mod take;
pub mod upsample;

// Graph Neural Networks (Category 13)
pub mod edge_conv;
pub mod gat_conv;
pub mod gcn_conv;
pub mod gin_conv;
pub mod global_pooling;
pub mod graph_batch_norm;
pub mod graph_conv;
pub mod graph_norm;
pub mod message_passing;
pub mod sage_conv;

// Advanced Optimizers & Learning (Category 14)
pub mod adafactor;
pub mod adamw;
pub mod lamb;
pub mod lookahead;
pub mod radam;
// pub mod nadam; // Already exists above
pub mod adabound;
pub mod cyclical_lr;
pub mod onecycle;
pub mod sgdw;

// Audio/Signal Processing (Category 15)
pub mod griffin_lim;
pub mod istft;
pub mod mel_scale;
pub mod mfcc;
pub mod pitch_shift;
pub mod spectral_norm_1d;
pub mod spectrogram;
pub mod stft;
pub mod time_stretch;
pub mod window_function;

// Advanced Sampling & Augmentation (Category 16)
pub mod color_jitter;
pub mod cutmix;
pub mod elastic_transform;
pub mod grid_mask;
pub mod mixup;
pub mod mosaic;
pub mod random_affine;
pub mod random_crop;
pub mod random_erasing;
pub mod random_perspective;

// Specialized Losses & Metrics (Category 17)
pub mod psnr;
pub mod ssim;
// pub mod dice_loss; // Already exists above
pub mod center_loss;
pub mod chamfer_distance;
pub mod earth_mover_distance;
pub mod iou_loss;
pub mod perceptual_loss;
pub mod tversky_loss;
pub mod wasserstein_loss;

pub use abs::Abs;
pub use add::Add;
pub use argmax::Argmax;
pub use batch_norm::BatchNorm;
pub use ceil::Ceil;
pub use clamp::Clamp;
pub use concat::Concat;
pub use cos::Cos;
pub use div::Div;
pub use dropout::Dropout;
pub use elu::ELU;
pub use eq::Eq;
pub use exp::Exp;
pub use floor::Floor;
pub use gather::Gather;
pub use gelu::GELU;
pub use gt::Gt;
pub use hardswish::HardSwish;
pub use layer_norm::LayerNorm;
pub use leaky_relu::LeakyReLU;
pub use log::Log;
pub use lt::Lt;
pub use max::Max;
pub use mean::Mean;
pub use min::Min;
pub use mish::Mish;
pub use mul::Mul;
pub use neg::Neg;
pub use norm::Norm;
pub use pad::Pad;
pub use pow::Pow;
pub use prod::Prod;
pub use reciprocal::Reciprocal;
/// Re-exports
pub use relu::ReLU;
pub use round::Round;
pub use scatter::Scatter;
pub use selu::SELU;
pub use sigmoid::Sigmoid;
pub use sign::Sign;
pub use sin::Sin;
pub use slice::Slice;
pub use softmax::Softmax;
pub use sqrt::Sqrt;
pub use squeeze::Squeeze;
pub use std::Std;
pub use sub::Sub;
pub use sum::Sum;
pub use swish::Swish;
pub use tanh::Tanh;
pub use transpose::Transpose;
pub use unsqueeze::Unsqueeze;
pub use variance::Variance;
pub use where_op::Where;
// topk exports functions, not struct
pub use adaptive_avgpool2d::AdaptiveAvgPool2D;
pub use adaptive_maxpool2d::AdaptiveMaxPool2D;
pub use avgpool2d::AvgPool2D;
pub use batch_matmul::BatchMatMul;
pub use binary_cross_entropy::BinaryCrossEntropy;
pub use broadcast::Broadcast;
pub use cast::Cast;
pub use conv1d::Conv1D;
pub use conv2d::Conv2D;
pub use conv3d::Conv3D;
pub use cross_entropy::CrossEntropy;
pub use cumsum::Cumsum;
pub use depthwise_conv2d::DepthwiseConv2D;
pub use embedding::Embedding;
pub use fill::Fill;
pub use flip::Flip;
pub use focal_loss::FocalLoss;
pub use global_avgpool::GlobalAvgPool;
pub use global_maxpool::GlobalMaxPool;
pub use groupnorm::GroupNorm;
pub use instancenorm::InstanceNorm;
pub use l1_loss::L1Loss;
pub use matmul::MatMul;
pub use maxpool2d::MaxPool2D;
pub use mse_loss::MseLoss;
pub use one_hot::OneHot;
pub use repeat::Repeat;
pub use rmsnorm::RMSNorm;
pub use softplus::Softplus;
pub use split::Split;
pub use transposed_conv2d::TransposedConv2D;
// pub use dice_loss::DiceLoss; // Function-only module
pub use huber_loss::HuberLoss;
pub use rmsprop::RMSprop;
pub use sgd::SGD;
// pub use nadam::Nadam; // Function-only module with state struct
pub use adadelta::AdaDelta;
pub use adagrad::AdaGrad;
pub use adam::Adam;
pub use dotproduct::DotProduct;
pub use filter::{Filter, FilterOperation};
pub use mae_loss::MAELoss;
pub use map::{Map, MapOperation};
pub use matmul_tiled::MatmulTiled;
pub use reduce::{Reduce, ReduceOperation};
pub use scan::Scan;
// Note: Reshape is already defined in tensor.rs
