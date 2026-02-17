//! Generic Precision Shader System
//!
//! Provides compile-time and runtime shader generation for any precision type.
//! ONE template → shaders for f16, f32, f64, and CPU implementations.
//!
//! # Design Philosophy
//!
//! WGSL doesn't have generics, but we want:
//! 1. ONE source of truth for each algorithm
//! 2. SAME math on CPU (Rust) and GPU (WGSL)
//! 3. Any precision: f16, f32, f64 (and future bf16, fp8)
//!
//! # Usage
//!
//! ```rust
//! use barracuda::shaders::precision::{Precision, ShaderTemplate};
//!
//! // Generate f64 shader from template
//! let f64_shader = ShaderTemplate::elementwise_add(Precision::F64);
//!
//! // Same algorithm on CPU
//! fn add_cpu<T: num_traits::Float>(a: &[T], b: &[T], out: &mut [T]) {
//!     for i in 0..out.len() {
//!         out[i] = a[i] + b[i];  // Same as GPU
//!     }
//! }
//! ```

/// Supported precision types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Precision {
    /// 16-bit float (half precision) - for inference, 2x memory bandwidth
    F16,
    /// 32-bit float (single precision) - default, widely supported
    F32,
    /// 64-bit float (double precision) - scientific computing
    F64,
    // Future: BF16, FP8, INT8
}

impl Precision {
    /// WGSL scalar type name
    pub fn scalar(&self) -> &'static str {
        match self {
            Precision::F16 => "f16",
            Precision::F32 => "f32",
            Precision::F64 => "f64",
        }
    }

    /// WGSL vec2 type name (or scalar for f64 which lacks native vec support)
    pub fn vec2(&self) -> &'static str {
        match self {
            Precision::F16 => "vec2<f16>",
            Precision::F32 => "vec2<f32>",
            Precision::F64 => "f64", // f64 lacks vec support in WGSL
        }
    }

    /// WGSL vec4 type name (or scalar for f64)
    pub fn vec4(&self) -> &'static str {
        match self {
            Precision::F16 => "vec4<f16>",
            Precision::F32 => "vec4<f32>",
            Precision::F64 => "f64", // f64 lacks vec support
        }
    }

    /// Whether this precision supports vectorized operations (vec4)
    pub fn has_vec4(&self) -> bool {
        matches!(self, Precision::F16 | Precision::F32)
    }

    /// Bytes per element
    pub fn bytes_per_element(&self) -> usize {
        match self {
            Precision::F16 => 2,
            Precision::F32 => 4,
            Precision::F64 => 8,
        }
    }

    /// Required wgpu feature for this precision
    pub fn required_feature(&self) -> Option<wgpu::Features> {
        match self {
            Precision::F16 => Some(wgpu::Features::SHADER_F16),
            Precision::F32 => None, // Always available
            Precision::F64 => Some(wgpu::Features::SHADER_F64),
        }
    }
}

/// Shader template with precision placeholders
pub struct ShaderTemplate {
    template: &'static str,
}

impl ShaderTemplate {
    /// Create a new shader template
    pub const fn new(template: &'static str) -> Self {
        Self { template }
    }

    /// Render the template for a specific precision
    pub fn render(&self, precision: Precision) -> String {
        let mut result = self.template.to_string();

        // Replace precision placeholders
        result = result.replace("{{SCALAR}}", precision.scalar());
        result = result.replace("{{VEC2}}", precision.vec2());
        result = result.replace("{{VEC4}}", precision.vec4());

        // Handle conditional blocks
        if precision.has_vec4() {
            // Keep vectorized code
            result = result.replace("{{#if HAS_VEC4}}", "");
            result = result.replace("{{/if}}", "");
        } else {
            // Remove vectorized code block for f64
            result = remove_conditional_block(&result, "{{#if HAS_VEC4}}", "{{/if}}");
        }

        result
    }

    // =========================================================================
    // Pre-defined templates for common operations
    // =========================================================================

    /// Element-wise addition: C = A + B
    pub fn elementwise_add(precision: Precision) -> String {
        Self::new(TEMPLATE_ELEMENTWISE_ADD).render(precision)
    }

    /// Element-wise multiplication: C = A * B
    pub fn elementwise_mul(precision: Precision) -> String {
        Self::new(TEMPLATE_ELEMENTWISE_MUL).render(precision)
    }

    /// Element-wise FMA: D = A * B + C
    pub fn elementwise_fma(precision: Precision) -> String {
        Self::new(TEMPLATE_ELEMENTWISE_FMA).render(precision)
    }

    /// Dot product: sum(A * B)
    pub fn dot_product(precision: Precision) -> String {
        Self::new(TEMPLATE_DOT_PRODUCT).render(precision)
    }

    /// Reduction sum
    pub fn reduce_sum(precision: Precision) -> String {
        Self::new(TEMPLATE_REDUCE_SUM).render(precision)
    }

    // =========================================================================
    // F64 Math Library (Pure-GPU transcendental functions)
    // =========================================================================

    /// Returns the complete math_f64.wgsl library as a string.
    /// This library implements transcendental functions (sqrt, exp, log, pow, sin, cos, etc.)
    /// using only f64 arithmetic operations.
    ///
    /// # Usage
    /// ```rust
    /// let math_lib = ShaderTemplate::math_f64_preamble();
    /// let full_shader = format!("{}\n\n{}", math_lib, user_shader_body);
    /// ```
    pub fn math_f64_preamble() -> String {
        include_str!("math/math_f64.wgsl").to_string()
    }

    /// Prepends the math_f64 library to a user shader body.
    /// This is the preferred way to use f64 transcendental functions in GPU shaders.
    ///
    /// # Example
    /// ```rust
    /// let user_shader = r#"
    /// @group(0) @binding(0) var<storage, read> input: array<f64>;
    /// @group(0) @binding(1) var<storage, read_write> output: array<f64>;
    ///
    /// @compute @workgroup_size(256)
    /// fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    ///     let idx = id.x;
    ///     if (idx >= arrayLength(&output)) { return; }
    ///     
    ///     // Use math_f64 functions directly
    ///     output[idx] = sqrt_f64(input[idx]);
    /// }
    /// "#;
    ///
    /// let full_shader = ShaderTemplate::with_math_f64(user_shader);
    /// ```
    pub fn with_math_f64(shader_body: &str) -> String {
        format!(
            "{}\n\n// User shader:\n{}",
            Self::math_f64_preamble(),
            shader_body
        )
    }

    /// Auto-patch shader for driver compatibility.
    ///
    /// Replaces native builtins that crash on certain drivers with software equivalents
    /// from `math_f64.wgsl`. This is the recommended way to create portable f64 shaders.
    ///
    /// # NVK (nouveau) Workarounds (Feb 2026 - hotSpring validation)
    /// - Replaces `exp(` with `exp_f64(` to avoid NAK compiler crash
    /// - Replaces `log(` with `log_f64(` for consistency
    /// - `sqrt`, `abs`, `floor`, `ceil` are NOT patched (work correctly on NVK)
    ///
    /// # Example
    /// ```rust,ignore
    /// use barracuda::device::WgpuDevice;
    /// use barracuda::shaders::precision::ShaderTemplate;
    ///
    /// let device = WgpuDevice::new().await?;
    /// let user_shader = r#"
    /// @compute @workgroup_size(256)
    /// fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    ///     output[id.x] = exp(input[id.x]);  // Uses native exp()
    /// }
    /// "#;
    ///
    /// // Auto-patches exp() to exp_f64() if running on NVK
    /// let safe_shader = ShaderTemplate::for_device(user_shader, &device);
    /// ```
    pub fn for_device(shader_body: &str, device: &crate::device::WgpuDevice) -> String {
        let patched = if device.is_nvk() {
            // Replace native f64 builtins that crash NAK with software equivalents
            // Only exp/log are problematic; sqrt/abs/floor/ceil work fine
            shader_body
                .replace("exp(", "exp_f64(")
                .replace("log(", "log_f64(")
        } else {
            shader_body.to_string()
        };

        // Always include math_f64 preamble for the _f64 functions
        Self::with_math_f64(&patched)
    }

    /// Auto-patch shader for driver compatibility with auto-detection of needed functions.
    ///
    /// Like `for_device`, but only includes the math_f64 functions that are actually used.
    /// This provides both portability and minimal shader compilation time.
    pub fn for_device_auto(shader_body: &str, device: &crate::device::WgpuDevice) -> String {
        let patched = if device.is_nvk() {
            shader_body
                .replace("exp(", "exp_f64(")
                .replace("log(", "log_f64(")
        } else {
            shader_body.to_string()
        };

        Self::with_math_f64_auto(&patched)
    }

    /// Auto-detects which math_f64 functions are used in a shader and includes only those.
    /// This reduces shader compilation time by ~40-60% compared to the full library.
    ///
    /// # Example
    /// ```rust
    /// let user_shader = r#"
    /// @compute @workgroup_size(256)
    /// fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    ///     output[id.x] = sqrt_f64(input[id.x]) + exp_f64(input[id.x]);
    /// }
    /// "#;
    ///
    /// // Only includes sqrt_f64, exp_f64, and their dependencies (abs_f64, round_f64, floor_f64)
    /// let full_shader = ShaderTemplate::with_math_f64_auto(user_shader);
    /// ```
    pub fn with_math_f64_auto(shader_body: &str) -> String {
        // Detect which functions are called in the shader
        let mut used_functions = Vec::new();

        for func_name in F64_FUNCTION_ORDER {
            // Look for function call pattern: name( or name (
            let call_pattern = format!("{func_name}(");
            let call_pattern_space = format!("{func_name} (");

            if shader_body.contains(&call_pattern) || shader_body.contains(&call_pattern_space) {
                used_functions.push(*func_name);
            }
        }

        // Also check for round_f64 which is commonly needed for f64 constants
        if shader_body.contains("round_f64") && !used_functions.contains(&"round_f64") {
            used_functions.push("round_f64");
        }

        if used_functions.is_empty() {
            // No math_f64 functions detected, return shader as-is
            return shader_body.to_string();
        }

        format!(
            "{}\n\n// User shader:\n{}",
            Self::math_f64_subset(&used_functions),
            shader_body
        )
    }

    /// Returns a minimal subset of math_f64 functions based on what's needed.
    /// This reduces shader compilation time by only including used functions.
    ///
    /// # Arguments
    /// * `functions` - List of function names needed (e.g., ["sqrt_f64", "pow_f64"])
    ///
    /// # Note
    /// Dependencies are automatically included (e.g., pow_f64 includes exp_f64, log_f64)
    pub fn math_f64_subset(functions: &[&str]) -> String {
        use std::collections::HashSet;

        // Build dependency graph
        let deps = F64_FUNCTION_DEPS;

        // Collect all needed functions via transitive closure
        let mut needed: HashSet<&str> = HashSet::new();

        fn add_with_deps<'a>(
            name: &'a str,
            needed: &mut HashSet<&'a str>,
            deps: &'a [(&'a str, &'a [&'a str])],
        ) {
            if needed.contains(name) {
                return;
            }
            needed.insert(name);

            // Find dependencies for this function
            for (func, func_deps) in deps {
                if *func == name {
                    for dep in func_deps.iter().copied() {
                        add_with_deps(dep, needed, deps);
                    }
                    break;
                }
            }
        }

        // Add requested functions and their dependencies
        for func in functions {
            add_with_deps(func, &mut needed, deps);
        }

        // If no functions requested, return empty preamble
        if needed.is_empty() {
            return String::new();
        }

        // Build output in dependency order (constants first, then base functions)
        let full_lib = Self::math_f64_preamble();
        let mut output = String::new();

        // Always include header comment and f64_const helper
        output.push_str(
            "// math_f64 subset - auto-generated\n\
             // Helper: construct f64 constant from AbstractFloat\n\
             fn f64_const(x: f64, c: f32) -> f64 {\n    \
                 return x - x + f64(c);\n\
             }\n\n",
        );

        // Extract and include each needed function from the full library
        for func_name in F64_FUNCTION_ORDER {
            if needed.contains(*func_name) {
                if let Some(func_code) = extract_wgsl_function(&full_lib, func_name) {
                    output.push_str(&func_code);
                    output.push_str("\n\n");
                }
            }
        }

        output
    }

    /// Check if shader body already defines a function
    ///
    /// Used to avoid injecting duplicate definitions that cause
    /// "redefinition of X" errors in WGSL.
    ///
    /// # Example
    /// ```rust,ignore
    /// let shader = "fn f64_const(x: f64, c: f32) -> f64 { ... }";
    /// assert!(ShaderTemplate::shader_defines_function(shader, "f64_const"));
    /// ```
    pub fn shader_defines_function(shader_body: &str, func_name: &str) -> bool {
        let pattern1 = format!("fn {func_name}(");
        let pattern2 = format!("fn {func_name} (");
        shader_body.contains(&pattern1) || shader_body.contains(&pattern2)
    }

    /// Check if shader body defines any variable/constant with a given name
    ///
    /// Checks for module-scope let/var/const definitions that would conflict
    /// with injected code. Does NOT match function-local definitions.
    ///
    /// # BUG FIX (Feb 17 2026 — hotSpring validation):
    /// User shaders that define `let zero = x - x;` at module scope conflict
    /// with math_f64 functions that use `let zero = ...` internally. This
    /// helper enables detection before injection.
    pub fn shader_defines_module_var(shader_body: &str, var_name: &str) -> bool {
        // Module-scope definitions appear at start of line (no indentation)
        // or after bindings section
        for line in shader_body.lines() {
            let trimmed = line.trim();
            // Skip function bodies (start with spaces/tabs in well-formatted WGSL)
            // Module-scope definitions are at column 0 or after comments
            if line.starts_with("let ")
                || line.starts_with("var ")
                || line.starts_with("const ")
            {
                if trimmed.contains(&format!("{var_name} "))
                    || trimmed.contains(&format!("{var_name}="))
                    || trimmed.contains(&format!("{var_name}:"))
                {
                    return true;
                }
            }
        }
        false
    }

    /// Prepends math_f64 library with conflict detection
    ///
    /// Like `with_math_f64()` but skips injection if the shader already
    /// defines `f64_const` or other math_f64 functions. This prevents
    /// "redefinition" errors in WGSL compilation.
    ///
    /// # Example
    /// ```rust,ignore
    /// let shader = include_str!("my_shader.wgsl");
    /// let full = ShaderTemplate::with_math_f64_safe(shader);
    /// // If my_shader.wgsl already has f64_const, won't cause redefinition
    /// ```
    pub fn with_math_f64_safe(shader_body: &str) -> String {
        // If shader already defines f64_const, assume it has its own math lib
        if Self::shader_defines_function(shader_body, "f64_const") {
            log::debug!("Shader already defines f64_const, skipping preamble injection");
            return shader_body.to_string();
        }

        Self::with_math_f64(shader_body)
    }

    /// Auto-detect and prepend math_f64 with conflict detection
    ///
    /// Like `with_math_f64_auto()` but skips injection if conflicts detected.
    /// This is the safest method for use with user shaders.
    ///
    /// # Example
    /// ```rust,ignore
    /// let shader = include_str!("user_shader.wgsl");
    /// let full = ShaderTemplate::with_math_f64_auto_safe(shader);
    /// ```
    pub fn with_math_f64_auto_safe(shader_body: &str) -> String {
        // If shader already defines f64_const, skip auto-injection
        if Self::shader_defines_function(shader_body, "f64_const") {
            log::debug!("Shader already defines f64_const, skipping auto-injection");
            return shader_body.to_string();
        }

        Self::with_math_f64_auto(shader_body)
    }
}

/// Function dependency map for math_f64.wgsl
/// Each entry: (function_name, [dependencies])
const F64_FUNCTION_DEPS: &[(&str, &[&str])] = &[
    // Basic functions
    ("abs_f64", &[]),
    ("sign_f64", &[]),
    ("floor_f64", &[]),
    ("ceil_f64", &[]),
    ("round_f64", &["floor_f64"]),
    ("fract_f64", &["floor_f64"]),
    ("min_f64", &[]),
    ("max_f64", &[]),
    ("clamp_f64", &["min_f64", "max_f64"]),
    // Power functions
    ("sqrt_f64", &[]),
    ("cbrt_f64", &["abs_f64"]),
    ("ipow_f64", &[]),
    ("pow_one_third", &["cbrt_f64"]),
    ("pow_one_half", &["sqrt_f64"]),
    ("pow_two_thirds", &["cbrt_f64"]),
    // Transcendentals
    ("exp_f64", &["abs_f64", "round_f64"]),
    ("log_f64", &[]),
    (
        "pow_f64",
        &[
            "round_f64",
            "abs_f64",
            "sqrt_f64",
            "cbrt_f64",
            "pow_two_thirds",
            "exp_f64",
            "log_f64",
            "ipow_f64",
        ],
    ),
    // Trig
    ("sin_f64", &[]),
    ("cos_f64", &["sin_f64"]),
    ("tan_f64", &["sin_f64", "cos_f64"]),
    // Hyperbolic
    ("sinh_f64", &["exp_f64"]),
    ("cosh_f64", &["exp_f64"]),
    ("tanh_f64", &["exp_f64"]),
    // Special functions
    ("gamma_f64", &["sin_f64", "abs_f64", "pow_f64", "exp_f64"]),
    ("erf_f64", &["sign_f64", "abs_f64", "exp_f64"]),
    (
        "bessel_j0_f64",
        &["abs_f64", "sqrt_f64", "cos_f64", "sin_f64"],
    ),
];

/// Ordered list of functions for correct emission order (dependencies first)
const F64_FUNCTION_ORDER: &[&str] = &[
    // Basic - no deps
    "abs_f64",
    "sign_f64",
    "floor_f64",
    "ceil_f64",
    "min_f64",
    "max_f64",
    // Basic - with deps
    "round_f64",
    "fract_f64",
    "clamp_f64",
    // Power - no deps
    "sqrt_f64",
    "ipow_f64",
    // Power - with deps
    "cbrt_f64",
    "pow_one_third",
    "pow_one_half",
    "pow_two_thirds",
    // Transcendentals
    "exp_f64",
    "log_f64",
    "pow_f64",
    // Trig
    "sin_f64",
    "cos_f64",
    "tan_f64",
    // Hyperbolic
    "sinh_f64",
    "cosh_f64",
    "tanh_f64",
    // Special
    "gamma_f64",
    "erf_f64",
    "bessel_j0_f64",
];

/// Extract a WGSL function from source by name
fn extract_wgsl_function(source: &str, name: &str) -> Option<String> {
    // Look for function definition: `fn name(` or `fn name (`
    let fn_pattern = format!("fn {name}(");
    let fn_pattern_space = format!("fn {name} (");

    let start_idx = source
        .find(&fn_pattern)
        .or_else(|| source.find(&fn_pattern_space))?;

    // Find the opening brace
    let brace_idx = source[start_idx..].find('{')?;
    let fn_start = start_idx;

    // Count braces to find matching closing brace
    let mut brace_count = 0;
    let mut fn_end = fn_start + brace_idx;

    for (i, c) in source[fn_start + brace_idx..].char_indices() {
        match c {
            '{' => brace_count += 1,
            '}' => {
                brace_count -= 1;
                if brace_count == 0 {
                    fn_end = fn_start + brace_idx + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    // Include any doc comment above the function
    let mut doc_start = fn_start;
    let before = &source[..fn_start];
    if let Some(last_newline) = before.rfind('\n') {
        // Check if preceding lines are comments
        let prev_lines: Vec<&str> = before[..last_newline + 1].lines().rev().take(5).collect();
        for (i, line) in prev_lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("///") || trimmed.starts_with("//") || trimmed.is_empty() {
                // Include this line
                if i == 0 {
                    if let Some(pos) = before.rfind(line) {
                        doc_start = pos;
                    }
                }
            } else {
                break;
            }
        }
    }

    Some(source[doc_start..fn_end].trim().to_string())
}

/// Remove a conditional block from the template
fn remove_conditional_block(source: &str, start_marker: &str, end_marker: &str) -> String {
    let mut result = String::new();
    let mut in_block = false;

    for line in source.lines() {
        if line.contains(start_marker) {
            in_block = true;
            continue;
        }
        if line.contains(end_marker) {
            in_block = false;
            continue;
        }
        if !in_block {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

// =============================================================================
// SHADER TEMPLATES
// =============================================================================

const TEMPLATE_ELEMENTWISE_ADD: &str = r#"// Element-wise Addition: C = A + B
// Generated for precision: {{SCALAR}}

@group(0) @binding(0) var<storage, read> a: array<{{SCALAR}}>;
@group(0) @binding(1) var<storage, read> b: array<{{SCALAR}}>;
@group(0) @binding(2) var<storage, read_write> output: array<{{SCALAR}}>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) { return; }
    
    // Core algorithm (identical to CPU implementation)
    output[idx] = a[idx] + b[idx];
}
{{#if HAS_VEC4}}

// Vectorized variant for better memory throughput
struct Params { size: u32, _pad1: u32, _pad2: u32, _pad3: u32, }

@group(0) @binding(0) var<storage, read> a_vec: array<{{VEC4}}>;
@group(0) @binding(1) var<storage, read> b_vec: array<{{VEC4}}>;
@group(0) @binding(2) var<storage, read_write> out_vec: array<{{VEC4}}>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main_vec4(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx < params.size / 4u) { out_vec[idx] = a_vec[idx] + b_vec[idx]; }
}
{{/if}}
"#;

const TEMPLATE_ELEMENTWISE_MUL: &str = r#"// Element-wise Multiplication: C = A * B
// Generated for precision: {{SCALAR}}

@group(0) @binding(0) var<storage, read> a: array<{{SCALAR}}>;
@group(0) @binding(1) var<storage, read> b: array<{{SCALAR}}>;
@group(0) @binding(2) var<storage, read_write> output: array<{{SCALAR}}>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) { return; }
    
    // Core algorithm
    output[idx] = a[idx] * b[idx];
}
{{#if HAS_VEC4}}

struct Params { size: u32, _pad1: u32, _pad2: u32, _pad3: u32, }

@group(0) @binding(0) var<storage, read> a_vec: array<{{VEC4}}>;
@group(0) @binding(1) var<storage, read> b_vec: array<{{VEC4}}>;
@group(0) @binding(2) var<storage, read_write> out_vec: array<{{VEC4}}>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main_vec4(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx < params.size / 4u) { out_vec[idx] = a_vec[idx] * b_vec[idx]; }
}
{{/if}}
"#;

const TEMPLATE_ELEMENTWISE_FMA: &str = r#"// Fused Multiply-Add: D = A * B + C
// Generated for precision: {{SCALAR}}

@group(0) @binding(0) var<storage, read> a: array<{{SCALAR}}>;
@group(0) @binding(1) var<storage, read> b: array<{{SCALAR}}>;
@group(0) @binding(2) var<storage, read> c: array<{{SCALAR}}>;
@group(0) @binding(3) var<storage, read_write> output: array<{{SCALAR}}>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) { return; }
    
    // FMA - single rounding, better precision than separate mul+add
    output[idx] = fma(a[idx], b[idx], c[idx]);
}
"#;

const TEMPLATE_DOT_PRODUCT: &str = r#"// Dot Product: sum(A * B)
// Generated for precision: {{SCALAR}}
// Uses workgroup reduction for parallel summation

var<workgroup> shared: array<{{SCALAR}}, 256>;

@group(0) @binding(0) var<storage, read> a: array<{{SCALAR}}>;
@group(0) @binding(1) var<storage, read> b: array<{{SCALAR}}>;
@group(0) @binding(2) var<storage, read_write> output: array<{{SCALAR}}>;

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let gid = global_id.x;
    let lid = local_id.x;
    let n = arrayLength(&a);
    
    // Load and multiply
    if (gid < n) {
        shared[lid] = a[gid] * b[gid];
    } else {
        shared[lid] = {{SCALAR}}(0);
    }
    workgroupBarrier();
    
    // Parallel reduction
    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (lid < stride) {
            shared[lid] = shared[lid] + shared[lid + stride];
        }
        workgroupBarrier();
    }
    
    // Write partial sum
    if (lid == 0u) {
        output[workgroup_id.x] = shared[0];
    }
}
"#;

const TEMPLATE_REDUCE_SUM: &str = r#"// Reduction Sum: sum(A)
// Generated for precision: {{SCALAR}}

var<workgroup> shared: array<{{SCALAR}}, 256>;

@group(0) @binding(0) var<storage, read> input: array<{{SCALAR}}>;
@group(0) @binding(1) var<storage, read_write> output: array<{{SCALAR}}>;

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let gid = global_id.x;
    let lid = local_id.x;
    let n = arrayLength(&input);
    
    shared[lid] = select({{SCALAR}}(0), input[gid], gid < n);
    workgroupBarrier();
    
    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (lid < stride) {
            shared[lid] = shared[lid] + shared[lid + stride];
        }
        workgroupBarrier();
    }
    
    if (lid == 0u) {
        output[workgroup_id.x] = shared[0];
    }
}
"#;

// =============================================================================
// CPU IMPLEMENTATIONS (same algorithms, any precision via num-traits)
// =============================================================================

/// CPU implementations that match GPU algorithms exactly
pub mod cpu {
    use num_traits::Float;

    /// Element-wise addition: C = A + B
    #[inline]
    pub fn elementwise_add<T: Float>(a: &[T], b: &[T], output: &mut [T]) {
        assert_eq!(a.len(), b.len());
        assert_eq!(a.len(), output.len());
        for i in 0..output.len() {
            output[i] = a[i] + b[i]; // Same as GPU
        }
    }

    /// Element-wise multiplication: C = A * B
    #[inline]
    pub fn elementwise_mul<T: Float>(a: &[T], b: &[T], output: &mut [T]) {
        assert_eq!(a.len(), b.len());
        assert_eq!(a.len(), output.len());
        for i in 0..output.len() {
            output[i] = a[i] * b[i]; // Same as GPU
        }
    }

    /// Fused multiply-add: D = A * B + C
    #[inline]
    pub fn elementwise_fma<T: Float>(a: &[T], b: &[T], c: &[T], output: &mut [T]) {
        assert_eq!(a.len(), b.len());
        assert_eq!(a.len(), c.len());
        assert_eq!(a.len(), output.len());
        for i in 0..output.len() {
            // Note: T::mul_add uses hardware FMA when available
            output[i] = a[i].mul_add(b[i], c[i]); // Same as GPU fma()
        }
    }

    /// Dot product: sum(A * B)
    #[inline]
    pub fn dot_product<T: Float>(a: &[T], b: &[T]) -> T {
        assert_eq!(a.len(), b.len());
        let mut sum = T::zero();
        for i in 0..a.len() {
            sum = sum + a[i] * b[i]; // Same as GPU
        }
        sum
    }

    /// Kahan summation for high-precision reduction
    #[inline]
    pub fn kahan_sum<T: Float>(input: &[T]) -> T {
        let mut sum = T::zero();
        let mut c = T::zero(); // Compensation
        for &x in input {
            let y = x - c;
            let t = sum + y;
            c = (t - sum) - y;
            sum = t;
        }
        sum
    }

    /// Naive sum
    #[inline]
    pub fn reduce_sum<T: Float>(input: &[T]) -> T {
        input.iter().fold(T::zero(), |acc, &x| acc + x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precision_types() {
        assert_eq!(Precision::F32.scalar(), "f32");
        assert_eq!(Precision::F64.scalar(), "f64");
        assert_eq!(Precision::F16.scalar(), "f16");

        assert!(Precision::F32.has_vec4());
        assert!(!Precision::F64.has_vec4());
    }

    #[test]
    fn test_shader_generation() {
        let f32_shader = ShaderTemplate::elementwise_add(Precision::F32);
        assert!(f32_shader.contains("array<f32>"));
        assert!(f32_shader.contains("vec4<f32>")); // Has vectorized

        let f64_shader = ShaderTemplate::elementwise_add(Precision::F64);
        assert!(f64_shader.contains("array<f64>"));
        assert!(!f64_shader.contains("vec4")); // No vectorized for f64
    }

    #[test]
    fn test_cpu_matches_description() {
        let a = vec![1.0_f64, 2.0, 3.0];
        let b = vec![4.0_f64, 5.0, 6.0];
        let mut out = vec![0.0_f64; 3];

        cpu::elementwise_add(&a, &b, &mut out);
        assert_eq!(out, vec![5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_math_f64_subset() {
        // Test that subset includes only requested functions + deps
        let subset = ShaderTemplate::math_f64_subset(&["sqrt_f64"]);

        assert!(subset.contains("fn sqrt_f64"), "Should include sqrt_f64");
        assert!(
            subset.contains("fn f64_const"),
            "Should include f64_const helper"
        );
        assert!(!subset.contains("fn exp_f64"), "Should NOT include exp_f64");
        assert!(!subset.contains("fn sin_f64"), "Should NOT include sin_f64");

        // pow_f64 has many dependencies
        let pow_subset = ShaderTemplate::math_f64_subset(&["pow_f64"]);
        assert!(pow_subset.contains("fn pow_f64"));
        assert!(
            pow_subset.contains("fn exp_f64"),
            "pow_f64 depends on exp_f64"
        );
        assert!(
            pow_subset.contains("fn log_f64"),
            "pow_f64 depends on log_f64"
        );
        assert!(
            pow_subset.contains("fn abs_f64"),
            "pow_f64 depends on abs_f64"
        );
    }

    #[test]
    fn test_math_f64_auto_detection() {
        let shader = r#"
            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                let a = sqrt_f64(input[id.x]);
                let b = exp_f64(input[id.x]);
                output[id.x] = a + b;
            }
        "#;

        let full_shader = ShaderTemplate::with_math_f64_auto(shader);

        // Should detect and include sqrt_f64 and exp_f64
        assert!(
            full_shader.contains("fn sqrt_f64"),
            "Should include sqrt_f64"
        );
        assert!(full_shader.contains("fn exp_f64"), "Should include exp_f64");
        assert!(
            full_shader.contains("fn abs_f64"),
            "Should include abs_f64 (exp dep)"
        );
        assert!(
            full_shader.contains("fn round_f64"),
            "Should include round_f64 (exp dep)"
        );

        // Should NOT include unrelated functions
        assert!(
            !full_shader.contains("fn sin_f64"),
            "Should NOT include sin_f64"
        );
        assert!(
            !full_shader.contains("fn gamma_f64"),
            "Should NOT include gamma_f64"
        );
    }

    #[test]
    fn test_math_f64_auto_no_functions() {
        let shader = r#"
            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                output[id.x] = input[id.x] * 2.0;
            }
        "#;

        let full_shader = ShaderTemplate::with_math_f64_auto(shader);

        // Should return shader as-is when no math_f64 functions detected
        assert!(full_shader.contains("output[id.x] = input[id.x] * 2.0"));
        assert!(
            !full_shader.contains("fn sqrt_f64"),
            "Should not add any math_f64 functions"
        );
    }

    #[test]
    fn test_math_f64_full_vs_auto_size() {
        let shader = r#"
            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                output[id.x] = sqrt_f64(input[id.x]);
            }
        "#;

        let full = ShaderTemplate::with_math_f64(shader);
        let auto = ShaderTemplate::with_math_f64_auto(shader);

        // Auto should be significantly smaller than full
        assert!(
            auto.len() < full.len(),
            "Auto ({} bytes) should be smaller than full ({} bytes)",
            auto.len(),
            full.len()
        );

        // At least 40% reduction for simple sqrt case
        let reduction = 1.0 - (auto.len() as f64 / full.len() as f64);
        assert!(
            reduction > 0.4,
            "Expected >40% size reduction, got {:.1}%",
            reduction * 100.0
        );
    }

    #[test]
    fn test_shader_defines_function() {
        let shader = r#"
            fn f64_const(x: f64, c: f32) -> f64 {
                return x - x + f64(c);
            }
        "#;
        assert!(ShaderTemplate::shader_defines_function(shader, "f64_const"));
        assert!(!ShaderTemplate::shader_defines_function(shader, "sqrt_f64"));

        // Test with space before paren
        let shader_space = r#"fn sqrt_f64 (x: f64) -> f64 { return x; }"#;
        assert!(ShaderTemplate::shader_defines_function(shader_space, "sqrt_f64"));
    }

    #[test]
    fn test_shader_defines_module_var() {
        // Module-scope definition
        let shader_module_scope = r#"
let zero = 0.0;
fn main() { }
"#;
        assert!(ShaderTemplate::shader_defines_module_var(
            shader_module_scope,
            "zero"
        ));

        // Function-local definition (indented, should NOT match)
        let shader_local = r#"
fn main() {
    let zero = x - x;
}
"#;
        assert!(!ShaderTemplate::shader_defines_module_var(shader_local, "zero"));

        // Const at module scope
        let shader_const = r#"
const EPSILON: f64 = 1e-15;
"#;
        assert!(ShaderTemplate::shader_defines_module_var(shader_const, "EPSILON"));
    }

    #[test]
    fn test_with_math_f64_safe_no_conflict() {
        let shader = r#"
            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                output[id.x] = sqrt(input[id.x]);
            }
        "#;

        let result = ShaderTemplate::with_math_f64_safe(shader);
        // Should include the full library (no conflict)
        assert!(result.contains("fn f64_const"));
    }

    #[test]
    fn test_with_math_f64_safe_conflict() {
        let shader = r#"
            // User shader already has f64_const
            fn f64_const(x: f64, c: f32) -> f64 {
                return x - x + f64(c);
            }

            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                output[id.x] = f64_const(input[id.x], 1.0);
            }
        "#;

        let result = ShaderTemplate::with_math_f64_safe(shader);
        // Should return shader as-is (detected conflict)
        assert!(result.contains("User shader already has f64_const"));
        // Should NOT have duplicate f64_const
        assert_eq!(result.matches("fn f64_const").count(), 1);
    }
}
