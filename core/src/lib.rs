pub mod kociemba;
pub mod nvme;
pub mod temporal;
pub mod tensor;

use std::sync::atomic::{AtomicU32, AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

/// Result of the synthetic stress test.
#[derive(Debug, Serialize, Deserialize)]
pub struct StressResult {
    pub avg_temp: f32,
    pub peak_temp: f32,
    pub duration_ms: u64,
}

/// Minimal thermal model derived from the stress test.
#[derive(Debug, Serialize, Deserialize)]
pub struct ThermalModel {
    pub thermal_mass: f32,
    pub thermal_resistance: f32,
}

/// PID controller configuration that will be cached.
#[derive(Debug, Serialize, Deserialize)]
pub struct PIDConfig {
    pub p_gain: f32,
    pub i_gain: f32,
    pub d_gain: f32,
    pub hysteresis_low: f32,
    pub hysteresis_high: f32,
    pub ema_alpha: f32,
}

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
pub enum SensoryEvent {
    AudioAmplitude(f32),
    Terminate,
    VisualKinetic(f32),
    KeyboardHash(u32),                    // BPE token ID
    VisualPixel(f32, f32, f32, f32, f32), // R, G, B, X, Y
    WebData(String),
    Navigation(f32, f32, f32), // dx, dy, dz
    ConsentOverride(bool),     // true for Y, false for ESC
    CommitPrompt,              // Signals the Brain to process the current context
}

pub struct PIDController {
    pub p_gain: f32,
    pub i_gain: f32,
    pub d_gain: f32,
    pub integral: f32,
    pub prev_error: f32,
    pub hysteresis_band: (f32, f32),
}

impl Default for PIDController {
    fn default() -> Self {
        Self::desktop_rtx4090()
    }
}

impl PIDController {
    pub fn desktop_rtx4090() -> Self {
        Self { 
            p_gain: 0.1, 
            i_gain: 0.01, 
            d_gain: 0.05, 
            integral: 0.0, 
            prev_error: 0.0,
            hysteresis_band: (78.0, 82.0),
        }
    }

    pub fn edge_orin_nano() -> Self {
        Self { 
            p_gain: 0.05, 
            i_gain: 0.005, 
            d_gain: 0.02, 
            integral: 0.0, 
            prev_error: 0.0,
            hysteresis_band: (65.0, 75.0),
        }
    }
    
    // [COMMERCIALIZATION TODO]: Auto-Tuning Calibration & Persistence
    pub fn calibrate_on_boot() -> Self {
        // Attempt to load from cache
        if let Ok(data) = std::fs::read("/var/lib/tesseract/pid.json") {
            if let Ok(cached_cfg) = serde_json::from_slice::<PIDConfig>(&data) {
                tracing::info!("Loaded cached PID configuration from disk.");
                return Self {
                    p_gain: cached_cfg.p_gain,
                    i_gain: cached_cfg.i_gain,
                    d_gain: cached_cfg.d_gain,
                    integral: 0.0,
                    prev_error: 0.0,
                    hysteresis_band: (cached_cfg.hysteresis_low, cached_cfg.hysteresis_high),
                };
            }
        }
        
        tracing::info!("Running PID auto-calibration synthetic stress test...");
        let stress = Self::run_stress_test(1024, 5);
        let model = Self::estimate_thermal_model(&stress, 40.0, 10.0);
        let cfg = Self::compute_pid_params(&model, 80.0, 10.0);
        
        let _ = Self::persist_pid_config(&cfg);
        
        Self {
            p_gain: cfg.p_gain,
            i_gain: cfg.i_gain,
            d_gain: cfg.d_gain,
            integral: 0.0,
            prev_error: 0.0,
            hysteresis_band: (cfg.hysteresis_low, cfg.hysteresis_high),
        }
    }
    
    fn run_stress_test(matrix_dim: usize, iterations: usize) -> StressResult {
        let start = std::time::Instant::now();
        let mut peak_temp = 40.0;
        
        for _ in 0..iterations {
            // Dummy computation to prevent LLVM optimization
            let mut _dummy = 0.0;
            for i in 0..matrix_dim {
                for j in 0..matrix_dim {
                    _dummy += (i * j) as f32 * 0.0001;
                }
            }
            
            // Read thermal zone
            if let Ok(temp_str) = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
                if let Ok(milli_c) = temp_str.trim().parse::<f32>() {
                    let c = milli_c / 1000.0;
                    if c > peak_temp { peak_temp = c; }
                }
            }
        }
        
        StressResult {
            avg_temp: peak_temp * 0.9,
            peak_temp,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }
    
    fn estimate_thermal_model(stress: &StressResult, baseline_temp: f32, power_estimate_w: f32) -> ThermalModel {
        let dt = (stress.peak_temp - baseline_temp).max(0.1);
        let r = dt / power_estimate_w;
        let c = power_estimate_w * (stress.duration_ms as f32 / 1000.0) / dt;
        ThermalModel { thermal_mass: c, thermal_resistance: r }
    }
    
    fn compute_pid_params(model: &ThermalModel, target_temp: f32, max_power_w: f32) -> PIDConfig {
        let kp = ((model.thermal_resistance * max_power_w - target_temp) / max_power_w).max(0.01);
        PIDConfig {
            p_gain: kp,
            i_gain: 0.1 * kp,
            d_gain: 0.05 * kp,
            hysteresis_low: target_temp - 2.0,
            hysteresis_high: target_temp + 2.0,
            ema_alpha: 0.2,
        }
    }
    
    fn persist_pid_config(cfg: &PIDConfig) -> std::io::Result<()> {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        
        std::fs::create_dir_all("/var/lib/tesseract")?;
        let tmp_path = "/var/lib/tesseract/pid.json.tmp";
        let final_path = "/var/lib/tesseract/pid.json";
        
        let mut tmp = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(tmp_path)?;
        let data = serde_json::to_vec_pretty(cfg)?;
        tmp.write_all(&data)?;
        tmp.sync_all()?;
        
        std::fs::rename(tmp_path, final_path)?;
        std::fs::set_permissions(final_path, std::fs::Permissions::from_mode(0o600))?;
        Ok(())
    }
}

pub struct KuramotoState {
    pub phase: f32,
    pub natural_freq: f32,
    pub coupling_strength: f32,
    pub pid: PIDController,
}

impl Default for KuramotoState {
    fn default() -> Self {
        Self { phase: 0.0, natural_freq: 1.0, coupling_strength: 0.5, pid: PIDController::default() }
    }
}

// Remove duplicate AtomicU64 import

/// The Shared Physical State of the OS (Refactored for Production Prototype)
pub struct GlobalContext {
    pub inference_latency_ms: AtomicU32,
    pub gpu_thermal_celsius: AtomicU32,
    pub active_tokens: AtomicU32,
    pub hallucination_threshold: AtomicU32,
    pub thermal_limit_celsius: AtomicU32,
    
    // [COMMERCIALIZATION TODO]: ABA Prevention Sequence Numbers
    // To prevent ABA race conditions under extreme thread contention, 
    // we must track sequence epochs for the sensory buffers.
    pub event_epoch_seq: AtomicU64,
    pub vocal_chord: crossbeam::queue::ArrayQueue<u32>,
    pub camera_pos: Mutex<[f32; 3]>,
    pub audio_oscillator_hz: AtomicU32,
    pub consent_flag: Mutex<Option<bool>>,
    pub exiled_nodes: Mutex<Vec<String>>,
    pub causal_feedback_buffer: Mutex<Vec<f32>>,
    pub sandboxed_payloads: crossbeam::queue::ArrayQueue<Vec<u8>>,
}

impl GlobalContext {
    pub fn new(hidden_size: usize) -> Self {
        Self {
            inference_latency_ms: AtomicU32::new(0),
            gpu_thermal_celsius: AtomicU32::new(40.0f32.to_bits()), // Start at 40C
            active_tokens: AtomicU32::new(0),
            hallucination_threshold: AtomicU32::new(f32::to_bits(0.85)),
            thermal_limit_celsius: AtomicU32::new(85.0f32.to_bits()), // Default throttle point
            event_epoch_seq: AtomicU64::new(0),
            vocal_chord: crossbeam::queue::ArrayQueue::new(1024),
            camera_pos: Mutex::new([0.0, 0.0, -5.0]),
            audio_oscillator_hz: AtomicU32::new(f32::to_bits(440.0)),
            consent_flag: Mutex::new(None),
            exiled_nodes: Mutex::new(Vec::new()),
            causal_feedback_buffer: Mutex::new(Vec::with_capacity(hidden_size)),
            sandboxed_payloads: crossbeam::queue::ArrayQueue::new(128),
        }
    }
}
