// Batch Numerical Gradient Computation
//
// Computes ∇f(x) using central differences, FULLY PARALLEL
// Each thread computes one partial derivative: ∂f/∂xᵢ
//
// Algorithm:
//   ∂f/∂xᵢ ≈ (f(x + εeᵢ) - f(x - εeᵢ)) / (2ε)
//
// This is SHADER-FIRST gradient computation:
// - All n partial derivatives computed in parallel
// - No sequential loop over dimensions
// - Foundation for parallel BFGS, Newton methods
//
// For BFGS: This shader provides the ∇f needed for search direction d = -H⁻¹∇f
//
// Note: This shader handles the SAMPLING of f at perturbed points.
// The actual function evaluation may need to be batched separately
// depending on the objective function complexity.

struct Params {
    n: u32,           // Dimension of x
    epsilon: f32,     // Finite difference step size
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> x: array<f32>;          // Current point (length n)
@group(0) @binding(2) var<storage, read> f_plus: array<f32>;     // f(x + ε*eᵢ) for each i
@group(0) @binding(3) var<storage, read> f_minus: array<f32>;    // f(x - ε*eᵢ) for each i
@group(0) @binding(4) var<storage, read_write> gradient: array<f32>;  // Output: ∇f (length n)

// Central difference gradient - fully parallel
@compute @workgroup_size(256, 1, 1)
fn central_difference(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    
    if (i >= params.n) {
        return;
    }
    
    // ∂f/∂xᵢ = (f(x + εeᵢ) - f(x - εeᵢ)) / (2ε)
    gradient[i] = (f_plus[i] - f_minus[i]) / (2.0 * params.epsilon);
}

// Forward difference (less accurate, fewer evaluations)
// ∂f/∂xᵢ ≈ (f(x + εeᵢ) - f(x)) / ε
@compute @workgroup_size(256, 1, 1)
fn forward_difference(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    
    if (i >= params.n) {
        return;
    }
    
    // f_minus[0] contains f(x) (reused for all dimensions)
    let f_x = f_minus[0];
    gradient[i] = (f_plus[i] - f_x) / params.epsilon;
}

// Generate perturbed points for batch evaluation
// Output: x_perturbed[2n] where [0..n) = x+εeᵢ, [n..2n) = x-εeᵢ
struct PerturbParams {
    n: u32,
    epsilon: f32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> perturb_params: PerturbParams;
@group(0) @binding(1) var<storage, read> base_x: array<f32>;
@group(0) @binding(2) var<storage, read_write> x_perturbed: array<f32>;  // [2n * n] flattened

// Generate all 2n perturbed points in parallel
@compute @workgroup_size(256, 1, 1)
fn generate_perturbed_points(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let n = perturb_params.n;
    
    // Total points: 2n (one +ε and one -ε per dimension)
    // Each point has n components
    // idx ranges over [0, 2n*n)
    
    let point_idx = idx / n;      // Which perturbed point (0 to 2n-1)
    let component = idx % n;      // Which component of that point
    
    if (point_idx >= 2u * n) {
        return;
    }
    
    // Copy base point
    var val = base_x[component];
    
    // Perturb the appropriate dimension
    let perturb_dim = point_idx % n;
    let is_plus = point_idx < n;
    
    if (component == perturb_dim) {
        if (is_plus) {
            val = val + perturb_params.epsilon;
        } else {
            val = val - perturb_params.epsilon;
        }
    }
    
    x_perturbed[idx] = val;
}
