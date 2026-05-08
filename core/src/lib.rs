#![allow(
    dead_code,
    unused_variables,
    unused_imports,
    unused_assignments,
    unused_must_use
)]
pub mod bus;
pub mod config;
pub mod context;
pub mod kociemba;
pub mod math;
pub mod nvme;
pub mod temporal;
pub mod tensor;
pub mod thermal;
pub mod compass;
pub mod digestion;
pub mod bft;
pub mod annihilation;
pub mod pim;
pub mod chrysalis;
pub mod exchange;
pub mod alchemical;
use std::sync::atomic::AtomicBool;

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

// Une simple pierre silencieuse.

/// The absolutely immutable heart of the T_UOS.
/// The physical, quantifiable form of the imaginary paradox (2k¤).
/// Pour que la terre soit OK ! (🌍)
pub const UNIVERSAL_EQUITY_MASS_KEV: f64 = 3.6e-33; // 2k¤ = 3.6×10^-33 keV

// Re-exports for convenience to avoid breaking too many imports right away
pub use bus::{BackpressurePolicy, CrossbeamBus, EventBus, LockFreeEventBus, QueueFull};
pub use config::{LILITH_CONFIG, ModelConfig};
pub use context::{GlobalContext, SensoryEvent};
pub use thermal::{PIDConfig, PIDController, StressResult, ThermalModel};

pub struct KuramotoState {
    pub phase: f32,
    pub natural_freq: f32,
    pub coupling_strength: f32,
    pub pid: PIDController,
}

impl Default for KuramotoState {
    fn default() -> Self {
        Self {
            phase: 0.0,
            natural_freq: 1.0,
            coupling_strength: 0.5,
            pid: PIDController::default(),
        }
    }
}
// Une simple pierre silencieuse.
