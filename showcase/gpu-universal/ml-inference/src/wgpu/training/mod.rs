//! Training operations
//!
//! Loss functions and optimizers for neural network training.
//! Full GPU execution for efficient backpropagation and parameter updates.
//!
//! ## Module structure
//!
//! - `loss_functions` - CrossEntropy, MSE, MAE, Huber, BCE, Focal, Dice
//! - `optimizers_momentum` - Adam, SGD, RMSprop
//! - `optimizers_adaptive` - AdaGrad, Nadam, AdaDelta

mod loss_functions;
mod optimizers_adaptive;
mod optimizers_momentum;
