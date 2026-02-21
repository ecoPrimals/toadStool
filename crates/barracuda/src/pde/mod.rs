//! Partial Differential Equation Solvers
//!
//! This module provides numerical methods for solving PDEs commonly
//! encountered in scientific computing and physics simulations.
//!
//! # Solvers
//!
//! - **Crank-Nicolson**: Implicit 2nd-order scheme for diffusion equations
//!
//! # Applications
//!
//! - **Two-Temperature Model (TTM)**: Ultrafast laser heating
//! - **Heat diffusion**: Thermal transport simulations
//! - **Schrödinger equation**: Time-dependent quantum mechanics
//!
//! # References
//!
//! - Numerical Recipes, §19.2
//! - J. Crank & P. Nicolson (1947), "A practical method for numerical
//!   evaluation of solutions of partial differential equations"

pub mod crank_nicolson;

pub use crank_nicolson::{
    crank_nicolson_step, CrankNicolson1D, CrankNicolsonConfig, HeatEquation1D,
    WGSL_CRANK_NICOLSON_F64,
};
