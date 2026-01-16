//! Training Operations
//!
//! Loss functions and optimizers for neural network training.
//! All operations run on GPU for efficient backpropagation and parameter updates.
//!
//! ## Loss Functions (7)
//!
//! 1. **CrossEntropy** - Multi-class classification
//! 2. **MSE** - Mean Squared Error (regression)
//! 3. **MAE** - Mean Absolute Error
//! 4. **Huber** - Robust regression loss
//! 5. **BCE** - Binary Cross-Entropy
//! 6. **Focal** - Handles class imbalance
//! 7. **Dice** - Segmentation tasks
//!
//! ## Optimizers (6)
//!
//! 1. **SGD** - Stochastic Gradient Descent
//! 2. **Adam** - Adaptive Moment Estimation
//! 3. **RMSprop** - Root Mean Square Propagation
//! 4. **Adagrad** - Adaptive Gradient
//! 5. **NAdam** - Nesterov-accelerated Adam
//! 6. **Adadelta** - Adaptive Delta
//!
//! ## Deep Debt Compliance
//!
//! - ✅ **Runtime Configuration**: Reduction modes, learning rates configurable
//! - ✅ **No Hardcoding**: All parameters passed at runtime
//! - ✅ **Pure Rust**: Zero unsafe code

// Include the implementation files
// These contain `impl WgpuExecutor { ... }` blocks
// that extend the executor with training operations
mod loss_functions;
mod optimizers;

// No re-exports needed - the methods are added directly to WgpuExecutor
// via the impl blocks in each file
