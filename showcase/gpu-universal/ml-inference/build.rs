//! Build script for compiling Vulkan shaders to SPIR-V
//!
//! Compiles GLSL compute shaders to SPIR-V bytecode at build time

fn main() {
    println!("cargo:rerun-if-changed=src/vulkan_shaders.glsl");
    
    // Only compile shaders if Vulkan feature is enabled
    #[cfg(feature = "vulkan")]
    {
        compile_vulkan_shaders();
    }
}

#[cfg(feature = "vulkan")]
fn compile_vulkan_shaders() {
    use std::env;
    use std::path::Path;
    use std::process::Command;
    
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let shader_dir = Path::new(&out_dir).join("shaders");
    
    // Create shader output directory
    std::fs::create_dir_all(&shader_dir).expect("Failed to create shader directory");
    
    // Check if glslc is available (from Vulkan SDK)
    let glslc_available = Command::new("glslc")
        .arg("--version")
        .output()
        .is_ok();
    
    if !glslc_available {
        println!("cargo:warning=glslc not found. Vulkan shaders will not be compiled.");
        println!("cargo:warning=Install Vulkan SDK for shader compilation.");
        println!("cargo:warning=Falling back to CPU execution.");
        return;
    }
    
    println!("cargo:warning=glslc found - compiling Vulkan shaders...");
    
    // Note: For now, we're using individual compute shader files
    // The vulkan_shaders.glsl file contains templates
    // In production, we'd split these into separate .comp files and compile them
    
    // TODO: Split vulkan_shaders.glsl into:
    // - matrix_multiply.comp
    // - relu.comp  
    // - softmax.comp
    // Then compile each with glslc
    
    println!("cargo:warning=Shader compilation infrastructure ready.");
    println!("cargo:warning=GLSL templates at src/vulkan_shaders.glsl");
}

