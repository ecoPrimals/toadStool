//! Generic Precision Shader System
//!
//! Provides compile-time and runtime shader generation for any precision type.
//! ONE template → shaders for f16, f32, f64, and CPU implementations.

pub mod cpu;
mod math_f64;
mod templates;

use templates::{
    remove_conditional_block, TEMPLATE_DOT_PRODUCT, TEMPLATE_ELEMENTWISE_ADD,
    TEMPLATE_ELEMENTWISE_FMA, TEMPLATE_ELEMENTWISE_MUL, TEMPLATE_REDUCE_SUM,
};

use math_f64::{
    extract_wgsl_function, F64_FOSSIL_FUNCTIONS, F64_FUNCTION_DEPS, F64_FUNCTION_ORDER,
};

/// Supported precision types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Precision {
    /// 16-bit float (half precision) - for inference, 2x memory bandwidth
    F16,
    /// 32-bit float (single precision) - default, widely supported
    F32,
    /// 64-bit float (double precision) - scientific computing
    F64,
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
            Precision::F64 => "f64",
        }
    }

    /// WGSL vec4 type name (or scalar for f64)
    pub fn vec4(&self) -> &'static str {
        match self {
            Precision::F16 => "vec4<f16>",
            Precision::F32 => "vec4<f32>",
            Precision::F64 => "f64",
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
            Precision::F32 => None,
            Precision::F64 => Some(wgpu::Features::SHADER_F64),
        }
    }
}

/// Shader template with precision placeholders
pub struct ShaderTemplate {
    template: &'static str,
}

impl ShaderTemplate {
    pub const fn new(template: &'static str) -> Self {
        Self { template }
    }

    pub fn render(&self, precision: Precision) -> String {
        let mut result = self.template.to_string();
        result = result.replace("{{SCALAR}}", precision.scalar());
        result = result.replace("{{VEC2}}", precision.vec2());
        result = result.replace("{{VEC4}}", precision.vec4());
        if precision.has_vec4() {
            result = result.replace("{{#if HAS_VEC4}}", "");
            result = result.replace("{{/if}}", "");
        } else {
            result = remove_conditional_block(&result, "{{#if HAS_VEC4}}", "{{/if}}");
        }
        result
    }

    pub fn elementwise_add(precision: Precision) -> String {
        Self::new(TEMPLATE_ELEMENTWISE_ADD).render(precision)
    }

    pub fn elementwise_mul(precision: Precision) -> String {
        Self::new(TEMPLATE_ELEMENTWISE_MUL).render(precision)
    }

    pub fn elementwise_fma(precision: Precision) -> String {
        Self::new(TEMPLATE_ELEMENTWISE_FMA).render(precision)
    }

    pub fn dot_product(precision: Precision) -> String {
        Self::new(TEMPLATE_DOT_PRODUCT).render(precision)
    }

    pub fn reduce_sum(precision: Precision) -> String {
        Self::new(TEMPLATE_REDUCE_SUM).render(precision)
    }

    pub fn math_f64_preamble() -> String {
        let core = include_str!("../math/math_f64.wgsl");
        let special = include_str!("../math/math_f64_special.wgsl");
        format!("{core}\n{special}")
    }

    pub fn with_math_f64(shader_body: &str) -> String {
        format!(
            "{}\n\n// User shader:\n{}",
            Self::math_f64_preamble(),
            shader_body
        )
    }

    /// Generate f64 shader with driver-aware exp/log patching (synchronous).
    ///
    /// Uses `needs_f64_exp_log_workaround()` (name-based heuristic). For definitive
    /// detection, async callers should use `device.probe_f64_exp_capable().await` and
    /// pass `!capable` as the workaround flag — probe overrides heuristic when run.
    pub fn for_device(shader_body: &str, device: &crate::device::WgpuDevice) -> String {
        Self::for_driver_auto(shader_body, device.needs_f64_exp_log_workaround())
    }

    pub fn for_device_auto(shader_body: &str, device: &crate::device::WgpuDevice) -> String {
        Self::for_driver_auto(shader_body, device.needs_f64_exp_log_workaround())
    }

    /// Patch a WGSL shader's `WARP_SIZE` constant and `@workgroup_size` annotation.
    ///
    /// Replaces `const WARP_SIZE: u32 = 32u;` with the given `wave_size` and
    /// adjusts `@workgroup_size(32, 1, 1)` accordingly. Used to specialise the
    /// single-dispatch Jacobi eigensolve for AMD RDNA2/3 (wave_size=64) vs
    /// NVIDIA warp (wave_size=32) at shader-compilation time.
    pub fn patch_warp_size(shader_body: &str, wave_size: u32) -> String {
        shader_body
            .replace(
                "const WARP_SIZE: u32 = 32u;",
                &format!("const WARP_SIZE: u32 = {wave_size}u;"),
            )
            .replace(
                "@workgroup_size(32, 1, 1)",
                &format!("@workgroup_size({wave_size}, 1, 1)"),
            )
    }

    /// Replace legacy fossil f64 function calls with native WGSL equivalents.
    ///
    /// Probe-confirmed native on all `SHADER_F64` hardware via Vulkan (Feb 2026):
    /// `abs`, `sign`, `floor`, `ceil`, `round`, `fract`, `min`, `max`, `clamp`, `sqrt`.
    ///
    /// Rewrites `abs_f64(` → `abs(`, `sqrt_f64(` → `sqrt(` etc. in legacy shaders.
    /// New shaders must use native WGSL builtins directly — this method is the
    /// migration path for older code still using the `_f64` names.
    pub fn substitute_fossil_f64(shader_body: &str) -> String {
        let mut result = shader_body.to_string();
        for (fossil_name, native_name) in F64_FOSSIL_FUNCTIONS {
            let from = format!("{fossil_name}(");
            let to = format!("{native_name}(");
            result = result.replace(&from, &to);
        }
        result
    }

    pub fn for_driver_auto(shader_body: &str, needs_exp_log_workaround: bool) -> String {
        // Upgrade any legacy fossil calls to native WGSL builtins first.
        let substituted = Self::substitute_fossil_f64(shader_body);
        let patched = if needs_exp_log_workaround {
            Self::apply_transcendental_workaround(&substituted)
        } else {
            substituted
        };
        let injected = Self::inject_missing_math_f64(&patched);
        // ILP-reorders @ilp_region blocks and unrolls @unroll_hint loops.
        // ConservativeModel is used as the latency model (safe fallback when no driver profile).
        crate::shaders::optimizer::WgslOptimizer::default().optimize(&injected)
    }

    /// Variant of `for_driver_auto` that uses the accurate `LatencyModel` from
    /// a `GpuDriverProfile` for precise ILP scheduling.
    ///
    /// Prefer this when a `GpuDriverProfile` is available at shader-compile time.
    pub fn for_driver_profile(
        shader_body: &str,
        needs_exp_log_workaround: bool,
        profile: &crate::device::capabilities::GpuDriverProfile,
    ) -> String {
        let substituted = Self::substitute_fossil_f64(shader_body);
        let patched = if needs_exp_log_workaround {
            Self::apply_transcendental_workaround(&substituted)
        } else {
            substituted
        };
        let injected = Self::inject_missing_math_f64(&patched);
        crate::shaders::optimizer::WgslOptimizer::new(profile.latency_model()).optimize(&injected)
    }

    /// Replace native f64 transcendentals with their polyfill equivalents while
    /// preserving WGSL comments so generated shader source stays readable.
    ///
    /// Covers: `exp`, `log`, `pow`, `sin`/`cos`/`tan` (plus inverse variants
    /// `asin`/`acos`/`atan` via suffix matching), `sinh`/`cosh`/`tanh`, `atan2`.
    ///
    /// Processes the shader line-by-line:
    /// - Pure comment lines (`//…`) are passed through unchanged.
    /// - Lines with inline comments have only the code portion patched.
    /// - Block comments `/* … */` are not yet handled (rare in WGSL compute
    ///   shaders; revisit when encountered).
    fn apply_transcendental_workaround(shader: &str) -> String {
        shader
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    return line.to_string();
                }
                if let Some(comment_start) = line.find("//") {
                    let code = &line[..comment_start];
                    let comment = &line[comment_start..];
                    let patched = Self::patch_transcendentals_in_code(code);
                    format!("{patched}{comment}")
                } else {
                    Self::patch_transcendentals_in_code(line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Replace native f64 transcendentals with polyfill calls in a code fragment.
    ///
    /// NVVM's PTXAS does not implement double-precision transcendentals —
    /// they require libdevice which SPIR-V cannot link. All known NVIDIA
    /// proprietary drivers (Ampere, Ada Lovelace, Hopper) and NVK/RADV
    /// exhibit this. The `_f64` polyfills (Cody-Waite + minimax polynomial)
    /// are defined in `math_f64.wgsl` and auto-injected by
    /// `inject_missing_math_f64`.
    ///
    /// Ordering note: `sin(` naturally catches `asin(` → `asin_f64(`,
    /// `cos(` catches `acos(`, `tan(` catches `atan(` — all correct since
    /// our polyfills cover these inverse trig functions too. `atan2` and
    /// hyperbolic functions need explicit entries.
    #[inline]
    fn patch_transcendentals_in_code(code: &str) -> String {
        // Protect WGSL builtins and DF64 functions whose names contain
        // transcendental substrings (e.g. ldexp contains "exp", exp_df64
        // contains "exp") from being mangled by the substring replacer.
        code.replace("ldexp(", "\x00LDEXP\x00")
            .replace("exp_df64(", "\x00EXP_DF64\x00")
            .replace("exp_f64(", "\x00EXP_F64\x00")
            .replace("log_df64(", "\x00LOG_DF64\x00")
            .replace("log_f64(", "\x00LOG_F64\x00")
            .replace("exp(", "exp_f64(")
            .replace("log(", "log_f64(")
            .replace("pow(", "pow_f64(")
            .replace("sinh(", "sinh_f64(")
            .replace("cosh(", "cosh_f64(")
            .replace("tanh(", "tanh_f64(")
            .replace("sin(", "sin_f64(")
            .replace("cos(", "cos_f64(")
            .replace("tan(", "tan_f64(")
            .replace("atan2(", "atan2_f64(")
            .replace("\x00LDEXP\x00", "ldexp(")
            .replace("\x00EXP_DF64\x00", "exp_df64(")
            .replace("\x00EXP_F64\x00", "exp_f64(")
            .replace("\x00LOG_DF64\x00", "log_df64(")
            .replace("\x00LOG_F64\x00", "log_f64(")
    }

    fn inject_missing_math_f64(shader_body: &str) -> String {
        let mut missing_functions: Vec<&str> = Vec::new();
        for func_name in F64_FUNCTION_ORDER {
            // Fossil functions are universally-native WGSL builtins on all
            // SHADER_F64 hardware — never inject them; use native calls directly.
            if F64_FOSSIL_FUNCTIONS.iter().any(|(f, _)| f == func_name) {
                continue;
            }
            let call_pattern = format!("{func_name}(");
            if shader_body.contains(&call_pattern)
                && !Self::shader_defines_function(shader_body, func_name)
            {
                missing_functions.push(func_name);
            }
        }
        if missing_functions.is_empty() {
            return shader_body.to_string();
        }
        let full_lib = Self::math_f64_preamble();
        let mut preamble = String::from("// math_f64 driver workaround - auto-injected\n");
        if !Self::shader_defines_function(shader_body, "f64_const") {
            preamble.push_str(
                "fn f64_const(x: f64, c: f32) -> f64 {\n    return x - x + f64(c);\n}\n\n",
            );
        }
        let mut all_needed = std::collections::HashSet::new();
        for func in &missing_functions {
            Self::collect_deps(func, &mut all_needed);
        }
        for func_name in F64_FUNCTION_ORDER {
            if all_needed.contains(*func_name)
                && !Self::shader_defines_function(shader_body, func_name)
            {
                if let Some(func_code) = extract_wgsl_function(&full_lib, func_name) {
                    // Substitute fossil calls (sqrt_f64, etc.) with native builtins in extracted code
                    let substituted = Self::substitute_fossil_f64(&func_code);
                    preamble.push_str(&substituted);
                    preamble.push_str("\n\n");
                }
            }
        }
        format!("{preamble}\n{shader_body}")
    }

    fn collect_deps<'a>(name: &'a str, needed: &mut std::collections::HashSet<&'a str>) {
        if needed.contains(name) {
            return;
        }
        needed.insert(name);
        for (func, func_deps) in F64_FUNCTION_DEPS {
            if *func == name {
                for dep in func_deps.iter().copied() {
                    Self::collect_deps(dep, needed);
                }
                break;
            }
        }
    }

    pub fn with_math_f64_auto(shader_body: &str) -> String {
        let mut used_functions = Vec::new();
        for func_name in F64_FUNCTION_ORDER {
            let call_pattern = format!("{func_name}(");
            let call_pattern_space = format!("{func_name} (");
            if shader_body.contains(&call_pattern) || shader_body.contains(&call_pattern_space) {
                used_functions.push(*func_name);
            }
        }
        if shader_body.contains("round_f64") && !used_functions.contains(&"round_f64") {
            used_functions.push("round_f64");
        }
        if used_functions.is_empty() {
            return shader_body.to_string();
        }
        format!(
            "{}\n\n// User shader:\n{}",
            Self::math_f64_subset(&used_functions),
            shader_body
        )
    }

    pub fn math_f64_subset(functions: &[&str]) -> String {
        use std::collections::HashSet;
        let deps = F64_FUNCTION_DEPS;
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
            for (func, func_deps) in deps {
                if *func == name {
                    for dep in func_deps.iter().copied() {
                        add_with_deps(dep, needed, deps);
                    }
                    break;
                }
            }
        }
        for func in functions {
            add_with_deps(func, &mut needed, deps);
        }
        if needed.is_empty() {
            return String::new();
        }
        let full_lib = Self::math_f64_preamble();
        let mut output = String::new();
        output.push_str(
            "// math_f64 subset - auto-generated\n\
             fn f64_const(x: f64, c: f32) -> f64 {\n    return x - x + f64(c);\n}\n\n",
        );
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

    pub fn shader_defines_function(shader_body: &str, func_name: &str) -> bool {
        let pattern1 = format!("fn {func_name}(");
        let pattern2 = format!("fn {func_name} (");
        shader_body.contains(&pattern1) || shader_body.contains(&pattern2)
    }

    pub fn shader_defines_module_var(shader_body: &str, var_name: &str) -> bool {
        for line in shader_body.lines() {
            let trimmed = line.trim();
            if (line.starts_with("let ") || line.starts_with("var ") || line.starts_with("const "))
                && (trimmed.contains(&format!("{var_name} "))
                    || trimmed.contains(&format!("{var_name}="))
                    || trimmed.contains(&format!("{var_name}:")))
            {
                return true;
            }
        }
        false
    }

    pub fn with_math_f64_safe(shader_body: &str) -> String {
        Self::inject_missing_math_f64(shader_body)
    }

    pub fn with_math_f64_auto_safe(shader_body: &str) -> String {
        Self::inject_missing_math_f64(shader_body)
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
        assert!(f32_shader.contains("vec4<f32>"));
        let f64_shader = ShaderTemplate::elementwise_add(Precision::F64);
        assert!(f64_shader.contains("array<f64>"));
        assert!(!f64_shader.contains("vec4"));
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
        let subset = ShaderTemplate::math_f64_subset(&["sqrt_f64"]);
        // sqrt_f64 is fossil — still extractable from the library for reference
        assert!(subset.contains("fn sqrt_f64"));
        assert!(subset.contains("fn f64_const"));
        assert!(!subset.contains("fn exp_f64"));
        assert!(!subset.contains("fn sin_f64"));
        let pow_subset = ShaderTemplate::math_f64_subset(&["pow_f64"]);
        assert!(pow_subset.contains("fn pow_f64"));
        assert!(pow_subset.contains("fn exp_f64"));
        assert!(pow_subset.contains("fn log_f64"));
        // abs_f64 is no longer a dep of pow_f64 — pow body uses native abs() directly
        assert!(!pow_subset.contains("fn abs_f64"));
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
        // sqrt_f64 is fossil but still in the library — with_math_f64_auto includes it
        assert!(full_shader.contains("fn sqrt_f64"));
        assert!(full_shader.contains("fn exp_f64"));
        // abs_f64 and round_f64 are no longer deps of exp_f64 — body uses native builtins
        assert!(!full_shader.contains("fn abs_f64"));
        assert!(!full_shader.contains("fn round_f64"));
        assert!(!full_shader.contains("fn sin_f64"));
        assert!(!full_shader.contains("fn gamma_f64"));
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
        assert!(full_shader.contains("output[id.x] = input[id.x] * 2.0"));
        assert!(!full_shader.contains("fn sqrt_f64"));
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
        let shader_space = r#"fn sqrt_f64 (x: f64) -> f64 { return x; }"#;
        assert!(ShaderTemplate::shader_defines_function(
            shader_space,
            "sqrt_f64"
        ));
    }

    #[test]
    fn test_shader_defines_module_var() {
        let shader_module_scope = r#"
let zero = 0.0;
fn main() { }
"#;
        assert!(ShaderTemplate::shader_defines_module_var(
            shader_module_scope,
            "zero"
        ));
        let shader_local = r#"
fn main() {
    let zero = x - x;
}
"#;
        assert!(!ShaderTemplate::shader_defines_module_var(
            shader_local,
            "zero"
        ));
        let shader_const = r#"
const EPSILON: f64 = 1e-15;
"#;
        assert!(ShaderTemplate::shader_defines_module_var(
            shader_const,
            "EPSILON"
        ));
    }

    #[test]
    fn test_safe_injects_only_called_functions() {
        // Fossil functions (sqrt_f64) are NOT injected — native sqrt() handles them.
        // Active fallbacks (cbrt_f64) ARE injected when called.
        let fossil_shader = r#"
            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                output[id.x] = sqrt_f64(input[id.x]);
            }
        "#;
        let fossil_result = ShaderTemplate::with_math_f64_safe(fossil_shader);
        assert!(
            !fossil_result.contains("fn sqrt_f64"),
            "fossil must not be injected"
        );
        assert!(
            !fossil_result.contains("fn f64_const"),
            "no injection means no preamble"
        );

        let active_shader = r#"
            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                output[id.x] = cbrt_f64(input[id.x]);
            }
        "#;
        let active_result = ShaderTemplate::with_math_f64_safe(active_shader);
        assert!(
            active_result.contains("fn cbrt_f64"),
            "active fallback must be injected"
        );
        assert!(active_result.contains("fn f64_const"));
        assert!(!active_result.contains("fn exp_f64"));
    }

    #[test]
    fn test_safe_no_injection_for_native_calls() {
        let shader = r#"
            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                output[id.x] = sqrt(input[id.x]);
            }
        "#;
        let result = ShaderTemplate::with_math_f64_safe(shader);
        assert!(!result.contains("fn f64_const"));
    }

    #[test]
    fn test_substitute_fossil_f64() {
        let legacy = "let y = sqrt_f64(x); let z = abs_f64(y); let w = min_f64(y, z);";
        let upgraded = ShaderTemplate::substitute_fossil_f64(legacy);
        assert!(upgraded.contains("sqrt(x)"));
        assert!(upgraded.contains("abs(y)"));
        assert!(upgraded.contains("min(y, z)"));
        assert!(!upgraded.contains("sqrt_f64("));
        assert!(!upgraded.contains("abs_f64("));
        assert!(!upgraded.contains("min_f64("));
        // Active fallbacks must NOT be touched
        let with_active = "let e = exp_f64(x); let c = cbrt_f64(x);";
        let result = ShaderTemplate::substitute_fossil_f64(with_active);
        assert!(result.contains("exp_f64("));
        assert!(result.contains("cbrt_f64("));
    }

    #[test]
    fn test_for_driver_auto_applies_fossil_substitution() {
        // for_driver_auto should substitute fossils AND apply exp/log workaround
        let legacy_shader = r#"
            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                let s = sqrt_f64(input[id.x]);
                let e = exp(s);
                output[id.x] = s + e;
            }
        "#;
        let result = ShaderTemplate::for_driver_auto(legacy_shader, true);
        // sqrt_f64 → sqrt (fossil substitution)
        assert!(
            result.contains("sqrt("),
            "fossil sqrt_f64 must become native sqrt"
        );
        assert!(!result.contains("sqrt_f64("), "fossil name must be gone");
        // exp → exp_f64 (workaround)
        assert!(result.contains("exp_f64("));
        assert!(
            result.contains("fn exp_f64"),
            "exp fallback must be injected"
        );
    }

    #[test]
    fn test_safe_partial_definitions_respected() {
        let shader = r#"
            fn f64_const(x: f64, c: f32) -> f64 {
                return x - x + f64(c);
            }

            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                output[id.x] = exp_f64(f64_const(input[id.x], 1.0));
            }
        "#;
        let result = ShaderTemplate::with_math_f64_safe(shader);
        assert_eq!(result.matches("fn f64_const").count(), 1);
        assert!(result.contains("fn exp_f64"));
    }

    #[test]
    fn test_safe_all_defined_no_injection() {
        let shader = r#"
            fn f64_const(x: f64, c: f32) -> f64 {
                return x - x + f64(c);
            }

            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                output[id.x] = f64_const(input[id.x], 1.0);
            }
        "#;
        let result = ShaderTemplate::with_math_f64_safe(shader);
        assert_eq!(result.matches("fn f64_const").count(), 1);
    }

    #[test]
    fn test_driver_workaround_with_partial_definitions() {
        let shader = r#"
            fn f64_const(x: f64, c: f32) -> f64 {
                return x - x + f64(c);
            }
            fn erfc_f64(x: f64) -> f64 {
                return f64_const(x, 1.0) - erf_f64(x);
            }
            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                let v = exp(-input[id.x]);
                output[id.x] = erfc_f64(v);
            }
        "#;
        let result = ShaderTemplate::for_driver_auto(shader, true);
        assert!(result.contains("exp_f64("));
        assert!(result.contains("fn exp_f64"));
        assert!(result.contains("fn erf_f64"));
        assert_eq!(result.matches("fn f64_const").count(), 1);
    }

    #[test]
    fn test_driver_workaround_disabled() {
        let shader = r#"
            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                output[id.x] = exp(input[id.x]);
            }
        "#;
        let result = ShaderTemplate::for_driver_auto(shader, false);
        assert!(result.contains("exp("));
        assert!(!result.contains("exp_f64("));
    }
}
