//! Recurrent Neural Networks (RNN/LSTM/GRU)
//!
//! **Week 4 Implementation**: Sequence modeling operations for speech, video, NLP
//!
//! ## Operations (8/8)
//!
//! 1. **RNNCell** - Basic recurrent cell (Elman network)
//! 2. **LSTMCell** - Long Short-Term Memory cell (forget gates)
//! 3. **GRUCell** - Gated Recurrent Unit (simplified LSTM)
//! 4. **LSTMLayer** - Full LSTM layer with sequence processing
//! 5. **GRULayer** - Full GRU layer with sequence processing
//! 6. **BidirectionalRNN** - Forward + backward RNN
//! 7. **StackedLSTM** - Stacked LSTM layers
//! 8. **RecurrentDropout** - RNN-specific dropout (preserves temporal consistency)
//!
//! ## Philosophy
//!
//! - ✅ **Pure Rust**: No unsafe code, vendor-agnostic
//! - ✅ **Memory Efficient**: Optimized hidden state management
//! - ✅ **Batched**: Parallel sequence processing
//! - ✅ **Adaptive**: Uses adaptive optimization system
//!
//! ## Impact
//!
//! **Enables Sequence Modeling**:
//! - Speech recognition (ASR)
//! - Machine translation (seq2seq)
//! - Video processing (temporal features)
//! - Time series forecasting
//! - Music generation

mod rnn;
mod lstm;
mod gru;
mod architectures;
mod dropout;

// Re-export all public types for backward compatibility
pub use rnn::RNNCell;
pub use lstm::{LSTMCell, LSTMLayer};
pub use gru::{GRUCell, GRULayer};
pub use architectures::{BidirectionalRNN, StackedLSTM};
pub use dropout::RecurrentDropout;

// Glob re-exports for test compatibility
pub use rnn::*;
pub use lstm::*;
pub use gru::*;
pub use architectures::*;
pub use dropout::*;
