//! Normalization operations
//!
//! Softmax, LayerNorm, BatchNorm, GroupNorm, InstanceNorm, RMSNorm, etc.
//! Complex multi-pass GPU operations for neural network normalization.

mod batch_norm;
mod group_norm;
mod instance_norm;
mod layer_norm;
mod layer_norm_2dispatch;
mod layer_norm_fused;
mod layer_norm_fused_v2;
mod layer_norm_optimized;
mod rms_norm;
mod softmax;
