// Morse Force Kernel
//
// **Purpose**: Bonded interactions (chemical bonds)
// **Use Case**: Molecular mechanics, reactive MD, bond stretching
// **Range**: Short-range (anharmonic bonded potential)
//
// **Potential**: U(r) = D[1 - exp(-a(r-r0))]^2
// **Force**: F = 2Da[1 - exp(-a(r-r0))] * exp(-a(r-r0)) * r_hat
//   where D = bond dissociation energy
//         a = width parameter
//         r0 = equilibrium bond distance

@group(0) @binding(0) var<storage, read> positions: array<f32>;
@group(0) @binding(1) var<storage, read> bond_pairs: array<u32>;  // [N_bonds, 2]
@group(0) @binding(2) var<storage, read> bond_params: array<f32>; // [N_bonds, 3] (D, a, r0)
@group(0) @binding(3) var<storage, read_write> forces: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

struct Params {
    n_bonds: u32,
    pad1: f32,
    pad2: f32,
    pad3: f32,
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let bond_idx = global_id.x;
    
    if (bond_idx >= params.n_bonds) {
        return;
    }
    
    // Load bond pair
    let i = bond_pairs[bond_idx * 2u];
    let j = bond_pairs[bond_idx * 2u + 1u];
    
    // Load positions
    let pos_i = vec3<f32>(
        positions[i * 3u],
        positions[i * 3u + 1u],
        positions[i * 3u + 2u]
    );
    let pos_j = vec3<f32>(
        positions[j * 3u],
        positions[j * 3u + 1u],
        positions[j * 3u + 2u]
    );
    
    // Load bond parameters
    let D = bond_params[bond_idx * 3u];
    let a = bond_params[bond_idx * 3u + 1u];
    let r0 = bond_params[bond_idx * 3u + 2u];
    
    // Compute distance
    let r_vec = pos_j - pos_i;
    let r = length(r_vec);
    
    if (r < 1e-6) {
        return;
    }
    
    // Morse force: F = 2Da[1 - exp(-a(r-r0))] * exp(-a(r-r0)) * r_hat
    let delta_r = r - r0;
    let exp_term = exp(-a * delta_r);
    let force_magnitude = 2.0 * D * a * (1.0 - exp_term) * exp_term;
    
    let r_hat = r_vec / r;
    let force_vec = force_magnitude * r_hat;
    
    // Apply forces (Newton's third law)
    atomicAdd(&forces[i * 3u], force_vec.x);
    atomicAdd(&forces[i * 3u + 1u], force_vec.y);
    atomicAdd(&forces[i * 3u + 2u], force_vec.z);
    
    atomicAdd(&forces[j * 3u], -force_vec.x);
    atomicAdd(&forces[j * 3u + 1u], -force_vec.y);
    atomicAdd(&forces[j * 3u + 2u], -force_vec.z);
}
