// msd_f64.wgsl — Mean Squared Displacement
//
// **Physics**: MSD(τ) = <|r(t+τ) - r(t)|²> — squared displacement with PBC.
// This kernel computes per-particle |r(τ) - r(0)|² with minimum-image convention.
//
// **Algorithm**: For each particle i:
//   dx = pbc_min_image(pos_t[i].x - pos_0[i].x, box_x)
//   dy = pbc_min_image(pos_t[i].y - pos_0[i].y, box_y)
//   dz = pbc_min_image(pos_t[i].z - pos_0[i].z, box_z)
//   output[i] = dx² + dy² + dz²
//
// **Precision**: f64 via bitcast<f64>(vec2<u32>)
// **Workgroup**: @compute @workgroup_size(256)
//
// Bindings:
//   0: pos_0   array<vec2<u32>>  read       — positions at t=0 [x0,y0,z0, x1,...]
//   1: pos_t   array<vec2<u32>>  read       — positions at lag t (same layout)
//   2: output  array<vec2<u32>>  read_write — per-particle |r(t)-r(0)|²
//
// Params: { n: u32, box_x, box_x_hi, box_y, box_y_hi, box_z, box_z_hi } (f64 as u32 pairs)
//
// Reference: Allen & Tildesley "Computer Simulation of Liquids"

@group(0) @binding(0) var<storage, read> pos_0: array<vec2<u32>>;
@group(0) @binding(1) var<storage, read> pos_t: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> output: array<vec2<u32>>;
@group(0) @binding(3) var<uniform> params: Params;

struct Params {
    n: u32,
    box_x: u32,
    box_x_hi: u32,
    box_y: u32,
    box_y_hi: u32,
    box_z: u32,
    box_z_hi: u32,
}

fn unpack_f64(lo: u32, hi: u32) -> f64 {
    return bitcast<f64>(vec2<u32>(lo, hi));
}

// Minimum image: delta - box * round(delta / box)
fn pbc_min_image(delta: f64, box_size: f64) -> f64 {
    return delta - box_size * round(delta / box_size);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let n = params.n;
    if (i >= n) {
        return;
    }

    let base = i * 3u;
    let x0 = bitcast<f64>(pos_0[base]);
    let y0 = bitcast<f64>(pos_0[base + 1u]);
    let z0 = bitcast<f64>(pos_0[base + 2u]);
    let xt = bitcast<f64>(pos_t[base]);
    let yt = bitcast<f64>(pos_t[base + 1u]);
    let zt = bitcast<f64>(pos_t[base + 2u]);

    let box_x = unpack_f64(params.box_x, params.box_x_hi);
    let box_y = unpack_f64(params.box_y, params.box_y_hi);
    let box_z = unpack_f64(params.box_z, params.box_z_hi);

    let dx = pbc_min_image(xt - x0, box_x);
    let dy = pbc_min_image(yt - y0, box_y);
    let dz = pbc_min_image(zt - z0, box_z);

    let msd_val = dx * dx + dy * dy + dz * dz;
    output[i] = bitcast<vec2<u32>>(msd_val);
}
