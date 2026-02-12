// Pearson correlation coefficient for batch vectors
// r = Σ(x-μx)(y-μy) / (σx·σy·n)
//
// Input: two vectors x and y of same length
// Output: single correlation coefficient
//
// Applications: signal correlation, feature selection, portfolio analysis
// Reference: Standard statistical formula

@group(0) @binding(0) var<storage, read> x: array<f32>;
@group(0) @binding(1) var<storage, read> y: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

struct Params {
    size: u32,        // Length of each vector
    num_pairs: u32,   // Number of (x,y) vector pairs
    stride: u32,      // Stride between vectors (typically == size)
}

// Compute mean of a vector slice
fn compute_mean(data: ptr<storage, array<f32>, read>, offset: u32, size: u32) -> f32 {
    var sum: f32 = 0.0;
    for (var i = 0u; i < size; i = i + 1u) {
        sum = sum + data[offset + i];
    }
    return sum / f32(size);
}

// Compute Pearson correlation between two vectors
fn pearson_correlation(
    x_data: ptr<storage, array<f32>, read>,
    y_data: ptr<storage, array<f32>, read>,
    x_offset: u32,
    y_offset: u32,
    size: u32
) -> f32 {
    // Compute means
    var sum_x: f32 = 0.0;
    var sum_y: f32 = 0.0;
    for (var i = 0u; i < size; i = i + 1u) {
        sum_x = sum_x + x_data[x_offset + i];
        sum_y = sum_y + y_data[y_offset + i];
    }
    let mean_x = sum_x / f32(size);
    let mean_y = sum_y / f32(size);

    // Compute covariance and standard deviations
    var cov: f32 = 0.0;
    var var_x: f32 = 0.0;
    var var_y: f32 = 0.0;

    for (var i = 0u; i < size; i = i + 1u) {
        let dx = x_data[x_offset + i] - mean_x;
        let dy = y_data[y_offset + i] - mean_y;
        cov = cov + dx * dy;
        var_x = var_x + dx * dx;
        var_y = var_y + dy * dy;
    }

    let denom = sqrt(var_x * var_y);
    if (denom < 1e-10) {
        return 0.0;  // Avoid division by zero
    }

    return cov / denom;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.num_pairs) {
        return;
    }

    let x_offset = idx * params.stride;
    let y_offset = idx * params.stride;

    output[idx] = pearson_correlation(&x, &y, x_offset, y_offset, params.size);
}
