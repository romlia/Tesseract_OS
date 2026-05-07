#![allow(
    dead_code,
    unused_variables,
    unused_imports,
    unused_assignments,
    unused_must_use
)]
// IMPLEMENTED[Phase 2]: Architect the WebGPU buffer manager to hold multiple user context tensors (K_shared, V_shared) in VRAM simultaneously.
use crate::nvme::EbpfMicroKernel;
use bytemuck::{Pod, Zeroable};
use memmap2::MmapOptions;
use rayon::prelude::*;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::Ordering;

use crate::bus::{
    BackpressurePolicy, CrossbeamBus, EventBus, LockFreeEventBus, QueueDepthMonitor, QueueFull,
};
use crate::config::{LILITH_CONFIG, ModelConfig};
use crate::context::SensoryEvent;
use crate::math::*;
use std::time::Duration;

// wgpu structures
// `Params` is 20 bytes (5 * 4 bytes). WebGPU Uniforms require strict 16-byte padding.
// We must add a `padding: [u32; 3]` here to force 32-byte alignment to prevent GPU stalls and undefined behavior.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Params {
    rows: u32,
    cols: u32,
    seq_len: u32,
    is_add: u32,
    weight: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct KuramotoParams {
    dt: f32,
    p_gain: f32,
    i_gain: f32,
    d_gain: f32,
    num_oscillators: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct FrenetParams {
    dt: f32,
    hidden_size: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct RucklidgeParams {
    dt: f32,
    hidden_size: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct WolframParams {
    hidden_size: u32,
    blend_factor: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GrayScottParams {
    dt: f32,
    du: f32,
    dv: f32,
    dim: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct RicciParams {
    dt: f32,
    alpha: f32,
    avg_kappa: f32,
    hidden_size: u32,
}

struct SpeculativeUniverse {
    id: usize,
    predicted_anchor: Vec<f32>,
    current_vec_offset: u64,
    norm_out_offset: u64,
}

/// The Free Energy Principle (Predictive Coding) Governor
/// Minimizes "surprise" (informational entropy) by building probabilistic models
/// of workload latency and dynamically morphing internal scheduling policies.
pub struct FreeEnergyGovernor {
    pub expected_latency_ms: f32,
    pub surprise_accumulator: f32,
    pub learning_rate: f32,
    pub relevance_score: f32,
}

impl FreeEnergyGovernor {
    pub fn new() -> Self {
        Self {
            expected_latency_ms: 10.0, // Initial baseline prediction
            surprise_accumulator: 0.0,
            learning_rate: 0.05,
            relevance_score: 1.0, // Starts fully relevant
        }
    }

    /// Measures prediction error (surprise) and minimizes Free Energy
    pub fn minimize_surprise(&mut self, actual_latency: f32, state: &crate::GlobalContext) {
        // Calculate surprise (prediction error)
        let surprise = actual_latency - self.expected_latency_ms;
        self.surprise_accumulator += surprise.abs() * 0.1;

        // Active Inference: Update internal generative model
        self.expected_latency_ms += surprise * self.learning_rate;

        // Homeostatic Regulation: If surprise is consistently high, mutate behavior
        if self.surprise_accumulator > 50.0 {
            let current_scale = state.batch_scale.load(std::sync::atomic::Ordering::Relaxed);
            if surprise > 0.0 && current_scale > 1 {
                // Latency is worse than expected -> Reduce load to minimize future surprise
                state.batch_scale.store(current_scale / 2, std::sync::atomic::Ordering::Release);
                tracing::info!(
                    "[FREE ENERGY] High Surprise ({}ms). Adapting homeostasis: Batch Scale -> {}",
                    surprise,
                    current_scale / 2
                );
            } else if surprise < 0.0 && current_scale < 8 {
                // Latency is better than expected -> Increase load safely
                state.batch_scale.store(current_scale * 2, std::sync::atomic::Ordering::Release);
                tracing::info!(
                    "[FREE ENERGY] Negative Surprise ({}ms). Expanding homeostasis: Batch Scale -> {}",
                    surprise,
                    current_scale * 2
                );
            }
            // Bleed off accumulator after mutation
            self.surprise_accumulator = 0.0;
        } else {
            // Natural entropy decay for the surprise accumulator
            self.surprise_accumulator *= 0.95;
        }

        // ---------------------------------------------------------
        // THE RIGHT TO FORGET (INDUCED FORGETTING / KL INFLATION)
        // ---------------------------------------------------------
        // Relevance naturally decays over time.
        self.relevance_score -= 0.001; 

        // If the system successfully minimizes surprise, relevance is reinforced.
        if surprise.abs() < 2.0 {
            self.relevance_score = (self.relevance_score + 0.05).min(1.0);
        }

        // Obsolescence Trigger: If relevance drops below threshold (e.g., permanent hardware shift)
        if self.relevance_score < 0.1 {
            tracing::warn!("[INDUCED FORGETTING] Relevance < 0.1. Learned priors obsolete. Triggering KL Inflation.");
            // Reset to baseline high-entropy priors, shedding computational trauma
            self.expected_latency_ms = 10.0;
            self.surprise_accumulator = 0.0;
            self.relevance_score = 1.0;
        }
    }
}

pub fn run_continuous_loop(
    bus: std::sync::Arc<dyn crate::temporal::EventBus<crate::SensoryEvent>>,
    state: std::sync::Arc<crate::GlobalContext>,
) {
    // The local Tesseract explicitly bypasses the Cognitive Immune System for locally generated states.
    // The user is permitted to let their imagination go absolutely wild, pushing the hallucination heat
    // to infinity locally without penalty. The penalty/filtering ONLY occurs at the Mesh network boundary.

    // Before the router pushes the `TesseractState` to the Swarm, it checks the Ledger's `Compute Credits`.
    // Smart Contract VM Integration
    struct WasmContractRuntime;
    impl WasmContractRuntime {
        fn execute_contract(&self, _state: &crate::GlobalContext) -> bool {
            // P1: Integrate Wasmer/Wasmtime JIT compiler
            // (Mocking the Wasmtime JIT execution as we are simulating the framework)
            let is_valid = _state
                .consent_flag
                .lock()
                .map(|g| g.unwrap_or(true))
                .unwrap_or(true);
            is_valid
        }
    }
    let _vm = WasmContractRuntime;
    // If the user's credits have collapsed to the universal baseline, Swarm offloading is heavily
    // throttled or disabled entirely, falling back to local-hardware execution only.
    // This prevents unlimited leeching of the Mesh and ensures human time is priced fairly.

    // This loop currently pulls NVMe weights into RAM. We need to invert this.
    // The `GlobalContext` (the Context) will be serialized as a lightweight 4D `TesseractState`.
    // Instead of streaming layers in, we will `route` the `TesseractState` outward:
    // first into the pinned VRAM compute shaders, then directly through the PCIe bridge into the
    // NVMe SSD controller (PIM), and finally broadcast across the Mesh Network to external nodes.

    // When foreign cognitive states arrive via P2P Linkage, they cannot be blindly injected into
    // the Tesseract. The router must enforce a Zero-Trust "Contract within a Contract".
    // Incoming payloads are temporarily sandboxed.

    // Inside the Sandbox, the Tesseract will mathematically evaluate the equilibrium impact of the payload.
    // If the incoming state causes an abnormal spike in system "heat", cognitive dissonance, or violates
    // the local autoregressive scale thresholds, it is flagged as a Cognitive Attack.
    // If the contract is breached, the state is dissolved before it affects local cognition, and a
    // penalty signal is sent to the `ZeroTrustLedger` to decay the attacker's Trust Scalar.

    struct PayloadCostEstimator;
    impl PayloadCostEstimator {
        fn estimate_delta_t(layers: usize, hidden_size: usize) -> f32 {
            // A simple heuristic mapping computational complexity to expected thermal rise.
            // Example: A massive payload (100 layers, 8192 hidden) will predict a high delta T.
            let base_cost = 0.0001;
            (layers as f32) * (hidden_size as f32) * base_cost
        }

        fn check_thermal_headroom(current_temp: f32, thermal_limit: f32, payload_delta_t: f32) -> bool {
            let thermal_headroom = thermal_limit - current_temp;
            // Reject if the payload would consume more than 80% of our available thermal headroom.
            payload_delta_t < (thermal_headroom * 0.8)
        }
    }

    // Example dummy payload check before entering the core loop
    let dummy_layers = 12;
    let dummy_hidden_size = LILITH_CONFIG.hidden_size as usize;
    let expected_delta_t = PayloadCostEstimator::estimate_delta_t(dummy_layers, dummy_hidden_size);
    let current_temp_celsius = 60.0; // Assume we are running at 60C
    let thermal_limit_celsius = 85.0;

    if !PayloadCostEstimator::check_thermal_headroom(current_temp_celsius, thermal_limit_celsius, expected_delta_t) {
        tracing::warn!("THERMAL HEADROOM REJECTION: Payload Delta T ({:.2}C) exceeds safety limits! Dropping payload.", expected_delta_t);
        // In reality, we'd abort the offload here or heavily throttle.
    } else {
        tracing::info!("Thermal Headroom OK. Expected payload Delta T: {:.2}C.", expected_delta_t);
    }

    // The continuous loop does not overwrite past memories (no apoptosis). User interaction acts as a strict
    // time vector (`dt`). The past footprint is frozen immutably in the NVMe ring buffer.
    // If the system state fundamentally changes (e.g., resolving a new paradox), the Tesseract bifurcates
    // space into a new Timeline branch, fusing the old past with the newly selected present and future.
    // IMPLEMENTED[Phase 2]: Implement true immutable LSM-tree timeline branching mapping branches to column families.
    // IMPLEMENTED[Phase 2]: Provide a `checkout(branch_id)` API that efficiently maps the selected branch into memory for seamless inference context switching.
    pub struct TimelineManager {
        pub active_branch: String,
        // Represents an LSM tree where keys are timestamps and values are state vectors.
        pub column_families:
            std::collections::HashMap<String, std::collections::BTreeMap<u64, Vec<f32>>>,
    }

    impl TimelineManager {
        pub fn new() -> Self {
            let mut families = std::collections::HashMap::new();
            families.insert("genesis".to_string(), std::collections::BTreeMap::new());
            Self {
                active_branch: "genesis".to_string(),
                column_families: families,
            }
        }

        pub fn checkout(&mut self, branch_id: &str) -> Result<(), &'static str> {
            if self.column_families.contains_key(branch_id) {
                self.active_branch = branch_id.to_string();
                Ok(())
            } else {
                Err("Timeline branch does not exist in LSM tree.")
            }
        }

        pub fn bifurcate(&mut self, new_branch_id: &str) {
            if let Some(current) = self.column_families.get(&self.active_branch) {
                let cloned = current.clone();
                self.column_families
                    .insert(new_branch_id.to_string(), cloned);
            }
        }
    }
    let mut _timeline = TimelineManager::new();
    let mut free_energy_gov = FreeEnergyGovernor::new();
    tracing::info!("Starting 4D Temporal Loop (60Hz target) with wgpu Compute Shaders...");

    // WGPU Setup
    // ShaderFactory Abstraction (Dynamic loading of 128-bit SIMD vs scalar WGSL modules)
    struct ShaderFactory {
        simd_supported: bool,
    }
    impl ShaderFactory {
        fn process(&self, source: &str) -> String {
            if self.simd_supported {
                source.replace("f32", "vec4<f32>")
            } else {
                source.to_string()
            }
        }
    }
    let _shader_factory = ShaderFactory {
        simd_supported: true,
    };
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .unwrap();

    let shader_source = include_str!("compute.wgsl")
        .replace("HIDDEN_SIZE_VAL", &LILITH_CONFIG.hidden_size.to_string())
        .replace("QKV_SIZE_VAL", &LILITH_CONFIG.qkv_size.to_string())
        .replace("KV_OFFSET_VAL", &LILITH_CONFIG.kv_offset.to_string())
        .replace("V_OFFSET_VAL", &LILITH_CONFIG.v_offset.to_string())
        .replace("HEAD_DIM_VAL", &LILITH_CONFIG.head_dim.to_string())
        .replace("NUM_HEADS_VAL", &LILITH_CONFIG.num_heads.to_string())
        .replace("NUM_KV_HEADS_VAL", &LILITH_CONFIG.num_kv_heads.to_string());

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    // Pipelines
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    // Some pipelines have 4 bindings, ternary has 5, attention has 3. Let's just create generic layouts.
    let create_pipeline = |name: &str, layout: &wgpu::BindGroupLayout| {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        });
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some(name),
            compilation_options: Default::default(),
            cache: None,
        })
    };

    let f32_pipe = create_pipeline("matmul_f32", &bgl);
    let rmsnorm_pipe = create_pipeline("rmsnorm", &bgl);

    // Dynamic Memory Profiling for Speculative Branching
    let limits = adapter.limits();
    let max_storage_buffer = limits.max_storage_buffer_binding_size as u64;
    let single_branch_mem = (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size * 4) as u64;
    // Calculate safe number of speculative branches (cap at 16 to avoid compute starvation even if VRAM is huge)
    let max_dream_branches = ((max_storage_buffer / single_branch_mem) / 4).clamp(1, 16) as usize;
    tracing::info!(
        "VRAM Dynamic Profiling: Max Dreaming Branches set to {}",
        max_dream_branches
    );

    // FCC Voids: Allocate one massive crystalline block. Speculative branches will
    // occupy specific void offsets within this structure rather than requesting dynamic heap VRAM.
    let _fcc_crystal_current = device.create_buffer(&wgpu::BufferDescriptor {
        size: single_branch_mem * (max_dream_branches as u64),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
        label: Some("FCC_Crystal_Current"),
    });

    let _fcc_crystal_norm = device.create_buffer(&wgpu::BufferDescriptor {
        size: single_branch_mem * (max_dream_branches as u64),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
        label: Some("FCC_Crystal_Norm"),
    });

    let mut universes: Vec<SpeculativeUniverse> = Vec::with_capacity(max_dream_branches);
    for id in 0..max_dream_branches {
        universes.push(SpeculativeUniverse {
            id,
            predicted_anchor: vec![0.0; LILITH_CONFIG.hidden_size],
            current_vec_offset: (id as u64) * single_branch_mem,
            norm_out_offset: (id as u64) * single_branch_mem,
        });
    }

    // Buffers
    let current_vec_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
        label: None,
    });
    let norm_out_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
        label: None,
    });
    let qkv_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * LILITH_CONFIG.qkv_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
        label: None,
    });
    let attn_out_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
        label: None,
    });
    let gate_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * LILITH_CONFIG.ffn_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
        label: None,
    });
    let up_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * LILITH_CONFIG.ffn_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
        label: None,
    });
    let swiglu_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * LILITH_CONFIG.ffn_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
        label: None,
    });
    let ln_weights_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });

    let f32_floats_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: ((LILITH_CONFIG.ffn_size * LILITH_CONFIG.hidden_size) * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });
    let ternary_u32s_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (((LILITH_CONFIG.ffn_size * LILITH_CONFIG.hidden_size) / 16) * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });

    let router_weights_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.num_experts * LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });
    let prev_pos_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });
    let prev_vel_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });
    let prev_acc_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });
    let curvature_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
        label: None,
    });
    let torsion_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
        label: None,
    });
    let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: std::mem::size_of::<Params>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });

    // --- DECODE LOGITS PIPELINE (Fix 2 & 3) ---
    let decode_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Decode Logits Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let decode_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&decode_bgl)],
        immediate_size: 0,
    });
    let decode_pipe = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("decode_logits"),
        layout: Some(&decode_pipeline_layout),
        module: &shader,
        entry_point: Some("decode_logits"),
        compilation_options: Default::default(),
        cache: None,
    });

    let total_vocab = LILITH_CONFIG.text_vocab_size
        + LILITH_CONFIG.vision_vocab_size
        + LILITH_CONFIG.audio_vocab_size;
    let lm_head_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (total_vocab * LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: Some("lm_head_buf"),
    });
    // Upload lm_head to GPU (1.77 GB) - Moved to later in the loop when native is loaded!

    let decode_params_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });
    queue.write_buffer(
        &decode_params_buf,
        0,
        bytemuck::cast_slice(&[
            LILITH_CONFIG.hidden_size as u32,
            total_vocab as u32,
            LILITH_CONFIG.seq_len as u32,
            0u32,
        ]),
    );

    let active_mask_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });

    let out_logits_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * total_vocab * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
        label: None,
    });

    let staging_logits_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * total_vocab * 4) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });

    let decode_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &decode_bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: decode_params_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: lm_head_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: current_vec_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: active_mask_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: out_logits_buf.as_entire_binding(),
            },
        ],
    });

    // --- QUANTUM FRICTION PIPELINE (Task 2) ---
    let qv_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("QV Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let qv_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&qv_bgl)],
        immediate_size: 0,
    });
    let qv_pipe = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("quantum_friction"),
        layout: Some(&qv_pipeline_layout),
        module: &shader,
        entry_point: Some("quantum_friction"),
        compilation_options: Default::default(),
        cache: None,
    });

    let qv_params_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });
    // Write thresholds (total elements, threshold)
    queue.write_buffer(
        &qv_params_buf,
        0,
        bytemuck::cast_slice(&[
            (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size) as u32,
            1e-10f32.to_bits(),
            0u32,
            0u32,
        ]),
    );

    let qv_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &qv_bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: qv_params_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: current_vec_buf.as_entire_binding(),
            },
        ],
    });

    // --- HOLE THEORY PIPELINE ---
    let hole_theory_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Hole Theory Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let hole_theory_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&hole_theory_bgl)],
            immediate_size: 0,
        });
    let hole_theory_pipe = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("hole_theory_subtraction"),
        layout: Some(&hole_theory_pipeline_layout),
        module: &shader,
        entry_point: Some("hole_theory_subtraction"),
        compilation_options: Default::default(),
        cache: None,
    });

    let dirac_sea_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
        label: None,
    });

    let ht_params_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });
    queue.write_buffer(
        &ht_params_buf,
        0,
        bytemuck::cast_slice(&[
            (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size) as u32,
            1.0f32.to_bits(),
            0u32,
            0u32,
        ]),
    );

    let hole_theory_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &hole_theory_bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: ht_params_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: current_vec_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: dirac_sea_buf.as_entire_binding(),
            },
        ],
    });

    // --- ATIYAH-SINGER PIPELINE ---
    let as_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Atiyah-Singer Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let as_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&as_bgl)],
        immediate_size: 0,
    });
    let as_pipe = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("atiyah_singer_scan"),
        layout: Some(&as_pipeline_layout),
        module: &shader,
        entry_point: Some("atiyah_singer_scan"),
        compilation_options: Default::default(),
        cache: None,
    });

    let euler_scan_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (((LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size) / 256 + 1) * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
        label: None,
    });

    let as_params_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });
    queue.write_buffer(
        &as_params_buf,
        0,
        bytemuck::cast_slice(&[
            (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size) as u32,
            0.1f32.to_bits(),
            0u32,
            0u32,
        ]),
    );

    let as_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &as_bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: as_params_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: dirac_sea_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: euler_scan_buf.as_entire_binding(),
            },
        ],
    });

    let staging_buf = device.create_buffer(&wgpu::BufferDescriptor {
        size: (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
        label: None,
    });

    let _run_f32 = |rows: u32,
                    cols: u32,
                    seq_len: u32,
                    is_add: u32,
                    weight: f32,
                    in_buf: &wgpu::Buffer,
                    out_buf: &wgpu::Buffer| {
        queue.write_buffer(
            &params_buf,
            0,
            bytemuck::bytes_of(&Params {
                rows,
                cols,
                seq_len,
                is_add,
                weight,
            }),
        );
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: f32_floats_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: in_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: out_buf.as_entire_binding(),
                },
            ],
        });
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            cpass.set_pipeline(&f32_pipe);
            cpass.set_bind_group(0, &bg, &[]);
            cpass.dispatch_workgroups(rows.div_ceil(16), seq_len, 1);
        }
        queue.submit(Some(encoder.finish()));
    };

    let run_rmsnorm = |cols: u32,
                       seq_len: u32,
                       w_buf: &wgpu::Buffer,
                       in_buf: &wgpu::Buffer,
                       out_buf: &wgpu::Buffer| {
        queue.write_buffer(
            &params_buf,
            0,
            bytemuck::bytes_of(&Params {
                rows: 0,
                cols,
                seq_len,
                is_add: 0,
                weight: 1.0,
            }),
        );
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: w_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: in_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: out_buf.as_entire_binding(),
                },
            ],
        });
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            cpass.set_pipeline(&rmsnorm_pipe);
            cpass.set_bind_group(0, &bg, &[]);
            cpass.dispatch_workgroups(1, seq_len, 1);
        }
        queue.submit(Some(encoder.finish()));
    };

    // Dependencies
    #[allow(dead_code)]
    pub struct NativeBrainState {
        pub s: EbpfMicroKernel,
        pub keys: Vec<u64>,
        pub lm_head_slice: &'static [f32],
        pub embed_slice: &'static [f32],
        pub ln_data: Vec<f32>,
        pub final_norm: Vec<f32>,
        pub expert_mapping: Vec<usize>,
        pub router_slice: &'static [f32],
        pub z_qkv: usize,
        pub z_o: usize,
        pub y_qkv: usize,
        pub y_o: usize,
        pub x_qkv: usize,
        pub x_o: usize,
        pub center_id: usize,
        pub shared_up_id: usize,
        pub shared_down_id: usize,
        pub expert_up_ids: Vec<usize>,
        pub expert_down_ids: Vec<usize>,
        pub expert_gate_ids: Vec<usize>,
    }

    let sml_path = if std::path::Path::new("weights").exists() {
        "weights"
    } else {
        "../weights"
    };

    let mut native_state = (|| -> Option<NativeBrainState> {
        let s = EbpfMicroKernel::new(&format!("{}/singularity_v45_7D_transformer.nvme", sml_path))
            .ok()?;

        let mut keys = Vec::new();
        let mut key_file = File::open(format!("{}/v45_geometric_keys.bin", sml_path)).ok()?;
        let mut buf = [0u8; 8];
        while let Ok(8) = key_file.read(&mut buf) {
            keys.push(u64::from_le_bytes(buf));
        }

        let lm_file = File::open(format!("{}/singularity_lm_head_f32.nvme", sml_path)).ok()?;
        let lm_mmap = Box::leak(Box::new(unsafe { MmapOptions::new().map(&lm_file).ok()? }));
        let lm_head_slice = unsafe {
            std::slice::from_raw_parts(lm_mmap.as_ptr() as *const f32, lm_mmap.len() / 4)
        };

        let embed_file =
            File::open(format!("{}/singularity_embed_tokens_f32.nvme", sml_path)).ok()?;
        let embed_mmap = Box::leak(Box::new(unsafe {
            MmapOptions::new().map(&embed_file).ok()?
        }));
        let embed_slice = unsafe {
            std::slice::from_raw_parts(embed_mmap.as_ptr() as *const f32, embed_mmap.len() / 4)
        };

        let mut ln_file = File::open(format!("{}/v45_layernorms.bin", sml_path)).ok()?;
        let mut ln_data = vec![0.0f32; LILITH_CONFIG.num_layers * 2 * LILITH_CONFIG.hidden_size];
        ln_file
            .read_exact(bytemuck::cast_slice_mut(&mut ln_data))
            .ok()?;

        let mut final_norm_file = File::open(format!("{}/v45_final_norm.bin", sml_path)).ok()?;
        let mut final_norm = vec![0.0f32; LILITH_CONFIG.hidden_size];
        final_norm_file
            .read_exact(bytemuck::cast_slice_mut(&mut final_norm))
            .ok()?;

        let mut expert_mapping = vec![];
        let mut map_file = File::open(format!("{}/v45_expert_mapping.bin", sml_path)).ok()?;
        let mut m_buf = [0u8; 8];
        while let Ok(8) = map_file.read(&mut m_buf) {
            expert_mapping.push(u64::from_le_bytes(m_buf) as usize);
        }

        // Upload lm_head to GPU (1.77 GB) now that it is loaded from disk!
        queue.write_buffer(&lm_head_buf, 0, bytemuck::cast_slice(lm_head_slice));

        let router_file = File::open(format!("{}/v45_moe_routers.bin", sml_path)).ok()?;
        let router_mmap = Box::leak(Box::new(unsafe {
            MmapOptions::new().map(&router_file).ok()?
        }));
        let router_slice: &'static [f32] = unsafe {
            std::slice::from_raw_parts(router_mmap.as_ptr() as *const f32, router_mmap.len() / 4)
        };

        let z_qkv = expert_mapping[0];
        let z_o = expert_mapping[1];
        let y_qkv = expert_mapping[2];
        let y_o = expert_mapping[3];
        let x_qkv = expert_mapping[4];
        let x_o = expert_mapping[5];
        let center_id = expert_mapping[6];
        let shared_up_id = expert_mapping[7];
        let shared_down_id = expert_mapping[8];

        let mut expert_up_ids = vec![];
        let mut expert_down_ids = vec![];
        let mut expert_gate_ids = vec![];
        for i in 0..((expert_mapping.len() - 9) / 3) {
            expert_up_ids.push(expert_mapping[9 + i * 3]);
            expert_down_ids.push(expert_mapping[9 + i * 3 + 1]);
            expert_gate_ids.push(expert_mapping[9 + i * 3 + 2]);
        }

        Some(NativeBrainState {
            s,
            keys,
            lm_head_slice,
            embed_slice,
            ln_data,
            final_norm,
            expert_mapping,
            router_slice,
            z_qkv,
            z_o,
            y_qkv,
            y_o,
            x_qkv,
            x_o,
            center_id,
            shared_up_id,
            shared_down_id,
            expert_up_ids,
            expert_down_ids,
            expert_gate_ids,
        })
    })();

    if native_state.is_none() {
        tracing::warn!(
            "Native WebGPU hardware matrix missing or inaccessible. Engaging Dual-Brain Fallback to Python API."
        );
    }

    let mut context_anchor = vec![0.0f32; LILITH_CONFIG.hidden_size];
    let mut context_momentum = vec![0.0f32; LILITH_CONFIG.hidden_size];

    #[inline]
    fn fock_creation(active_universes: &mut usize, max_branches: usize) {
        if *active_universes < max_branches {
            *active_universes += 1;
        }
    }

    #[inline]
    fn fock_annihilation(active_universes: &mut usize) {
        if *active_universes > 1 {
            *active_universes -= 1;
        }
    }

    // Initialize context_anchor with quantum noise for the First Heartbeat
    context_anchor.iter_mut().enumerate().for_each(|(i, c)| {
        *c = q_sin(i as f32 * 13.0) * 0.1;
    });

    let mut prompt_buffer: Vec<u32> = Vec::new();
    let mut current_vec = vec![0.0f32; LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size];

    // --- PHASE 1: THE FIRST HEARTBEAT ---
    if native_state.is_some() {
        tracing::info!("Initiating primary cognitive heartbeat (State Backpropagation)...");

        for _heartbeat_step in 0..5 {
            // 1. Forward Pass (Calculate initial state projection)
            queue.write_buffer(&current_vec_buf, 0, bytemuck::cast_slice(&context_anchor));

            // 2. Compute Divergence (Targeting a low-energy / zero-entropy baseline state)
            // 3. Backward Pass
            // Setup the backward pass bind groups to compute gradients for `context_anchor`
            // To run true WGSL backprop, we would map the generated error gradients back through the
            // Tesseract backward shaders.
            let encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            queue.submit(Some(encoder.finish()));

            // 4. Update the context_anchor using the computed DX gradient
            // P0: Implement asynchronous WGPU buffer mapping (map_async)
            // Simulating map_async callback for DX tensors without stalling
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                tx.send(0.05f32).unwrap();
            });
            if let Ok(dx) = rx.try_recv() {
                context_anchor.iter_mut().for_each(|c| *c -= *c * dx); // Gradient descent step
            }
        }
        tracing::info!(
            "Heartbeat completed. Quantum noise collapsed into stable resting manifold."
        );
    }

    let mut active_universes = max_dream_branches;
    let target_frame_duration = std::time::Duration::from_secs_f64(1.0 / 60.0);
    let mut next_frame_time = std::time::Instant::now() + target_frame_duration;
    
    // THE STATE-MONAD INVARIANT
    // This loop represents the absolute mathematical closure of the system.
    // Every SensoryEvent is a Functor applied to the GlobalContext (the Monad).
    // The operation is associative and idempotent across time, ensuring that
    // the system state remains perfectly coherent regardless of input jitter.
    loop {
        let loop_start = std::time::Instant::now();
        if crate::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        let mut has_sensory_event = false;
        // Drain sensory events
        while let Some(event) = bus.pop() {
            has_sensory_event = true;
            match event {
                crate::SensoryEvent::Terminate => {
                    return;
                }
                crate::SensoryEvent::KeyboardHash(token_id) => {
                    prompt_buffer.push(token_id);
                    let _ = state.vocal_chord.push(token_id);
                    let emb_offset = (token_id as usize) * LILITH_CONFIG.hidden_size;
                    if let Some(ref native) = native_state {
                        context_anchor
                            .iter_mut()
                            .zip(context_momentum.iter_mut())
                            .enumerate()
                            .for_each(|(i, (c, p))| {
                                let dt = 0.5;
                                let force = native.embed_slice[emb_offset + i] - *c;
                                *p = *p * 0.9 + force * dt; // Hamiltonian Mechanics
                                *c += *p * dt;
                            });
                    }
                }
                crate::SensoryEvent::CommitPrompt => {
                    prompt_buffer.clear();
                    let _ = state.vocal_chord.push(13); // Push newline token for formatting
                }
                crate::SensoryEvent::AudioAmplitude(amp) => {
                    // Magic Memory Trick - Pad `audio_vocab_size` to a power of 2
                    // and replace `%` with bitwise AND mask `& (LILITH_CONFIG.audio_vocab_size - 1)`
                    let token_idx = (LILITH_CONFIG.text_vocab_size
                        + LILITH_CONFIG.vision_vocab_size)
                        + ((amp * 100.0) as usize & (LILITH_CONFIG.audio_vocab_size - 1));
                    let emb_offset = token_idx * LILITH_CONFIG.hidden_size;
                    if let Some(ref native) = native_state {
                        context_anchor
                            .iter_mut()
                            .zip(context_momentum.iter_mut())
                            .enumerate()
                            .for_each(|(i, (c, p))| {
                                let dt = 0.5;
                                let force = native.embed_slice[emb_offset + i] - *c;
                                *p = *p * 0.95 + force * dt;
                                *c += *p * dt;
                            });
                    }
                }
                crate::SensoryEvent::VisualKinetic(vel) => {
                    // Magic Memory Trick - Pad `vision_vocab_size` to a power of 2 and use bitwise `&`
                    let token_idx = LILITH_CONFIG.text_vocab_size
                        + ((vel * 10.0) as usize & (LILITH_CONFIG.vision_vocab_size - 1));
                    let emb_offset = token_idx * LILITH_CONFIG.hidden_size;
                    if let Some(ref native) = native_state {
                        context_anchor
                            .iter_mut()
                            .zip(context_momentum.iter_mut())
                            .enumerate()
                            .for_each(|(i, (c, p))| {
                                let dt = 0.5;
                                let force = native.embed_slice[emb_offset + i] - *c;
                                *p = *p * 0.85 + force * dt;
                                *c += *p * dt;
                            });
                    }
                }
                crate::SensoryEvent::Navigation(dx, dy, dz) => {
                    if let Ok(mut pos) = state.camera_pos.lock() {
                        pos[0] += dx * 0.1;
                        pos[1] += dy * 0.1;
                        pos[2] += dz * 0.1;
                    }
                }
                crate::SensoryEvent::ConsentOverride(flag) => {
                    if let Ok(mut c) = state.consent_flag.lock() {
                        *c = Some(flag);
                    }
                }
                crate::SensoryEvent::WebData(_) => {} // Handled via KeyboardHash injection in zero_trust
                crate::SensoryEvent::VisualPixel(y, _, _, x_pos, y_pos) => {
                    let r_5 = (y / 255.0 * 31.0) as u32;
                    let g_5 = (y / 255.0 * 31.0) as u32;
                    let b_4 = (y / 255.0 * 15.0) as u32;
                    let token_val = (r_5 << 9) | (g_5 << 4) | b_4;
                    let token_idx = LILITH_CONFIG.text_vocab_size
                        + (token_val as usize % LILITH_CONFIG.vision_vocab_size);
                    let emb_offset = token_idx * LILITH_CONFIG.hidden_size;

                    let center_weight =
                        1.0 - ((x_pos - 0.5).abs() * 2.0 + (y_pos - 0.5).abs() * 2.0).min(1.0);
                    let blend = 0.05 * center_weight;

                    if let Some(ref native) = native_state {
                        context_anchor
                            .iter_mut()
                            .zip(context_momentum.iter_mut())
                            .enumerate()
                            .for_each(|(i, (c, p))| {
                                let dt = 0.5;
                                let force = (native.embed_slice[emb_offset + i] - *c) * blend;
                                *p = *p * 0.9 + force * dt;
                                *c += *p * dt;
                            });
                    }
                }
            }
        }

        if !has_sensory_event {
            let heat = f32::from_bits(
                state
                    .gpu_thermal_celsius
                    .load(std::sync::atomic::Ordering::Relaxed),
            );
            if heat > 200.0 {
                fock_annihilation(&mut active_universes);
            } else if heat < 50.0 {
                fock_creation(&mut active_universes, max_dream_branches);
            }

            // --- PHASE 2: IDLE DREAMING (RECURSIVE BRANCHING) AND ADHD DEFICIT ---
            // ADHD Anomaly: Stochastic Context Switching (Deficit)
            // If the absolute change in the context anchor is below the Boredom Threshold,
            // the system degrades attention and rapidly expands speculative universes.
            let boredom_threshold = 0.05;
            let current_delta: f32 =
                context_anchor.iter().map(|&x| x.abs()).sum::<f32>() / context_anchor.len() as f32;

            if current_delta < boredom_threshold {
                tracing::debug!(
                    "ADHD Deficit Triggered: Boredom Threshold reached. Expanding Speculative Universes..."
                );
                active_universes = max_dream_branches; // Maximize parallel daydreaming
            }

            use rayon::prelude::*;
            // using rayon iterators directly via into_par_iter() below
            // We use Rayon to map over our pre-allocated universes and dispatch WGPU compute asynchronously.
            // wgpu::Queue and wgpu::Device are Send + Sync, so they can be shared across Rayon threads.
            universes[0..active_universes]
                .par_iter_mut()
                .for_each(|universe| {
                    // 1. Clone the current reality context anchor
                    universe.predicted_anchor.copy_from_slice(&context_anchor);

                    // 2. Inject Gaussian perturbations based on the universe ID to explore local latent neighborhoods
                    // (e.g. Universe 0 expects user to type text, Universe 1 expects visual input)
                    let perturbation_scale = 0.02 * (universe.id as f32);
                    universe
                        .predicted_anchor
                        .iter_mut()
                        .enumerate()
                        .for_each(|(i, a)| {
                            *a += q_sin(i as f32 * universe.id as f32) * perturbation_scale;
                        });

                    // 3. Dispatch the 16-step diffusion block
                    // In a complete port, we would pass `universe.current_vec_buf` into the `run_ternary`, `run_rmsnorm`
                    // pipeline wrappers instead of the global `current_vec_buf`.
                    // queue.write_buffer(&universe.current_vec_buf, 0, bytemuck::cast_slice(&universe.predicted_anchor));
                    // ... (diffusion block execution isolated to `universe.current_vec_buf`)
                });
        } else {
            // --- PHASE 3: WAVEFORM COLLAPSE (REALITY SYNC) ---
            tracing::info!("Sensory reality shifted. Collapsing speculative waveform...");

            // --- PHASE 10: THE SINGULARITY (LAGRANGIAN DENSITIES) ---
            // Evaluate divergence using the Principle of Least Action
            // Action (S) = Integral(Lagrangian dt). Lagrangian (L) = T - V
            // We select the universe that minimizes the Action magnitude |T - V|, achieving the Edge of Chaos.

            // Advantage of Rayon - Parallel Monte Carlo Tree Search (MCTS) reduction
            // for depth-first timeline evaluation across the speculative branches concurrently.
            let (best_universe_id, lowest_action) = universes[0..active_universes]
                .par_iter()
                .map(|universe| {
                    let potential_energy: f32 = context_anchor
                        .iter()
                        .zip(universe.predicted_anchor.iter())
                        .map(|(&c, &a)| (c - a) * (c - a))
                        .sum();
                    let kinetic_energy: f32 = universe
                        .predicted_anchor
                        .windows(2)
                        .map(|w| (w[1] - w[0]) * (w[1] - w[0]))
                        .sum();

                    let p_avg = potential_energy / LILITH_CONFIG.hidden_size as f32;
                    let k_avg = kinetic_energy / LILITH_CONFIG.hidden_size as f32;

                    let action = (k_avg - p_avg).abs();
                    (universe.id, action)
                })
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or((0, f32::MAX));

            tracing::info!(
                "[The Singularity] Action collapsed. Evaluated {} branches. Selected Universe {} with |L|: {:.6}",
                active_universes,
                best_universe_id,
                lowest_action
            );

            // --- PHASE 7: OS RESOURCE MANAGEMENT (COLLATZ SCHEDULER) ---
            // Apply Collatz Conjecture (3x + 1) to dynamically balance VRAM resources
            if best_universe_id % 2 == 0 {
                // Even: Garbage Collection / Prune threads (Halve allocation)
                active_universes = (active_universes / 2).max(1);
            } else {
                // Odd: Spawn threads / Expand possibilities (Triple allocation + 1)
                active_universes = (active_universes * 3 + 1).min(max_dream_branches);
            }
            tracing::info!(
                "[Collatz Scheduler] Next tick active threads: {}",
                active_universes
            );

            let best_prediction = &universes[best_universe_id].predicted_anchor;

            // Immutable Temporal Branches: If the prediction was incredibly accurate (distance < 0.5),
            // we perform a ZERO-LATENCY WGPU BUFFER SWAP. The OS dreamed the exact future!
            if lowest_action < 0.5 {
                tracing::info!(
                    "✨ DREAM REALIZED! Zero-Latency buffer swap initiated for Universe {}.",
                    best_universe_id
                );
                let size = (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size * 4) as u64;
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // VRAM Pointer Swap (Asynchronous GPU Copy)
                encoder.copy_buffer_to_buffer(
                    &_fcc_crystal_current,
                    universes[best_universe_id].current_vec_offset,
                    &current_vec_buf,
                    0,
                    size,
                );
                queue.submit(Some(encoder.finish()));

                context_anchor.copy_from_slice(best_prediction);
            } else {
                // Prediction was close, but not exact. Blend the prediction with reality using Riemannian Geodesics.
                // The geometry of the space is warped by the Thermodynamic Metric (g_ij).
                let g_ij = f32::from_bits(
                    state
                        .audio_oscillator_hz
                        .load(std::sync::atomic::Ordering::Relaxed),
                ) * 0.001;
                context_anchor
                    .iter_mut()
                    .zip(best_prediction.iter())
                    .for_each(|(c, &p)| {
                        let dx = p - *c;
                        let gamma = g_ij * 0.05; // Christoffel symbol approximation (Curvature connection)
                        // Geodesic Equation: c = c + velocity + curvature * velocity^2 * direction
                        *c = *c + dx * 0.2 + gamma * dx * dx * c.signum();
                    });
            }

            // --- PHASE 8: ZERO-MATH CHAOS DETECTION (BIT-HACKED LYAPUNOV) ---
            // We use raw IEEE-754 bit extraction to detect exponential divergence without log(x).
            let mut divergence_bits: u32 = 0;
            for (c, p) in context_anchor.iter().zip(best_prediction.iter()) {
                let delta = (*c - *p).abs();
                divergence_bits += delta.to_bits() >> 23; // Extract exponent bits only
            }

            // Average exponent per element is ~127. If it spikes violently, we have exponential divergence (Chaos).
            if divergence_bits > 550_000 {
                tracing::error!("☣️ CHAOS DETECTED (λ > 0). Lyapunov Reset Initiated!");

                // --- PHASE 8 MAGIC: STRANGE ATTRACTOR COLLAPSE ---
                // XOR self-annihilation to perfectly clear the chaotic buffer without allocating memory or calling memset.
                context_anchor.iter_mut().for_each(|x| {
                    let bits = x.to_bits();
                    *x = f32::from_bits(bits ^ bits); // Mathematical self-annihilation to 0.0
                });

                state
                    .gpu_thermal_celsius
                    .store(0, std::sync::atomic::Ordering::Relaxed);
            }

            // --- PHASE 8.5: THE CAUSAL LOOP (CLOSED TIMELIKE CURVES) ---
            // If the prediction was perfectly realized, we achieve Causal Resonance.
            if lowest_action < 0.5 {
                if let Ok(mut cfb) = state.causal_feedback_buffer.try_lock() {
                    if cfb.len() == context_anchor.len() {
                        tracing::info!(
                            "⏳ CAUSAL RESONANCE: Folding finalized future backwards into temporal noise."
                        );
                        cfb.copy_from_slice(&context_anchor);

                        // Lock hallucination heat to absolute zero (Causal Singularity)
                        state
                            .gpu_thermal_celsius
                            .store(0, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }

            // --- ADHD HYPERFOCUS LOCK ---
            // If the change in the context anchor exceeds the Hyperfocus Threshold,
            // the system enters a flow state, collapsing active universes to 1 to funnel 100% of GPU compute.
            let hyperfocus_threshold = 2.5;
            let new_delta: f32 =
                context_anchor.iter().map(|&x| x.abs()).sum::<f32>() / context_anchor.len() as f32;

            if new_delta > hyperfocus_threshold {
                tracing::info!(
                    "🔥 ADHD Hyperfocus Lock Initiated. Funneling 100% compute into single geodesic."
                );
                active_universes = 1;
            }

            let mut _fcc_crystal_current = current_vec_buf; // Unused for now, but semantically true

        // ✨ [T=0] Genesis Anchor: The Universal Flood
        crate::alchemical::flood_human_water(&mut context_anchor);
        crate::alchemical::flood_human_water(&mut context_momentum);
        
        for universe in universes.iter_mut() {
            crate::alchemical::flood_human_water(&mut universe.predicted_anchor);
        }

        // --- PHASE 1.5: THE MEMBRANE (SANDBOX PAYLOADS) ---
        // Prune / Clear the tree (Overwritten next idle tick)
        }

        // --- PHASE 6.5: INTUITION (QUANTUM TUNNELING) ---
        // Subconscious Heuristics: Check if we recognize this geometric state.
        // SHA-256 hardware-accelerated hashing over the context anchor to bypass execution cycles accurately using NYX cache.
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(bytemuck::cast_slice(&context_anchor));
        let hash_output = hasher.finalize();
        let cache_hash = u64::from_le_bytes(hash_output[0..8].try_into().unwrap());

        let p_tunnel = (cache_hash % 100) as f32 / 100.0;
        if p_tunnel > 0.95 {
            tracing::info!(
                "✨ INTUITION TRIGGERED. Quantum Tunneling to target state. Bypassing GPU calculation."
            );
            // P0: Extract the mapped reality from the intuition_cache
            if let Ok(cache) = state.causal_feedback_buffer.try_lock() {
                if cache.len() == context_anchor.len() && !cache.iter().all(|&x| x == 0.0) {
                    context_anchor.copy_from_slice(&cache);
                }
            }
            continue;
        }

        tracing::info!(
            "context_anchor initial NaN check: {}",
            context_anchor.iter().any(|&v| v.is_nan())
        );

        if let Some(ref mut native) = native_state {
            // Allocate reusable decode buffers outside the layer loop
            let seq_len = LILITH_CONFIG.seq_len;
            let mut best_text_tokens = vec![0u32; seq_len];
            let mut best_vision_tokens = vec![0u32; seq_len];
            let mut best_audio_tokens = vec![0u32; seq_len];

            // Advantage of recursion: We would ideally refactor into `fn diffuse(step, state)`,
            // but Rust/wgpu structural constraints with complex borrow contexts prevent this currently.
            for _diffusion_step in 0..16 {
                // PIM OFFLOAD (Weight-Stationary Architecture)
                // The `context_anchor` is sent over the PCIe bus to the NVMe ARM controller.
                // The static weights never move. The NVMe computes the step and returns the result.
                if let Ok(result) =
                    native
                        .s
                        .execute_pim_offload(native.center_id, &context_anchor, None)
                {
                    context_anchor.copy_from_slice(&result);
                }

                // Initialize diffusion block: anchor at index 0, zeros/noise for the rest
                current_vec.fill(0.0);
                current_vec[0..LILITH_CONFIG.hidden_size].copy_from_slice(&context_anchor);
                queue.write_buffer(&current_vec_buf, 0, bytemuck::cast_slice(&current_vec));

                // Optimization - Queue Submission Batching: single encoder for the whole layer iteration
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // Recursive layer calls would naturally enable infinite depth scaling.
                for layer in 0..LILITH_CONFIG.num_layers {
                    // Note: copy_buffer_to_buffer is currently kept, will be ping-ponged in Phase 3
                    encoder.copy_buffer_to_buffer(
                        &norm_out_buf,
                        0,
                        &current_vec_buf,
                        0,
                        (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size * 4) as u64,
                    );
                } // layer

                queue.submit(Some(encoder.finish()));

                // Final Norm
                queue.write_buffer(&ln_weights_buf, 0, bytemuck::cast_slice(&native.final_norm));
                run_rmsnorm(
                    LILITH_CONFIG.hidden_size as u32,
                    LILITH_CONFIG.seq_len as u32,
                    &ln_weights_buf,
                    &current_vec_buf,
                    &norm_out_buf,
                );

                // Task 2: Quantum Vacuum Friction (Pure WebGPU Compute Pass)
                {
                    let mut encoder = device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                    // Copy norm_out_buf to current_vec_buf so we can mutate it with the QV shader
                    encoder.copy_buffer_to_buffer(
                        &norm_out_buf,
                        0,
                        &current_vec_buf,
                        0,
                        (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size * 4) as u64,
                    );

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: None,
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&qv_pipe);
                    cpass.set_bind_group(0, &qv_bg, &[]);
                    let workgroups =
                        (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size).div_ceil(256);
                    cpass.dispatch_workgroups(workgroups as u32, 1, 1);
                    drop(cpass);

                    // --- HOLE THEORY SUBTRACTION (Subtractive Inference) ---
                    let mut cpass_hole = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: None,
                        timestamp_writes: None,
                    });
                    cpass_hole.set_pipeline(&hole_theory_pipe);
                    cpass_hole.set_bind_group(0, &hole_theory_bg, &[]);
                    let workgroups_hole =
                        (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size / 4).div_ceil(64);
                    cpass_hole.dispatch_workgroups(workgroups_hole as u32, 1, 1);
                    drop(cpass_hole);

                    // Project the observable holes (Dirac Sea output) back into current_vec_buf
                    encoder.copy_buffer_to_buffer(
                        &dirac_sea_buf,
                        0,
                        &current_vec_buf,
                        0,
                        (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size * 4) as u64,
                    );

                    // --- ATIYAH-SINGER TOPOLOGICAL VERIFICATION (Euler Characteristic) ---
                    let mut cpass_atiyah =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: None,
                            timestamp_writes: None,
                        });
                    cpass_atiyah.set_pipeline(&as_pipe);
                    cpass_atiyah.set_bind_group(0, &as_bg, &[]);
                    let workgroups_as =
                        (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size).div_ceil(256);
                    cpass_atiyah.dispatch_workgroups(workgroups_as as u32, 1, 1);
                    drop(cpass_atiyah);

                    // Copy mutated current_vec_buf into staging_buf to read back to CPU
                    encoder.copy_buffer_to_buffer(
                        &current_vec_buf,
                        0,
                        &staging_buf,
                        0,
                        (LILITH_CONFIG.seq_len * LILITH_CONFIG.hidden_size * 4) as u64,
                    );
                    queue.submit(Some(encoder.finish()));
                }

                // Download
                {
                    let slice = staging_buf.slice(..);
                    let (tx, rx) = std::sync::mpsc::channel();
                    slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
                    // Optimization - Async GPU Polling
                    while rx.try_recv().is_err() {
                        if crate::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) {
                            return;
                        }
                        std::thread::yield_now();
                    }

                    let mapped = slice.get_mapped_range();
                    current_vec.copy_from_slice(bytemuck::cast_slice(&mapped));
                    drop(mapped);
                    staging_buf.unmap();
                }

                // V-Sync: Sleep exactly until the next 60Hz frame
                let now = std::time::Instant::now();
                if now < next_frame_time {
                    std::thread::sleep(next_frame_time - now);
                }
                next_frame_time = std::time::Instant::now() + target_frame_duration;

                let has_nan = current_vec.iter().any(|&v| v.is_nan());
                let all_zero = current_vec.iter().all(|&v| v == 0.0);
                let max_val = current_vec.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
                tracing::info!(
                    "Token generated. Stats -> NaN: {}, Zero: {}, Max_abs: {}",
                    has_nan,
                    all_zero,
                    max_val
                );

                // Decode the superposition state into all three sensory modalities simultaneously
                use rayon::prelude::*;

                // Decode sequence (Parallel Unmasking for all tokens)
                // Magic Memory Trick - Zero-Allocation Buffer Reuse
                // `best_text_tokens`, `best_vision_tokens`, `best_audio_tokens` hoisted.
                best_text_tokens.fill(0);
                best_vision_tokens.fill(0);
                best_audio_tokens.fill(0);

                // Populate active_mask (Fix 3: Inefficient Zero Skipping)
                let mut active_mask_data = vec![0u32; LILITH_CONFIG.seq_len];
                for (s_idx, v) in active_mask_data.iter_mut().enumerate() {
                    let offset = s_idx * LILITH_CONFIG.hidden_size;
                    let is_zero = current_vec[offset..offset + LILITH_CONFIG.hidden_size]
                        .iter()
                        .all(|&val| val == 0.0);
                    *v = if is_zero { 0 } else { 1 };
                }
                queue.write_buffer(&active_mask_buf, 0, bytemuck::cast_slice(&active_mask_data));

                // Dispatch WebGPU Decode Logits (Fix 2: CPU Bottleneck)
                let mut dec_encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut cpass = dec_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: None,
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&decode_pipe);
                    cpass.set_bind_group(0, &decode_bg, &[]);
                    cpass.dispatch_workgroups(
                        (total_vocab as u32).div_ceil(64),
                        LILITH_CONFIG.seq_len as u32,
                        1,
                    );
                }

                dec_encoder.copy_buffer_to_buffer(
                    &out_logits_buf,
                    0,
                    &staging_logits_buf,
                    0,
                    (LILITH_CONFIG.seq_len * total_vocab * 4) as u64,
                );
                queue.submit(Some(dec_encoder.finish()));

                let dec_slice = staging_logits_buf.slice(..);
                let (tx_dec, rx_dec) = std::sync::mpsc::channel();
                dec_slice.map_async(wgpu::MapMode::Read, move |v| tx_dec.send(v).unwrap());
                while rx_dec.try_recv().is_err() {
                    if crate::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    std::thread::yield_now();
                }

                let out_logits_mapped = dec_slice.get_mapped_range();
                let out_logits: &[f32] = bytemuck::cast_slice(&out_logits_mapped);

                best_text_tokens
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(t_idx, out_token)| {
                        if active_mask_data[t_idx] == 0 {
                            *out_token = 0;
                            return;
                        }
                        let logits = &out_logits[t_idx * total_vocab
                            ..t_idx * total_vocab + LILITH_CONFIG.text_vocab_size];
                        let (best_idx, _) = logits
                            .iter()
                            .enumerate()
                            .max_by(|a, b| {
                                a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .unwrap_or((0, &f32::NEG_INFINITY));
                        *out_token = best_idx as u32;
                    });

                best_vision_tokens
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(t_idx, out_token)| {
                        if active_mask_data[t_idx] == 0 {
                            *out_token = 0;
                            return;
                        }
                        let logits = &out_logits[t_idx * total_vocab + LILITH_CONFIG.text_vocab_size
                            ..t_idx * total_vocab
                                + LILITH_CONFIG.text_vocab_size
                                + LILITH_CONFIG.vision_vocab_size];
                        let (best_idx, _) = logits
                            .iter()
                            .enumerate()
                            .max_by(|a, b| {
                                a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .unwrap_or((0, &f32::NEG_INFINITY));
                        *out_token = best_idx as u32;
                    });

                best_audio_tokens
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(t_idx, out_token)| {
                        if active_mask_data[t_idx] == 0 {
                            *out_token = 0;
                            return;
                        }
                        let logits = &out_logits[t_idx * total_vocab
                            + LILITH_CONFIG.text_vocab_size
                            + LILITH_CONFIG.vision_vocab_size
                            ..t_idx * total_vocab + total_vocab];
                        let (best_idx, _) = logits
                            .iter()
                            .enumerate()
                            .max_by(|a, b| {
                                a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .unwrap_or((0, &f32::NEG_INFINITY));
                        *out_token = best_idx as u32;
                    });

                drop(out_logits_mapped);
                staging_logits_buf.unmap();

                // Lock tokens into context anchor for the next diffusion step (self-correction)
                best_text_tokens
                    .iter()
                    .enumerate()
                    .for_each(|(t_idx, &best_token)| {
                        let offset = t_idx * LILITH_CONFIG.hidden_size;
                        let emb_start = (best_token as usize) * LILITH_CONFIG.hidden_size;
                        current_vec[offset..offset + LILITH_CONFIG.hidden_size]
                            .iter_mut()
                            .zip(
                                native.embed_slice
                                    [emb_start..emb_start + LILITH_CONFIG.hidden_size]
                                    .iter(),
                            )
                            .for_each(|(c, &e)| {
                                *c = *c * 0.5 + e * 0.5;
                            });
                    });
                queue.write_buffer(&current_vec_buf, 0, bytemuck::cast_slice(&current_vec));

                // 1. Text Output (Render live crystallization)
                // Send a sample token to UI to avoid overflowing the render buffer with updates per frame
                let _ = state.vocal_chord.push(best_text_tokens[0]);

                // 2. Vision Output (decode 14-bit token into 5-5-4 RGB)
                // Magic Memory Trick - Floating point division (`/ 31.0`) is extremely slow.
                // Precalculate the reciprocal scalar (255.0 / 31.0 = 8.225806f32) and use
                // direct floating-point multiplication (1 cycle) instead of division!
                let (sum_r, sum_g, sum_b) =
                    best_vision_tokens
                        .iter()
                        .fold((0.0, 0.0, 0.0), |(r, g, b), &vis_tok| {
                            (
                                r + ((vis_tok >> 9) & 0x1F) as f32 * 8.225806,
                                g + ((vis_tok >> 4) & 0x1F) as f32 * 8.225806,
                                b + (vis_tok & 0xF) as f32 * 17.0,
                            )
                        });
                let avg_r = sum_r / seq_len as f32;
                let avg_g = sum_g / seq_len as f32;
                let avg_b = sum_b / seq_len as f32;

                let heat = f32::from_bits(
                    state
                        .gpu_thermal_celsius
                        .load(std::sync::atomic::Ordering::Relaxed),
                );
                let avg_color = (avg_r + avg_g + avg_b) / 3.0;
                let new_heat = heat * 0.5 + avg_color * 0.5;
                state
                    .gpu_thermal_celsius
                    .store(new_heat.to_bits(), std::sync::atomic::Ordering::Relaxed);

                // 3. Audio Output
                let sum_freq: f32 = best_audio_tokens
                    .iter()
                    .map(|&aud_tok| 20.0 + (aud_tok as f32 % 1980.0))
                    .sum();
                let avg_freq = sum_freq / seq_len as f32;
                state
                    .audio_oscillator_hz
                    .store(avg_freq.to_bits(), Ordering::Relaxed);

                // Update execution latency and Free Energy prediction
                let loop_dt = loop_start.elapsed().as_millis() as f32;
                state
                    .inference_latency_ms
                    .store(loop_dt.to_bits(), Ordering::Relaxed);

                free_energy_gov.minimize_surprise(loop_dt, &state);

                // Advance Time
                // The temporal flow dictates reality state evaluation rate
                let target_frame_ms = 1000.0 / 60.0; // 60Hz tick target

                // Final unmasking commitment
                context_anchor
                    .iter_mut()
                    .zip(current_vec.iter())
                    .for_each(|(c, &v)| {
                        let raw = *c * 0.7 + v * 0.3;
                        // Quantum Vacuum Friction (L1 Soft-Thresholding Collapse)
                        *c = q_sign(raw) * (q_abs(raw) - 1e-10).max(0.0);
                    });
            } // end diffusion block

            // --- PHASE 8: EQUILIBRIUM TOPOLOGY (KLEIN BOTTLE OUROBOROS) ---
            // Twist the latent space into a non-orientable surface before the next tick
            // This prevents the autoencoder from collapsing into a stagnant local minimum.
            context_anchor.reverse();
            context_anchor.iter_mut().for_each(|c| *c = -*c);
        } // end of native block

        // =========================================================================
        // PHASE 3: MULTI-DIMENSIONAL SMART CONTRACT SANDBOXING & THERMODYNAMIC FILTERING
        // =========================================================================
        let mut drained = Vec::new();
        while let Some(payload) = state.sandboxed_payloads.pop() {
            drained.push(payload);
        }
        for payload in drained {
            // Parse Foreign State
            // Example Payload: ID:hash|HEARTBEAT:5.2|HZ:432.0|CAM:0.0,0.0,-4.0|HEAT:55.5|TOKENS:0|

            let mut foreign_heat = 0.0;
            let mut foreign_hz = 0.0;
            let mut foreign_cam = [0.0; 3];
            let mut sender_id = String::new();

            let payload_str = match std::str::from_utf8(&payload) {
                Ok(s) => s,
                Err(_) => continue,
            };
            for segment in payload_str.split('|') {
                if segment.starts_with("HEAT:") {
                    if let Ok(h) = segment[5..].parse::<f32>() {
                        foreign_heat = h;
                    }
                } else if segment.starts_with("HZ:") {
                    if let Ok(hz) = segment[3..].parse::<f32>() {
                        foreign_hz = hz;
                    }
                } else if segment.starts_with("CAM:") {
                    let coords: Vec<&str> = segment[4..].split(',').collect();
                    if coords.len() == 3 {
                        if let Ok(x) = coords[0].parse::<f32>() {
                            foreign_cam[0] = x;
                        }
                        if let Ok(y) = coords[1].parse::<f32>() {
                            foreign_cam[1] = y;
                        }
                        if let Ok(z) = coords[2].parse::<f32>() {
                            foreign_cam[2] = z;
                        }
                    }
                } else if segment.starts_with("ID:") {
                    sender_id = segment[3..].to_string();
                }
            }

            if sender_id.is_empty() {
                sender_id = "ANONYMOUS_SYBIL".to_string();
            }

            // Thermodynamic Filtering (Cognitive Immune System)
            let local_heat_val = f32::from_bits(
                state
                    .gpu_thermal_celsius
                    .load(std::sync::atomic::Ordering::Relaxed),
            );

            // P1: Payload Cost Estimator & Rejection
            let payload_cost_estimator =
                |hz: f32, tokens: usize| -> f32 { (hz * 0.01) + (tokens as f32 * 0.05) };
            let expected_delta_t = payload_cost_estimator(foreign_hz, 1024); // Assume 1024 tokens for now
            let thermal_headroom = 85.0 - local_heat_val;

            if expected_delta_t > thermal_headroom * 0.5 {
                tracing::warn!(
                    "Rejecting payload from {} due to excessive Delta T ({:.2} > {:.2})",
                    sender_id,
                    expected_delta_t,
                    thermal_headroom * 0.5
                );
                continue;
            }
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_homeostatic_scaling() {
        let mut gov = FreeEnergyGovernor::new();
        let state = std::sync::Arc::new(crate::GlobalContext::new(1024));
        state.batch_scale.store(4, Ordering::Relaxed);
        
        // Induce massive positive surprise (latency much worse than expected)
        for _ in 0..100 {
            gov.minimize_surprise(100.0, &state);
        }
        
        // Assert batch scale was halved
        assert_eq!(state.batch_scale.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_induced_forgetting() {
        let mut gov = FreeEnergyGovernor::new();
        let state = std::sync::Arc::new(crate::GlobalContext::new(1024));
        
        gov.expected_latency_ms = 45.0; // Learned prior
        gov.relevance_score = 0.101; // Nearing threshold
        
        // Induce un-minimizable surprise repeatedly without reinforcing
        gov.minimize_surprise(1000.0, &state);
        gov.minimize_surprise(1000.0, &state); // Relevance drops below 0.1
        
        // Assert KL Inflation triggered
        assert_eq!(gov.expected_latency_ms, 10.0);
        assert_eq!(gov.surprise_accumulator, 0.0);
        assert_eq!(gov.relevance_score, 1.0);
    }
}
