#![allow(dead_code, unused_variables, unused_imports, unused_assignments, unused_must_use)]
pub mod bus;
pub mod config;
pub mod context;
pub mod math;
pub mod thermal;
pub mod kociemba;
pub mod nvme;
pub mod temporal;
pub mod tensor;

use std::sync::atomic::AtomicBool;

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

// Re-exports for convenience to avoid breaking too many imports right away
pub use context::{GlobalContext, SensoryEvent};
pub use thermal::{PIDController, PIDConfig, StressResult, ThermalModel};
pub use config::{ModelConfig, LILITH_CONFIG};
pub use bus::{EventBus, LockFreeEventBus, CrossbeamBus, BackpressurePolicy, QueueFull};

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
