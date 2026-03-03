// SPDX-License-Identifier: AGPL-3.0-or-later
//! Attention Mechanisms for Transformers
//!
//! **Week 3 Implementation**: Core attention operations for BERT, GPT, LLaMA
//!
//! ## Operations (5/5)
//!
//! 1. **ScaledDotProductAttention** - Q·K^T / √d_k softmax(·)V
//! 2. **MultiHeadAttention** - Parallel attention heads with concat
//! 3. **CausalMask** - Autoregressive masking for GPT-style models
//! 4. **AttentionBias** - Positional and attention biases
//! 5. **FlashAttention** - Memory-efficient attention (O(N) memory vs O(N²))
//!
//! ## Philosophy
//!
//! - ✅ **Pure Rust**: No unsafe code, vendor-agnostic
//! - ✅ **Memory Efficient**: Flash attention for long sequences
//! - ✅ **Batched**: Optimized for parallel execution
//! - ✅ **Adaptive**: Uses adaptive optimization system
//!
//! ## Impact
//!
//! **Enables Production Transformers**:
//! - BERT (bidirectional attention)
//! - GPT (causal/autoregressive attention)
//! - LLaMA (efficient attention + RoPE)
//! - Vision Transformers (ViT)
//! - Multimodal models (CLIP, Flamingo)

mod bias;
mod flash;
mod masks;
mod multi_head;
mod scaled_dot_product;

// Re-export all public types for backward compatibility
pub use bias::AttentionBias;
pub use flash::FlashAttention;
pub use masks::CausalMask;
pub use multi_head::MultiHeadAttention;
pub use scaled_dot_product::ScaledDotProductAttention;

// Glob re-exports for test compatibility
// Modules not yet exporting public items
// pub use scaled_dot_product::*;
// pub use multi_head::*;
// pub use masks::*;
// pub use bias::*;
// pub use flash::*;
