// SPDX-License-Identifier: AGPL-3.0-or-later
//! Safe SPIR-V shader module creation.
//!
//! Wraps wgpu's `ShaderSource::SpirV` behind a validated, safe API.
//! As of wgpu 28, SPIR-V passthrough uses naga validation and no longer
//! requires the unsafe `create_shader_module_spirv` method.

/// Create a wgpu shader module from validated SPIR-V bytes.
///
/// Validates the SPIR-V magic number before passing to wgpu's naga-validated
/// SPIR-V pipeline.
///
/// # Errors
///
/// Returns `Err` if the binary is not valid SPIR-V (wrong magic or too short).
pub fn create_spirv_shader_module(
    device: &wgpu::Device,
    label: &str,
    spirv_binary: &[u8],
) -> Result<wgpu::ShaderModule, &'static str> {
    if spirv_binary.len() < 4 {
        return Err("SPIR-V binary too short");
    }

    let magic = u32::from_le_bytes([
        spirv_binary[0],
        spirv_binary[1],
        spirv_binary[2],
        spirv_binary[3],
    ]);
    if magic != 0x0723_0203 {
        return Err("invalid SPIR-V magic number");
    }

    let spirv_words: Vec<u32> = spirv_binary
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::SpirV(std::borrow::Cow::Borrowed(&spirv_words)),
    });

    Ok(module)
}
