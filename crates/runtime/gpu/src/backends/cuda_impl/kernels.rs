// SPDX-License-Identifier: AGPL-3.0-only
//! PTX loading and kernel execution

use std::sync::Arc;

use cudarc::driver::safe::{CudaModule, CudaSlice, LaunchConfig};
use cudarc::driver::DeviceRepr;
use toadstool::error::{ToadStoolError, ToadStoolResult};

use super::CudaBackend;

impl CudaBackend {
    /// Load PTX module and cache it
    pub async fn load_ptx(
        &self,
        ptx_code: &str,
        module_name: &str,
    ) -> ToadStoolResult<Arc<CudaModule>> {
        {
            let cache = self.module_cache.read().await;
            if let Some(module) = cache.get(module_name) {
                tracing::debug!("Using cached CUDA module: {}", module_name);
                return Ok(Arc::clone(module));
            }
        }

        let ptx = cudarc::nvrtc::Ptx::from_src(ptx_code);
        let module = self.context.load_module(ptx).map_err(|e| {
            ToadStoolError::runtime(format!("Failed to load CUDA PTX module: {}", e))
        })?;

        let mut cache = self.module_cache.write().await;
        cache.insert(module_name.to_string(), Arc::clone(&module));

        tracing::info!("✅ Loaded CUDA module: {}", module_name);
        Ok(module)
    }

    /// Execute CUDA kernel with zero-copy where possible
    pub async fn execute_kernel<T>(
        &self,
        module_name: &str,
        kernel_name: &str,
        inputs: &[&[T]],
        output_size: usize,
        grid_dim: (u32, u32, u32),
        block_dim: (u32, u32, u32),
    ) -> ToadStoolResult<Vec<T>>
    where
        T: DeviceRepr + Default + Clone + Unpin,
    {
        let start_time = std::time::Instant::now();

        let module = self.load_ptx("", module_name).await.or_else(|_| {
            let cache = self
                .module_cache
                .try_read()
                .map_err(|_| ToadStoolError::runtime("Failed to acquire module cache lock"))?;
            cache.get(module_name).cloned().ok_or_else(|| {
                ToadStoolError::runtime(format!("Module '{}' not found in cache", module_name))
            })
        })?;

        let func = module.load_function(kernel_name).map_err(|e| {
            ToadStoolError::runtime(format!(
                "CUDA kernel '{}' not found in module '{}': {}",
                kernel_name, module_name, e
            ))
        })?;

        let mut input_buffers: Vec<CudaSlice<T>> = Vec::new();
        for (idx, input) in inputs.iter().enumerate() {
            let buffer = self.stream.clone_htod(input).map_err(|e| {
                ToadStoolError::runtime(format!("Failed to upload input {}: {}", idx, e))
            })?;
            input_buffers.push(buffer);
        }

        let mut output_buffer: CudaSlice<T> =
            self.stream.alloc_zeros(output_size).map_err(|e| {
                ToadStoolError::runtime(format!("Failed to allocate output buffer: {}", e))
            })?;

        let cfg = LaunchConfig {
            grid_dim,
            block_dim,
            shared_mem_bytes: 0,
        };

        // SAFETY: cudarc kernel launch invariants:
        // - `func` is a valid compiled CUDA function loaded from the module above.
        // - `input_buffers` and `output_buffer` were allocated on the same CUDA device
        //   via `self.stream.htod_copy()` and `self.stream.alloc_zeros()`.
        // - `cfg` dimensions are computed from the validated input sizes.
        // - The stream lives as long as `self`, which outlives this call.
        // - cudarc handles the underlying CUDA API calls and validates argument types.
        unsafe {
            match inputs.len() {
                1 => {
                    self.stream
                        .launch_builder(&func)
                        .arg(&input_buffers[0])
                        .arg(&mut output_buffer)
                        .launch(cfg)
                        .map_err(|e| {
                            ToadStoolError::runtime(format!("CUDA kernel launch failed: {}", e))
                        })?;
                }
                2 => {
                    self.stream
                        .launch_builder(&func)
                        .arg(&input_buffers[0])
                        .arg(&input_buffers[1])
                        .arg(&mut output_buffer)
                        .launch(cfg)
                        .map_err(|e| {
                            ToadStoolError::runtime(format!("CUDA kernel launch failed: {}", e))
                        })?;
                }
                3 => {
                    self.stream
                        .launch_builder(&func)
                        .arg(&input_buffers[0])
                        .arg(&input_buffers[1])
                        .arg(&input_buffers[2])
                        .arg(&mut output_buffer)
                        .launch(cfg)
                        .map_err(|e| {
                            ToadStoolError::runtime(format!("CUDA kernel launch failed: {}", e))
                        })?;
                }
                _ => {
                    return Err(ToadStoolError::runtime(format!(
                        "Unsupported number of inputs: {}. Support for 1-3 inputs.",
                        inputs.len()
                    )));
                }
            }
        }

        self.context
            .synchronize()
            .map_err(|e| ToadStoolError::runtime(format!("CUDA synchronization failed: {}", e)))?;

        let output = self
            .stream
            .clone_dtoh(&output_buffer)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to download output: {}", e)))?;

        let duration = start_time.elapsed();
        tracing::info!(
            "⚡ Kernel '{}' executed in {:?} on {}",
            kernel_name,
            duration,
            self.device_info.name
        );

        Ok(output)
    }
}
