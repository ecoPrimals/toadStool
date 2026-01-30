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

mod scaled_dot_product;
mod multi_head;
mod masks;
mod bias;
mod flash;

// Re-export all public types for backward compatibility
pub use scaled_dot_product::ScaledDotProductAttention;
pub use multi_head::MultiHeadAttention;
pub use masks::CausalMask;
pub use bias::AttentionBias;
pub use flash::FlashAttention;

// Glob re-exports for test compatibility
// Modules not yet exporting public items
// pub use scaled_dot_product::*;
// pub use multi_head::*;
// pub use masks::*;
// pub use bias::*;
// pub use flash::*;
