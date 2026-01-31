//! Operations module - Pure WGSL operations
//!
//! **Pure WGSL Architecture**:
//! - WGSL shaders ONLY (no CPU code!)
//! - wgpu handles execution on any device
//! - Single implementation per operation
//! - Zero duplication

// Neuromorphic operations (NPU/GPU/CPU universal)
pub mod spike_encode;
pub mod spike_decode;
pub mod lif_neuron;
pub mod temporal_pool;

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
pub mod flash_attention;
pub mod causal_attention;
pub mod cross_attention;
pub mod grouped_query_attention;
pub mod rotary_embedding;
pub mod alibi_position;
pub mod local_attention;
pub mod sparse_attention;

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

// Advanced convolutions
pub mod dilated_conv2d;
pub mod grouped_conv2d;
pub mod separable_conv2d;
pub mod avgpool3d;
pub mod maxpool3d;
pub mod reflection_pad2d;
pub mod replication_pad2d;
pub mod circular_pad2d;

// Advanced loss functions
pub mod kl_divergence;
pub mod contrastive_loss;
pub mod triplet_loss;
pub mod hinge_loss;
pub mod cosine_embedding_loss;
pub mod margin_ranking_loss;
pub mod multi_margin_loss;

// Advanced normalization
pub mod weight_normalization;
pub mod spectral_normalization;
pub mod adaptive_instance_norm;
pub mod local_response_norm;
pub mod filter_response_norm;

// Advanced utilities
pub mod interpolate;
pub mod grid_sample;
pub mod affine_grid;
pub mod index_select;
pub mod masked_select;
pub mod nonzero;
pub mod unique;
pub mod bincount;
pub mod unfold;
pub mod fold;
pub mod histc;
pub mod bucketize;
pub mod searchsorted;
pub mod cdist;
pub mod pdist;
pub mod normalize;
pub mod renorm;
pub mod logsumexp;
pub mod trace;
pub mod diag;
pub mod triu;
pub mod tril;

// Tensor manipulation
pub mod stack;
pub mod chunk;
pub mod narrow;
pub mod permute;
pub mod expand;
pub mod flatten;
pub mod tensor_split;
pub mod movedim;
pub mod repeat_interleave;
pub mod tile;

// Advanced matrix operations
pub mod matrix_inverse;
pub mod determinant;
pub mod matrix_rank;
pub mod matrix_power;
pub mod outer_product;
pub mod cross_product;
pub mod tensor_dot;

// Gradient operations
pub mod clip_grad_norm;
pub mod clip_grad_value;

// Quantization
pub mod quantize;
pub mod dequantize;
pub mod fake_quantize;

// Object detection
pub mod nms;
pub mod soft_nms;
pub mod bbox_transform;
pub mod box_iou;
pub mod anchor_generator;
pub mod roi_pool;
pub mod roi_align;

// Advanced pooling
pub mod adaptive_max_pool1d;
pub mod adaptive_avg_pool1d;
pub mod fractional_max_pool2d;
pub mod lp_pool2d;

// Enhanced losses
pub mod focal_loss_v2;
pub mod smooth_l1_loss;

// Utility operations (original)
pub mod layer_scale;
pub mod channel_shuffle;
pub mod pixel_shuffle;
pub mod upsample;
pub mod take;
pub mod put;
pub mod masked_fill;
pub mod roll;
pub mod reshape;

// Graph Neural Networks (Category 13)
pub mod graph_conv;
pub mod gcn_conv;
pub mod gat_conv;
pub mod sage_conv;
pub mod gin_conv;
pub mod edge_conv;
pub mod message_passing;
pub mod global_pooling;
pub mod graph_norm;
pub mod graph_batch_norm;

// Advanced Optimizers & Learning (Category 14)
pub mod adamw;
pub mod radam;
pub mod lookahead;
pub mod lamb;
pub mod adafactor;
// pub mod nadam; // Already exists above
pub mod adabound;
pub mod sgdw;
pub mod cyclical_lr;
pub mod onecycle;

// Audio/Signal Processing (Category 15)
pub mod stft;
pub mod istft;
pub mod mel_scale;
pub mod mfcc;
pub mod spectrogram;
pub mod griffin_lim;
pub mod time_stretch;
pub mod pitch_shift;
pub mod window_function;
pub mod spectral_norm_1d;

// Advanced Sampling & Augmentation (Category 16)
pub mod random_crop;
pub mod random_erasing;
pub mod cutmix;
pub mod mixup;
pub mod mosaic;
pub mod random_affine;
pub mod color_jitter;
pub mod random_perspective;
pub mod elastic_transform;
pub mod grid_mask;

// Specialized Losses & Metrics (Category 17)
pub mod ssim;
pub mod psnr;
// pub mod dice_loss; // Already exists above
pub mod iou_loss;
pub mod tversky_loss;
pub mod wasserstein_loss;
pub mod chamfer_distance;
pub mod earth_mover_distance;
pub mod perceptual_loss;
pub mod center_loss;

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
// pub use dice_loss::DiceLoss; // Function-only module
pub use huber_loss::HuberLoss;
pub use sgd::SGD;
pub use rmsprop::RMSprop;
// pub use nadam::Nadam; // Function-only module with state struct
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
