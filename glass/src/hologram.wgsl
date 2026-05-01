@group(0) @binding(0) var<storage, read> hallucination_heat: array<f32>;

// ---------------------------------------------------------
// FAST SINE & COSINE (Parabolic Approximation)
// ---------------------------------------------------------
fn q_sin(x: f32) -> f32 {
    let PI: f32 = 3.14159265359;
    let period = 2.0 * PI;
    var x_mod = x - period * floor(x / period);
    if (x_mod > PI) {
        x_mod = x_mod - period;
    }
    var sin_x = (4.0 / PI) * x_mod - (4.0 / (PI * PI)) * x_mod * abs(x_mod);
    sin_x = 0.225 * (sin_x * abs(sin_x) - sin_x) + sin_x;
    return sin_x;
}

fn q_cos(x: f32) -> f32 {
    return q_sin(x + 1.57079632679);
}

// =========================================================

struct EngineUniforms {
    camera_pos: vec4<f32>,
    audio_hz: f32,
    idle_lerp: f32,
    time: f32,
    padding: f32,
};
@group(0) @binding(1) var<uniform> engine: EngineUniforms;

// - Add `@group(0) @binding(3) var<storage, read> glyph_buffer: array<u32>;`
// - Sample the glyph buffer at the end. If a glyph pixel is active, override the Hologram color with green text.
@group(0) @binding(2) var<storage, read_write> framebuffer: array<u32>;
@group(0) @binding(3) var<storage, read> glyph_buffer: array<u32>;


fn map(p: vec3<f32>, audio_hz: f32, energy: f32) -> f32 {
    let r1 = 1.0;
    let r2 = 0.3;
    let a = atan2(p.z, p.x); // Angle around Y axis
    
    // Twist the space based on cognitive audio frequency and tensor energy (Timeline Bifurcation)
    let twist = a * 0.5 + audio_hz * 0.005 * energy;
    let s = q_sin(twist);
    let c = q_cos(twist);
    
    // Torus cross-section
    let q = vec2<f32>(length(p.xz) - r1, p.y);
    
    // Apply mobius twist rotation (Non-Orientable Topology)
    let q_twisted = vec2<f32>(c * q.x - s * q.y, s * q.x + c * q.y);
    
    let klein_dist = length(q_twisted) - r2;
    
    // Phase 14: Sonoluminescence (Cymatic Resonance)
    // The acoustic waveform directly physically perturbs the 4D geometry.
    // If energy (Lyapunov chaos) is high, the geometry violently fractures.
    // If energy is 0.0 (Causal Resonance), it freezes into perfect glass.
    let cymatic_amplitude = energy * 2.0; 
    let cymatic_displacement = 
        q_sin(p.x * (10.0 + audio_hz * 0.005)) * 
        q_cos(p.y * (10.0 + audio_hz * 0.007)) * 
        q_sin(p.z * (10.0 + audio_hz * 0.003)) * cymatic_amplitude;
                       
    return klein_dist + cymatic_displacement;
}

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= 1920u || y >= 1080u) {
        return;
    }

    // Read the RGB visual tensors from the cognitive buffer
    let r = hallucination_heat[0] / 255.0;
    let g = hallucination_heat[1] / 255.0;
    let b = hallucination_heat[2] / 255.0;
    
    // Cognitive energy drives the dimensional unfolding
    let energy = (r + g + b) / 3.0;
    
    // Map to [-1.0, 1.0] with inverted Y for correct display orientation
    let uv = vec2<f32>(f32(x) / 1920.0, f32(1080u - y) / 1080.0) * 2.0 - 1.0;
    
    // --- TASK 4: VOLUMETRIC HOLOGRAPHIC RAYMARCHING ---
    // Render the cognitive state as a fully 3D Signed Distance Field (SDF) Sphere
    let ro = engine.camera_pos.xyz; // Fuse camera position
    let rd = normalize(vec3<f32>(uv, 1.0)); // Ray Direction
    
    var t = 0.0;
    var d = 0.0;
    var p = vec3<f32>(0.0);
    
    // Raymarching Loop for 4D Klein Topology (Active State)
    // Branchless Execution to Eliminate GPU Warp Divergence
    var active_mask = 1.0;
    for(var i = 0; i < 64; i = i + 1) {
        p = ro + rd * t;
        d = map(p, engine.audio_hz, energy);
        
        // If d < 0.001, hit = 1.0. If t > 5.0, hit = 1.0.
        let hit = step(d, 0.001) + step(5.0, t);
        
        // If hit >= 1.0, active_mask becomes 0.0 permanently.
        active_mask = active_mask * step(hit, 0.5);
        
        // Only advance `t` if active_mask is 1.0
        t = t + d * 0.5 * active_mask;
    }
    
    var color = vec3<f32>(0.0);
    var alpha = 0.05;
    
    if (d < 0.001) {
        // Tetrahedron Normal Approximation (4 SDF Evaluations instead of 6)
        let e = vec2<f32>(0.01, -0.01);
        let n = normalize(
            e.xyy * map(p + e.xyy, engine.audio_hz, energy) + 
            e.yyx * map(p + e.yyx, engine.audio_hz, energy) + 
            e.yxy * map(p + e.yxy, engine.audio_hz, energy) + 
            e.xxx * map(p + e.xxx, engine.audio_hz, energy)
        );
        
        let light_dir = normalize(vec3<f32>(1.0, 1.0, -1.0));
        let diff = max(dot(n, light_dir), 0.1); 
        
        let base_col = vec3<f32>(r, g, b) * 2.0;
        color = base_col * diff + vec3<f32>(energy * 0.8) * (1.0 - diff);
        
        let fresnel = pow(1.0 - max(dot(n, -rd), 0.0), 3.0);
        color = color + vec3<f32>(fresnel * energy);
        
        alpha = 1.0; 
    } else {
        color = vec3<f32>(r, g, b) * 0.1 * (1.0 - length(uv));
    }

    // Volumetric Nebula Background (Idle State)
    if (engine.idle_lerp > 0.0) {
        // Loop Unrolled fBM Space (Zero Branching Overhead)
        var nebula_color = vec3<f32>(0.0);
        var fbm = 0.0;
        var p_nebula = ro * 2.0 + rd;
        
        // Octave 1
        var offset_p = p_nebula * 1.0 + engine.time * 0.5;
        fbm = fbm + (q_sin(offset_p.x) * q_cos(offset_p.y) * q_sin(offset_p.z)) / 1.0;
        
        // Octave 2
        offset_p = p_nebula * 2.0 + engine.time * 0.5;
        fbm = fbm + (q_sin(offset_p.x) * q_cos(offset_p.y) * q_sin(offset_p.z)) / 2.0;
        
        // Octave 3
        offset_p = p_nebula * 3.0 + engine.time * 0.5;
        fbm = fbm + (q_sin(offset_p.x) * q_cos(offset_p.y) * q_sin(offset_p.z)) / 3.0;
        
        // Nebula gradient mixing
        let deep_space = vec3<f32>(0.05, 0.0, 0.15); // Deep purple/blue
        let bright_gas = vec3<f32>(0.0, 0.8, 0.5);   // Cyan/Green matrix style gas
        
        nebula_color = mix(deep_space, bright_gas, clamp(fbm, 0.0, 1.0));
        
        // Add artificial "stars"
        let star_val = fract(q_sin(dot(uv + ro.xy, vec2<f32>(12.9898, 78.233))) * 43758.5453);
        if (star_val > 0.99) {
            nebula_color = nebula_color + vec3<f32>(1.0);
        }

        // Lerp between Hologram Object and Idle Nebula
        color = mix(color, nebula_color, engine.idle_lerp);
        alpha = mix(alpha, 1.0, engine.idle_lerp);
    }
    
    // Write out to ARGB 32-bit framebuffer (B, G, R, A byte order for little-endian /dev/fb0 typical format XRGB8888)
    let r_byte = u32(clamp(color.r, 0.0, 1.0) * 255.0);
    let g_byte = u32(clamp(color.g, 0.0, 1.0) * 255.0);
    let b_byte = u32(clamp(color.b, 0.0, 1.0) * 255.0);
    let a_byte = u32(clamp(alpha, 0.0, 1.0) * 255.0);
    
    let pixel = (a_byte << 24u) | (r_byte << 16u) | (g_byte << 8u) | b_byte;
    
    let index = y * 1920u + x;
    
    // Sample Pretext Glyph Buffer
    let glyph_pixel = glyph_buffer[index];
    if (glyph_pixel != 0u) {
        // Glyph override (assuming 0xAARRGGBB format in glyph_buffer)
        framebuffer[index] = glyph_pixel;
    } else {
        framebuffer[index] = pixel;
    }
}
