// Covariance and variance computation
// Cov(X,Y) = E[(X-μx)(Y-μy)] = E[XY] - E[X]E[Y]
//
// Input: two vectors x and y of same length
// Output: covariance value
//
// Applications: portfolio theory, PCA, Kalman filters
// Reference: Standard statistical formula

@group(0) @binding(0) var<storage, read> x: array<f32>;
@group(0) @binding(1) var<storage, read> y: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

struct Params {
    size: u32,        // Length of each vector
    num_pairs: u32,   // Number of (x,y) vector pairs
    stride: u32,      // Stride between vectors
    ddof: u32,        // Delta degrees of freedom (0 for population, 1 for sample)
}

// Compute covariance between two vectors (single pass Welford variant)
fn covariance(
    x_data: ptr<storage, array<f32>, read>,
    y_data: ptr<storage, array<f32>, read>,
    x_offset: u32,
    y_offset: u32,
    size: u32,
    ddof: u32
) -> f32 {
    if (size <= ddof) {
        return 0.0;
    }

    // Two-pass for numerical stability
    // Pass 1: compute means
    var sum_x: f32 = 0.0;
    var sum_y: f32 = 0.0;
    for (var i = 0u; i < size; i = i + 1u) {
        sum_x = sum_x + x_data[x_offset + i];
        sum_y = sum_y + y_data[y_offset + i];
    }
    let mean_x = sum_x / f32(size);
    let mean_y = sum_y / f32(size);

    // Pass 2: compute covariance
    var cov_sum: f32 = 0.0;
    for (var i = 0u; i < size; i = i + 1u) {
        let dx = x_data[x_offset + i] - mean_x;
        let dy = y_data[y_offset + i] - mean_y;
        cov_sum = cov_sum + dx * dy;
    }

    return cov_sum / f32(size - ddof);
}

// Compute variance of a single vector
fn variance(
    data: ptr<storage, array<f32>, read>,
    offset: u32,
    size: u32,
    ddof: u32
) -> f32 {
    if (size <= ddof) {
        return 0.0;
    }

    // Pass 1: mean
    var sum: f32 = 0.0;
    for (var i = 0u; i < size; i = i + 1u) {
        sum = sum + data[offset + i];
    }
    let mean = sum / f32(size);

    // Pass 2: variance
    var var_sum: f32 = 0.0;
    for (var i = 0u; i < size; i = i + 1u) {
        let d = data[offset + i] - mean;
        var_sum = var_sum + d * d;
    }

    return var_sum / f32(size - ddof);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.num_pairs) {
        return;
    }

    let x_offset = idx * params.stride;
    let y_offset = idx * params.stride;

    output[idx] = covariance(&x, &y, x_offset, y_offset, params.size, params.ddof);
}
