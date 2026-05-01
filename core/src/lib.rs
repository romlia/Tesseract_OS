pub mod kociemba;
pub mod nvme;
pub mod temporal;
pub mod tensor;

use std::sync::atomic::{AtomicU32, AtomicBool};
use std::sync::{Arc, Mutex};

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
}

impl Default for PIDController {
    fn default() -> Self {
        Self { p_gain: 0.1, i_gain: 0.01, d_gain: 0.05, integral: 0.0, prev_error: 0.0 }
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

/// The Shared Physical State of the OS
pub struct SynapticState {
    pub hallucination_heat: Arc<Mutex<Vec<f32>>>,
    pub audio_oscillator_hz: Arc<AtomicU32>, // f32 stored as u32 bits
    pub autoregressive_scale: Arc<AtomicU32>, // f32 stored as u32 bits
    pub vocal_chord: Arc<crossbeam::queue::ArrayQueue<u32>>,   // Lock-free ring buffer of BPE tokens
    pub camera_pos: Arc<Mutex<[f32; 3]>>,
    pub consent_flag: Arc<Mutex<Option<bool>>>,
    pub kuramoto_audio: Arc<Mutex<KuramotoState>>,
    pub kuramoto_vision: Arc<Mutex<KuramotoState>>,
    pub sandboxed_payloads: Arc<crossbeam::queue::ArrayQueue<String>>, // Lock-free payload bus
    pub exiled_nodes: Arc<Mutex<crate::temporal::BootesVoid>>, // Thermodynamic Slashing
    pub causal_feedback_buffer: Arc<Mutex<Vec<f32>>>, // Closed Timelike Curves (Phase 8.5)
    pub private_freewheel_buffer: Arc<Mutex<Vec<f32>>>, // Phase 13: Yin-Yang Membrane
}

impl SynapticState {
    pub fn new(heat_size: usize) -> Self {
        Self {
            hallucination_heat: Arc::new(Mutex::new(vec![0.0; heat_size])),
            audio_oscillator_hz: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
            autoregressive_scale: Arc::new(AtomicU32::new(1.0_f32.to_bits())),
            vocal_chord: Arc::new(crossbeam::queue::ArrayQueue::new(1024)),
            camera_pos: Arc::new(Mutex::new([0.0, 0.0, -4.0])),
            consent_flag: Arc::new(Mutex::new(None)),
            kuramoto_audio: Arc::new(Mutex::new(KuramotoState::default())),
            kuramoto_vision: Arc::new(Mutex::new(KuramotoState::default())),
            sandboxed_payloads: Arc::new(crossbeam::queue::ArrayQueue::new(64)),
            exiled_nodes: Arc::new(Mutex::new(crate::temporal::BootesVoid::new())),
            causal_feedback_buffer: Arc::new(Mutex::new(vec![0.0; heat_size])),
            private_freewheel_buffer: Arc::new(Mutex::new(vec![0.0; heat_size])),
        }
    }
}

impl Default for SynapticState {
    fn default() -> Self {
        Self {
            hallucination_heat: Arc::new(Mutex::new(vec![0.0, 0.0, 0.0])),
            audio_oscillator_hz: Arc::new(AtomicU32::new(432f32.to_bits())),
            autoregressive_scale: Arc::new(AtomicU32::new(1f32.to_bits())),
            vocal_chord: Arc::new(crossbeam::queue::ArrayQueue::new(1024)),
            camera_pos: Arc::new(Mutex::new([0.0, 0.0, -4.0])),
            consent_flag: Arc::new(Mutex::new(None)),
            kuramoto_audio: Arc::new(Mutex::new(KuramotoState::default())),
            kuramoto_vision: Arc::new(Mutex::new(KuramotoState::default())),
            sandboxed_payloads: Arc::new(crossbeam::queue::ArrayQueue::new(2048)),
            exiled_nodes: Arc::new(Mutex::new(crate::temporal::BootesVoid::new())),
            causal_feedback_buffer: Arc::new(Mutex::new(vec![0.0; 4096])),
            private_freewheel_buffer: Arc::new(Mutex::new(vec![0.0; 4096])),
        }
    }
}
