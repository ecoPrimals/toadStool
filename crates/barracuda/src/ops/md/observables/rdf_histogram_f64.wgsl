// RDF Pair Distance Histogram — f64 precision
//
// **Physics**: Radial distribution function g(r)
// **Algorithm**: All-pairs distance with PBC, bin into histogram
// **Use Case**: Structure analysis, validation vs Sarkas
//
// **f64 precision**: f64 positions, u32 histogram bins
//
// Note: Uses atomicAdd for histogram — requires workgroup synchronization
//
// Bindings:
//   0: positions   [N*3] f64, read
//   1: histogram   [n_bins] atomic<u32>, read-write
//   2: params      uniform

@group(0) @binding(0) var<storage, read> positions: array<f64>;
@group(0) @binding(1) var<storage, read_write> histogram: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> params: Params;

struct Params {
    n_particles: u32,
    n_bins: u32,
    dr: f64,        // Bin width
    box_x: f64,
    box_y: f64,
    box_z: f64,
}

fn pbc_delta(delta: f64, box_size: f64) -> f64 {
    return delta - box_size * round(delta / box_size);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    let n = params.n_particles;
    let n_bins = params.n_bins;
    let dr = params.dr;
    let box_x = params.box_x;
    let box_y = params.box_y;
    let box_z = params.box_z;

    if (i >= n) { return; }

    let xi = positions[i * 3u];
    let yi = positions[i * 3u + 1u];
    let zi = positions[i * 3u + 2u];

    // Only count pairs where j > i to avoid double-counting
    for (var j = i + 1u; j < n; j = j + 1u) {
        let dx = pbc_delta(positions[j * 3u]      - xi, box_x);
        let dy = pbc_delta(positions[j * 3u + 1u] - yi, box_y);
        let dz = pbc_delta(positions[j * 3u + 2u] - zi, box_z);

        let r = sqrt(dx * dx + dy * dy + dz * dz);
        let bin = u32(r / dr);

        if (bin < n_bins) {
            atomicAdd(&histogram[bin], 1u);
        }
    }
}

// Normalize histogram to g(r)
// g(r) = histogram[bin] * V / (N * N_pairs_in_shell)
// where N_pairs_in_shell = 4π r² dr * ρ
@compute @workgroup_size(256)
fn normalize(@builtin(global_invocation_id) gid: vec3<u32>) {
    let bin = gid.x;
    if (bin >= params.n_bins) { return; }
    
    // This kernel would need additional output buffer for g(r)
    // Left as placeholder - normalization typically done on CPU
}
