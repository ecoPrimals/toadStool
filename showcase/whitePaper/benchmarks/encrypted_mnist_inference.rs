// ❌ DEPRECATED: This benchmark is superseded by encrypted_mnist_pipeline.rs
//
// The simulate_fhe_matmul_time() function this file depends on was removed
// in favor of REAL BarraCUDA FHE operations.
//
// ✅ USE INSTEAD: encrypted_mnist_pipeline.rs
// - Real BarraCUDA FHE operations (FhePolyAdd, FhePolyMul, FheNtt, FheIntt)
// - Real GPU/NPU execution (zero simulation)
// - Complete training + inference pipeline
// - Full power measurement integration
//
// This file is kept for historical reference but will not compile.

fn main() {
    eprintln!("❌ This benchmark is deprecated!");
    eprintln!("✅ Use encrypted_mnist_pipeline instead:");
    eprintln!("   cargo run --bin encrypted_mnist_pipeline");
    eprintln!();
    eprintln!("The new pipeline uses REAL BarraCUDA FHE operations,");
    eprintln!("not simulations. See encrypted_mnist_pipeline.rs for details.");
    std::process::exit(1);
}
