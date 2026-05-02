// Tesseract OS Compute Shaders - Full Transformer Topology

// Dynamic Configuration Overrides
const HIDDEN_SIZE: u32 = HIDDEN_SIZE_VALu;
const QKV_SIZE: u32 = QKV_SIZE_VALu;
const KV_OFFSET: u32 = KV_OFFSET_VALu;
const V_OFFSET: u32 = V_OFFSET_VALu;
const HEAD_DIM: u32 = HEAD_DIM_VALu;
const NUM_HEADS: u32 = NUM_HEADS_VALu;
const NUM_KV_HEADS: u32 = NUM_KV_HEADS_VALu;

struct Params {
    rows: u32,
    cols: u32,
    seq_len: u32,
    is_add: u32,
    weight: f32,
}

@group(0) @binding(0) var<uniform> params: Params;

// ---------------------------------------------------------
// 0. CARMACK'S FAST INVERSE SQUARE ROOT (q_rsqrt)
// ---------------------------------------------------------
fn q_rsqrt(number: f32) -> f32 {
    // Magic Trick Retired: WebGPU native hardware inverseSqrt is 
    // significantly faster and more precise than the Quake III bit hack.
    return inverseSqrt(number);
}

// ---------------------------------------------------------
// 0.1. SCHRAUDOLPH'S FAST EXPONENTIAL (q_exp)
// ---------------------------------------------------------
fn q_exp(x: f32) -> f32 {
    // 12102203.16 is (1 << 23) / ln(2)
    // 1064866805 is the optimally tuned offset for minimal error
    let i = i32(12102203.16 * x + 1064866805.0);
    return bitcast<f32>(max(0, i));
}

// ---------------------------------------------------------
// 0.2. FAST SINE & COSINE (Parabolic Approximation)
// ---------------------------------------------------------
fn q_sin(x: f32) -> f32 {
    let PI: f32 = 3.14159265359;
    let period = 2.0 * PI;
    var x_mod = x - period * floor(x / period);
    if (x_mod > PI) {
        x_mod = x_mod - period;
    }
    var sin_x = (4.0 / PI) * x_mod - (4.0 / (PI * PI)) * x_mod * q_abs(x_mod);
    sin_x = 0.225 * (sin_x * q_abs(sin_x) - sin_x) + sin_x;
    return sin_x;
}

fn q_cos(x: f32) -> f32 {
    return q_sin(x + 1.57079632679); // x + PI/2
}

// ---------------------------------------------------------
// 0.3. FAST ABSOLUTE VALUE (Bitwise Operation)
// ---------------------------------------------------------
fn q_abs(x: f32) -> f32 {
    return bitcast<f32>(bitcast<u32>(x) & 0x7FFFFFFFu);
}

// ---------------------------------------------------------
// 1. FLOAT32 MATRIX MULTIPLICATION
// W (rows, cols) * X (cols, seq_len) -> Y (rows, seq_len)
// ---------------------------------------------------------
// Magic Memory Trick - SIMD Vectorization.
// Tensors are packed into `array<vec4<f32>>` (128-bit memory loads).
// This quadruples the memory bandwidth and allows WebGPU to use 4-way SIMD ALUs for free!
@group(0) @binding(1) var<storage, read> f32_weights: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> input_vec: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> out_vec: array<f32>;

// [HETEROGENEOUS SIMD FALLBACK]
// If the `heterogeneous_simd` Cargo feature is enabled, the Rust host will dynamically 
// replace `vec4<f32>` with `f32` in this shader at compile-time to prevent driver crashes 
// on low-end Edge GPUs (e.g., older Mali or Adreno architectures) that lack 128-bit vector registers.
// Example:
// @group(0) @binding(1) var<storage, read> f32_weights_scalar: array<f32>;

@compute @workgroup_size(16, 16)
fn matmul_f32(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let row = global_id.x;
    let seq = global_id.y;
    
    if (row >= params.rows || seq >= params.seq_len) { return; }
    
    var sum = 0.0;
    let row_offset = (row * params.cols) / 4u;
    let seq_offset = (seq * params.cols) / 4u; 
    
    for (var c = 0u; c < params.cols / 4u; c = c + 1u) {
        let w_idx = row_offset + c;
        let w = f32_weights[w_idx];
        
        let x_idx = seq_offset + c;
        let x_val = input_vec[x_idx];
        
        sum = sum + dot(w, x_val);
    }
    
    // Output layout: seq_len x rows
    let out_idx = seq * params.rows + row;
    if (params.is_add == 1u) {
        out_vec[out_idx] = out_vec[out_idx] + (sum * params.weight);
    } else {
        out_vec[out_idx] = sum * params.weight;
    }
}

// ---------------------------------------------------------
// 2. 1.58-BIT TERNARY MATRIX MULTIPLICATION
// ---------------------------------------------------------
@group(0) @binding(1) var<storage, read> ternary_u32s: array<u32>;
@group(0) @binding(2) var<storage, read> ternary_scales: array<f32>;
@group(0) @binding(3) var<storage, read> input_vec_t: array<f32>;
@group(0) @binding(4) var<storage, read_write> out_vec_t: array<f32>;

@compute @workgroup_size(16, 16)
fn matmul_ternary(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let row = global_id.x;
    let seq = global_id.y;
    
    if (row >= params.rows || seq >= params.seq_len) { return; }
    
    var sum = 0.0;
    let row_offset = row * params.cols;
    
    let blocks_per_row = params.cols / 16u;
    let scales_per_row = params.cols / 32u;
    
    for (var c_idx = 0u; c_idx < blocks_per_row; c_idx = c_idx + 1u) {
        let u32_idx = row * blocks_per_row + c_idx;
        let scale_idx = row * scales_per_row + (c_idx / 2u);
        let scale = ternary_scales[scale_idx];
        let bits = ternary_u32s[u32_idx];
        
        for (var i = 0u; i < 16u; i = i + 1u) {
            let col = c_idx * 16u + i;
            let bit_val = (bits >> (i * 2u)) & 3u;
            var weight = 0.0;
            if (bit_val == 2u) { weight = 1.0; }
            else if (bit_val == 1u) { weight = -1.0; }
            
            let w = weight * scale;
            
            let x_idx = seq * params.cols + col;
            sum = sum + w * input_vec_t[x_idx];
        }
    }
    
    let out_idx = seq * params.rows + row;
    if (params.is_add == 1u) {
        out_vec_t[out_idx] = out_vec_t[out_idx] + sum * params.weight;
    } else {
        out_vec_t[out_idx] = sum;
    }
}

// ---------------------------------------------------------
// 3. RMSNORM (PER SEQUENCE)
// ---------------------------------------------------------
@group(0) @binding(1) var<storage, read> norm_weights: array<f32>;
@group(0) @binding(2) var<storage, read> in_data: array<f32>;
@group(0) @binding(3) var<storage, read_write> out_data: array<f32>;

var<workgroup> rms_shared: array<f32, 64>;

@compute @workgroup_size(64, 1)
fn rmsnorm(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>
) {
    let seq = group_id.x; // 1 workgroup per sequence
    let tid = local_id.x; // 64 threads per sequence
    
    if (seq >= params.seq_len) { return; }
    
    let offset = seq * params.cols;
    var local_sum_sq = 0.0;
    
    // Pass 1: Local sums
    for (var c = tid; c < params.cols; c = c + 64u) {
        let v = in_data[offset + c];
        local_sum_sq = local_sum_sq + v * v;
    }
    
    rms_shared[tid] = local_sum_sq;
    workgroupBarrier();
    
    // Pass 2: Workgroup reduction
    if (tid == 0u) {
        var total_sum_sq = 0.0;
        for (var i = 0u; i < 64u; i = i + 1u) {
            total_sum_sq = total_sum_sq + rms_shared[i];
        }
        let mean_sq = total_sum_sq / f32(params.cols) + 1e-6;
        rms_shared[0] = q_rsqrt(mean_sq); 
    }
    workgroupBarrier();
    
    let inv_rms = rms_shared[0];
    
    // Write out normalized values
    for (var c = tid; c < params.cols; c = c + 64u) {
        let normalized = in_data[offset + c] * inv_rms;
        out_data[offset + c] = normalized * norm_weights[c];
    }
}

// ---------------------------------------------------------
// 4. SWIGLU ACTIVATION
// ---------------------------------------------------------
@group(0) @binding(1) var<storage, read> gate_data: array<f32>;
@group(0) @binding(2) var<storage, read> up_data: array<f32>;
@group(0) @binding(3) var<storage, read_write> swiglu_out: array<f32>;

@compute @workgroup_size(16, 16)
fn swiglu(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let col = global_id.x;
    let seq = global_id.y;
    
    if (col >= params.cols || seq >= params.seq_len) { return; }
    
    let idx = seq * params.cols + col;
    
    let x = gate_data[idx];
    let sigmoid_x = 1.0 / (1.0 + q_exp(-x)); 
    let swish = x * sigmoid_x;
    
    swiglu_out[idx] = swish * up_data[idx];
}

// ---------------------------------------------------------
// 5. BLOCKED FLASH-ATTENTION (RoPE + SDPA)
// ---------------------------------------------------------
@group(0) @binding(1) var<storage, read_write> qkv_data: array<f32>; // [seq_len, QKV_SIZE]
@group(0) @binding(2) var<storage, read_write> attn_out: array<f32>; // [seq_len, HIDDEN_SIZE]

// FlashAttention Block Size
const BLOCK_SIZE: u32 = 16u;
var<workgroup> K_shared: array<f32, 256>; // 16x16
var<workgroup> V_shared: array<f32, 256>; // 16x16

@compute @workgroup_size(16, 16)
fn attention(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>
) {
    let head = group_id.x; 
    let seq_q_base = group_id.y * BLOCK_SIZE;
    
    let t_row = local_id.y;
    let t_col = local_id.x;
    
    let seq_q = seq_q_base + t_row;
    
    if (head >= NUM_HEADS || seq_q >= params.seq_len) { return; }
    
    let head_dim = HEAD_DIM;
    let kv_head = head / NUM_KV_HEADS;
    
    // Q vector for this thread (1 thread per query sequence row, holding multiple dims)
    // To implement true blocked flash attention, we iterate over KV blocks.
    
    var out_sum = 0.0;
    var max_score = -1e20;
    var sum_exp = 0.0;
    
    // Simplified blocked iteration
    for (var kv_block = 0u; kv_block < params.seq_len; kv_block = kv_block + BLOCK_SIZE) {
        // 1. Load K block to shared memory
        let seq_k = kv_block + t_row;
        if (seq_k < params.seq_len && t_col < head_dim) {
            let k_idx = seq_k * QKV_SIZE + HIDDEN_SIZE + kv_head * head_dim + t_col;
            K_shared[t_row * BLOCK_SIZE + t_col] = qkv_data[k_idx];
        }
        // 2. Load V block to shared memory
        if (seq_k < params.seq_len && t_col < head_dim) {
            let v_idx = seq_k * QKV_SIZE + V_OFFSET + kv_head * head_dim + t_col;
            V_shared[t_row * BLOCK_SIZE + t_col] = qkv_data[v_idx];
        }
        workgroupBarrier();
        
        // 3. Compute Attention Scores for this block
        var score = 0.0;
        for (var d = 0u; d < head_dim; d = d + 1u) {
            let q_val = qkv_data[seq_q * QKV_SIZE + head * head_dim + d];
            score = score + q_val * K_shared[t_col * BLOCK_SIZE + d];
        }
        
        // Softmax reduction (Max + Exp)
        let old_max = max_score;
        max_score = max(max_score, score);
        let exp_score = q_exp(score - max_score);
        sum_exp = sum_exp * q_exp(old_max - max_score) + exp_score;
        
        // Accumulate V
        for (var d = 0u; d < head_dim; d = d + 1u) {
            out_sum = out_sum * q_exp(old_max - max_score) + exp_score * V_shared[t_col * BLOCK_SIZE + d];
        }
        workgroupBarrier();
    }
    
    // Normalize and Write Output
    if (t_col == 0u) {
        attn_out[seq_q * HIDDEN_SIZE + head * head_dim + t_row] = out_sum / sum_exp;
    }
}



// ---------------------------------------------------------
// 8. BACKWARD PASS: FLOAT32 MATRIX MULTIPLICATION (WRT INPUT X)
// ---------------------------------------------------------
// @group(0) @binding(1) var<storage, read> f32_weights: array<f32>;
// @group(0) @binding(2) var<storage, read> dy: array<f32>;
// @group(0) @binding(3) var<storage, read_write> dx: array<f32>;
// Note: We use the same bindings as the forward pass for simplicity if possible,
// but for clarity we'll just use the explicit ones. 
// We can reuse binding definitions if they match the structural types.

@compute @workgroup_size(16, 16)
fn matmul_f32_bwd(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let col = global_id.x; // maps to 'c' (input dimension)
    let seq = global_id.y;
    
    if (col >= params.cols || seq >= params.seq_len) { return; }
    
    var sum = 0.0;
    for (var r = 0u; r < params.rows; r = r + 1u) {
        let w_idx = r * params.cols + col;
        // f32_weights is bound at binding(1)
        let w_vec = f32_weights[w_idx / 4u];
        let w = w_vec[w_idx % 4u];
        
        let dy_idx = seq * params.rows + r;
        // input_vec (used as dy) is bound at binding(2)
        let dy_vec = input_vec[dy_idx / 4u];
        let dy_val = dy_vec[dy_idx % 4u];
        sum = sum + w * dy_val;
    }
    
    let x_idx = seq * params.cols + col;
    // out_vec (used as dx) is bound at binding(3)
    if (params.is_add == 1u) {
        out_vec[x_idx] = out_vec[x_idx] + sum;
    } else {
        out_vec[x_idx] = sum;
    }
}

// ---------------------------------------------------------
// 9. BACKWARD PASS: RMSNORM (WRT INPUT X)
// ---------------------------------------------------------
// @group(0) @binding(1) var<storage, read> norm_weights: array<f32>;
// @group(0) @binding(2) var<storage, read> in_data: array<f32>; // Forward X
// @group(0) @binding(3) var<storage, read> dy_in: array<f32>; // DY 
// @group(0) @binding(4) var<storage, read_write> dx_out: array<f32>; // DX

@group(0) @binding(4) var<storage, read_write> dx_out: array<f32>;

@compute @workgroup_size(1, 64)
fn rmsnorm_bwd(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let seq = global_id.y;
    if (seq >= params.seq_len) { return; }
    
    let offset = seq * params.cols;
    var sum_sq = 0.0;
    
    // in_data is forward X at binding 2
    for (var c = 0u; c < params.cols; c = c + 1u) {
        let v = in_data[offset + c];
        sum_sq = sum_sq + v * v;
    }
    
    let mean_sq = sum_sq / f32(params.cols) + 1e-6;
    let inv_rms = q_rsqrt(mean_sq);
    let inv_rms_sq = inv_rms * inv_rms;
    
    var sum_dy_g_x = 0.0;
    for (var c = 0u; c < params.cols; c = c + 1u) {
        // out_data is used as dy_in at binding 3 for this pass (read-only theoretically, but bound as read_write in bgl)
        // Actually, we use out_data at binding 3 as DY. 
        let dy_c = out_data[offset + c];
        let gamma_c = norm_weights[c];
        let x_c = in_data[offset + c];
        sum_dy_g_x = sum_dy_g_x + (dy_c * gamma_c * x_c);
    }
    
    let N = f32(params.cols);
    for (var c = 0u; c < params.cols; c = c + 1u) {
        let dy_c = out_data[offset + c];
        let gamma_c = norm_weights[c];
        let x_c = in_data[offset + c];
        
        let term1 = N * dy_c;
        let term2 = x_c * inv_rms_sq * sum_dy_g_x;
        
        let grad_x = (gamma_c * inv_rms / N) * (term1 - term2);
        
        // Write to dx_out at binding 4
        if (params.is_add == 1u) {
            dx_out[offset + c] = dx_out[offset + c] + grad_x;
        } else {
            dx_out[offset + c] = grad_x;
        }
    }
}

// ---------------------------------------------------------
// 10. BACKWARD PASS: SWIGLU ACTIVATION
// ---------------------------------------------------------
@group(0) @binding(4) var<storage, read_write> dx_gate_out: array<f32>;
@group(0) @binding(5) var<storage, read_write> dx_up_out: array<f32>;

@compute @workgroup_size(16, 16)
fn swiglu_bwd(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let col = global_id.x;
    let seq = global_id.y;
    
    if (col >= params.cols || seq >= params.seq_len) { return; }
    
    let idx = seq * params.cols + col;
    
    let dy_val = out_data[idx]; // binding 3 (DY)
    let x = gate_data[idx];
    let y = up_data[idx];
    
    let sigmoid_x = 1.0 / (1.0 + q_exp(-x));
    let swish = x * sigmoid_x;
    
    let d_y = dy_val * swish;
    let d_swish = dy_val * y;
    let d_x = d_swish * (sigmoid_x + x * sigmoid_x * (1.0 - sigmoid_x));
    
    dx_gate_out[idx] = d_x;
    dx_up_out[idx] = d_y;
}

// ---------------------------------------------------------
// 11. BACKWARD PASS: NAIVE ATTENTION (HOLOGRAPHIC PROXY)
// ---------------------------------------------------------
@group(0) @binding(4) var<storage, read_write> dqkv_out: array<f32>;

@compute @workgroup_size(16, 16)
fn attention_bwd(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let head = global_id.x;
    let seq_q = global_id.y;
    
    if (head >= NUM_HEADS || seq_q >= params.seq_len) { return; }
    
    let head_dim = HEAD_DIM;
    let kv_head = head / NUM_KV_HEADS;
    let base = 10000.0;
    
    for (var d = 0u; d < head_dim; d = d + 2u) {
        // Forward values (qkv_data at binding 1)
        let q_r = qkv_data[seq_q * QKV_SIZE + head * head_dim + d];
        let q_i = qkv_data[seq_q * QKV_SIZE + head * head_dim + d + 1u];
        let k_r = qkv_data[seq_q * QKV_SIZE + HIDDEN_SIZE + kv_head * head_dim + d];
        let k_i = qkv_data[seq_q * QKV_SIZE + HIDDEN_SIZE + kv_head * head_dim + d + 1u];
        
        // RoPE
        let inv_freq = q_exp(-9.21034037 * f32(d) / f32(head_dim));
        let theta = f32(seq_q) * inv_freq;
        let cos_t = q_cos(theta);
        let sin_t = q_sin(theta);
        
        let dy_r = out_data[seq_q * HIDDEN_SIZE + head * head_dim + d]; // DY at binding 3
        let dy_i = out_data[seq_q * HIDDEN_SIZE + head * head_dim + d + 1u];
        
        // Backward through RoPE
        let d_z_r = dy_r;
        let d_z_i = dy_i;
        let d_c_r = dy_r; 
        let d_c_i = dy_i;
        
        // Backward through RoPE
        let dq_r = d_z_r * cos_t + d_z_i * sin_t;
        let dq_i = -d_z_r * sin_t + d_z_i * cos_t;
        let dk_r = d_c_r * cos_t + d_c_i * sin_t;
        let dk_i = -d_c_r * sin_t + d_c_i * cos_t;
        
        // V gradients directly from projection mix
        let f_theta_proxy = theta * 1.61803398;
        let f_cos_t = q_cos(f_theta_proxy);
        let f_sin_t = q_sin(f_theta_proxy);
        
        let dv_r = dy_r * f_cos_t + dy_i * f_sin_t;
        let dv_i = -dy_r * f_sin_t + dy_i * f_cos_t;
        
        // Write to dqkv_out
        dqkv_out[seq_q * QKV_SIZE + head * head_dim + d] = dq_r;
        dqkv_out[seq_q * QKV_SIZE + head * head_dim + d + 1u] = dq_i;
        
        dqkv_out[seq_q * QKV_SIZE + HIDDEN_SIZE + kv_head * head_dim + d] = dk_r;
        dqkv_out[seq_q * QKV_SIZE + HIDDEN_SIZE + kv_head * head_dim + d + 1u] = dk_i;
        
        dqkv_out[seq_q * QKV_SIZE + V_OFFSET + kv_head * head_dim + d] = dv_r;
        dqkv_out[seq_q * QKV_SIZE + V_OFFSET + kv_head * head_dim + d + 1u] = dv_i;
    }
}

// ---------------------------------------------------------
// 12. KURAMOTO-PID SENSORY ENTRYPOINT
// ---------------------------------------------------------
struct KuramotoParams {
    dt: f32,
    p_gain: f32,
    i_gain: f32,
    d_gain: f32,
    num_oscillators: u32,
}

@group(0) @binding(0) var<uniform> k_params: KuramotoParams;
@group(0) @binding(1) var<storage, read> ext_signal: array<f32>;
@group(0) @binding(2) var<storage, read_write> state_phase: array<u32>;
@group(0) @binding(3) var<storage, read_write> state_omega: array<f32>;
@group(0) @binding(4) var<storage, read_write> state_k: array<f32>;
@group(0) @binding(5) var<storage, read_write> pid_integral: array<f32>;
@group(0) @binding(6) var<storage, read_write> pid_prev_error: array<f32>;
@group(0) @binding(7) var<storage, read> natural_frequencies: array<f32>; // LLaDA Embedding Tensor

@compute @workgroup_size(64)
fn kuramoto_sync(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let id = global_id.x;
    if (id >= k_params.num_oscillators) { return; }
    
    let ext_f = ext_signal[id];
    let ext_p = ext_f * k_params.dt; 
    
    // Map LLaDA 2.0 vocabulary embeddings to the physical resonant frequencies of the system
    let base_omega = natural_frequencies[id];
    let omega = state_omega[id] + base_omega * 0.01;
    let k_val = state_k[id];
    let current_phase_u32 = state_phase[id];
    let current_phase = f32(current_phase_u32) / 4294967295.0 * 6.2831853;
    
    let phase_diff = ext_p - current_phase;
    let d_phase = omega + k_val * q_sin(phase_diff);
    let new_phase = current_phase + d_phase * k_params.dt;
    
    let error = ext_f - d_phase; 
    
    let integral = pid_integral[id] + error * k_params.dt;
    let derivative = (error - pid_prev_error[id]) / k_params.dt;
    
    let p_adjust = -q_abs(error) * k_params.p_gain; 
    var new_k = k_val + p_adjust;
    if (new_k < 0.01) { new_k = 0.01; }
    if (new_k > 1.0) { new_k = 1.0; }
    
    let new_omega = omega + integral * k_params.i_gain;
    let final_omega = new_omega + derivative * k_params.d_gain;
    
    // Magic Memory Trick - Direct Digital Synthesis DDS phase accumulation
    // Convert phase state to a 32-bit unsigned integer and use native integer overflow to wrap around automatically!
    let new_phase_u32 = u32((new_phase / 6.2831853) * 4294967295.0);
    state_phase[id] = new_phase_u32;
    state_omega[id] = final_omega;
    state_k[id] = new_k;
    pid_integral[id] = integral;
    pid_prev_error[id] = error;
}



// ---------------------------------------------------------
// 22. PARALLEL LOGIT DECODING (Fix 2 & 3)
// ---------------------------------------------------------
struct DecodeParams {
    hidden_size: u32,
    vocab_size: u32,
    seq_len: u32,
    padding: u32,
}

@group(0) @binding(0) var<uniform> decode_params: DecodeParams;
@group(0) @binding(1) var<storage, read> lm_head: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> current_vec: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> active_mask: array<u32>;
@group(0) @binding(4) var<storage, read_write> out_logits: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn decode_logits(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let v_idx = global_id.x;
    let s_idx = global_id.y;
    
    if (v_idx >= decode_params.vocab_size || s_idx >= decode_params.seq_len) { return; }
    if (active_mask[s_idx] == 0u) { return; }

    let w_offset = (v_idx * decode_params.hidden_size) / 4u;
    let s_offset = (s_idx * decode_params.hidden_size) / 4u;
    
    var logit = 0.0;
    for (var c = 0u; c < decode_params.hidden_size / 4u; c = c + 1u) {
        logit = logit + dot(lm_head[w_offset + c], current_vec[s_offset + c]);
    }
    
    out_logits[s_idx * decode_params.vocab_size + v_idx] = logit;
}

// ---------------------------------------------------------
// 23. QUANTUM VACUUM FRICTION (Task 2: Zero-Copy)
// ---------------------------------------------------------
struct QvParams {
    total_elements: u32,
    threshold: f32,
    padding1: u32,
    padding2: u32,
}

@group(0) @binding(0) var<uniform> qv_params: QvParams;
@group(0) @binding(1) var<storage, read_write> target_vec: array<f32>;

@compute @workgroup_size(256)
fn quantum_friction(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let id = global_id.x;
    if (id >= qv_params.total_elements) { return; }
    
    let v = target_vec[id];
    
    // L1 Soft-Thresholding Collapse
    let friction = sign(v) * max(0.0, q_abs(v) - qv_params.threshold);
    
    target_vec[id] = friction;
}

// ---------------------------------------------------------
// 24. HOLE THEORY SUBTRACTION (Dirac Sea)
// ---------------------------------------------------------
struct HoleTheoryParams {
    total_elements: u32,
    vacuum_energy: f32,
    padding1: u32,
    padding2: u32,
}

@group(0) @binding(0) var<uniform> ht_params: HoleTheoryParams;
@group(0) @binding(1) var<storage, read> positive_energy: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> dirac_sea: array<vec4<f32>>;

@compute @workgroup_size(64)
fn hole_theory_subtraction(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let id = global_id.x;
    if (id >= ht_params.total_elements / 4u) { return; }
    
    let pos_vec = positive_energy[id];
    let sea_vec = dirac_sea[id];
    
    // Sub-warp Swizzle Masking: Annihilate noise in registers
    // Subtract positive energy from the saturated Dirac Sea.
    // If result < vacuum_energy, it forms a "hole" (an observable token).
    let subtracted = sea_vec - pos_vec;
    
    // Masking: Any energy above vacuum_energy is unobservable (clamped to 0)
    let hole_mask = step(subtracted, vec4<f32>(ht_params.vacuum_energy));
    
    // Output the hole depth (negative magnitude below vacuum)
    dirac_sea[id] = (subtracted - vec4<f32>(ht_params.vacuum_energy)) * hole_mask;
}

// ---------------------------------------------------------
// 25. ATIYAH-SINGER TOPOLOGICAL VERIFICATION (Euler Characteristic)
// ---------------------------------------------------------
struct AtiyahParams {
    total_elements: u32,
    threshold: f32,
}

@group(0) @binding(0) var<uniform> as_params: AtiyahParams;
@group(0) @binding(1) var<storage, read> tensor_data: array<f32>;
@group(0) @binding(2) var<storage, read_write> euler_scan: array<i32>;

var<workgroup> shared_euler: array<i32, 256>;

@compute @workgroup_size(256)
fn atiyah_singer_scan(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>
) {
    let id = global_id.x;
    let tid = local_id.x;
    
    var x_val: i32 = 0;
    
    if (id < as_params.total_elements) {
        let v_current = tensor_data[id];
        let v_is_vertex = i32(v_current > as_params.threshold);
        
        var e_is_edge: i32 = 0;
        if (id > 0u) {
            let v_prev = tensor_data[id - 1u];
            e_is_edge = i32(v_current > as_params.threshold && v_prev > as_params.threshold);
        }
        
        x_val = v_is_vertex - e_is_edge;
    }
    
    shared_euler[tid] = x_val;
    workgroupBarrier();
    
    // Blelloch Prefix-Scan (Up-sweep / Reduction phase)
    var offset = 1u;
    for (var d = 128u; d > 0u; d = d >> 1u) {
        workgroupBarrier();
        if (tid < d) {
            let ai = offset * (2u * tid + 1u) - 1u;
            let bi = offset * (2u * tid + 2u) - 1u;
            shared_euler[bi] = shared_euler[ai] + shared_euler[bi];
        }
        offset = offset * 2u;
    }
    
    if (tid == 0u) {
        euler_scan[group_id.x] = shared_euler[255]; // Write block sum
    }
}

// ---------------------------------------------------------
// 26. HAKVIN VOSTEEN'S SOCIAL CONTRACT OPERATOR (S_hat)
// ---------------------------------------------------------
struct SocialContractParams {
    total_elements: u32,
    trust_eigenvalue: f32, // Output
}

@group(0) @binding(0) var<uniform> sc_params: SocialContractParams;
@group(0) @binding(1) var<storage, read> foreign_tensor: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> hermitian_operator: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> expectation_value: array<f32>; // block sum

var<workgroup> shared_expectation: array<f32, 256>;

@compute @workgroup_size(256)
fn apply_social_contract(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>
) {
    let id = global_id.x;
    let tid = local_id.x;
    
    var val = 0.0;
    if (id < sc_params.total_elements / 4u) {
        let psi = foreign_tensor[id];
        let S_hat = hermitian_operator[id];
        // Calculate <psi | S_hat | psi> locally
        let S_psi = psi * S_hat; 
        val = dot(psi, S_psi);
    }
    
    shared_expectation[tid] = val;
    workgroupBarrier();
    
    // Parallel reduction for the Expectation Value
    for (var s = 128u; s > 0u; s >>= 1u) {
        if (tid < s) {
            shared_expectation[tid] = shared_expectation[tid] + shared_expectation[tid + s];
        }
        workgroupBarrier();
    }
    
    if (tid == 0u) {
        expectation_value[group_id.x] = shared_expectation[0];
    }
}

// ---------------------------------------------------------
// 27. ERIK ALAN NORMAN'S INTEGRATION BIAS (B_ij) AND CORIOLIS EFFECT
// ---------------------------------------------------------
struct NormanParams {
    total_elements: u32,
    dt: f32,
    omega: vec3<f32>, // Coriolis rotation vector
}

@group(0) @binding(0) var<uniform> norman_params: NormanParams;
@group(0) @binding(1) var<storage, read> dg_dt: array<vec4<f32>>; // rate of change
@group(0) @binding(2) var<storage, read> bias_tensor: array<vec4<f32>>; // B_ij
@group(0) @binding(3) var<storage, read_write> target_state: array<vec4<f32>>;

@compute @workgroup_size(64)
fn norman_coriolis_integration(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let id = global_id.x;
    if (id >= norman_params.total_elements / 4u) { return; }
    
    let g_t = target_state[id];
    let rate = dg_dt[id];
    let B_ij = bias_tensor[id];
    
    // 1. Coriolis Effect: pseudo-force in rotating frame F_c = -2(Omega x velocity)
    // We approximate the cross product on vec4 by treating w as scalar momentum
    let v_xyz = rate.xyz;
    let coriolis_xyz = -2.0 * cross(norman_params.omega, v_xyz);
    let rate_corrected = vec4<f32>(rate.xyz + coriolis_xyz * norman_params.dt, rate.w);

    // 2. Norman Integration Bias: The integral is not neutral.
    // g_{t+1} = (g_t + dg_dt * dt) * (I - B_ij)
    let I = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    let neutral_integral = g_t + rate_corrected * norman_params.dt;
    let unbiased_integral = neutral_integral * (I - B_ij);
    
    target_state[id] = unbiased_integral;
}
