use crate::bus::EventBus;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize};
use std::sync::{Arc, Mutex};

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

/// The Shared Physical State of the OS (Refactored for Production Prototype)
pub struct GlobalContext {
    pub inference_latency_ms: AtomicU32,
    pub gpu_thermal_celsius: AtomicU32,
    pub active_tokens: AtomicU32,
    pub hallucination_threshold: AtomicU32,
    pub thermal_limit_celsius: AtomicU32,

    pub batch_scale: AtomicUsize,
    pub event_epoch_seq: AtomicU64,
    pub vocal_chord: Arc<dyn EventBus<u32>>,
    pub camera_pos: Mutex<[f32; 3]>,
    pub audio_oscillator_hz: AtomicU32,
    pub consent_flag: Mutex<Option<bool>>,
    pub exiled_nodes: Mutex<Vec<String>>,
    pub causal_feedback_buffer: Mutex<Vec<f32>>,
    pub sandboxed_payloads: Arc<dyn EventBus<Vec<u8>>>,
}

impl GlobalContext {
    pub fn new(hidden_size: usize) -> Self {
        Self {
            inference_latency_ms: AtomicU32::new(0),
            gpu_thermal_celsius: AtomicU32::new(40.0f32.to_bits()), // Start at 40C
            active_tokens: AtomicU32::new(0),
            hallucination_threshold: AtomicU32::new(f32::to_bits(0.85)),
            thermal_limit_celsius: AtomicU32::new(85.0f32.to_bits()), // Default throttle point
            batch_scale: AtomicUsize::new(1),
            event_epoch_seq: AtomicU64::new(0),
            vocal_chord: Arc::new(crate::bus::CrossbeamBus::new(1024)),
            camera_pos: Mutex::new([0.0, 0.0, -5.0]),
            audio_oscillator_hz: AtomicU32::new(f32::to_bits(440.0)),
            consent_flag: Mutex::new(None),
            exiled_nodes: Mutex::new(Vec::new()),
            causal_feedback_buffer: Mutex::new(Vec::with_capacity(hidden_size)),
            sandboxed_payloads: Arc::new(crate::bus::CrossbeamBus::new(128)),
        }
    }

    pub fn spawn_diagnostic_socket(state: Arc<GlobalContext>) {
        std::thread::spawn(move || {
            let socket_path = "/tmp/tesseract_shader.sock";
            let _ = std::fs::remove_file(socket_path);
            if let Ok(listener) = std::os::unix::net::UnixListener::bind(socket_path) {
                tracing::info!("Diagnostic Socket bound at {}", socket_path);
                for stream in listener.incoming() {
                    if let Ok(mut stream) = stream {
                        use std::io::Write;
                        let state_ref = state.clone();
                        std::thread::spawn(move || {
                            loop {
                                if crate::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) {
                                    break;
                                }

                                let gpu_temp = f32::from_bits(
                                    state_ref
                                        .gpu_thermal_celsius
                                        .load(std::sync::atomic::Ordering::Relaxed),
                                );
                                let latency = f32::from_bits(
                                    state_ref
                                        .inference_latency_ms
                                        .load(std::sync::atomic::Ordering::Relaxed),
                                );
                                let tokens = state_ref
                                    .active_tokens
                                    .load(std::sync::atomic::Ordering::Relaxed);
                                let epoch = state_ref
                                    .event_epoch_seq
                                    .load(std::sync::atomic::Ordering::Relaxed);

                                let payload = serde_json::json!({
                                    "gpu_thermal_celsius": gpu_temp,
                                    "inference_latency_ms": latency,
                                    "active_tokens": tokens,
                                    "event_epoch_seq": epoch,
                                    "status": "WGSL SIMD128 Active"
                                });

                                let mut json_bytes =
                                    serde_json::to_vec(&payload).unwrap_or_default();
                                json_bytes.push(b'\n');

                                if stream.write_all(&json_bytes).is_err() {
                                    break; // Client disconnected
                                }

                                std::thread::sleep(std::time::Duration::from_millis(100)); // 10Hz streaming
                            }
                        });
                    }
                }
            }
        });
    }
}
