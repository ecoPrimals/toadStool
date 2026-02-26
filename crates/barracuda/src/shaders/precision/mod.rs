//! Generic Precision Shader System
//!
//! Provides compile-time and runtime shader generation for any precision type.
//! ONE template → shaders for f16, f32, f64, and CPU implementations.

pub mod cpu;
mod math_f64;
mod templates;

use templates::{
    remove_conditional_block, TEMPLATE_DOT_PRODUCT, TEMPLATE_ELEMENTWISE_ABS,
    TEMPLATE_ELEMENTWISE_ADD, TEMPLATE_ELEMENTWISE_CLAMP, TEMPLATE_ELEMENTWISE_FMA,
    TEMPLATE_ELEMENTWISE_MUL, TEMPLATE_ELEMENTWISE_NEG, TEMPLATE_ELEMENTWISE_SUB,
    TEMPLATE_MAE_LOSS, TEMPLATE_MSE_LOSS, TEMPLATE_REDUCE_MEAN, TEMPLATE_REDUCE_SUM,
    TEMPLATE_SAXPY,
};

use math_f64::{
    extract_wgsl_function, F64_FOSSIL_FUNCTIONS, F64_FUNCTION_DEPS, F64_FUNCTION_ORDER,
};

/// Supported precision types.
///
/// Math is universal — precision is a silicon detail. The same algorithm runs
/// at every precision; the compilation pipeline (`compile_shader_universal`)
/// handles type specialization, polyfill injection, and driver patching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Precision {
    /// 16-bit float (half precision) — inference, 2× memory bandwidth
    F16,
    /// 32-bit float (single precision) — default, widely supported
    F32,
    /// 64-bit float (double precision) — scientific computing
    F64,
    /// Double-float f32-pair (~48-bit mantissa, ~14 decimal digits) —
    /// unleashes FP32 cores for f64-class work. 9.9× throughput vs native
    /// f64 on consumer GPUs.
    Df64,
}

impl Precision {
    /// WGSL scalar type name
    pub fn scalar(&self) -> &'static str {
        match self {
            Precision::F16 => "f16",
            Precision::F32 => "f32",
            Precision::F64 => "f64",
            Precision::Df64 => "vec2<f32>",
        }
    }

    /// WGSL vec2 type name (or scalar for f64 which lacks native vec support)
    pub fn vec2(&self) -> &'static str {
        match self {
            Precision::F16 => "vec2<f16>",
            Precision::F32 => "vec2<f32>",
            Precision::F64 => "f64",
            Precision::Df64 => "vec2<f32>",
        }
    }

    /// WGSL vec4 type name (or scalar for f64/df64)
    pub fn vec4(&self) -> &'static str {
        match self {
            Precision::F16 => "vec4<f16>",
            Precision::F32 => "vec4<f32>",
            Precision::F64 => "f64",
            Precision::Df64 => "vec2<f32>",
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
            Precision::Df64 => 8,
        }
    }

    /// Required wgpu feature for this precision
    pub fn required_feature(&self) -> Option<wgpu::Features> {
        match self {
            Precision::F16 => Some(wgpu::Features::SHADER_F16),
            Precision::F32 => None,
            Precision::F64 => Some(wgpu::Features::SHADER_F64),
            Precision::Df64 => None,
        }
    }

    /// Whether this is an f64-class precision (native f64 or df64 emulation)
    pub fn is_f64_class(&self) -> bool {
        matches!(self, Precision::F64 | Precision::Df64)
    }

    /// Generate the operation preamble for this precision.
    ///
    /// The preamble defines abstract operations (`op_add`, `op_mul`, etc.)
    /// whose implementation varies per precision. Shaders written against
    /// these ops are truly universal — math is the same, precision is silicon.
    ///
    /// For f32/f64: trivial inline wrappers around native operators.
    /// For DF64: routes to df64_add/df64_mul/etc from the DF64 core library.
    ///
    /// All preambles provide identity `op_pack`/`op_unpack` for uniform
    /// array access patterns. DF64 uses these for `vec2<f32>` ↔ `Df64`
    /// conversion; other precisions are identity (compiler eliminates them).
    pub fn op_preamble(&self) -> &'static str {
        match self {
            Precision::F32 => OP_PREAMBLE_F32,
            Precision::F64 => OP_PREAMBLE_F64,
            Precision::Df64 => OP_PREAMBLE_DF64,
            Precision::F16 => OP_PREAMBLE_F16,
        }
    }
}

/// f32 operation preamble — trivial wrappers, compiler inlines everything.
const OP_PREAMBLE_F32: &str = r#"
// Universal operation preamble — f32 precision
alias Scalar = f32;
fn op_add(a: f32, b: f32) -> f32 { return a + b; }
fn op_sub(a: f32, b: f32) -> f32 { return a - b; }
fn op_mul(a: f32, b: f32) -> f32 { return a * b; }
fn op_div(a: f32, b: f32) -> f32 { return a / b; }
fn op_neg(a: f32) -> f32 { return -a; }
fn op_abs(a: f32) -> f32 { return abs(a); }
fn op_max(a: f32, b: f32) -> f32 { return max(a, b); }
fn op_min(a: f32, b: f32) -> f32 { return min(a, b); }
fn op_gt(a: f32, b: f32) -> bool { return a > b; }
fn op_lt(a: f32, b: f32) -> bool { return a < b; }
fn op_ge(a: f32, b: f32) -> bool { return a >= b; }
fn op_le(a: f32, b: f32) -> bool { return a <= b; }
fn op_from_f32(v: f32) -> f32 { return v; }
fn op_zero() -> f32 { return 0.0; }
fn op_one() -> f32 { return 1.0; }
fn op_pack(v: f32) -> f32 { return v; }
fn op_unpack(v: f32) -> f32 { return v; }
"#;

/// f64 operation preamble — same structure, f64 types.
const OP_PREAMBLE_F64: &str = r#"
// Universal operation preamble — f64 precision
alias Scalar = f64;
fn op_add(a: f64, b: f64) -> f64 { return a + b; }
fn op_sub(a: f64, b: f64) -> f64 { return a - b; }
fn op_mul(a: f64, b: f64) -> f64 { return a * b; }
fn op_div(a: f64, b: f64) -> f64 { return a / b; }
fn op_neg(a: f64) -> f64 { return -a; }
fn op_abs(a: f64) -> f64 { return abs(a); }
fn op_max(a: f64, b: f64) -> f64 { return max(a, b); }
fn op_min(a: f64, b: f64) -> f64 { return min(a, b); }
fn op_gt(a: f64, b: f64) -> bool { return a > b; }
fn op_lt(a: f64, b: f64) -> bool { return a < b; }
fn op_ge(a: f64, b: f64) -> bool { return a >= b; }
fn op_le(a: f64, b: f64) -> bool { return a <= b; }
fn op_from_f32(v: f32) -> f64 { return f64(v); }
fn op_zero() -> f64 { return f64(0.0); }
fn op_one() -> f64 { return f64(1.0); }
fn op_pack(v: f64) -> f64 { return v; }
fn op_unpack(v: f64) -> f64 { return v; }
"#;

/// DF64 operation preamble — routes to df64_core library functions.
/// Requires df64_core.wgsl + df64_transcendentals.wgsl prepended.
const OP_PREAMBLE_DF64: &str = r#"
// Universal operation preamble — DF64 precision (f32-pair, ~48-bit mantissa)
alias Scalar = Df64;
alias StorageType = vec2<f32>;
fn op_add(a: Df64, b: Df64) -> Df64 { return df64_add(a, b); }
fn op_sub(a: Df64, b: Df64) -> Df64 { return df64_sub(a, b); }
fn op_mul(a: Df64, b: Df64) -> Df64 { return df64_mul(a, b); }
fn op_div(a: Df64, b: Df64) -> Df64 { return df64_div(a, b); }
fn op_neg(a: Df64) -> Df64 { return df64_neg(a); }
fn op_abs(a: Df64) -> Df64 { return df64_abs(a); }
fn op_max(a: Df64, b: Df64) -> Df64 { if df64_gt(a, b) { return a; } return b; }
fn op_min(a: Df64, b: Df64) -> Df64 { if df64_lt(a, b) { return a; } return b; }
fn op_gt(a: Df64, b: Df64) -> bool { return df64_gt(a, b); }
fn op_lt(a: Df64, b: Df64) -> bool { return df64_lt(a, b); }
fn op_ge(a: Df64, b: Df64) -> bool { return !df64_lt(a, b); }
fn op_le(a: Df64, b: Df64) -> bool { return !df64_gt(a, b); }
fn op_from_f32(v: f32) -> Df64 { return df64_from_f32(v); }
fn op_zero() -> Df64 { return df64_zero(); }
fn op_one() -> Df64 { return df64_from_f32(1.0); }
fn op_pack(v: Df64) -> vec2<f32> { return vec2<f32>(v.hi, v.lo); }
fn op_unpack(v: vec2<f32>) -> Df64 { return Df64(v.x, v.y); }
"#;

/// f16 operation preamble — trivial wrappers.
const OP_PREAMBLE_F16: &str = r#"
// Universal operation preamble — f16 precision
alias Scalar = f16;
fn op_add(a: f16, b: f16) -> f16 { return a + b; }
fn op_sub(a: f16, b: f16) -> f16 { return a - b; }
fn op_mul(a: f16, b: f16) -> f16 { return a * b; }
fn op_div(a: f16, b: f16) -> f16 { return a / b; }
fn op_neg(a: f16) -> f16 { return -a; }
fn op_abs(a: f16) -> f16 { return abs(a); }
fn op_max(a: f16, b: f16) -> f16 { return max(a, b); }
fn op_min(a: f16, b: f16) -> f16 { return min(a, b); }
fn op_gt(a: f16, b: f16) -> bool { return a > b; }
fn op_lt(a: f16, b: f16) -> bool { return a < b; }
fn op_ge(a: f16, b: f16) -> bool { return a >= b; }
fn op_le(a: f16, b: f16) -> bool { return a <= b; }
fn op_from_f32(v: f32) -> f16 { return f16(v); }
fn op_zero() -> f16 { return f16(0.0); }
fn op_one() -> f16 { return f16(1.0); }
fn op_pack(v: f16) -> f16 { return v; }
fn op_unpack(v: f16) -> f16 { return v; }
"#;

/// Downcast an f64 shader source to f32 via text substitution.
///
/// This is the core of "math is universal, precision is silicon": the shader
/// is written once in f64 (the conceptually true math), and this function
/// produces the f32 variant by replacing type declarations. Only safe for
/// shaders that use basic arithmetic (`+`, `-`, `*`, `/`, `fma`). Shaders
/// with f64 polyfill calls (`exp_f64`, `sin_f64`, etc.) need
/// `downcast_f64_to_f32_with_transcendentals` instead.
pub fn downcast_f64_to_f32(f64_source: &str) -> String {
    // Protect _f64( function-name suffixes from the f64( cast replacement.
    // WGSL uses f64(...) for type casts, but _f64( appears in polyfill names.
    let result = f64_source
        .replace("_f64(", "\x00_F64_CALL\x00")
        .replace("array<f64>", "array<f32>")
        .replace("array<f64,", "array<f32,")
        .replace(": f64", ": f32")
        .replace("-> f64", "-> f32")
        .replace("f64(", "f32(")
        .replace("<f64>", "<f32>")
        .replace("\x00_F64_CALL\x00", "_f64(");

    clamp_f64_range_literals(&result)
}

/// Replace f64-range sentinel literals with f32-safe equivalents.
///
/// f64 canonical shaders use values like `-1e308` or `1.7976931348623157e+308`
/// as min/max initialization sentinels. These exceed f32 range (~3.4e38) and
/// cause WGSL parse errors when downcasted. We replace them with the
/// corresponding f32 extremes.
fn clamp_f64_range_literals(source: &str) -> String {
    source
        .replace("-1.7976931348623157e+308", "-3.4028235e+38")
        .replace("1.7976931348623157e+308", "3.4028235e+38")
        .replace("-1.0e308", "-3.4028235e+38")
        .replace("1.0e308", "3.4028235e+38")
        .replace("-1e308", "-3.4028235e+38")
        .replace("1e308", "3.4028235e+38")
        .replace("-1.0e300", "-3.4028235e+38")
        .replace("1.0e300", "3.4028235e+38")
        .replace("-1e300", "-3.4028235e+38")
        .replace("1e300", "3.4028235e+38")
}

/// Downcast an f64 shader source to f16 via text substitution.
///
/// Same sentinel protection and literal clamping as the f32 downcast.
/// f16 range is ~65504 so f64-range sentinels need aggressive clamping.
pub fn downcast_f64_to_f16(f64_source: &str) -> String {
    let result = f64_source
        .replace("_f64(", "\x00_F64_CALL\x00")
        .replace("array<f64>", "array<f16>")
        .replace("array<f64,", "array<f16,")
        .replace(": f64", ": f16")
        .replace("-> f64", "-> f16")
        .replace("f64(", "f16(")
        .replace("<f64>", "<f16>")
        .replace("\x00_F64_CALL\x00", "_f64(");

    clamp_f64_range_literals_f16(&result)
}

/// Replace f64-range sentinel literals with f16-safe equivalents.
/// f16 max is ~65504.
fn clamp_f64_range_literals_f16(source: &str) -> String {
    source
        .replace("-1.7976931348623157e+308", "-65504.0")
        .replace("1.7976931348623157e+308", "65504.0")
        .replace("-1.0e308", "-65504.0")
        .replace("1.0e308", "65504.0")
        .replace("-1e308", "-65504.0")
        .replace("1e308", "65504.0")
        .replace("-1.0e300", "-65504.0")
        .replace("1.0e300", "65504.0")
        .replace("-1e300", "-65504.0")
        .replace("1e300", "65504.0")
        .replace("-3.4028235e+38", "-65504.0")
        .replace("3.4028235e+38", "65504.0")
}

/// Downcast an f64 shader source to f32, also replacing polyfill
/// transcendental calls with native WGSL builtins.
///
/// `exp_f64(x)` → `exp(x)`, `sin_f64(x)` → `sin(x)`, etc.
/// Use for shaders that call math_f64 polyfill functions.
pub fn downcast_f64_to_f32_with_transcendentals(f64_source: &str) -> String {
    let base = downcast_f64_to_f32(f64_source);
    base.replace("exp_f64(", "exp(")
        .replace("log_f64(", "log(")
        .replace("pow_f64(", "pow(")
        .replace("sin_f64(", "sin(")
        .replace("cos_f64(", "cos(")
        .replace("tan_f64(", "tan(")
        .replace("asin_f64(", "asin(")
        .replace("acos_f64(", "acos(")
        .replace("atan_f64(", "atan(")
        .replace("atan2_f64(", "atan2(")
        .replace("sinh_f64(", "sinh(")
        .replace("cosh_f64(", "cosh(")
        .replace("tanh_f64(", "tanh(")
        .replace("sqrt_f64(", "sqrt(")
        .replace("abs_f64(", "abs(")
        .replace("erf_f64(", "erf(")
}

/// Transform an f64 shader source to DF64 (f32-pair) representation.
///
/// Handles:
/// - Storage types: `array<f64>` → `array<vec2<f32>>`
/// - Type declarations: `: f64` → `: Df64`, `-> f64` → `-> Df64`
/// - Constructors: `f64(X)` → `df64_from_f32(X)` for literal casts
/// - Transcendentals: `exp_f64(` → `exp_df64(`, `sin_f64(` → `sin_df64(`, etc.
/// - Polyfill builtins: `abs_f64(` → `df64_abs(`, `sqrt_f64(` → `sqrt_df64(`
/// - Sentinels: f64-range literals clamped to f32-range (same as f32 downcast)
///
/// Does NOT handle infix arithmetic operators (`+`, `-`, `*`, `/`) between
/// f64 values — these require naga-IR-based rewriting (see `df64_rewrite` module).
/// Shaders that only use `_f64()` function calls work fully with this transform.
///
/// The caller must compile through `compile_shader_df64()` which prepends the
/// DF64 core library (df64_core.wgsl + df64_transcendentals.wgsl).
pub fn downcast_f64_to_df64(f64_source: &str) -> String {
    let result = f64_source
        // Protect function-name _f64( from constructor replacement
        .replace("_f64(", "\x00_F64_CALL\x00")
        // Storage: array<f64> → array<vec2<f32>> (DF64 wire format)
        .replace("array<f64>", "array<vec2<f32>>")
        .replace("array<f64,", "array<vec2<f32>,")
        // Type declarations: f64 → Df64
        .replace(": f64", ": Df64")
        .replace("-> f64", "-> Df64")
        // Generic angle brackets
        .replace("<f64>", "<Df64>")
        // Constructor: f64(X) → df64_from_f32(X)
        // df64_from_f32 takes an f32 arg; WGSL auto-converts abstract literals
        .replace("f64(", "df64_from_f32(")
        // Restore function-name suffixes
        .replace("\x00_F64_CALL\x00", "_f64(");

    let with_transcendentals = result
        // Transcendentals with DF64 implementations in df64_transcendentals.wgsl:
        .replace("exp_f64(", "exp_df64(")
        .replace("log_f64(", "log_df64(")
        .replace("pow_f64(", "pow_df64(")
        .replace("sin_f64(", "sin_df64(")
        .replace("cos_f64(", "cos_df64(")
        .replace("tanh_f64(", "tanh_df64(")
        .replace("sqrt_f64(", "sqrt_df64(")
        .replace("abs_f64(", "df64_abs(");

    // Clamp f64-range sentinels to f32-range (DF64 uses f32 components)
    clamp_f64_range_literals(&with_transcendentals)
}

/// Inject DF64 pack/unpack helpers for array load/store patterns.
///
/// Converts:
/// - `let x: Df64 = arr[i]` → `let x: Df64 = Df64(arr[i].x, arr[i].y)`
/// - Adds pack helper: `fn df64_pack(v: Df64) -> vec2<f32> { return vec2<f32>(v.hi, v.lo); }`
///
/// This is injected into the shader source after the DF64 core library.
pub const DF64_PACK_UNPACK: &str = r#"
fn df64_pack(v: Df64) -> vec2<f32> { return vec2<f32>(v.hi, v.lo); }
fn df64_unpack(v: vec2<f32>) -> Df64 { return Df64(v.x, v.y); }
"#;

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

    pub fn elementwise_sub(precision: Precision) -> String {
        Self::new(TEMPLATE_ELEMENTWISE_SUB).render(precision)
    }

    pub fn elementwise_abs(precision: Precision) -> String {
        Self::new(TEMPLATE_ELEMENTWISE_ABS).render(precision)
    }

    pub fn elementwise_neg(precision: Precision) -> String {
        Self::new(TEMPLATE_ELEMENTWISE_NEG).render(precision)
    }

    pub fn elementwise_clamp(precision: Precision) -> String {
        Self::new(TEMPLATE_ELEMENTWISE_CLAMP).render(precision)
    }

    pub fn reduce_sum(precision: Precision) -> String {
        Self::new(TEMPLATE_REDUCE_SUM).render(precision)
    }

    pub fn reduce_mean(precision: Precision) -> String {
        Self::new(TEMPLATE_REDUCE_MEAN).render(precision)
    }

    pub fn mse_loss(precision: Precision) -> String {
        Self::new(TEMPLATE_MSE_LOSS).render(precision)
    }

    pub fn mae_loss(precision: Precision) -> String {
        Self::new(TEMPLATE_MAE_LOSS).render(precision)
    }

    pub fn saxpy(precision: Precision) -> String {
        Self::new(TEMPLATE_SAXPY).render(precision)
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
#[path = "precision_tests.rs"]
mod tests;
