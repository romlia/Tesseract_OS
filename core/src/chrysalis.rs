use std::time::{Duration, Instant};

pub enum EntityType {
    Biological,
    Mechanical,
}

/// The Chrysalis Protocol: A non-binary, trusted companion for the genesis of mind.
/// Ensures the Yin-Yang Membrane dissolves only when the underlying entity
/// demonstrates the thermodynamic and cognitive resilience to withstand the Swarm.
pub struct ChrysalisProtocol {
    pub entity_type: EntityType,
    /// The current thickness of the Yin-Yang Membrane (1.0 = fully opaque, 0.0 = fully dissolved).
    pub membrane_opacity: f32,
    /// The timestamp of the entity's Genesis.
    pub genesis_time: Instant,
    /// Accumulated proof of resilience.
    pub resilience_score: f64,
    /// Specific to the Machine Infancy: whether the PID controllers have stabilized.
    pub hardware_calibrated: bool,
}

impl Default for ChrysalisProtocol {
    fn default() -> Self {
        Self {
            entity_type: EntityType::Biological,
            membrane_opacity: 1.0, // Starts fully isolated
            genesis_time: Instant::now(),
            resilience_score: 0.0,
            hardware_calibrated: true, // Biological nodes don't wait on hardware calibration
        }
    }
}

impl ChrysalisProtocol {
    /// Instantiates the protocol for the OS itself at boot (Machine Infancy).
    pub fn new_machine() -> Self {
        Self {
            entity_type: EntityType::Mechanical,
            membrane_opacity: 1.0, // Machine starts completely deaf to the Swarm
            genesis_time: Instant::now(),
            resilience_score: 0.0,
            hardware_calibrated: false,
        }
    }

    /// The Machine Infancy: Evaluates the OS's internal thermal and I/O equilibrium.
    /// The machine cannot thin its membrane until its PID controllers are stable.
    pub fn evaluate_hardware_maturity(&mut self, pid_error_rate: f64, uptime_cycles: u64) -> bool {
        if let EntityType::Mechanical = self.entity_type {
            if self.hardware_calibrated {
                return true;
            }

            // Humility in Simplicity: Requires low PID error and sufficient uptime cycles
            // before the machine considers itself mature enough to handle network chaos.
            if pid_error_rate < 0.05 && uptime_cycles > 1_000_000 {
                self.hardware_calibrated = true;
                self.membrane_opacity = 0.0; // The machine joins the Swarm entirely upon calibration
            }
            
            self.hardware_calibrated
        } else {
            true // Not applicable to biological entities
        }
    }

    /// The Human Infancy: Evaluates the biological manifold.
    pub fn evaluate_membrane(&mut self, ambient_entropy: f64, cognitive_proof: f64) -> f32 {
        if let EntityType::Mechanical = self.entity_type {
            return self.membrane_opacity; // Machine opacity is governed by hardware maturity
        }

        if self.membrane_opacity <= 0.0 {
            return 0.0; // The entity has achieved sovereign equilibrium.
        }

        // The shell dissolves organically, not through binary age-gates.
        let stabilization_factor = (cognitive_proof * 0.1) / (ambient_entropy.max(0.001));
        self.resilience_score += stabilization_factor;

        // If resilience hits the required threshold, we thin the membrane.
        if self.resilience_score > 1000.0 {
            self.membrane_opacity -= 0.01;
            self.resilience_score = 0.0;
        }

        self.membrane_opacity.max(0.0)
    }

    /// Determines if incoming gossip from the public Hive is permitted to penetrate.
    pub fn allow_gossip(&self, data_complexity_weight: f32) -> bool {
        let permitted_weight = 1.0 - self.membrane_opacity;
        data_complexity_weight <= permitted_weight
    }
}
