// GPU Raytracing Shader (WGSL)
// Deep Debt: Dense parallel raytracing, excels at full scenes

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

struct Sphere {
    center: vec3<f32>,
    radius: f32,
    color: vec3<f32>,
}

struct Scene {
    sphere_count: u32,
}

@group(0) @binding(0) var<storage, read> spheres: array<Sphere>;
@group(0) @binding(1) var<storage, read> scene: Scene;
@group(0) @binding(2) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(3) var<uniform> dimensions: vec2<u32>;

fn intersect_sphere(ray: Ray, sphere: Sphere) -> f32 {
    let oc = ray.origin - sphere.center;
    let a = dot(ray.direction, ray.direction);
    let b = 2.0 * dot(oc, ray.direction);
    let c = dot(oc, oc) - sphere.radius * sphere.radius;
    let discriminant = b * b - 4.0 * a * c;
    
    if (discriminant < 0.0) {
        return -1.0;
    }
    
    let t = (-b - sqrt(discriminant)) / (2.0 * a);
    if (t > 0.0) {
        return t;
    }
    
    return -1.0;
}

fn camera_ray(x: u32, y: u32) -> Ray {
    let width = f32(dimensions.x);
    let height = f32(dimensions.y);
    let aspect_ratio = width / height;
    let fov = radians(60.0);
    let scale = tan(fov / 2.0);
    
    let px = (2.0 * (f32(x) + 0.5) / width - 1.0) * aspect_ratio * scale;
    let py = (1.0 - 2.0 * (f32(y) + 0.5) / height) * scale;
    
    return Ray(
        vec3<f32>(0.0, 0.0, 0.0),
        normalize(vec3<f32>(px, py, -1.0))
    );
}

@compute @workgroup_size(8, 8, 1)
fn raytrace(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= dimensions.x || y >= dimensions.y) {
        return;
    }
    
    let ray = camera_ray(x, y);
    
    // GPU strength: Process ALL rays in parallel
    // Even if most miss, GPU doesn't care
    
    var closest_t = 1e10;
    var hit_color = vec3<f32>(0.1, 0.1, 0.1); // Background
    
    // Check all spheres (GPU parallelizes this efficiently)
    for (var i = 0u; i < scene.sphere_count; i = i + 1u) {
        let sphere = spheres[i];
        let t = intersect_sphere(ray, sphere);
        
        if (t > 0.0 && t < closest_t) {
            closest_t = t;
            hit_color = sphere.color;
        }
    }
    
    // Write result
    let pixel_idx = y * dimensions.x + x;
    output[pixel_idx] = vec4<f32>(hit_color, 1.0);
}
