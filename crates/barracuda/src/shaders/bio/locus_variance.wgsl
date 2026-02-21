// locus_variance.wgsl — Per-Locus Allele Frequency Variance (FST decomposition)
//
// Computes the variance of allele frequencies across populations for each
// locus independently. Each thread handles one locus. Core building block
// for Weir-Cockerham FST estimation.
//
// Input: allele_freqs[pop * n_loci + locus]
// Output: per_locus_var[locus] — population variance of AF across pops
//
// Provenance: neuralSpring metalForge (Feb 21, 2026) → ToadStool absorption

@group(0) @binding(0) var<storage, read> allele_freqs: array<f32>;
@group(0) @binding(1) var<storage, read_write> per_locus_var: array<f32>;

struct VarianceParams {
    n_pops: u32,
    n_loci: u32,
}
@group(0) @binding(2) var<uniform> params: VarianceParams;

@compute @workgroup_size(256)
fn locus_variance(@builtin(global_invocation_id) gid: vec3<u32>) {
    let locus = gid.x;
    if locus >= params.n_loci {
        return;
    }

    // Two-pass: mean then variance (numerically stable for small n_pops)
    var sum: f32 = 0.0;
    for (var p: u32 = 0u; p < params.n_pops; p = p + 1u) {
        sum = sum + allele_freqs[p * params.n_loci + locus];
    }
    let mean = sum / f32(params.n_pops);

    var var_sum: f32 = 0.0;
    for (var p: u32 = 0u; p < params.n_pops; p = p + 1u) {
        let diff = allele_freqs[p * params.n_loci + locus] - mean;
        var_sum = fma(diff, diff, var_sum);
    }

    per_locus_var[locus] = var_sum / f32(params.n_pops);
}
