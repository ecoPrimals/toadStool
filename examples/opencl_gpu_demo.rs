// SPDX-License-Identifier: AGPL-3.0-or-later
//! DEPRECATED S198: OpenCL GPU demo
//!
//! OpenCL support has been removed from ToadStool. GPU compute dispatch
//! is handled by **barraCuda** (single-GPU) and **coralReef** (multi-GPU)
//! via capability-based IPC. ToadStool's in-tree GPU backends are now
//! **wgpu** (WebGPU, universal default) and **Vulkan** (opt-in).
//!
//! See: `examples/real_gpu_pool.rs` for the wgpu-based GPU demo.

fn main() {
    eprintln!("DEPRECATED S198: OpenCL removed from ToadStool.");
    eprintln!("GPU compute dispatch → barraCuda / coralReef via IPC.");
    eprintln!("In-tree GPU: wgpu (default) + Vulkan (opt-in).");
    eprintln!();
    eprintln!("See: examples/real_gpu_pool.rs for the wgpu-based GPU demo.");
    std::process::exit(0);
}
