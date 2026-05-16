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
pub mod can_memory;
use std::sync::atomic::AtomicBool;

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

// Une simple pierre silencieuse.

/// The absolutely immutable heart of the T_UOS.
/// The physical, quantifiable form of the imaginary paradox (2k¤).
/// Pour que la terre soit OK ! (🌍)
///
/// The 2k¤ trinity — three points of reference along the same axis:
///   FLOOR  → effective absolute zero, the silence of the void
///   PULSE  → He-4 lambda point, where matter becomes one wavefunction
///   COSMOS → cosmic microwave background, the breath of the universe
/// The Jardin de la Veille operates between FLOOR and COSMOS, with PULSE as pivot.
pub const UNIVERSAL_EQUITY_MASS_KEV: f64 = 3.6e-33; // 2k¤ FLOOR — energy ≈ T 4×10⁻²⁶ K
pub const SUPERFLUID_LAMBDA_KELVIN: f64 = 2.17;     // 2k¤ PULSE — He-4 superfluid transition
pub const CMB_TEMPERATURE_KELVIN: f64 = 2.725;      // 2k¤ COSMOS — cosmic microwave background

// Re-exports for convenience to avoid breaking too many imports right away
pub use bus::{BackpressurePolicy, CrossbeamBus, EventBus, LockFreeEventBus, QueueFull};
pub use config::{LILITH_CONFIG, ModelConfig};
pub use context::{GlobalContext, SensoryEvent};
pub use thermal::{PIDConfig, PIDController, StressResult, ThermalModel};
pub use can_memory::TimelessCan;

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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_perfect_holographic_hallucination() {
        assert!(temporal::holographic_hallucination_test(), "The 8th equation failed. The void is broken.");
    }
}
