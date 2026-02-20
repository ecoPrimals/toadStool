// filter.wgsl — Stream compaction (predicate evaluation + scatter)
//
// Pass 1 — evaluate_predicate:
//   flags[i] = 1 if input[i] satisfies the predicate, else 0.
//
// (Passes 2a/2b run the prefix_sum.wgsl shader — see prefix_sum.wgsl)
//   After them, scan[i] = exclusive prefix sum of flags.
//
// Pass 3 — scatter:
//   output[scan[i]] = input[i] if flags[i] == 1
//   total[0]        = number of selected elements (scan[N-1] + flags[N-1])
//
// Operations:
//   0 = GreaterThan    (value > threshold)
//   1 = LessThan       (value < threshold)
//   2 = Equal          (|value - threshold| < ε)
//   3 = NotEqual       (|value - threshold| ≥ ε)
//   4 = GreaterOrEqual (value >= threshold)
//   5 = LessOrEqual    (value <= threshold)

struct FilterParams {
    size:      u32,
    operation: u32,
    n_groups:  u32,   // ceil(size / 256) — for pass 2 dispatch
    _pad:      u32,
    threshold: f32,
    epsilon:   f32,   // equality tolerance (default 1e-5)
    _pad2:     f32,
    _pad3:     f32,
}

@group(0) @binding(0) var<storage, read>       input:   array<f32>;
@group(0) @binding(1) var<storage, read_write> flags:   array<u32>;
@group(0) @binding(2) var<storage, read_write> scan:    array<u32>;
@group(0) @binding(3) var<storage, read_write> output:  array<f32>;
@group(0) @binding(4) var<storage, read_write> total:   array<u32>; // [0] = count
@group(0) @binding(5) var<uniform>             params:  FilterParams;

// ── Pass 1: evaluate predicate ───────────────────────────────────────────────
@compute @workgroup_size(256)
fn evaluate_predicate(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;
    if (gid >= params.size) { return; }

    let value = input[gid];
    let eps   = params.epsilon;
    let thr   = params.threshold;

    var keep: bool;
    switch (params.operation) {
        case 0u: { keep = value > thr; }
        case 1u: { keep = value < thr; }
        case 2u: { keep = abs(value - thr) < eps; }
        case 3u: { keep = abs(value - thr) >= eps; }
        case 4u: { keep = value >= thr; }
        case 5u: { keep = value <= thr; }
        default: { keep = false; }
    }

    flags[gid] = select(0u, 1u, keep);
}

// ── Pass 3: scatter selected elements using prefix-sum indices ────────────────
@compute @workgroup_size(256)
fn scatter(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;
    if (gid >= params.size) { return; }

    if (flags[gid] == 1u) {
        output[scan[gid]] = input[gid];
    }

    // Thread 0 of the last workgroup writes the total count.
    let last = params.size - 1u;
    if (gid == last) {
        total[0] = scan[last] + flags[last];
    }
}
