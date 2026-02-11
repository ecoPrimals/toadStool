#!/usr/bin/env python3
"""
BarraCUDA Shader Reorganization Script

Reorganizes 414 WGSL shaders from flat structure into categorized directories.
Updates all include_str! references in Rust files.
"""

import os
import re
import shutil
from pathlib import Path
from collections import defaultdict

# Shader categorization rules
CATEGORIES = {
    'activation': [
        'gelu', 'silu', 'elu', 'relu', 'prelu', 'rrelu', 'leaky_relu', 'celu',
        'mish', 'tanh', 'hardswish', 'glu', 'swish', 'threshold', 'hardsigmoid',
        'hardtanh', 'tanhshrink', 'logsigmoid', 'selu', 'softsign', 'hardshrink',
        'softshrink', 'softmax', 'log_softmax', 'softplus', 'softmin'
    ],
    'loss': [
        'focal_loss', 'dice_loss', 'giou_loss', 'iou_loss', 'binary_cross_entropy',
        'cross_entropy', 'nll_loss', 'hinge_loss', 'chamfer_distance',
        'earth_mover_distance', 'lovasz_loss', 'smooth_l1_loss', 'bce_loss',
        'cosine_embedding_loss', 'margin_ranking_loss', 'kl_divergence',
        'kldiv_loss', 'label_smoothing', 'center_loss', 'contrastive_loss',
        'triplet_loss', 'huber_loss', 'mae_loss', 'l1_loss', 'mse_loss',
        'tversky_loss', 'perceptual_loss', 'wasserstein_loss'
    ],
    'optimizer': [
        'adam', 'adamw', 'adabound', 'adadelta', 'adagrad', 'adafactor',
        'lamb', 'nadam', 'radam', 'rmsprop', 'sgd', 'sgdw', 'lookahead',
        'cyclical_lr'
    ],
    'pooling': [
        'avg_pool', 'max_pool', 'avgpool', 'maxpool', 'global_avgpool',
        'global_maxpool', 'global_pooling', 'adaptive_avg_pool', 'adaptive_max_pool',
        'fractional_max_pool', 'lp_pool', 'roi_pool', 'roi_align'
    ],
    'conv': [
        'conv1d', 'conv2d', 'conv3d', 'depthwise_conv', 'separable_conv',
        'dilated_conv', 'transposed_conv', 'deformable_conv', 'grouped_conv',
        'octave_conv', 'gated_conv'
    ],
    'norm': [
        'batch_norm', 'batchnorm', 'layer_norm', 'layernorm', 'group_norm',
        'groupnorm', 'instance_norm', 'instancenorm', 'graph_batch_norm',
        'filter_response_norm', 'local_response_norm', 'adaptive_instance_norm',
        'spectral_norm', 'graph_norm', 'weight_norm', 'renorm', 'rmsnorm',
        'normalize'
    ],
    'math': [
        'acos', 'asin', 'atan', 'atanh', 'acosh', 'asinh', 'cos', 'cosh',
        'sin', 'sinh', 'tan', 'abs', 'exp', 'log', 'pow', 'sqrt', 'rsqrt',
        'floor', 'ceil', 'trunc', 'round', 'frac', 'sign', 'neg', 'reciprocal',
        'clamp', 'erf', 'erfc', 'lgamma', 'min', 'max', 'add', 'sub', 'mul',
        'div', 'mod', 'remainder'
    ],
    'reduce': [
        'sum_reduce', 'mean_reduce', 'max_reduce', 'min_reduce', 'prod_dim',
        'prod_reduce', 'std_dim', 'std_reduce', 'argmax', 'argmin',
        'logsumexp', 'variance_reduce', 'variance_dim', 'mean_dim',
        'max_dim', 'min_dim', 'sum_dim', 'norm_dim', 'norm_reduce',
        'cumsum', 'cumprod', 'reduce_all'
    ],
    'linalg': [
        'cholesky', 'triangular_solve', 'eigh', 'linsolve', 'determinant',
        'matrix_power', 'matrix_rank', 'inverse', 'trace', 'diag',
        'tril', 'triu', 'qr', 'svd', 'lu'
    ],
    'tensor': [
        'broadcast', 'concat', 'chunk', 'slice', 'scatter', 'gather',
        'gather_nd', 'scatter_nd', 'expand', 'flatten', 'reshape',
        'transpose', 'permute', 'repeat', 'tile', 'fold', 'unfold',
        'split', 'stack', 'put', 'take', 'index_select', 'index_add',
        'narrow', 'roll', 'flip', 'pad', 'circular_pad', 'reflection_pad',
        'replication_pad', 'view', 'squeeze', 'unsqueeze', 'masked_select',
        'masked_fill', 'where_op', 'select', 'movedim', 'channel_shuffle'
    ],
    'attention': [
        'attention', 'causal_attention', 'cross_attention', 'local_attention',
        'sparse_attention', 'flash_attention', 'alibi', 'rotary_embedding',
        'gqa', 'mha', 'multi_head_attention', 'scaled_dot_product_attention',
        'grouped_query_attention'
    ],
    'rnn': [
        'lstm_cell', 'gru_cell', 'rnn_cell', 'bi_lstm'
    ],
    'gnn': [
        'gcn_conv', 'gat_conv', 'gin_conv', 'graph_conv', 'sage_conv',
        'edge_conv', 'message_passing'
    ],
    'detection': [
        'anchor_generator', 'box_iou', 'bbox_transform', 'nms', 'soft_nms'
    ],
    'augmentation': [
        'cutmix', 'mixup', 'color_jitter', 'random_crop', 'random_rotation',
        'random_erasing', 'random_perspective', 'random_affine',
        'elastic_transform', 'grid_mask', 'mosaic'
    ],
    'audio': [
        'stft', 'istft', 'mfcc', 'mel_scale', 'spectrogram', 'griffin_lim',
        'pitch_shift', 'time_stretch', 'window_function'
    ],
    'gradient': [
        'clip_grad_norm', 'clip_grad_value'
    ],
    'dropout': [
        'dropout', 'spatial_dropout'
    ],
    'special': [
        'bessel_j0', 'bessel_j1', 'bessel_i0', 'bessel_k0',
        'spherical_harmonics'
    ],
    'interpolation': [
        'rbf_kernel', 'loo_cv'
    ],
    'fhe': [
        'fhe_and', 'fhe_extract', 'fhe_intt', 'fhe_key_switch',
        'fhe_modulus_switch', 'fhe_ntt', 'fhe_or', 'fhe_pointwise_mul',
        'fhe_poly_add', 'fhe_poly_mul', 'fhe_poly_sub', 'fhe_rotate',
        'fhe_xor'
    ],
    'complex': [
        'complex_abs', 'complex_add', 'complex_sub', 'complex_mul',
        'complex_div', 'complex_conj', 'complex_exp', 'complex_log',
        'complex_pow', 'complex_sqrt'
    ],
    'fft': [
        'fft_1d', 'fft_2d', 'fft_3d', 'ifft', 'rfft'
    ],
    'md': [
        'pbc', 'coulomb', 'lennard_jones', 'yukawa', 'morse', 'born_mayer',
        'velocity_verlet', 'rk4', 'laplacian'
    ],
}

def categorize_shader(filename):
    """Determine the category for a shader file."""
    basename = filename.replace('.wgsl', '')
    
    # Check each category for a match
    for category, patterns in CATEGORIES.items():
        for pattern in patterns:
            if pattern in basename:
                # Special handling for MD shaders
                if category == 'md':
                    if any(force in basename for force in ['coulomb', 'lennard_jones', 'yukawa', 'morse', 'born_mayer']):
                        return 'md/forces'
                    elif any(integ in basename for integ in ['velocity_verlet', 'rk4', 'laplacian']):
                        return 'md/integrators'
                    else:
                        return 'md'
                return category
    
    # Default to misc if no match
    return 'misc'

def main():
    """Main reorganization function."""
    
    # Setup paths
    barracuda_root = Path(__file__).parent.parent / 'crates' / 'barracuda'
    shaders_dir = barracuda_root / 'src' / 'shaders'
    ops_dir = barracuda_root / 'src' / 'ops'
    
    if not shaders_dir.exists():
        print(f"Error: {shaders_dir} does not exist")
        return 1
    
    print("=" * 80)
    print("BarraCUDA Shader Reorganization")
    print("=" * 80)
    print()
    
    # Step 1: Analyze current structure
    print("Step 1: Analyzing current structure...")
    shader_files = list(shaders_dir.glob('*.wgsl'))
    print(f"  Found {len(shader_files)} shaders in flat structure")
    
    # Categorize all shaders
    categorized = defaultdict(list)
    for shader in shader_files:
        category = categorize_shader(shader.name)
        categorized[category].append(shader)
    
    print(f"  Categorized into {len(categorized)} categories:")
    for category, files in sorted(categorized.items()):
        print(f"    {category:20s}: {len(files):3d} shaders")
    print()
    
    # Step 2: Create directory structure
    print("Step 2: Creating directory structure...")
    for category in categorized.keys():
        target_dir = shaders_dir / category
        target_dir.mkdir(parents=True, exist_ok=True)
        print(f"  Created: {category}/")
    print()
    
    # Step 3: Move shaders
    print("Step 3: Moving shaders to categories...")
    move_count = 0
    for category, files in categorized.items():
        target_dir = shaders_dir / category
        for shader in files:
            target = target_dir / shader.name
            shutil.move(str(shader), str(target))
            move_count += 1
        print(f"  Moved {len(files):3d} shaders to {category}/")
    print(f"  Total moved: {move_count}")
    print()
    
    # Step 4: Update include_str! references in Rust files
    print("Step 4: Updating include_str! references...")
    rust_files = list(ops_dir.rglob('*.rs'))
    updated_count = 0
    file_update_count = 0
    
    for rust_file in rust_files:
        content = rust_file.read_text()
        original_content = content
        
        # Pattern: include_str!("../shaders/FILENAME.wgsl")
        # or: include_str!("../../shaders/FILENAME.wgsl")
        pattern = r'include_str!\("(\.\./)+shaders/([^/]+\.wgsl)"\)'
        
        def replace_include(match):
            nonlocal updated_count
            prefix = match.group(1)  # ../ or ../../
            filename = match.group(2)
            category = categorize_shader(filename)
            updated_count += 1
            return f'include_str!("{prefix}shaders/{category}/{filename}")'
        
        content = re.sub(pattern, replace_include, content)
        
        if content != original_content:
            rust_file.write_text(content)
            file_update_count += 1
    
    print(f"  Updated {updated_count} include_str! references in {file_update_count} files")
    print()
    
    # Step 5: Verification
    print("Step 5: Verification...")
    remaining_flat = list(shaders_dir.glob('*.wgsl'))
    if remaining_flat:
        print(f"  WARNING: {len(remaining_flat)} shaders still in flat structure:")
        for shader in remaining_flat:
            print(f"    {shader.name}")
    else:
        print("  ✓ All shaders moved to categories")
    
    # Count shaders in new structure
    total_organized = sum(len(list((shaders_dir / cat).glob('*.wgsl'))) 
                         for cat in categorized.keys())
    print(f"  ✓ {total_organized} shaders in organized structure")
    
    if total_organized == len(shader_files):
        print("  ✓ Shader count matches (no files lost)")
    else:
        print(f"  WARNING: Shader count mismatch! Started with {len(shader_files)}, now have {total_organized}")
    
    print()
    print("=" * 80)
    print("Reorganization complete!")
    print("=" * 80)
    print()
    print("Next steps:")
    print("  1. cargo check -p barracuda")
    print("  2. cargo test -p barracuda --lib")
    print("  3. cargo clippy -p barracuda")
    print()
    
    return 0

if __name__ == '__main__':
    exit(main())
