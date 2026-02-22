// cubic_spline_eval_f64.wgsl — Evaluate cubic spline at query points
//
// **Math**: Cubic spline on segments [x_i, x_{i+1}]: s(x) = a + b*dx + c*dx² + d*dx³
// where dx = x - x_i. Coefficients [a,b,c,d] per segment stored in order.
//
// **Algorithm**: For each query x, binary search for segment i such that knots[i] <= x < knots[i+1],
// then evaluate s(x) = coef[4*i] + coef[4*i+1]*dx + coef[4*i+2]*dx² + coef[4*i+3]*dx³
//
// **Precision**: f64 via bitcast<f64>(vec2<u32>)
// **Workgroup**: @compute @workgroup_size(256)
//
// Bindings:
//   0: query_x    array<vec2<u32>>  read       — query x values
//   1: knots      array<vec2<u32>>  read       — knot x array (n_segments + 1 values)
//   2: coefs      array<vec2<u32>>  read       — [a,b,c,d] per segment, 4*n_segments f64s
//   3: result     array<vec2<u32>>  read_write — interpolated y values
//
// Params: { n_query: u32, n_segments: u32 }
//
// Applications: Interpolation, curve fitting, smooth trajectories.
// Reference: De Boor "A Practical Guide to Splines"

@group(0) @binding(0) var<storage, read> query_x: array<vec2<u32>>;
@group(0) @binding(1) var<storage, read> knots: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read> coefs: array<vec2<u32>>;
@group(0) @binding(3) var<storage, read_write> result: array<vec2<u32>>;
@group(0) @binding(4) var<uniform> params: Params;

struct Params {
    n_query: u32,
    n_segments: u32,
}

// Binary search: find segment index i where knots[i] <= x < knots[i+1]
// Returns n_segments if x >= last knot (extrapolate with last segment)
fn find_segment(x: f64, n_segments: u32) -> u32 {
    if (n_segments == 0u) { return 0u; }
    var lo = 0u;
    var hi = n_segments;
    while (lo + 1u < hi) {
        let mid = (lo + hi) / 2u;
        let knot_mid = bitcast<f64>(knots[mid]);
        if (x >= knot_mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let knot_lo = bitcast<f64>(knots[lo]);
    if (x < knot_lo && lo > 0u) {
        return lo - 1u;
    }
    return lo;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let n_query = params.n_query;
    let n_segments = params.n_segments;
    if (i >= n_query) {
        return;
    }

    let x = bitcast<f64>(query_x[i]);
    let seg = find_segment(x, n_segments);

    // Clamp segment (query may be outside knot range)
    let seg_clamped = min(seg, n_segments - 1u);
    let x0 = bitcast<f64>(knots[seg_clamped]);
    let dx = x - x0;

    // Load coefficients: [a, b, c, d] at coefs[4*seg_clamped .. 4*seg_clamped+3]
    let base = seg_clamped * 4u;
    let a = bitcast<f64>(coefs[base]);
    let b = bitcast<f64>(coefs[base + 1u]);
    let c = bitcast<f64>(coefs[base + 2u]);
    let d = bitcast<f64>(coefs[base + 3u]);

    // s(x) = a + b*dx + c*dx² + d*dx³
    let dx2 = dx * dx;
    let dx3 = dx2 * dx;
    let y = a + b * dx + c * dx2 + d * dx3;

    result[i] = bitcast<vec2<u32>>(y);
}
