// CrossEntropy Loss: Classification loss function
// CUDA equivalent: Custom kernels or cuDNN loss functions
// Formula: loss = -sum(y_true * log(y_pred + epsilon))
// Use cases: Multi-class classification training, neural network optimization

@group(0) @binding(0) var<storage, read> predictions: array<f32>;  // Predicted probabilities (softmax output)
@group(0) @binding(1) var<storage, read> targets: array<f32>;      // True labels (one-hot encoded)
@group(0) @binding(2) var<storage, read_write> losses: array<f32>; // Per-sample losses

struct Params {
    batch_size: u32,
    num_classes: u32,
    epsilon: f32,  // Small constant to prevent log(0)
    reduction: u32,  // 0=none (per-sample), 1=mean, 2=sum
}
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256)
fn compute_loss(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let sample_idx = global_id.x;
    
    if (sample_idx >= params.batch_size) {
        return;
    }
    
    // Compute cross-entropy loss for this sample
    var loss = 0.0;
    let base_idx = sample_idx * params.num_classes;
    
    for (var i = 0u; i < params.num_classes; i++) {
        let pred = predictions[base_idx + i];
        let true_label = targets[base_idx + i];
        
        // CrossEntropy: -true_label * log(pred + epsilon)
        // Clamp prediction to avoid log(0)
        let safe_pred = max(pred, params.epsilon);
        loss += -true_label * log(safe_pred);
    }
    
    losses[sample_idx] = loss;
}
