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

#[test]
fn test_precision_df64() {
    assert_eq!(Precision::Df64.scalar(), "vec2<f32>");
    assert_eq!(Precision::Df64.bytes_per_element(), 8);
    assert!(!Precision::Df64.has_vec4());
    assert!(Precision::Df64.required_feature().is_none());
    assert!(Precision::Df64.is_f64_class());
    assert!(Precision::F64.is_f64_class());
    assert!(!Precision::F32.is_f64_class());
}

#[test]
fn test_downcast_f64_to_f32_elementwise() {
    let f64_source = r#"
@group(0) @binding(0) var<storage, read> a: array<f64>;
@group(0) @binding(1) var<storage, read> b: array<f64>;
@group(0) @binding(2) var<storage, read_write> output: array<f64>;

var<workgroup> shared: array<f64, 256>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&output)) { return; }
    output[idx] = a[idx] + b[idx];
}
"#;
    let f32_source = downcast_f64_to_f32(f64_source);
    assert!(f32_source.contains("array<f32>"));
    assert!(!f32_source.contains("array<f64>"));
    assert!(f32_source.contains("array<f32, 256>"));
    assert!(f32_source.contains("a[idx] + b[idx]"));
}

#[test]
fn test_downcast_f64_to_f32_with_transcendentals() {
    let f64_source = r#"
@group(0) @binding(0) var<storage, read> input: array<f64>;
@group(0) @binding(1) var<storage, read_write> output: array<f64>;
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    output[gid.x] = exp_f64(input[gid.x]) + sin_f64(input[gid.x]);
}
"#;
    let f32_source = downcast_f64_to_f32_with_transcendentals(f64_source);
    assert!(f32_source.contains("array<f32>"));
    assert!(f32_source.contains("exp(input"));
    assert!(f32_source.contains("sin(input"));
    assert!(!f32_source.contains("exp_f64"));
    assert!(!f32_source.contains("sin_f64"));
}

#[test]
fn test_downcast_preserves_u32_and_structure() {
    let f64_source = r#"
struct Params { size: u32, _pad1: u32, _pad2: u32, _pad3: u32, }
@group(0) @binding(0) var<storage, read> input: array<f64>;
@group(0) @binding(1) var<uniform> params: Params;
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.size) { return; }
    let val: f64 = input[gid.x];
}
"#;
    let f32_source = downcast_f64_to_f32(f64_source);
    assert!(f32_source.contains("size: u32"));
    assert!(f32_source.contains("vec3<u32>"));
    assert!(f32_source.contains("let val: f32"));
    assert!(f32_source.contains("array<f32>"));
}

#[test]
fn test_template_renders_df64() {
    let shader = ShaderTemplate::elementwise_add(Precision::Df64);
    assert!(shader.contains("array<vec2<f32>>"));
    assert!(!shader.contains("vec4"));
}
