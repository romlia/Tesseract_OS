@group(0) @binding(0) var<storage, read_write> hallucination_heat: array<f32>;
@group(0) @binding(1) var<storage, read> dt_uniform: f32; // Injected by CarnotThermodynamics

// Quantum Decoherence Model (Based on 14|15 Platform Silicon Processor)
// T2_star: Fast phase decoherence (unfocused) - e.g., 10.0ms
// T2_hahn: Refocused phase coherence via Hahn-echo pi-pulse - e.g., 660.0ms
// ---------------------------------------------------------
// FAST ABSOLUTE VALUE (Bitwise Operation)
// ---------------------------------------------------------
fn q_abs(x: f32) -> f32 {
    return bitcast<f32>(bitcast<u32>(x) & 0x7FFFFFFFu);
}

fn q_exp(x: f32) -> f32 {
    let i = i32(12102203.16 * x + 1064866805.0);
    return bitcast<f32>(max(0, i));
}

fn exponential_decay(heat: f32, tau: f32) -> f32 {
    let t_decay = tau * 0.5 + 0.1;
    return heat * q_exp(-dt_uniform / t_decay);
}

fn apply_quantum_decoherence(heat: f32, t2_star: f32, t2_hahn: f32) -> f32 {
    // If heat is extremely high (>0.9), it means a user interaction (pi-pulse) just occurred.
    // The geometry is refocused, preserving its quantum state for much longer.
    let is_refocused = step(0.9, heat); 
    
    // Larmor-frequency splitting: we mix the decay regimes based on the refocusing threshold
    let t_decay = mix(t2_star, t2_hahn, is_refocused);
    
    // Apply T2 phase decoherence (Quantum state collapses into noise/void)
    return exponential_decay(heat, t_decay);
}

@compute @workgroup_size(64)
fn process_hallucinations(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let id = global_id.x;
    let size = arrayLength(&hallucination_heat);
    if (id >= size) { return; }
    
    let current_heat = hallucination_heat[id];
    
    // T2* = 10.0 ms (fast decay)
    // T2_hahn = 660.0 ms (slow, refocused coherence)
    let new_heat = apply_quantum_decoherence(current_heat, 10.0, 660.0);
    
    // Prevent subnormal floats with Quantum Vacuum Friction (L1 Soft-Thresholding Collapse)
    // Continuous equation to annihilate microscopic values before they cause command queue OOM
    let final_heat = sign(new_heat) * max(0.0, q_abs(new_heat) - 1e-5);
    
    // Write the decohered value back. 
    hallucination_heat[id] = final_heat;
}
